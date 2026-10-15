//! End-to-end operator setup regressions for ADR 0009.
use std::{
    collections::BTreeMap,
    process::{Command, Output, Stdio},
};

use sqlx::{AssertSqlSafe, SqlitePool};
use tempfile::TempDir;
use web_app::{
    accounts::Accounts,
    application::{
        authoring::{Authoring, Edit, HistoryKind},
        types::Types,
    },
    auth::{Actor, Credential, IdentityProvider, Role, placeholder::PlaceholderProvider},
    persistence,
};

struct Lab {
    url: String,
    pool: SqlitePool,
    accounts: Accounts,
    _dir: TempDir,
}

impl Lab {
    async fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let url = format!("sqlite://{}/fixtures.db", dir.path().display());
        success(command(&url, &["migrate"], None).output().unwrap());
        let pool = persistence::connect(&url).await.unwrap();
        Self {
            accounts: Accounts::new(pool.clone(), 3600).unwrap(),
            pool,
            url,
            _dir: dir,
        }
    }

    fn run(&self, args: &[&str], dev_auth: Option<&str>) -> Output {
        command(&self.url, args, dev_auth).output().unwrap()
    }

    fn seed(&self) -> String {
        success(self.run(&["seed-fixtures"], Some("1")))
    }

    async fn user(&self, name: &str, role: Role) -> Actor {
        self.accounts
            .invite("placeholder", name, role, None)
            .await
            .unwrap();
        self.sign_in(name).await
    }

    async fn sign_in(&self, name: &str) -> Actor {
        let subject = PlaceholderProvider::new()
            .authenticate(Credential::DevUsername(name.into()))
            .await
            .unwrap();
        self.accounts.sign_in(subject).await.unwrap()
    }

    async fn count(&self, table: &str) -> i64 {
        sqlx::query_scalar(AssertSqlSafe(format!("SELECT count(*) FROM {table}")))
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }

    async fn consumed_by(&self, invite_id: &str) -> Option<String> {
        sqlx::query_scalar("SELECT consumed_by FROM invites WHERE id=?")
            .bind(invite_id)
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }

    async fn baseline(&self) {
        for (table, expected) in [
            ("species", 4),
            ("forms", 5),
            ("types", 3),
            ("worlds", 2),
            ("world_types", 7),
            ("variants", 5),
            ("audit_events", 28),
        ] {
            assert_eq!(self.count(table).await, expected, "{table}");
        }
        let attributed: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM audit_events WHERE system_key='bundled-fixture-loader' AND actor_user_id IS NULL",
        )
        .fetch_one(&self.pool)
        .await
        .unwrap();
        assert_eq!(attributed, 28);
        self.integrity().await;
    }

    async fn integrity(&self) {
        assert!(
            sqlx::query("PRAGMA foreign_key_check")
                .fetch_all(&self.pool)
                .await
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            sqlx::query_scalar::<_, String>("PRAGMA integrity_check")
                .fetch_one(&self.pool)
                .await
                .unwrap(),
            "ok"
        );
    }

    // Compare values, not just counts: audit bytes, sessions, invitation consumption,
    // provenance, pins and every revision/sidecar must survive a failed setup/rerun.
    async fn snapshot(&self) -> BTreeMap<String, Vec<String>> {
        let tables: Vec<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
        )
        .fetch_all(&self.pool)
        .await
        .unwrap();
        let mut snapshot = BTreeMap::new();
        for table in tables {
            let columns: Vec<String> =
                sqlx::query_scalar("SELECT name FROM pragma_table_info(?) ORDER BY cid")
                    .bind(&table)
                    .fetch_all(&self.pool)
                    .await
                    .unwrap();
            // quote() also represents blobs (migration checksums) and NULL losslessly.
            let projection = columns
                .iter()
                .map(|c| format!("quote(\"{c}\")"))
                .collect::<Vec<_>>()
                .join(",");
            let mut rows: Vec<String> = sqlx::query_scalar(AssertSqlSafe(format!(
                "SELECT json_array({projection}) FROM \"{table}\""
            )))
            .fetch_all(&self.pool)
            .await
            .unwrap();
            rows.sort();
            snapshot.insert(table, rows);
        }
        snapshot
    }

    async fn reject_unchanged(&self, args: &[&str], dev_auth: Option<&str>) -> String {
        let before = self.snapshot().await;
        let result = self.run(args, dev_auth);
        assert!(!result.status.success(), "accepted {args:?}");
        assert!(result.stdout.is_empty(), "printed success before commit");
        let error = String::from_utf8(result.stderr).unwrap();
        assert!(!error.trim().is_empty());
        assert_eq!(self.snapshot().await, before, "failed {args:?}: {error}");
        error
    }
}

