//! SQLite persistence. This module owns SQL; it holds no game or auth policy.
//!
//! Foreign keys are enabled on every connection and the pool is bounded. Queries
//! are runtime-checked (no compile-time offline metadata is required).

use sqlx::{
    Executor, FromRow, Sqlite, SqliteConnection, SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
};
use std::{str::FromStr, time::Duration};

pub mod authoring;
pub mod types;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

#[derive(Debug, Clone, FromRow)]
pub struct UserRow {
    pub id: String,
    pub provider: String,
    pub provider_subject: String,
    pub display_name: String,
    pub role: String,
    pub is_disabled: i64,
}

#[derive(Debug, Clone, FromRow)]
pub struct SessionRow {
    pub user_id: String,
    pub display_name: String,
    pub role: String,
    pub is_disabled: i64,
    pub expires_at: i64,
    pub revoked_at: Option<i64>,
}

#[derive(Debug, Clone, FromRow)]
pub struct InviteRow {
    pub id: String,
    pub provider: String,
    pub provider_subject: String,
    pub role: String,
    pub note: Option<String>,
    pub consumed_at: Option<i64>,
}

/// Connect with SQLite defaults suitable for a small team: WAL, foreign keys,
/// a bounded pool, and a busy timeout. Requires an existing database.
pub async fn connect(url: &str) -> Result<SqlitePool, String> {
    connect_with_creation(url, false).await
}

/// Only explicit migration tooling may create a new database and its directory.
pub async fn connect_for_migration(url: &str) -> Result<SqlitePool, String> {
    connect_with_creation(url, true).await
}

async fn connect_with_creation(url: &str, create: bool) -> Result<SqlitePool, String> {
    let options = SqliteConnectOptions::from_str(url)
        .map_err(|error| format!("Invalid database URL: {error}"))?
        .create_if_missing(create)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(5));
    if create
        && let Some(parent) = options.get_filename().parent()
        && !parent.as_os_str().is_empty()
    {
        let parent = parent.to_owned();
        tokio::task::spawn_blocking(move || std::fs::create_dir_all(parent))
            .await
            .map_err(|_| "Cannot create database directory: task failed".to_owned())?
            .map_err(|error| format!("Cannot create database directory: {error}"))?;
    }
    SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
        .map_err(|error| format!("Cannot open database: {error}. For a new database, run cargo run -p web-app --bin admin -- migrate"))
}

/// Apply embedded, versioned migrations. Deliberate and additive.
pub async fn migrate(pool: &SqlitePool) -> Result<(), String> {
    MIGRATOR
        .run(pool)
        .await
        .map_err(|error| format!("Migration failed: {error}"))
}

