//! Concrete authoring queries; all writes use the caller's transaction connection.
use crate::application::authoring::{
    self as app, AddressKey, Collaborator, Error, FormRecord, HistoryEntry, HistoryKind, SourcePin,
    SpeciesRecord, VariantRecord, WorldRecord,
};
use game_data::authoring::{ContentFields, FieldErrors};
use serde::{Deserialize, de::DeserializeOwned};
use sqlx::{FromRow, Row, SqliteConnection, sqlite::SqliteRow};

pub(crate) fn storage_error(error: sqlx::Error) -> Error {
    // SQLx messages can contain submitted values: log only fixed categories/codes.
    match error {
        sqlx::Error::Database(error) => eprintln!(
            "Authoring storage failure: database code {}",
            error.code().as_deref().unwrap_or("unknown")
        ),
        sqlx::Error::PoolClosed => eprintln!("Authoring storage failure: pool closed"),
        sqlx::Error::PoolTimedOut => eprintln!("Authoring storage failure: pool timeout"),
        _ => eprintln!("Authoring storage failure: query or connection error"),
    }
    Error::Unavailable
}
pub(crate) fn content_write_error(error: sqlx::Error) -> Error {
    if error
        .as_database_error()
        .is_some_and(|e| e.is_unique_violation())
    {
        Error::Invalid(FieldErrors::from([(
            "display_name".into(),
            "This address is already in use in this scope. Choose a different name.".into(),
        )]))
    } else {
        storage_error(error)
    }
}
pub(crate) fn valid_id(id: &str) -> bool {
    id.len() == 32
        && id
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
pub(crate) fn id(row: &SqliteRow, key: &str) -> Result<String, Error> {
    let id: String = row.try_get(key).map_err(storage_error)?;
    if valid_id(&id) {
        Ok(id)
    } else {
        Err(Error::Unavailable)
    }
}
pub(crate) fn revision(row: &SqliteRow, key: &str) -> Result<i64, Error> {
    let value = row.try_get(key).map_err(storage_error)?;
    if value > 0 {
        Ok(value)
    } else {
        Err(Error::Unavailable)
    }
}
pub(crate) fn fields(row: &SqliteRow) -> Result<ContentFields, Error> {
    let fields = ContentFields {
        slug: row.try_get("slug").map_err(storage_error)?,
        display_name: row.try_get("display_name").map_err(storage_error)?,
        description: row.try_get("description").map_err(storage_error)?,
    };
    if fields.normalized().map_err(|_| Error::Unavailable)? == fields {
        Ok(fields)
    } else {
        Err(Error::Unavailable)
    }
}
fn species_row(row: SqliteRow) -> Result<SpeciesRecord, Error> {
    Ok(SpeciesRecord {
        id: id(&row, "id")?,
        fields: fields(&row)?,
        revision: revision(&row, "revision")?,
        default_form_id: id(&row, "default_form_id")?,
        archived_at: row.try_get("archived_at").map_err(storage_error)?,
    })
}
fn form_row(row: SqliteRow) -> Result<FormRecord, Error> {
    let default: i64 = row.try_get("is_default").map_err(storage_error)?;
    if default != 0 && default != 1 {
        return Err(Error::Unavailable);
    }
    Ok(FormRecord {
        id: id(&row, "id")?,
        species_id: id(&row, "species_id")?,
        species_version: revision(&row, "species_version")?,
        fields: fields(&row)?,
        revision: revision(&row, "revision")?,
        is_default: default == 1,
        archived_at: row.try_get("archived_at").map_err(storage_error)?,
    })
}
fn world_row(row: SqliteRow) -> Result<WorldRecord, Error> {
    Ok(WorldRecord {
        id: id(&row, "id")?,
        owner_user_id: id(&row, "owner_user_id")?,
        fields: fields(&row)?,
        revision: revision(&row, "revision")?,
        access_revision: revision(&row, "access_revision")?,
        archived_at: row.try_get("archived_at").map_err(storage_error)?,
    })
}
fn variant_row(row: SqliteRow) -> Result<VariantRecord, Error> {
    let form: Option<String> = row.try_get("source_form_id").map_err(storage_error)?;
    let version: Option<i64> = row.try_get("source_form_version").map_err(storage_error)?;
    let source = match (form, version) {
        (None, None) => None,
        (Some(form_id), Some(version)) if valid_id(&form_id) && version > 0 => {
            Some(SourcePin { form_id, version })
        }
        _ => return Err(Error::Unavailable),
    };
    Ok(VariantRecord {
        id: id(&row, "id")?,
        world_id: id(&row, "world_id")?,
        source,
        fields: fields(&row)?,
        revision: revision(&row, "revision")?,
        archived_at: row.try_get("archived_at").map_err(storage_error)?,
    })
}

pub(crate) async fn species_list(c: &mut SqliteConnection) -> Result<Vec<SpeciesRecord>, Error> {
    sqlx::query("SELECT * FROM species ORDER BY display_name, id")
        .fetch_all(c)
        .await
        .map_err(storage_error)?
        .into_iter()
        .map(species_row)
        .collect()
}

/// Closed scope choices keep SQL identifiers independent of submitted input.
#[derive(Clone, Copy)]
pub(crate) enum SlugScope<'a> {
    Species,
    World,
    Form(&'a str),
    Variant(&'a str),
    Type,
    WorldType(&'a str),
}

pub(crate) async fn slug_exists(
    c: &mut SqliteConnection,
    scope: SlugScope<'_>,
    slug: &str,
    excluded_id: Option<&str>,
) -> Result<bool, Error> {
    let (sql, parent) = match scope {
        SlugScope::Type => (
            "SELECT EXISTS(SELECT 1 FROM types WHERE slug = ?1 AND id <> ?2)",
            None,
        ),
        SlugScope::WorldType(id) => (
            "SELECT EXISTS(SELECT 1 FROM world_types WHERE slug = ?1 AND id <> ?2 AND world_id = ?3)",
            Some(id),
        ),
        SlugScope::Species => (
            "SELECT EXISTS(SELECT 1 FROM species WHERE slug = ?1 AND id <> ?2)",
            None,
        ),
        SlugScope::World => (
            "SELECT EXISTS(SELECT 1 FROM worlds WHERE slug = ?1 AND id <> ?2)",
            None,
        ),
        SlugScope::Form(id) => (
            "SELECT EXISTS(SELECT 1 FROM forms WHERE slug = ?1 AND id <> ?2 AND species_id = ?3)",
            Some(id),
        ),
        SlugScope::Variant(id) => (
            "SELECT EXISTS(SELECT 1 FROM variants WHERE slug = ?1 AND id <> ?2 AND world_id = ?3)",
            Some(id),
        ),
    };
    let query = sqlx::query_scalar(sql)
        .bind(slug)
        .bind(excluded_id.unwrap_or(""));
    let query = if let Some(parent) = parent {
        query.bind(parent)
    } else {
        query
    };
    query.fetch_one(c).await.map_err(storage_error)
}

pub(crate) async fn resolve_species(
    c: &mut SqliteConnection,
    key: AddressKey,
) -> Result<SpeciesRecord, Error> {
    let (sql, value) = match key {
        AddressKey::Id(value) => ("SELECT * FROM species WHERE id = ?", value),
        AddressKey::Slug(value) => ("SELECT * FROM species WHERE slug = ?", value),
    };
    species_row(
        sqlx::query(sql)
            .bind(value)
            .fetch_optional(c)
            .await
            .map_err(storage_error)?
            .ok_or(Error::NotFound)?,
    )
}

pub(crate) async fn resolve_world(
    c: &mut SqliteConnection,
    key: AddressKey,
) -> Result<WorldRecord, Error> {
    let (sql, value) = match key {
        AddressKey::Id(value) => ("SELECT * FROM worlds WHERE id = ?", value),
        AddressKey::Slug(value) => ("SELECT * FROM worlds WHERE slug = ?", value),
    };
    world_row(
        sqlx::query(sql)
            .bind(value)
            .fetch_optional(c)
            .await
            .map_err(storage_error)?
            .ok_or(Error::NotFound)?,
    )
}

pub(crate) async fn resolve_form(
    c: &mut SqliteConnection,
    species_id: &str,
    key: AddressKey,
) -> Result<FormRecord, Error> {
    let (sql, value) = match key {
        AddressKey::Id(value) => ("SELECT * FROM forms WHERE species_id = ? AND id = ?", value),
        AddressKey::Slug(value) => (
            "SELECT * FROM forms WHERE species_id = ? AND slug = ?",
            value,
        ),
    };
    form_row(
        sqlx::query(sql)
            .bind(species_id)
            .bind(value)
            .fetch_optional(c)
            .await
            .map_err(storage_error)?
            .ok_or(Error::NotFound)?,
    )
}

pub(crate) async fn resolve_variant(
    c: &mut SqliteConnection,
    world_id: &str,
    key: AddressKey,
) -> Result<VariantRecord, Error> {
    let (sql, value) = match key {
        AddressKey::Id(value) => (
            "SELECT * FROM variants WHERE world_id = ? AND id = ?",
            value,
        ),
        AddressKey::Slug(value) => (
            "SELECT * FROM variants WHERE world_id = ? AND slug = ?",
            value,
        ),
    };
    variant_row(
        sqlx::query(sql)
            .bind(world_id)
            .bind(value)
            .fetch_optional(c)
            .await
            .map_err(storage_error)?
            .ok_or(Error::NotFound)?,
    )
}
pub(crate) async fn species(c: &mut SqliteConnection, key: &str) -> Result<SpeciesRecord, Error> {
    species_row(
        sqlx::query("SELECT * FROM species WHERE id = ?")
            .bind(key)
            .fetch_optional(c)
            .await
            .map_err(storage_error)?
            .ok_or(Error::NotFound)?,
    )
}
pub(crate) async fn species_exists(c: &mut SqliteConnection, key: &str) -> Result<bool, Error> {
    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM species WHERE id = ?)")
        .bind(key)
        .fetch_one(c)
        .await
        .map_err(storage_error)
}

pub(crate) async fn initial_slug_exists(
    c: &mut SqliteConnection,
    slug: &str,
) -> Result<bool, Error> {
    sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM species_versions WHERE revision = 1 AND slug = ?)",
    )
    .bind(slug)
    .fetch_one(c)
    .await
    .map_err(storage_error)
}
pub(crate) async fn species_version(
    c: &mut SqliteConnection,
    key: &str,
    version: i64,
) -> Result<SpeciesRecord, Error> {
    species_row(sqlx::query("SELECT species_id AS id, slug, display_name, description, revision, default_form_id, archived_at FROM species_versions WHERE species_id = ? AND revision = ?").bind(key).bind(version).fetch_optional(c).await.map_err(storage_error)?.ok_or(Error::NotFound)?)
}
pub(crate) async fn forms(c: &mut SqliteConnection, key: &str) -> Result<Vec<FormRecord>, Error> {
    sqlx::query(
        "SELECT * FROM forms WHERE species_id = ? ORDER BY is_default DESC, display_name, id",
    )
    .bind(key)
    .fetch_all(c)
    .await
    .map_err(storage_error)?
    .into_iter()
    .map(form_row)
    .collect()
}
pub(crate) async fn form(c: &mut SqliteConnection, key: &str) -> Result<FormRecord, Error> {
    form_row(
        sqlx::query("SELECT * FROM forms WHERE id = ?")
            .bind(key)
            .fetch_optional(c)
            .await
            .map_err(storage_error)?
            .ok_or(Error::NotFound)?,
    )
}
pub(crate) async fn form_version(
    c: &mut SqliteConnection,
    key: &str,
    version: i64,
) -> Result<FormRecord, Error> {
    let row = form_version_projection(c, key, version).await?;
    super::types::full_content(c, HistoryKind::Form, key, None, version).await?;
    Ok(row)
}
pub(crate) async fn form_version_projection(
    c: &mut SqliteConnection,
    key: &str,
    version: i64,
) -> Result<FormRecord, Error> {
    form_row(sqlx::query("SELECT form_id AS id, species_id, species_version, slug, display_name, description, revision, is_default, archived_at FROM form_versions WHERE form_id = ? AND revision = ?").bind(key).bind(version).fetch_optional(c).await.map_err(storage_error)?.ok_or(Error::NotFound)?)
}
pub(crate) async fn worlds(c: &mut SqliteConnection) -> Result<Vec<WorldRecord>, Error> {
    sqlx::query("SELECT * FROM worlds ORDER BY display_name, id")
        .fetch_all(c)
        .await
        .map_err(storage_error)?
        .into_iter()
        .map(world_row)
        .collect()
}
pub(crate) async fn world(c: &mut SqliteConnection, key: &str) -> Result<WorldRecord, Error> {
    world_row(
        sqlx::query("SELECT * FROM worlds WHERE id = ?")
            .bind(key)
            .fetch_optional(c)
            .await
            .map_err(storage_error)?
            .ok_or(Error::NotFound)?,
    )
}
pub(crate) async fn variants(
    c: &mut SqliteConnection,
    world_id: &str,
) -> Result<Vec<VariantRecord>, Error> {
    sqlx::query("SELECT * FROM variants WHERE world_id = ? ORDER BY display_name, id")
        .bind(world_id)
        .fetch_all(c)
        .await
        .map_err(storage_error)?
        .into_iter()
        .map(variant_row)
        .collect()
}
pub(crate) async fn variant(
    c: &mut SqliteConnection,
    world_id: &str,
    key: &str,
) -> Result<VariantRecord, Error> {
    variant_row(
        sqlx::query("SELECT * FROM variants WHERE world_id = ? AND id = ?")
            .bind(world_id)
            .bind(key)
            .fetch_optional(c)
            .await
            .map_err(storage_error)?
            .ok_or(Error::NotFound)?,
    )
}
pub(crate) async fn variant_ids(
    c: &mut SqliteConnection,
    world_id: &str,
) -> Result<Vec<String>, Error> {
    sqlx::query_scalar("SELECT id FROM variants WHERE world_id = ? ORDER BY id")
        .bind(world_id)
        .fetch_all(c)
        .await
        .map_err(storage_error)
}
pub(crate) async fn availability(
    c: &mut SqliteConnection,
    species_id: &str,
) -> Result<Vec<WorldRecord>, Error> {
    sqlx::query("SELECT w.* FROM worlds w WHERE EXISTS (SELECT 1 FROM variants v JOIN form_versions f ON f.form_id = v.source_form_id AND f.revision = v.source_form_version WHERE v.world_id = w.id AND f.species_id = ?) ORDER BY w.display_name, w.id").bind(species_id).fetch_all(c).await.map_err(storage_error)?.into_iter().map(world_row).collect()
}
pub(crate) async fn is_collaborator(
    c: &mut SqliteConnection,
    world_id: &str,
    user_id: &str,
) -> Result<bool, Error> {
    sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM world_collaborators WHERE world_id = ? AND user_id = ?)",
    )
    .bind(world_id)
    .bind(user_id)
    .fetch_one(c)
    .await
    .map_err(storage_error)
}
pub(crate) async fn collaborators(
    c: &mut SqliteConnection,
    world_id: &str,
) -> Result<Vec<Collaborator>, Error> {
    let rows: Vec<Collaborator> = sqlx::query_as("SELECT u.id AS user_id, u.display_name FROM world_collaborators g JOIN users u ON u.id = g.user_id WHERE g.world_id = ? ORDER BY u.display_name, u.id").bind(world_id).fetch_all(c).await.map_err(storage_error)?;
    if rows.iter().any(|r| {
        !valid_id(&r.user_id)
            || r.display_name.is_empty()
            || r.display_name.chars().any(char::is_control)
    }) {
        return Err(Error::Unavailable);
    }
    Ok(rows)
}
pub(crate) async fn collaborator_ids(
    c: &mut SqliteConnection,
    world_id: &str,
) -> Result<Vec<String>, Error> {
    sqlx::query_scalar(
        "SELECT user_id FROM world_collaborators WHERE world_id = ? ORDER BY user_id",
    )
    .bind(world_id)
    .fetch_all(c)
    .await
    .map_err(storage_error)
}
pub(crate) async fn set_collaborator(
    c: &mut SqliteConnection,
    world_id: &str,
    user_id: &str,
    grant: bool,
) -> Result<(), Error> {
    let sql = if grant {
        "INSERT INTO world_collaborators(world_id, user_id) VALUES (?, ?)"
    } else {
        "DELETE FROM world_collaborators WHERE world_id = ? AND user_id = ?"
    };
    sqlx::query(sql)
        .bind(world_id)
        .bind(user_id)
        .execute(c)
        .await
        .map_err(storage_error)?;
    Ok(())
}