fn command(url: &str, args: &[&str], dev_auth: Option<&str>) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_admin"));
    command
        .args(args)
        .env("KARTA_DATABASE_URL", url)
        .env("KARTA_SESSION_TTL_SECONDS", "3600")
        .env_remove("KARTA_DEV_AUTH");
    if let Some(value) = dev_auth {
        command.env("KARTA_DEV_AUTH", value);
    }
    command
}

fn success(output: Output) -> String {
    assert!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

#[tokio::test]
async fn default_bootstrap_first_sign_in_and_repeated_cli_share_one_account() {
    let l = Lab::new().await;
    let output = l.seed();
    assert!(
        output.contains("3 shared Types, 4 species, 5 forms, 2 worlds, 7 world Types, 5 variants")
    );
    assert!(output.contains("Sign in as admin"));
    l.baseline().await;
    let users = l.accounts.users().await.unwrap();
    assert_eq!(users.len(), 1);
    let user = &users[0];
    assert_eq!(
        (&*user.provider, &*user.provider_subject),
        ("placeholder", "admin")
    );
    assert_eq!((&*user.role, user.is_disabled), ("administrator", 0));
    assert_eq!(user.id.len(), 32);
    assert_eq!(l.count("sessions").await, 0);
    let invites = l.accounts.invites().await.unwrap();
    assert_eq!(invites.len(), 1);
    assert_eq!(invites[0].role, "administrator");
    assert!(invites[0].consumed_at.is_some());
    assert_eq!(
        l.consumed_by(&invites[0].id).await.as_deref(),
        Some(user.id.as_str())
    );
    let author = Authoring::new(l.accounts.clone());
    for world in author.worlds().await.unwrap() {
        assert_eq!(world.owner_user_id, user.id);
        let history = author
            .history_entry(HistoryKind::World, &world.id, 1)
            .await
            .unwrap();
        assert_eq!(history.actor_name, "bundled-fixture-loader");
    }

    // Even before first sign-in, another invocation must not bootstrap again.
    let before = l.snapshot().await;
    assert!(l.seed().contains("skipped 26 matching objects"));
    assert_eq!(l.snapshot().await, before);
    let actor = l.sign_in(" ADMIN ").await;
    assert_eq!(actor.user_id, user.id);
    assert_eq!(actor.role, Role::Administrator);
    assert_eq!(
        l.count("sessions").await,
        0,
        "sign_in itself issues no session"
    );
    let (token, _) = l.accounts.start_session(&actor.user_id).await.unwrap();
    let before = l.snapshot().await;
    let output = success(l.run(&["seed-fixtures"], None));
    assert!(output.contains("skipped 26 matching objects"));
    assert!(!output.contains("Created development administrator"));
    assert_eq!(l.snapshot().await, before);
    assert_eq!(l.accounts.authenticate(&token).await.unwrap(), Some(actor));
}

#[tokio::test]
async fn custom_subject_consumes_only_matching_admin_invite_and_is_not_an_account_selector() {
    for dev_auth in ["true", "yes"] {
        let l = Lab::new().await;
        let matching = l
            .accounts
            .invite(
                "placeholder",
                "alice",
                Role::Administrator,
                Some("Keep operator note"),
            )
            .await
            .unwrap();
        l.accounts
            .invite("placeholder", "waiting", Role::Editor, Some("Unrelated"))
            .await
            .unwrap();
        l.accounts
            .invite("other-provider", "alice", Role::Viewer, None)
            .await
            .unwrap();
        let before = l.snapshot().await;
        let output = success(l.run(
            &["seed-fixtures", "--admin-subject", "  ALIce  "],
            Some(dev_auth),
        ));
        assert!(output.contains("Sign in as alice"));
        l.baseline().await;
        let user = l.accounts.users().await.unwrap().remove(0);
        assert_eq!(user.provider_subject, "alice");
        assert_eq!(l.count("sessions").await, 0);
        let invites = l.accounts.invites().await.unwrap();
        assert_eq!(
            invites.len(),
            3,
            "reuse rather than replace the administrator invite"
        );
        let invite = invites.iter().find(|i| i.id == matching).unwrap();
        assert_eq!(invite.note.as_deref(), Some("Keep operator note"));
        assert_eq!(
            l.consumed_by(&invite.id).await.as_deref(),
            Some(user.id.as_str())
        );
        assert!(invite.consumed_at.is_some());
        let after = l.snapshot().await;
        let unrelated = |rows: &[String]| {
            rows.iter()
                .filter(|row| !row.contains(&matching))
                .cloned()
                .collect::<Vec<_>>()
        };
        assert_eq!(unrelated(&after["invites"]), unrelated(&before["invites"]));
        assert_eq!(l.sign_in("alice").await.user_id, user.id);
        success(l.run(&["seed-fixtures", "--admin-subject", "another-admin"], None));
        assert_eq!(
            l.snapshot().await,
            after,
            "existing users make the subject bootstrap-only"
        );
    }
}

#[tokio::test]
async fn custom_subject_without_invite_creates_and_consumes_its_own_admin_invite() {
    let l = Lab::new().await;
    success(l.run(
        &["seed-fixtures", "--admin-subject", "  DeSigner-1 "],
        Some("1"),
    ));
    let actor = l.sign_in("designer-1").await;
    assert_eq!(actor.role, Role::Administrator);
    let invites = l.accounts.invites().await.unwrap();
    assert_eq!(invites.len(), 1);
    assert_eq!(invites[0].provider_subject, "designer-1");
    assert_eq!(
        l.consumed_by(&invites[0].id).await.as_deref(),
        Some(actor.user_id.as_str())
    );
    assert_eq!(l.count("sessions").await, 0);
    l.baseline().await;
}

#[tokio::test]
async fn lower_role_bootstrap_invites_are_not_promoted_or_consumed() {
    for role in [Role::Viewer, Role::Editor] {
        let l = Lab::new().await;
        l.accounts
            .invite("placeholder", "alice", role, None)
            .await
            .unwrap();
        let error = l
            .reject_unchanged(&["seed-fixtures", "--admin-subject", " ALICE "], Some("1"))
            .await;
        assert!(error.contains("invitation"), "{error}");
        assert_eq!(l.count("users").await, 0);
        assert_eq!(l.count("species").await, 0);
    }
}

#[tokio::test]
async fn disabled_dev_auth_and_invalid_options_never_write_an_empty_database() {
    let l = Lab::new().await;
    for flag in [None, Some("0"), Some("false")] {
        let error = l.reject_unchanged(&["seed-fixtures"], flag).await;
        assert!(error.contains("KARTA_DEV_AUTH"), "{error}");
    }
    for args in [
        vec!["seed-fixtures", "--admin-subject", "bad name"],
        vec!["seed-fixtures", "--admin-subject", "   "],
        vec!["seed-fixtures", "--owner-user-id", "invalid"],
        vec![
            "seed-fixtures",
            "--owner-user-id",
            "00000000000000000000000000000000",
            "--admin-subject",
            "admin",
        ],
    ] {
        l.reject_unchanged(&args, Some("1")).await;
    }
    assert_eq!(l.count("users").await, 0);
    assert_eq!(l.count("invites").await, 0);
    assert_eq!(l.count("sessions").await, 0);
}

#[tokio::test]
async fn sole_enabled_administrator_is_selected_without_dev_auth_or_account_changes() {
    let l = Lab::new().await;
    let admin = l.user("existing-admin", Role::Administrator).await;
    let disabled = l.user("disabled-admin", Role::Administrator).await;
    l.accounts
        .set_disabled(&disabled.user_id, true)
        .await
        .unwrap();
    l.user("editor", Role::Editor).await;
    l.accounts
        .invite("placeholder", "admin", Role::Viewer, None)
        .await
        .unwrap();
    let before = l.snapshot().await;
    let output = success(l.run(&["seed-fixtures", "--admin-subject", "admin"], None));
    assert!(!output.contains("Created development administrator"));
    for world in Authoring::new(l.accounts.clone()).worlds().await.unwrap() {
        assert_eq!(world.owner_user_id, admin.user_id);
    }
    let after = l.snapshot().await;
    for table in ["users", "invites", "sessions"] {
        assert_eq!(after[table], before[table], "{table}");
    }
    l.baseline().await;
}

#[tokio::test]
async fn existing_users_never_trigger_bootstrap_promotion_or_reenable() {
    for (role, disabled) in [
        (Role::Viewer, false),
        (Role::Editor, false),
        (Role::Viewer, true),
        (Role::Editor, true),
        (Role::Administrator, true),
    ] {
        let l = Lab::new().await;
        let user = l.user("admin", role).await;
        l.accounts
            .set_disabled(&user.user_id, disabled)
            .await
            .unwrap();
        for args in [
            vec!["seed-fixtures"],
            vec!["seed-fixtures", "--admin-subject", "new-admin"],
        ] {
            let error = l.reject_unchanged(&args, Some("1")).await;
            assert!(error.contains("--owner-user-id"), "{error}");
        }
        assert_eq!(l.count("users").await, 1);
        assert_eq!(l.count("species").await, 0);
    }
}

#[tokio::test]
async fn ambiguous_administrators_require_explicit_owner_and_an_editor_is_eligible() {
    let l = Lab::new().await;
    l.user("first-admin", Role::Administrator).await;
    l.user("second-admin", Role::Administrator).await;
    let editor = l.user("editor", Role::Editor).await;
    let viewer = l.user("viewer", Role::Viewer).await;
    l.reject_unchanged(&["seed-fixtures"], Some("1")).await;
    for id in [viewer.user_id.as_str(), "00000000000000000000000000000000"] {
        l.reject_unchanged(&["seed-fixtures", "--owner-user-id", id], Some("1"))
            .await;
    }
    let before = l.snapshot().await;
    success(l.run(&["seed-fixtures", "--owner-user-id", &editor.user_id], None));
    for world in Authoring::new(l.accounts.clone()).worlds().await.unwrap() {
        assert_eq!(world.owner_user_id, editor.user_id);
    }
    let after = l.snapshot().await;
    for table in ["users", "invites", "sessions"] {
        assert_eq!(after[table], before[table]);
    }
    l.baseline().await;
}

#[tokio::test]
async fn editor_owned_worlds_retain_owner_despite_new_admins_and_reject_transfer_or_disabled_owner()
{
    let l = Lab::new().await;
    let owner = l.user("owner", Role::Editor).await;
    success(l.run(
        &["seed-world-fixtures", "--owner-user-id", &owner.user_id],
        None,
    ));
    let first = l.user("first-admin", Role::Administrator).await;
    l.user("second-admin", Role::Administrator).await;
    // Complete setup may add originals, but existing editor-owned labs win over admins.
    success(l.run(&["seed-fixtures"], None));
    l.baseline().await;
    for world in Authoring::new(l.accounts.clone()).worlds().await.unwrap() {
        assert_eq!(world.owner_user_id, owner.user_id);
    }
    let before = l.snapshot().await;
    assert!(l.seed().contains("skipped 26 matching objects"));
    assert_eq!(l.snapshot().await, before);
    l.reject_unchanged(
        &["seed-fixtures", "--owner-user-id", &first.user_id],
        Some("1"),
    )
    .await;
    l.accounts.set_disabled(&owner.user_id, true).await.unwrap();
    for args in [
        vec!["seed-fixtures"],
        vec!["seed-fixtures", "--owner-user-id", &owner.user_id],
        vec!["seed-fixtures", "--owner-user-id", &first.user_id],
    ] {
        l.reject_unchanged(&args, Some("1")).await;
    }
}

#[tokio::test]
async fn complete_rerun_preserves_edits_pins_archives_grants_audit_and_live_and_revoked_sessions() {
    let l = Lab::new().await;
    l.seed();
    let admin = l.sign_in("admin").await;
    let editor = l.user("collaborator", Role::Editor).await;
    let (token, _) = l.accounts.start_session(&admin.user_id).await.unwrap();
    let (revoked, _) = l.accounts.start_session(&admin.user_id).await.unwrap();
    l.accounts.sign_out(&revoked).await.unwrap();
    let (collaborator_token, _) = l.accounts.start_session(&editor.user_id).await.unwrap();
    let author = Authoring::new(l.accounts.clone());
    let types = Types::new(l.accounts.clone());
    let original = author
        .species()
        .await
        .unwrap()
        .into_iter()
        .find(|s| s.fields.slug == "emberling")
        .unwrap();
    let mut fields = original.fields.clone();
    fields.display_name = "Edited original creature".into();
    let original = author
        .save_species(
            &token,
            &original.id,
            Edit {
                expected_revision: original.revision,
                fields,
                reason: None,
            },
        )
        .await
        .unwrap();
    let form = author.form_detail(&original.default_form_id).await.unwrap();
    let mut fields = form.fields.clone();
    fields.display_name = "Edited original form".into();
    author
        .save_form(
            &token,
            &form.id,
            original.revision,
            Edit {
                expected_revision: form.revision,
                fields,
                reason: None,
            },
        )
        .await
        .unwrap();
    let world = author.worlds().await.unwrap().remove(0);
    author
        .grant_collaborator(
            &token,
            &world.id,
            world.access_revision,
            &editor.user_id,
            Some("Keep access"),
        )
        .await
        .unwrap();
    let mut fields = world.fields.clone();
    fields.display_name = "Designer renamed lab".into();
    author
        .save_world(
            &token,
            &world.id,
            Edit {
                expected_revision: world.revision,
                fields,
                reason: None,
            },
        )
        .await
        .unwrap();
    let variant = author
        .variants(&world.id)
        .await
        .unwrap()
        .into_iter()
        .find(|v| v.source.is_some())
        .unwrap();
    let typing = types
        .variant_typing(&world.id, &variant.id, None)
        .await
        .unwrap();
    let mut fields = variant.fields.clone();
    fields.display_name = "Collaborator edited variant".into();
    let edited = author
        .save_variant(
            &collaborator_token,
            &world.id,
            &variant.id,
            Edit {
                expected_revision: variant.revision,
                fields,
                reason: Some("Keep this edit".into()),
            },
        )
        .await
        .unwrap();
    let shared = types.types().await.unwrap().remove(0);
    let mut fields = shared.fields.clone();
    fields.display_name = "Edited shared reference".into();
    types
        .save_type(
            &token,
            &shared.id,
            Edit {
                expected_revision: shared.revision,
                fields,
                reason: None,
            },
        )
        .await
        .unwrap();
    // Archiving is injected through test-owned SQL, as in the existing Type tests.
    sqlx::query("UPDATE types SET archived_at=1 WHERE id=?")
        .bind(&shared.id)
        .execute(&l.pool)
        .await
        .unwrap();
    let before = l.snapshot().await;
    assert!(l.seed().contains("skipped 26 matching objects"));
    assert_eq!(l.snapshot().await, before);
    assert_eq!(
        author.variant_detail(&world.id, &variant.id).await.unwrap(),
        edited
    );
    assert_eq!(
        types
            .variant_typing(&world.id, &variant.id, None)
            .await
            .unwrap(),
        typing
    );
    assert_eq!(
        author
            .access(&token, &world.id)
            .await
            .unwrap()
            .collaborators
            .len(),
        1
    );
    assert_eq!(l.accounts.authenticate(&token).await.unwrap(), Some(admin));
    assert!(l.accounts.authenticate(&revoked).await.unwrap().is_none());
    assert_eq!(
        l.accounts.authenticate(&collaborator_token).await.unwrap(),
        Some(editor)
    );
    l.integrity().await;
}

#[tokio::test]
async fn second_world_access_failure_rolls_back_every_table_and_bootstrap_invitation_changes() {
    for pending in [false, true] {
        let l = Lab::new().await;
        if pending {
            l.accounts
                .invite(
                    "placeholder",
                    "admin",
                    Role::Administrator,
                    Some("Must remain pending"),
                )
                .await
                .unwrap();
        }
        l.accounts
            .invite("placeholder", "unrelated", Role::Editor, None)
            .await
            .unwrap();
        sqlx::query("CREATE TRIGGER fixture_fault BEFORE INSERT ON world_access_revisions WHEN NEW.world_id='5d2d9dba132840f3acbaa5ed1e2b0b49' BEGIN SELECT RAISE(ABORT,'injected second-world access failure'); END")
            .execute(&l.pool).await.unwrap();
        l.reject_unchanged(&["seed-fixtures"], Some("1")).await;
        assert_eq!(l.count("users").await, 0);
        assert_eq!(l.count("audit_events").await, 0);
        sqlx::query("DROP TRIGGER fixture_fault")
            .execute(&l.pool)
            .await
            .unwrap();
        l.seed();
        l.baseline().await;
        assert_eq!(l.count("users").await, 1);
        assert_eq!(l.count("invites").await, 2);
    }
}

#[tokio::test]
async fn concurrent_complete_cli_invocations_commit_one_bootstrap_and_one_content_baseline() {
    let l = Lab::new().await;
    // Both processes start while a test-owned writer holds the lock. Ownership
    // resolution and fixture creation must share the serialized write transaction.
    let tx = persistence::begin_write(&l.pool).await.unwrap();
    let mut children = Vec::new();
    for _ in 0..2 {
        children.push(
            command(&l.url, &["seed-fixtures"], Some("1"))
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap(),
        );
    }
    tx.rollback().await.unwrap();
    let outputs: Vec<String> = children
        .into_iter()
        .map(|child| success(child.wait_with_output().unwrap()))
        .collect();
    assert_eq!(
        outputs
            .iter()
            .filter(|o| o.contains("Created development administrator"))
            .count(),
        1
    );
    assert_eq!(
        outputs
            .iter()
            .filter(|o| o.contains("skipped 26 matching objects"))
            .count(),
        1
    );
    assert_eq!(l.count("users").await, 1);
    assert_eq!(l.count("invites").await, 1);
    assert_eq!(l.count("sessions").await, 0);
    l.baseline().await;
    let before = l.snapshot().await;
    l.seed();
    assert_eq!(l.snapshot().await, before);
}

#[tokio::test]
async fn targeted_commands_keep_their_scopes_and_complete_setup_accepts_original_or_reference_only()
{
    for (command, species, forms, types) in [
        ("seed-creature-fixtures", 3, 3, 0),
        ("seed-reference-fixtures", 1, 2, 3),
    ] {
        let l = Lab::new().await;
        success(l.run(&[command], Some("1")));
        for (table, count) in [
            ("species", species),
            ("forms", forms),
            ("types", types),
            ("worlds", 0),
            ("users", 0),
            ("invites", 0),
            ("sessions", 0),
        ] {
            assert_eq!(l.count(table).await, count, "{command}: {table}");
        }
        let before = l.snapshot().await;
        success(l.run(&[command], None));
        assert_eq!(l.snapshot().await, before);
        l.seed();
        l.baseline().await;
    }
    let l = Lab::new().await;
    l.reject_unchanged(
        &[
            "seed-world-fixtures",
            "--owner-user-id",
            "00000000000000000000000000000000",
        ],
        Some("1"),
    )
    .await;
    let owner = l.user("owner", Role::Editor).await;
    let before = l.snapshot().await;
    success(l.run(
        &["seed-world-fixtures", "--owner-user-id", &owner.user_id],
        Some("1"),
    ));
    for (table, count) in [
        ("species", 1),
        ("forms", 2),
        ("types", 3),
        ("worlds", 2),
        ("world_types", 7),
        ("variants", 5),
    ] {
        assert_eq!(l.count(table).await, count, "{table}");
    }
    let after = l.snapshot().await;
    for table in ["users", "invites", "sessions"] {
        assert_eq!(after[table], before[table]);
    }
}

// Deliberately create incomplete historical batches in test-owned databases.
// Restore retention triggers and satisfy all FKs before testing the CLI.
async fn remove_fixture_rows(l: &Lab, id: &str, tables: &[(&str, &str)]) {
    let mut tx = persistence::begin_write(&l.pool).await.unwrap();
    sqlx::query("PRAGMA defer_foreign_keys=ON")
        .execute(&mut *tx)
        .await
        .unwrap();
    let triggers: Vec<(String, String)> = sqlx::query_as(
        "SELECT name,sql FROM sqlite_schema WHERE type='trigger' AND upper(sql) LIKE '%BEFORE DELETE%'",
    ).fetch_all(&mut *tx).await.unwrap();
    for (name, _) in &triggers {
        sqlx::query(AssertSqlSafe(format!("DROP TRIGGER \"{name}\"")))
            .execute(&mut *tx)
            .await
            .unwrap();
    }
    for (table, column) in tables {
        sqlx::query(AssertSqlSafe(format!(
            "DELETE FROM {table} WHERE {column}=?"
        )))
        .bind(id)
        .execute(&mut *tx)
        .await
        .unwrap();
    }
    for (_, sql) in triggers {
        sqlx::query(AssertSqlSafe(sql))
            .execute(&mut *tx)
            .await
            .unwrap();
    }
    tx.commit().await.unwrap();
    l.integrity().await;
}

#[tokio::test]
async fn partial_reference_batch_rejects_without_originals_or_bootstrap_leaking() {
    let l = Lab::new().await;
    success(l.run(&["seed-reference-fixtures"], None));
    remove_fixture_rows(
        &l,
        "d4dfc82fd1bb40dbb8ee52947696f24f",
        &[
            ("form_version_sources", "form_id"),
            ("form_type_bindings", "form_id"),
            ("form_content_snapshots", "form_id"),
            ("form_versions", "form_id"),
            ("forms", "id"),
        ],
    )
    .await;
    assert_eq!(l.count("forms").await, 1);
    l.reject_unchanged(&["seed-fixtures"], Some("1")).await;
    assert_eq!(l.count("users").await, 0);
    assert_eq!(l.count("species").await, 1);
}

#[tokio::test]
async fn one_existing_fixture_world_is_rejected_without_repair_or_added_originals() {
    let l = Lab::new().await;
    let owner = l.user("owner", Role::Editor).await;
    success(l.run(
        &["seed-world-fixtures", "--owner-user-id", &owner.user_id],
        None,
    ));
    remove_fixture_rows(
        &l,
        "5d2d9dba132840f3acbaa5ed1e2b0b49",
        &[
            ("variant_type_bindings", "world_id"),
            ("variant_content_snapshots", "world_id"),
            ("variant_revisions", "world_id"),
            ("variants", "world_id"),
            ("world_type_versions", "world_id"),
            ("world_types", "world_id"),
            ("world_content_snapshots", "world_id"),
            ("world_revisions", "world_id"),
            ("world_access_revisions", "world_id"),
            ("world_collaborators", "world_id"),
            ("worlds", "id"),
        ],
    )
    .await;
    assert_eq!(l.count("worlds").await, 1);
    l.reject_unchanged(&["seed-fixtures"], Some("1")).await;
    l.reject_unchanged(
        &["seed-fixtures", "--owner-user-id", &owner.user_id],
        Some("1"),
    )
    .await;
    assert_eq!(l.count("species").await, 1);
}