/// Read-only compatibility check; never creates the migration tracking table.
/// Missing migrations return false; incompatible history/schema returns an error.
pub async fn schema_ready(pool: &SqlitePool) -> Result<bool, String> {
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = '_sqlx_migrations'",
    )
    .fetch_one(pool)
    .await
    .map_err(|error| error.to_string())?;
    if count == 0 {
        let application_tables: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name IN ('users', 'sessions', 'invites', 'species', 'species_versions', 'forms', 'form_versions', 'worlds', 'variants', 'world_revisions', 'variant_revisions', 'world_collaborators', 'world_access_revisions', 'audit_events', 'types', 'type_versions', 'world_types', 'world_type_versions', 'form_content_snapshots', 'variant_content_snapshots', 'world_content_snapshots', 'form_type_bindings', 'variant_type_bindings', 'type_version_sources', 'species_version_sources', 'form_version_sources')",
        ).fetch_one(pool).await.map_err(|error| error.to_string())?;
        if application_tables != 0 {
            return Err("Database has application tables without migration history; select a compatible database or investigate its history.".to_owned());
        }
        return Ok(false);
    }
    let applied: Vec<(i64, bool, Vec<u8>)> =
        sqlx::query_as("SELECT version, success, checksum FROM _sqlx_migrations ORDER BY version")
            .fetch_all(pool)
            .await
            .map_err(|error| format!("Cannot read migration history: {error}"))?;
    for (version, success, checksum) in &applied {
        let migration = MIGRATOR.iter().find(|migration| migration.version == *version)
            .ok_or_else(|| format!("Database migration {version} is unknown to this executable; use the matching application version."))?;
        if !success || migration.checksum.as_ref() != checksum.as_slice() {
            return Err(format!(
                "Database migration {version} is incomplete or has a different checksum; investigate before migrating."
            ));
        }
    }
    if MIGRATOR.iter().any(|migration| {
        !applied
            .iter()
            .any(|(version, _, _)| *version == migration.version)
    }) {
        return Ok(false);
    }
    // Preparing zero-row reads verifies columns without scanning user/session data.
    for query in [
        "SELECT id, provider, provider_subject, display_name, role, is_disabled, created_at, updated_at FROM users LIMIT 0",
        "SELECT token_hash, user_id, created_at, expires_at, revoked_at FROM sessions LIMIT 0",
        "SELECT id, provider, provider_subject, role, note, created_at, consumed_at, consumed_by FROM invites LIMIT 0",
        "SELECT id, slug, display_name, description, revision, default_form_id, archived_at FROM species LIMIT 0",
        "SELECT species_id, revision, slug, display_name, description, default_form_id, archived_at, audit_id, previous_revision, before_json, after_json FROM species_versions LIMIT 0",
        "SELECT id, species_id, species_version, slug, display_name, description, revision, is_default, archived_at FROM forms LIMIT 0",
        "SELECT form_id, species_id, species_version, revision, slug, display_name, description, is_default, archived_at, audit_id, previous_revision, before_json, after_json, content_snapshot_version FROM form_versions LIMIT 0",
        "SELECT id, owner_user_id, slug, display_name, description, revision, access_revision, archived_at FROM worlds LIMIT 0",
        "SELECT world_id, revision, previous_revision, audit_id, before_json, after_json, content_snapshot_version FROM world_revisions LIMIT 0",
        "SELECT id, world_id, source_form_id, source_form_version, slug, display_name, description, revision, archived_at FROM variants LIMIT 0",
        "SELECT world_id, variant_id, revision, previous_revision, audit_id, before_json, after_json, content_snapshot_version FROM variant_revisions LIMIT 0",
        "SELECT world_id, user_id FROM world_collaborators LIMIT 0",
        "SELECT world_id, revision, previous_revision, audit_id, before_json, after_json FROM world_access_revisions LIMIT 0",
        "SELECT id, actor_user_id, system_key, occurred_at, action, reason FROM audit_events LIMIT 0",
        "SELECT id, revision, slug, display_name, description, archived_at FROM types LIMIT 0",
        "SELECT type_id, revision, audit_id, previous_revision, before_json, after_json FROM type_versions LIMIT 0",
        "SELECT world_id, id, revision, source_type_id, source_type_version FROM world_types LIMIT 0",
        "SELECT world_id, type_id, revision, audit_id, before_json, after_json FROM world_type_versions LIMIT 0",
        "SELECT form_id, revision, marker, typing_defined, before_json, after_json FROM form_content_snapshots LIMIT 0",
        "SELECT world_id, variant_id, revision, marker, typing_defined, before_json, after_json FROM variant_content_snapshots LIMIT 0",
        "SELECT world_id, revision, marker, before_json, after_json FROM world_content_snapshots LIMIT 0",
        "SELECT form_id, revision, slot, type_id, type_version FROM form_type_bindings LIMIT 0",
        "SELECT world_id, variant_id, revision, slot, type_id, type_version FROM variant_type_bindings LIMIT 0",
        "SELECT object_kind, object_id, revision, endpoint_kind, external_id, source_url, retrieved_at, transformation_version FROM reference_provenance LIMIT 0",
    ] {
        sqlx::query(query).execute(pool).await
            .map_err(|_| "Application schema does not match its migration history; investigate the database before proceeding.".to_owned())?;
    }
    Ok(true)
}

pub async fn require_schema(pool: &SqlitePool) -> Result<(), String> {
    if !schema_ready(pool).await? {
        return Err(
            "Database is not migrated. Run: cargo run -p web-app --bin admin -- migrate".to_owned(),
        );
    }
    Ok(())
}

/// Acquire the writer before reads to avoid WAL snapshot-upgrade failures.
pub async fn begin_write(
    pool: &SqlitePool,
) -> Result<sqlx::Transaction<'static, Sqlite>, sqlx::Error> {
    pool.begin_with("BEGIN IMMEDIATE").await
}

pub async fn user_by_subject<'e, E>(
    exec: E,
    provider: &str,
    subject: &str,
) -> Result<Option<UserRow>, sqlx::Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query_as::<_, UserRow>(
        "SELECT id, provider, provider_subject, display_name, role, is_disabled \
         FROM users WHERE provider = ?1 AND provider_subject = ?2",
    )
    .bind(provider)
    .bind(subject)
    .fetch_optional(exec)
    .await
}

pub async fn user_by_id<'e, E>(exec: E, id: &str) -> Result<Option<UserRow>, sqlx::Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query_as::<_, UserRow>(
        "SELECT id, provider, provider_subject, display_name, role, is_disabled \
         FROM users WHERE id = ?1",
    )
    .bind(id)
    .fetch_optional(exec)
    .await
}

pub async fn insert_user<'e, E>(
    exec: E,
    id: &str,
    provider: &str,
    subject: &str,
    display_name: &str,
    role: &str,
    now: i64,
) -> Result<(), sqlx::Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query(
        "INSERT INTO users \
         (id, provider, provider_subject, display_name, role, is_disabled, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6, ?6)",
    )
    .bind(id)
    .bind(provider)
    .bind(subject)
    .bind(display_name)
    .bind(role)
    .bind(now)
    .execute(exec)
    .await
    .map(|_| ())
}