/// Audit and snapshot values supplied by the use case; actor None is operator seeding only.
pub(crate) struct Revision<'a> {
    pub(crate) actor: Option<&'a str>,
    pub(crate) action: &'a str,
    pub(crate) reason: Option<&'a str>,
    pub(crate) before: Option<String>,
    pub(crate) after: String,
}
impl<'a> Revision<'a> {
    pub(crate) fn new(
        actor: Option<&'a str>,
        action: &'a str,
        reason: Option<&'a str>,
        before: Option<String>,
        after: String,
    ) -> Self {
        Self {
            actor,
            action,
            reason,
            before,
            after,
        }
    }
}
pub(crate) async fn audit(c: &mut SqliteConnection, r: &Revision<'_>) -> Result<String, Error> {
    let id = app::new_id()?;
    sqlx::query("INSERT INTO audit_events(id, actor_user_id, system_key, action, reason) VALUES (?, ?, ?, ?, ?)")
        .bind(&id).bind(r.actor).bind(if r.actor.is_none() { Some("bundled-fixture-loader") } else { None }).bind(r.action).bind(r.reason).execute(c).await.map_err(storage_error)?;
    Ok(id)
}
pub(crate) fn previous(revision: i64) -> Option<i64> {
    if revision == 1 {
        None
    } else {
        Some(revision - 1)
    }
}
pub(crate) async fn put_species(
    c: &mut SqliteConnection,
    row: &SpeciesRecord,
    r: Revision<'_>,
) -> Result<(), Error> {
    let sql = if row.revision == 1 {
        "INSERT INTO species(id, slug, display_name, description, revision, default_form_id, archived_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"
    } else {
        "UPDATE species SET slug = ?2, display_name = ?3, description = ?4, revision = ?5, default_form_id = ?6, archived_at = ?7 WHERE id = ?1"
    };
    sqlx::query(sql)
        .bind(&row.id)
        .bind(&row.fields.slug)
        .bind(&row.fields.display_name)
        .bind(&row.fields.description)
        .bind(row.revision)
        .bind(&row.default_form_id)
        .bind(row.archived_at)
        .execute(&mut *c)
        .await
        .map_err(content_write_error)?;
    let audit_id = audit(c, &r).await?;
    sqlx::query("INSERT INTO species_versions(species_id, slug, display_name, description, revision, default_form_id, archived_at, audit_id, previous_revision, before_json, after_json) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(&row.id).bind(&row.fields.slug).bind(&row.fields.display_name).bind(&row.fields.description).bind(row.revision).bind(&row.default_form_id).bind(row.archived_at).bind(audit_id).bind(previous(row.revision)).bind(r.before).bind(r.after).execute(c).await.map_err(storage_error)?;
    Ok(())
}
pub(crate) async fn put_form(
    c: &mut SqliteConnection,
    row: &FormRecord,
    r: Revision<'_>,
) -> Result<(), Error> {
    put_form_typed(c, row, r, None).await
}
pub(crate) async fn put_form_typed(
    c: &mut SqliteConnection,
    row: &FormRecord,
    r: Revision<'_>,
    typing: Option<&crate::application::types::TypingState>,
) -> Result<(), Error> {
    put_form_content(c, row, r, typing, None).await
}
pub(crate) async fn put_form_stats(
    c: &mut SqliteConnection,
    row: &FormRecord,
    r: Revision<'_>,
    stats: &game_data::stats::BaseStats,
) -> Result<(), Error> {
    put_form_content(c, row, r, None, Some(stats)).await
}
async fn put_form_content(
    c: &mut SqliteConnection,
    row: &FormRecord,
    mut r: Revision<'_>,
    typing: Option<&crate::application::types::TypingState>,
    stats: Option<&game_data::stats::BaseStats>,
) -> Result<(), Error> {
    let seal = super::types::prepare_content(
        c,
        HistoryKind::Form,
        &row.id,
        None,
        row.revision,
        &r.after,
        typing,
        stats,
    )
    .await?;
    r.before = seal.projection_before.clone();
    let sql = if row.revision == 1 {
        "INSERT INTO forms(id, species_id, species_version, slug, display_name, description, revision, is_default, archived_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"
    } else {
        "UPDATE forms SET species_id = ?2, species_version = ?3, slug = ?4, display_name = ?5, description = ?6, revision = ?7, is_default = ?8, archived_at = ?9 WHERE id = ?1"
    };
    sqlx::query(sql)
        .bind(&row.id)
        .bind(&row.species_id)
        .bind(row.species_version)
        .bind(&row.fields.slug)
        .bind(&row.fields.display_name)
        .bind(&row.fields.description)
        .bind(row.revision)
        .bind(row.is_default)
        .bind(row.archived_at)
        .execute(&mut *c)
        .await
        .map_err(content_write_error)?;
    let audit_id = audit(c, &r).await?;
    super::types::seal_content(c, HistoryKind::Form, &row.id, None, row.revision, &seal).await?;
    sqlx::query("INSERT INTO form_versions(form_id, species_id, species_version, slug, display_name, description, revision, is_default, archived_at, audit_id, previous_revision, before_json, after_json, content_snapshot_version, stat_snapshot_version) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 2, 3)")
        .bind(&row.id).bind(&row.species_id).bind(row.species_version).bind(&row.fields.slug).bind(&row.fields.display_name).bind(&row.fields.description).bind(row.revision).bind(row.is_default).bind(row.archived_at).bind(audit_id).bind(previous(row.revision)).bind(r.before).bind(r.after).execute(&mut *c).await.map_err(storage_error)?;
    super::cards::seal_native(
        c,
        game_data::cards::CardIdentity {
            namespace: game_data::cards::CardNamespace::Form,
            world_id: None,
            id: row.id.clone(),
        },
        row.revision,
    )
    .await?;
    Ok(())
}
pub(crate) async fn put_world(
    c: &mut SqliteConnection,
    row: &WorldRecord,
    r: Revision<'_>,
) -> Result<(), Error> {
    put_world_head(c, row).await?;
    seal_world_revision(c, row, r).await
}
pub(crate) async fn seal_world_revision(
    c: &mut SqliteConnection,
    row: &WorldRecord,
    mut r: Revision<'_>,
) -> Result<(), Error> {
    let seal = super::types::prepare_content(
        c,
        HistoryKind::World,
        &row.id,
        None,
        row.revision,
        &r.after,
        None,
        None,
    )
    .await?;
    r.before = seal.projection_before.clone();
    let audit_id = audit(c, &r).await?;
    super::types::seal_content(c, HistoryKind::World, &row.id, None, row.revision, &seal).await?;
    sqlx::query("INSERT INTO world_revisions(world_id, revision, previous_revision, audit_id, before_json, after_json, content_snapshot_version) VALUES (?, ?, ?, ?, ?, ?, 2)")
        .bind(&row.id).bind(row.revision).bind(previous(row.revision)).bind(audit_id).bind(r.before).bind(r.after).execute(c).await.map_err(storage_error)?;
    Ok(())
}
/// Stage a head with deferred revision/access FKs; fixtures seal only after all children.
pub(crate) async fn put_world_head(
    c: &mut SqliteConnection,
    row: &WorldRecord,
) -> Result<(), Error> {
    let sql = if row.revision == 1 {
        "INSERT INTO worlds(id, owner_user_id, slug, display_name, description, revision, access_revision, archived_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"
    } else {
        "UPDATE worlds SET owner_user_id = ?2, slug = ?3, display_name = ?4, description = ?5, revision = ?6, access_revision = ?7, archived_at = ?8 WHERE id = ?1"
    };
    sqlx::query(sql)
        .bind(&row.id)
        .bind(&row.owner_user_id)
        .bind(&row.fields.slug)
        .bind(&row.fields.display_name)
        .bind(&row.fields.description)
        .bind(row.revision)
        .bind(row.access_revision)
        .bind(row.archived_at)
        .execute(&mut *c)
        .await
        .map_err(content_write_error)?;
    Ok(())
}
pub(crate) async fn put_variant(
    c: &mut SqliteConnection,
    row: &VariantRecord,
    r: Revision<'_>,
) -> Result<(), Error> {
    put_variant_typed(c, row, r, None).await
}
pub(crate) async fn put_variant_typed(
    c: &mut SqliteConnection,
    row: &VariantRecord,
    r: Revision<'_>,
    typing: Option<&crate::application::types::TypingState>,
) -> Result<(), Error> {
    put_variant_content(c, row, r, typing, None).await
}
pub(crate) async fn put_variant_stats(
    c: &mut SqliteConnection,
    row: &VariantRecord,
    r: Revision<'_>,
    stats: &game_data::stats::BaseStats,
) -> Result<(), Error> {
    put_variant_content(c, row, r, None, Some(stats)).await
}
async fn put_variant_content(
    c: &mut SqliteConnection,
    row: &VariantRecord,
    mut r: Revision<'_>,
    typing: Option<&crate::application::types::TypingState>,
    stats: Option<&game_data::stats::BaseStats>,
) -> Result<(), Error> {
    let seal = super::types::prepare_content(
        c,
        HistoryKind::Variant,
        &row.id,
        Some(&row.world_id),
        row.revision,
        &r.after,
        typing,
        stats,
    )
    .await?;
    r.before = seal.projection_before.clone();
    let sql = if row.revision == 1 {
        "INSERT INTO variants(id, world_id, source_form_id, source_form_version, slug, display_name, description, revision, archived_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"
    } else {
        "UPDATE variants SET world_id = ?2, source_form_id = ?3, source_form_version = ?4, slug = ?5, display_name = ?6, description = ?7, revision = ?8, archived_at = ?9 WHERE id = ?1"
    };
    sqlx::query(sql)
        .bind(&row.id)
        .bind(&row.world_id)
        .bind(row.source.as_ref().map(|p| p.form_id.as_str()))
        .bind(row.source.as_ref().map(|p| p.version))
        .bind(&row.fields.slug)
        .bind(&row.fields.display_name)
        .bind(&row.fields.description)
        .bind(row.revision)
        .bind(row.archived_at)
        .execute(&mut *c)
        .await
        .map_err(content_write_error)?;
    let audit_id = audit(c, &r).await?;
    super::types::seal_content(
        c,
        HistoryKind::Variant,
        &row.id,
        Some(&row.world_id),
        row.revision,
        &seal,
    )
    .await?;
    sqlx::query("INSERT INTO variant_revisions(world_id, variant_id, revision, previous_revision, audit_id, before_json, after_json, content_snapshot_version, stat_snapshot_version) VALUES (?, ?, ?, ?, ?, ?, ?, 2, 3)")
        .bind(&row.world_id).bind(&row.id).bind(row.revision).bind(previous(row.revision)).bind(audit_id).bind(r.before).bind(r.after).execute(&mut *c).await.map_err(storage_error)?;
    super::cards::seal_native(
        c,
        game_data::cards::CardIdentity {
            namespace: game_data::cards::CardNamespace::Variant,
            world_id: Some(row.world_id.clone()),
            id: row.id.clone(),
        },
        row.revision,
    )
    .await?;
    Ok(())
}
pub(crate) async fn put_access_revision(
    c: &mut SqliteConnection,
    row: &WorldRecord,
    r: Revision<'_>,
) -> Result<(), Error> {
    sqlx::query("UPDATE worlds SET access_revision = ? WHERE id = ?")
        .bind(row.access_revision)
        .bind(&row.id)
        .execute(&mut *c)
        .await
        .map_err(storage_error)?;
    let audit_id = audit(c, &r).await?;
    sqlx::query("INSERT INTO world_access_revisions(world_id, revision, previous_revision, audit_id, before_json, after_json) VALUES (?, ?, ?, ?, ?, ?)")
        .bind(&row.id).bind(row.access_revision).bind(previous(row.access_revision)).bind(audit_id).bind(r.before).bind(r.after).execute(c).await.map_err(storage_error)?;
    Ok(())
}

pub(crate) async fn fixture_baseline(
    c: &mut SqliteConnection,
    species_id: &str,
    form_id: &str,
) -> Result<bool, Error> {
    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM species_versions s JOIN audit_events a ON a.id = s.audit_id JOIN form_versions f ON f.form_id = ?2 AND f.revision = 1 JOIN audit_events b ON b.id = f.audit_id WHERE s.species_id = ?1 AND s.revision = 1 AND a.system_key = 'bundled-fixture-loader' AND b.system_key = 'bundled-fixture-loader' AND a.action = 'create' AND b.action = 'create')").bind(species_id).bind(form_id).fetch_one(c).await.map_err(storage_error)
}
// Table/column choices are closed enums, never input-derived identifiers.
pub(crate) async fn history(
    c: &mut SqliteConnection,
    kind: HistoryKind,
    key: &str,
    before: Option<i64>,
    exact: Option<i64>,
) -> Result<Vec<HistoryEntry>, Error> {
    let (table, column, scope) = match kind {
        HistoryKind::Species => ("species_versions", "species_id", "r.default_form_id"),
        HistoryKind::Form => ("form_versions", "form_id", "r.species_id"),
        HistoryKind::World => ("world_revisions", "world_id", "NULL"),
        HistoryKind::Variant => ("variant_revisions", "variant_id", "r.world_id"),
        HistoryKind::Type => ("type_versions", "type_id", "NULL"),
        HistoryKind::WorldType => ("world_type_versions", "type_id", "r.world_id"),
    };
    let sql = format!(
        "SELECT r.revision, r.previous_revision, COALESCE(u.display_name, a.system_key) AS actor_name, a.action, a.occurred_at, a.reason, r.before_json, r.after_json, {scope} AS scope_id FROM {table} r JOIN audit_events a ON a.id = r.audit_id LEFT JOIN users u ON u.id = a.actor_user_id WHERE r.{column} = ?1 AND (?2 IS NULL OR r.revision < ?2) AND (?3 IS NULL OR r.revision = ?3) ORDER BY r.revision DESC LIMIT 50"
    );
    let rows = sqlx::query(sqlx::AssertSqlSafe(sql))
        .bind(key)
        .bind(before)
        .bind(exact)
        .fetch_all(&mut *c)
        .await
        .map_err(storage_error)?;
    let mut entries = validate_history(rows, SnapshotKind::Public(kind), key)?;
    if matches!(
        kind,
        HistoryKind::Form | HistoryKind::Variant | HistoryKind::World
    ) {
        for entry in &mut entries {
            let full = super::types::full_content(c, kind, key, None, entry.revision).await?;
            entry.before_json = full.before;
            entry.after_json = full.after;
        }
    }
    if matches!(kind, HistoryKind::Type | HistoryKind::WorldType) {
        for entry in &entries {
            let after = if matches!(kind, HistoryKind::Type) {
                super::types::type_after(c, key, entry.revision).await?
            } else {
                let value: serde_json::Value =
                    serde_json::from_str(&entry.after_json).map_err(|_| Error::Unavailable)?;
                let world = value["content"]["world_id"]
                    .as_str()
                    .ok_or(Error::Unavailable)?;
                super::types::world_type_after(c, world, key, entry.revision).await?
            };
            if after != entry.after_json {
                return Err(Error::Unavailable);
            }
            if let Some(previous) = entry.previous_revision {
                let previous_after = if matches!(kind, HistoryKind::Type) {
                    super::types::type_after(c, key, previous).await?
                } else {
                    let value: serde_json::Value =
                        serde_json::from_str(&entry.after_json).map_err(|_| Error::Unavailable)?;
                    let world = value["content"]["world_id"]
                        .as_str()
                        .ok_or(Error::Unavailable)?;
                    super::types::world_type_after(c, world, key, previous).await?
                };
                if entry.before_json.as_deref() != Some(previous_after.as_str()) {
                    return Err(Error::Unavailable);
                }
            }
        }
    }
    Ok(entries)
}
pub(crate) async fn access_history(
    c: &mut SqliteConnection,
    key: &str,
) -> Result<Vec<HistoryEntry>, Error> {
    let rows = sqlx::query("SELECT r.revision, r.previous_revision, COALESCE(u.display_name, a.system_key) AS actor_name, a.action, a.occurred_at, a.reason, r.before_json, r.after_json, NULL AS scope_id FROM world_access_revisions r JOIN audit_events a ON a.id = r.audit_id LEFT JOIN users u ON u.id = a.actor_user_id WHERE r.world_id = ? ORDER BY r.revision DESC LIMIT 50").bind(key).fetch_all(c).await.map_err(storage_error)?;
    validate_history(rows, SnapshotKind::Access, key)
}
fn validate_history(
    rows: Vec<SqliteRow>,
    kind: SnapshotKind,
    key: &str,
) -> Result<Vec<HistoryEntry>, Error> {
    let mut entries = Vec::with_capacity(rows.len());
    for stored in rows {
        let row = HistoryEntry::from_row(&stored).map_err(storage_error)?;
        let scope: Option<String> = stored.try_get("scope_id").map_err(storage_error)?;
        if row.revision <= 0
            || row.previous_revision != previous(row.revision)
            || row.before_json.is_some() != row.previous_revision.is_some()
            || row.actor_name.is_empty()
            || row.actor_name.chars().any(char::is_control)
            || !row.occurred_at.ends_with('Z')
            || !row.occurred_at.contains('T')
            || game_data::authoring::validate_reason(row.reason.as_deref()).is_err()
        {
            return Err(Error::Unavailable);
        }
        validate_snapshot(&row.after_json, kind, key, scope.as_deref(), row.revision)?;
        if let (Some(json), Some(revision)) = (&row.before_json, row.previous_revision) {
            validate_snapshot(json, kind, key, scope.as_deref(), revision)?;
        }
        entries.push(row);
    }
    Ok(entries)
}