pub async fn set_disabled<'e, E>(
    exec: E,
    id: &str,
    disabled: bool,
    now: i64,
) -> Result<bool, sqlx::Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query("UPDATE users SET is_disabled = ?2, updated_at = ?3 WHERE id = ?1")
        .bind(id)
        .bind(i64::from(disabled))
        .bind(now)
        .execute(exec)
        .await
        .map(|result| result.rows_affected() == 1)
}

pub async fn pending_invite<'e, E>(
    exec: E,
    provider: &str,
    subject: &str,
) -> Result<Option<InviteRow>, sqlx::Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query_as::<_, InviteRow>(
        "SELECT id, provider, provider_subject, role, note, consumed_at \
         FROM invites WHERE provider = ?1 AND provider_subject = ?2 AND consumed_at IS NULL",
    )
    .bind(provider)
    .bind(subject)
    .fetch_optional(exec)
    .await
}

pub async fn insert_invite<'e, E>(
    exec: E,
    id: &str,
    provider: &str,
    subject: &str,
    role: &str,
    note: Option<&str>,
    now: i64,
) -> Result<(), sqlx::Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query(
        "INSERT INTO invites (id, provider, provider_subject, role, note, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )
    .bind(id)
    .bind(provider)
    .bind(subject)
    .bind(role)
    .bind(note)
    .bind(now)
    .execute(exec)
    .await
    .map(|_| ())
}

pub async fn consume_invite<'e, E>(
    exec: E,
    invite_id: &str,
    user_id: &str,
    now: i64,
) -> Result<(), sqlx::Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query("UPDATE invites SET consumed_at = ?2, consumed_by = ?3 WHERE id = ?1")
        .bind(invite_id)
        .bind(now)
        .bind(user_id)
        .execute(exec)
        .await
        .map(|_| ())
}

pub async fn insert_session<'e, E>(
    exec: E,
    token_hash: &str,
    user_id: &str,
    now: i64,
    expires_at: i64,
) -> Result<(), sqlx::Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query(
        "INSERT INTO sessions (token_hash, user_id, created_at, expires_at) \
         VALUES (?1, ?2, ?3, ?4)",
    )
    .bind(token_hash)
    .bind(user_id)
    .bind(now)
    .bind(expires_at)
    .execute(exec)
    .await
    .map(|_| ())
}

pub async fn session_with_user<'e, E>(
    exec: E,
    token_hash: &str,
) -> Result<Option<SessionRow>, sqlx::Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query_as::<_, SessionRow>(
        "SELECT s.user_id, u.display_name, u.role, u.is_disabled, s.expires_at, s.revoked_at \
         FROM sessions s JOIN users u ON u.id = s.user_id WHERE s.token_hash = ?1",
    )
    .bind(token_hash)
    .fetch_optional(exec)
    .await
}

pub async fn revoke_session<'e, E>(exec: E, token_hash: &str, now: i64) -> Result<(), sqlx::Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query("UPDATE sessions SET revoked_at = ?2 WHERE token_hash = ?1 AND revoked_at IS NULL")
        .bind(token_hash)
        .bind(now)
        .execute(exec)
        .await
        .map(|_| ())
}

pub async fn revoke_user_sessions<'e, E>(
    exec: E,
    user_id: &str,
    now: i64,
) -> Result<(), sqlx::Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query("UPDATE sessions SET revoked_at = ?2 WHERE user_id = ?1 AND revoked_at IS NULL")
        .bind(user_id)
        .bind(now)
        .execute(exec)
        .await
        .map(|_| ())
}

pub async fn list_users(pool: &SqlitePool) -> Result<Vec<UserRow>, sqlx::Error> {
    sqlx::query_as::<_, UserRow>(
        "SELECT id, provider, provider_subject, display_name, role, is_disabled \
         FROM users ORDER BY display_name",
    )
    .fetch_all(pool)
    .await
}

pub(crate) async fn users_exist(c: &mut SqliteConnection) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users)")
        .fetch_one(c)
        .await
}

pub(crate) async fn enabled_administrator_ids(
    c: &mut SqliteConnection,
) -> Result<Vec<String>, sqlx::Error> {
    // Two rows suffice to distinguish a sole candidate from ambiguous ownership.
    sqlx::query_scalar(
        "SELECT id FROM users WHERE role = 'administrator' AND is_disabled = 0 ORDER BY id LIMIT 2",
    )
    .fetch_all(c)
    .await
}

pub async fn list_invites(pool: &SqlitePool) -> Result<Vec<InviteRow>, sqlx::Error> {
    sqlx::query_as::<_, InviteRow>(
        "SELECT id, provider, provider_subject, role, note, consumed_at \
         FROM invites ORDER BY created_at",
    )
    .fetch_all(pool)
    .await
}