// These private v1 schemas describe stored history, independently of mutable DTOs.
// Unknown fields are rejected, particularly access data in public world content.
#[derive(Clone, Copy)]
pub(crate) enum SnapshotKind {
    Public(HistoryKind),
    Access,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SnapshotEnvelope<T> {
    snapshot_version: i64,
    content: T,
}

#[derive(Deserialize)]
#[serde(transparent)]
struct ArchivedAt(Option<i64>);

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SpeciesSnapshotV1 {
    id: String,
    fields: ContentFields,
    revision: i64,
    default_form_id: String,
    #[serde(deserialize_with = "ArchivedAt::deserialize")]
    archived_at: ArchivedAt,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FormSnapshotV1 {
    id: String,
    species_id: String,
    species_version: i64,
    fields: ContentFields,
    revision: i64,
    is_default: bool,
    #[serde(deserialize_with = "ArchivedAt::deserialize")]
    archived_at: ArchivedAt,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WorldSnapshotV1 {
    id: String,
    fields: ContentFields,
    revision: i64,
    #[serde(deserialize_with = "ArchivedAt::deserialize")]
    archived_at: ArchivedAt,
    variant_ids: Vec<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceSnapshotV1 {
    form_id: String,
    version: i64,
}
#[derive(Deserialize)]
#[serde(transparent)]
struct NullableSource(Option<SourceSnapshotV1>);
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct VariantSnapshotV1 {
    id: String,
    world_id: String,
    #[serde(deserialize_with = "NullableSource::deserialize")]
    source: NullableSource,
    fields: ContentFields,
    revision: i64,
    #[serde(deserialize_with = "ArchivedAt::deserialize")]
    archived_at: ArchivedAt,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AccessSnapshotV1 {
    world_id: String,
    owner_user_id: String,
    access_revision: i64,
    collaborator_ids: Vec<String>,
}

fn decode_v1<T: DeserializeOwned>(json: &str) -> Result<T, Error> {
    let envelope: SnapshotEnvelope<T> =
        serde_json::from_str(json).map_err(|_| Error::Unavailable)?;
    if envelope.snapshot_version != 1 {
        return Err(Error::Unavailable);
    }
    Ok(envelope.content)
}
fn normalized_fields(fields: &ContentFields) -> bool {
    fields
        .normalized()
        .is_ok_and(|normalized| normalized == *fields)
}
fn snapshot_identity(id: &str, key: &str, revision: i64, expected: i64) -> bool {
    valid_id(id) && id == key && revision > 0 && revision == expected
}
fn ordered_ids(ids: &[String]) -> bool {
    ids.iter().all(|id| valid_id(id)) && ids.windows(2).all(|pair| pair[0] < pair[1])
}
pub(crate) fn validate_snapshot(
    json: &str,
    kind: SnapshotKind,
    key: &str,
    scope: Option<&str>,
    revision: i64,
) -> Result<(), Error> {
    let valid = match kind {
        SnapshotKind::Public(HistoryKind::Type | HistoryKind::WorldType) => {
            super::types::validate_type_snapshot(
                json,
                match kind {
                    SnapshotKind::Public(k) => k,
                    _ => unreachable!(),
                },
                key,
                scope,
                revision,
            )?;
            true
        }
        SnapshotKind::Public(HistoryKind::Species) => {
            let row: SpeciesSnapshotV1 = decode_v1(json)?;
            let _ = row.archived_at.0;
            snapshot_identity(&row.id, key, row.revision, revision)
                && normalized_fields(&row.fields)
                && valid_id(&row.default_form_id)
                && scope == Some(row.default_form_id.as_str())
        }
        SnapshotKind::Public(HistoryKind::Form) => {
            let row: FormSnapshotV1 = decode_v1(json)?;
            let _ = (row.archived_at.0, row.is_default);
            snapshot_identity(&row.id, key, row.revision, revision)
                && normalized_fields(&row.fields)
                && valid_id(&row.species_id)
                && row.species_version > 0
                && scope == Some(row.species_id.as_str())
        }
        SnapshotKind::Public(HistoryKind::World) => {
            let row: WorldSnapshotV1 = decode_v1(json)?;
            let _ = row.archived_at.0;
            snapshot_identity(&row.id, key, row.revision, revision)
                && normalized_fields(&row.fields)
                && ordered_ids(&row.variant_ids)
        }
        SnapshotKind::Public(HistoryKind::Variant) => {
            let row: VariantSnapshotV1 = decode_v1(json)?;
            let _ = row.archived_at.0;
            snapshot_identity(&row.id, key, row.revision, revision)
                && normalized_fields(&row.fields)
                && valid_id(&row.world_id)
                && scope == Some(row.world_id.as_str())
                && row
                    .source
                    .0
                    .is_none_or(|pin| valid_id(&pin.form_id) && pin.version > 0)
        }
        SnapshotKind::Access => {
            let row: AccessSnapshotV1 = decode_v1(json)?;
            snapshot_identity(&row.world_id, key, row.access_revision, revision)
                && valid_id(&row.owner_user_id)
                && ordered_ids(&row.collaborator_ids)
                && !row.collaborator_ids.contains(&row.owner_user_id)
        }
    };
    if valid {
        Ok(())
    } else {
        Err(Error::Unavailable)
    }
}
