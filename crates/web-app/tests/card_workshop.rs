//! Opt-in original Workshop: atomic import, immutable baselines and old-seed isolation.
use game_data::{card_fields::*, cards::*};
use sqlx::{AssertSqlSafe, SqlitePool};
use std::{
    collections::BTreeMap,
    process::{Command, Output},
};
use tempfile::TempDir;
use web_app::{
    accounts::Accounts,
    application::{
        authoring::{AddressKey, Authoring, Edit, HistoryKind},
        card_workshop::*,
        cards::{CardEdit, CardMetadataEdit, Cards, FieldEdit, FieldMapping, NewCard, NewField},
        export::WorldExport,
        inspection::WorldInspection,
        moves::Moves,
    },
    auth::{Actor, Role, Subject},
    persistence,
};

struct Lab {
    _dir: TempDir,
    url: String,
    pool: SqlitePool,
    accounts: Accounts,
}
impl Lab {
    async fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let url = format!("sqlite://{}/workshop.db", dir.path().display());
        let pool = persistence::connect_for_migration(&url).await.unwrap();
        persistence::migrate(&pool).await.unwrap();
        Self {
            accounts: Accounts::new(pool.clone(), 3600).unwrap(),
            pool,
            url,
            _dir: dir,
        }
    }
    fn workshop(&self) -> CardWorkshop {
        CardWorkshop::new(self.accounts.clone())
    }
    fn cards(&self) -> Cards {
        Cards::new(self.accounts.clone())
    }
    async fn user(&self, name: &str, role: Role) -> Actor {
        self.accounts
            .invite("test", name, role, None)
            .await
            .unwrap();
        self.accounts
            .sign_in(Subject {
                provider: "test".into(),
                provider_subject: name.into(),
                display_name: name.into(),
            })
            .await
            .unwrap()
    }
    fn cli(&self, args: &[&str]) -> Output {
        command(&self.url, args).output().unwrap()
    }
    async fn count(&self, table: &str) -> i64 {
        sqlx::query_scalar(AssertSqlSafe(format!("SELECT count(*) FROM {table}")))
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }
    async fn snapshot(&self) -> BTreeMap<String, Vec<String>> {
        let tables: Vec<String> = sqlx::query_scalar("SELECT name FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name").fetch_all(&self.pool).await.unwrap();
        let mut out = BTreeMap::new();
        for table in tables {
            let cols: Vec<String> =
                sqlx::query_scalar("SELECT name FROM pragma_table_info(?) ORDER BY cid")
                    .bind(&table)
                    .fetch_all(&self.pool)
                    .await
                    .unwrap();
            let projection = cols
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
            out.insert(table, rows);
        }
        out
    }
    async fn reject(&self, owner: &str) {
        let before = self.snapshot().await;
        assert!(self.workshop().seed(owner).await.is_err());
        assert_eq!(self.snapshot().await, before);
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
    async fn card(&self, namespace: CardNamespace, id: &str, shared: bool) -> CardRecord {
        self.cards()
            .detail(&identity(namespace, id, shared))
            .await
            .unwrap()
    }
    // Corruption is confined to temporary test databases. Restore all guards before
    // invoking the seed, so a rejection is due to its baseline verification.
    async fn corrupt(&self, statement: &str) {
        let triggers: Vec<(String, String)> =
            sqlx::query_as("SELECT name,sql FROM sqlite_schema WHERE type='trigger'")
                .fetch_all(&self.pool)
                .await
                .unwrap();
        let mut tx = persistence::begin_write(&self.pool).await.unwrap();
        sqlx::query("PRAGMA defer_foreign_keys=ON")
            .execute(&mut *tx)
            .await
            .unwrap();
        for (name, _) in &triggers {
            sqlx::query(AssertSqlSafe(format!("DROP TRIGGER \"{name}\"")))
                .execute(&mut *tx)
                .await
                .unwrap();
        }
        sqlx::raw_sql(AssertSqlSafe(statement.to_owned()))
            .execute(&mut *tx)
            .await
            .unwrap();
        for (_, sql) in triggers {
            sqlx::query(AssertSqlSafe(sql))
                .execute(&mut *tx)
                .await
                .unwrap();
        }
        tx.commit().await.unwrap();
    }
}
fn command(url: &str, args: &[&str]) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_admin"));
    c.args(args)
        .env("KARTA_DATABASE_URL", url)
        .env("KARTA_SESSION_TTL_SECONDS", "3600")
        .env_remove("KARTA_DEV_AUTH");
    c
}
fn success(o: Output) -> String {
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    String::from_utf8(o.stdout).unwrap()
}
fn identity(namespace: CardNamespace, id: &str, shared: bool) -> CardIdentity {
    CardIdentity {
        namespace,
        id: id.into(),
        world_id: if shared { None } else { Some(WORLD_ID.into()) },
    }
}
fn value(row: &CardRecord, id: &str) -> FieldValue {
    row.component
        .fields
        .as_ref()
        .unwrap()
        .iter()
        .find(|f| f.definition.id == id)
        .unwrap()
        .value
        .clone()
}
fn field_edit(d: &FieldDefinition) -> FieldEdit {
    FieldEdit {
        expected_revision: d.revision,
        fields: d.fields.clone(),
        kind: d.kind.clone(),
        archived_at: d.archived_at,
        reason: Some("Keep authored field edits".into()),
    }
}

#[test]
fn cli_help_and_bad_options_are_validated_without_database_access() {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}/absent.db", dir.path().display());
    for args in [vec!["--help"], vec!["seed-card-workshop", "--help"]] {
        let output = success(
            command(&url, &args)
                .env("KARTA_SESSION_TTL_SECONDS", "bad-config")
                .output()
                .unwrap(),
        );
        assert!(output.contains("seed-card-workshop --owner-user-id <id>"));
    }
    for args in [
        vec!["seed-card-workshop"],
        vec!["seed-card-workshop", "--owner-user-id"],
        vec!["seed-card-workshop", "--owner-user-id", "bad"],
        vec![
            "seed-card-workshop",
            "--owner-user-id",
            "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        ],
        vec![
            "seed-card-workshop",
            "--owner-user-id",
            "000000000000000000000000000000000",
        ],
        vec![
            "seed-card-workshop",
            "--owner-user-id",
            WORLD_ID,
            "--owner-user-id",
            WORLD_ID,
        ],
        vec![
            "seed-card-workshop",
            "--owner-user-id",
            WORLD_ID,
            "--admin-subject",
            "admin",
        ],
        vec!["seed-card-workshop", "--owner-user-id", WORLD_ID, "extra"],
        vec!["seed-card-workshop", "--unknown", "value"],
    ] {
        let o = command(&url, &args)
            .env("KARTA_SESSION_TTL_SECONDS", "bad-config")
            .output()
            .unwrap();
        assert!(!o.status.success(), "accepted {args:?}");
        let error = String::from_utf8(o.stderr).unwrap();
        assert!(
            !error.contains("Cannot open database") && !error.contains("KARTA_SESSION"),
            "{error}"
        );
        assert!(o.stdout.is_empty());
    }
    assert!(!dir.path().join("absent.db").exists());
    let o = command(&url, &["seed-card-workshop", "--owner-user-id", WORLD_ID])
        .output()
        .unwrap();
    assert!(!o.status.success());
    assert!(!dir.path().join("absent.db").exists());
}

#[tokio::test]
async fn requires_existing_enabled_eligible_owner_and_migrated_schema_without_account_changes() {
    let l = Lab::new().await;
    l.reject(WORLD_ID).await;
    l.reject("bad").await;
    assert_eq!(l.count("users").await, 0);
    for (name, role, disabled) in [
        ("viewer", Role::Viewer, false),
        ("disabled-editor", Role::Editor, true),
        ("disabled-admin", Role::Administrator, true),
    ] {
        let actor = l.user(name, role).await;
        if disabled {
            l.accounts.set_disabled(&actor.user_id, true).await.unwrap();
        }
        l.reject(&actor.user_id).await;
        let before = l.snapshot().await;
        assert!(
            !l.cli(&["seed-card-workshop", "--owner-user-id", &actor.user_id])
                .status
                .success()
        );
        assert_eq!(l.snapshot().await, before);
    }
    let owner = l.user("owner", Role::Editor).await;
    sqlx::query("DELETE FROM _sqlx_migrations WHERE version=9")
        .execute(&l.pool)
        .await
        .unwrap();
    l.reject(&owner.user_id).await;
    let before = l.snapshot().await;
    assert!(
        !l.cli(&["seed-card-workshop", "--owner-user-id", &owner.user_id])
            .status
            .success()
    );
    assert_eq!(l.snapshot().await, before);
}

#[tokio::test]
async fn exact_original_batch_native_identity_revision_one_and_system_provenance() {
    let l = Lab::new().await;
    let owner = l.user("owner", Role::Editor).await;
    let (token, _) = l.accounts.start_session(&owner.user_id).await.unwrap();
    let before = l.snapshot().await;
    let report = l.workshop().seed(&owner.user_id).await.unwrap();
    assert_eq!(
        report,
        WorkshopReport {
            world_id: WORLD_ID.into(),
            created: 15,
            skipped: 0,
            cards: 7,
            fields: 7
        }
    );
    for (table, count) in [
        ("worlds", 1),
        ("world_revisions", 1),
        ("world_access_revisions", 1),
        ("variants", 1),
        ("variant_revisions", 1),
        ("world_types", 1),
        ("world_type_versions", 1),
        ("moves", 1),
        ("move_versions", 1),
        ("standalone_cards", 4),
        ("standalone_card_versions", 4),
        ("card_components", 7),
        ("card_field_definitions", 7),
        ("card_field_versions", 7),
        ("card_field_options", 3),
        ("audit_events", 16),
        ("species", 0),
        ("forms", 0),
        ("types", 0),
        ("stat_definitions", 0),
        ("stat_rosters", 0),
        ("charts", 0),
        ("move_attribute_definitions", 0),
        ("world_rulesets", 0),
        ("learnsets", 0),
        ("moveset_presets", 0),
        ("world_collaborators", 0),
    ] {
        assert_eq!(l.count(table).await, count, "{table}");
    }
    let after = l.snapshot().await;
    for table in ["users", "invites", "sessions"] {
        assert_eq!(before[table], after[table]);
    }
    let provenance: i64 = sqlx::query_scalar("SELECT count(*) FROM audit_events WHERE actor_user_id IS NULL AND system_key='bundled-fixture-loader' AND reason=? AND action IN ('create','own')").bind(REASON).fetch_one(&l.pool).await.unwrap();
    assert_eq!(provenance, 16);
    let mismatched: i64 = sqlx::query_scalar("SELECT count(*) FROM card_components c JOIN (SELECT 'variant' ns,world_id scope,variant_id id,audit_id FROM variant_revisions UNION ALL SELECT 'type',world_id,type_id,audit_id FROM world_type_versions UNION ALL SELECT 'move',scope,move_id,audit_id FROM move_versions UNION ALL SELECT 'standalone',scope,card_id,audit_id FROM standalone_card_versions) v ON c.namespace=v.ns AND c.scope=v.scope AND c.card_id=v.id WHERE c.audit_id<>v.audit_id OR c.audit_id IS NULL OR c.revision<>1").fetch_one(&l.pool).await.unwrap();
    assert_eq!(mismatched, 0);
    let author = Authoring::new(l.accounts.clone());
    let world = author
        .resolve_world(AddressKey::Slug("card-workshop".into()))
        .await
        .unwrap();
    assert_eq!(
        (
            world.id.as_str(),
            world.owner_user_id.as_str(),
            world.revision,
            world.access_revision
        ),
        (WORLD_ID, owner.user_id.as_str(), 1, 1)
    );
    let native_move = Moves::new(l.accounts.clone())
        .detail(Some(WORLD_ID), STEADY_ASCENT_ID)
        .await
        .unwrap();
    assert!(native_move.attributes.is_none());
    assert!(native_move.mappings.is_empty());
    for (ns, id, name, slug) in [
        (
            CardNamespace::Variant,
            MOSSRUNNER_ID,
            "Mossrunner",
            "mossrunner",
        ),
        (CardNamespace::Type, EMBER_ID, "Ember", "ember"),
        (
            CardNamespace::Move,
            STEADY_ASCENT_ID,
            "Steady Ascent",
            "steady-ascent",
        ),
        (
            CardNamespace::Standalone,
            TRAIL_KIT_ID,
            "Trail Kit",
            "trail-kit",
        ),
        (
            CardNamespace::Standalone,
            FOCUSED_EFFORT_ID,
            "Focused Effort",
            "focused-effort",
        ),
        (
            CardNamespace::Standalone,
            FIELD_WRAP_ID,
            "Field Wrap",
            "field-wrap",
        ),
    ] {
        let row = l
            .cards()
            .resolve_card(Some(WORLD_ID), ns, AddressKey::Slug(slug.into()))
            .await
            .unwrap();
        assert_eq!(
            (
                row.identity.id.as_str(),
                row.fields.display_name.as_str(),
                row.revision,
                row.legacy_unmodeled
            ),
            (id, name, 1, false)
        );
        let h = l.cards().history(&row.identity, None).await.unwrap();
        assert_eq!(h.len(), 1);
        assert_eq!(
            (h[0].actor_name.as_str(), h[0].reason.as_deref()),
            ("bundled-fixture-loader", Some(REASON))
        );
    }
    let tool = l.card(CardNamespace::Standalone, TRAIL_KIT_ID, false).await;
    let shared = l
        .card(CardNamespace::Standalone, SHARED_TRAIL_KIT_ID, true)
        .await;
    assert_eq!(tool.source, Some(shared.pin()));
    assert_eq!(shared.fields.slug, "trail-kit-example");
    assert_eq!(shared.component.rules_text, Some(String::new()));
    assert_eq!(
        value(&tool, ROLL_BONUS_ID),
        FieldValue::Number("3".parse().unwrap())
    );
    assert_eq!(
        value(&shared, SHARED_ROLL_BONUS_ID),
        value(&tool, ROLL_BONUS_ID)
    );
    assert_eq!(value(&tool, OPTIONAL_MARKER_ID), FieldValue::Boolean(false));
    let effort = l
        .card(CardNamespace::Standalone, FOCUSED_EFFORT_ID, false)
        .await;
    assert_eq!(
        value(&effort, ROLL_BONUS_ID),
        FieldValue::Number("5".parse().unwrap())
    );
    let ember = l.card(CardNamespace::Type, EMBER_ID, false).await;
    assert_eq!(
        ember.component.labels,
        Some(vec!["Type".into(), "Attachment".into(), "Resource".into()])
    );
    assert_eq!(value(&ember, ROLL_BONUS_ID), FieldValue::NotApplicable);
    assert_eq!(
        value(&ember, USAGE_NOTES_ID),
        FieldValue::Text(String::new())
    );
    assert_eq!(value(&ember, OPTIONAL_MARKER_ID), FieldValue::Unknown);
    let runner = l.card(CardNamespace::Variant, MOSSRUNNER_ID, false).await;
    assert_eq!(value(&runner, ROLL_BONUS_ID), FieldValue::Missing);
    assert_eq!(
        value(&runner, ASSOCIATED_CARD_ID),
        FieldValue::CardReference(ember.pin())
    );
    let ascent = l.card(CardNamespace::Move, STEADY_ASCENT_ID, false).await;
    assert!(
        ascent
            .component
            .rules_text
            .unwrap()
            .contains("10 + 3 + 5 = 18")
    );
    let defs = l.cards().fields(Some(WORLD_ID), 1).await.unwrap();
    assert_eq!(defs.len(), 6);
    for id in [ROLL_BONUS_ID, TARGET_NUMBER_ID] {
        let d = l.cards().field(Some(WORLD_ID), id).await.unwrap();
        let FieldKind::Number(n) = d.kind else {
            panic!("not numeric")
        };
        assert!(n.whole_only);
        assert!(!n.accepts("1.5".parse().unwrap()));
    }
    assert_eq!(
        l.cards()
            .field(Some(WORLD_ID), ROLL_BONUS_ID)
            .await
            .unwrap()
            .source
            .unwrap()
            .field_id,
        SHARED_ROLL_BONUS_ID
    );
    let choice = l
        .cards()
        .field(Some(WORLD_ID), USE_CONTEXT_ID)
        .await
        .unwrap();
    let FieldKind::Choice { options } = choice.kind else {
        panic!("not choice")
    };
    assert_eq!(
        options.iter().map(|o| o.id.as_str()).collect::<Vec<_>>(),
        [CONTEXT_TRAIL_ID, CONTEXT_ATTACHMENT_ID, CONTEXT_RESOURCE_ID]
    );
    let initial = author
        .history_entry(HistoryKind::World, WORLD_ID, 1)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_str(&initial.after_json).unwrap();
    assert_eq!(json["snapshot_version"], 2);
    assert_eq!(
        json["content"]["variant_ids"],
        serde_json::json!([MOSSRUNNER_ID])
    );
    assert_eq!(json["content"]["type_ids"], serde_json::json!([EMBER_ID]));
    assert_eq!(l.accounts.authenticate(&token).await.unwrap(), Some(owner));
    l.integrity().await;
}

#[tokio::test]
async fn rerun_preserves_every_row_after_metadata_components_archives_grants_and_source_adoption() {
    let l = Lab::new().await;
    let owner = l.user("owner", Role::Administrator).await;
    let collaborator = l.user("collaborator", Role::Editor).await;
    let (token, _) = l.accounts.start_session(&owner.user_id).await.unwrap();
    let (revoked, _) = l.accounts.start_session(&owner.user_id).await.unwrap();
    l.accounts.sign_out(&revoked).await.unwrap();
    l.workshop().seed(&owner.user_id).await.unwrap();
    let cards = l.cards();
    let shared_field = cards.field(None, SHARED_ROLL_BONUS_ID).await.unwrap();
    let mut edit = field_edit(&shared_field);
    edit.fields.display_name = "Authored shared bonus".into();
    cards
        .save_field(&token, None, SHARED_ROLL_BONUS_ID, edit)
        .await
        .unwrap();
    let local_field = cards.field(Some(WORLD_ID), ROLL_BONUS_ID).await.unwrap();
    let adopted = cards
        .adopt_field(&token, WORLD_ID, ROLL_BONUS_ID, 2, field_edit(&local_field))
        .await
        .unwrap();
    assert_eq!(adopted.revision, 2);
    let shared = l
        .card(CardNamespace::Standalone, SHARED_TRAIL_KIT_ID, true)
        .await;
    let mut component = shared.component.clone();
    component.fields.as_mut().unwrap()[0].value = FieldValue::Number("4".parse().unwrap());
    component.rules_text = Some("Authored source prose".into());
    cards
        .save_card(
            &token,
            &shared.identity,
            CardEdit {
                expected_revision: 1,
                component,
                reason: None,
            },
        )
        .await
        .unwrap();
    let local = l.card(CardNamespace::Standalone, TRAIL_KIT_ID, false).await;
    let adopted_card = cards
        .adopt_card(&token, &local.identity, 1, 2, None)
        .await
        .unwrap();
    assert_eq!(
        value(&adopted_card, ROLL_BONUS_ID),
        FieldValue::Number("3".parse().unwrap())
    );
    let preview = cards
        .preview_source_fields(
            &local.identity,
            2,
            &[FieldMapping {
                shared_field_id: SHARED_ROLL_BONUS_ID.into(),
                local_field: adopted.pin(),
            }],
        )
        .await
        .unwrap();
    assert!(!preview.removed.is_empty());
    cards
        .copy_source_fields(&token, &preview, Some("Explicit source mapping"))
        .await
        .unwrap();
    for (ns, id, shared) in [
        (CardNamespace::Variant, MOSSRUNNER_ID, false),
        (CardNamespace::Type, EMBER_ID, false),
        (CardNamespace::Move, STEADY_ASCENT_ID, false),
        (CardNamespace::Standalone, FOCUSED_EFFORT_ID, false),
    ] {
        let row = l.card(ns, id, shared).await;
        let mut component = row.component.clone();
        component.rules_text = Some("Preserve this later rules text.".into());
        cards
            .save_card(
                &token,
                &row.identity,
                CardEdit {
                    expected_revision: row.revision,
                    component,
                    reason: None,
                },
            )
            .await
            .unwrap();
    }
    let wrap = l
        .card(CardNamespace::Standalone, FIELD_WRAP_ID, false)
        .await;
    let mut metadata = wrap.fields.clone();
    metadata.display_name = "Renamed archived supply".into();
    cards
        .save_metadata(
            &token,
            &wrap.identity,
            CardMetadataEdit {
                expected_revision: 1,
                fields: metadata,
                archived_at: Some(1),
                reason: None,
            },
        )
        .await
        .unwrap();
    let notes = cards.field(Some(WORLD_ID), USAGE_NOTES_ID).await.unwrap();
    let mut edit = field_edit(&notes);
    edit.fields.description = "Later field prose".into();
    edit.archived_at = Some(1);
    cards
        .save_field(&token, Some(WORLD_ID), USAGE_NOTES_ID, edit)
        .await
        .unwrap();
    let author = Authoring::new(l.accounts.clone());
    let world = author.world_detail(WORLD_ID).await.unwrap();
    author
        .grant_collaborator(
            &token,
            WORLD_ID,
            1,
            &collaborator.user_id,
            Some("Keep grant"),
        )
        .await
        .unwrap();
    let mut metadata = world.fields;
    metadata.display_name = "Renamed original workshop".into();
    author
        .save_world(
            &token,
            WORLD_ID,
            Edit {
                expected_revision: 1,
                fields: metadata,
                reason: None,
            },
        )
        .await
        .unwrap();
    let before = l.snapshot().await;
    let report = l.workshop().seed(&owner.user_id).await.unwrap();
    assert_eq!(
        (report.created, report.skipped, report.cards, report.fields),
        (0, 15, 7, 7)
    );
    assert_eq!(l.snapshot().await, before);
    assert!(
        success(l.cli(&["seed-card-workshop", "--owner-user-id", &owner.user_id]))
            .contains("skipped 15 matching objects")
    );
    assert_eq!(l.snapshot().await, before);
    assert!(l.accounts.authenticate(&token).await.unwrap().is_some());
    assert!(l.accounts.authenticate(&revoked).await.unwrap().is_none());
    l.integrity().await;
}

#[tokio::test]
async fn wrong_owner_never_transfers_or_modifies_the_workshop() {
    let l = Lab::new().await;
    let owner = l.user("owner", Role::Editor).await;
    let other = l.user("other", Role::Administrator).await;
    l.workshop().seed(&owner.user_id).await.unwrap();
    l.reject(&other.user_id).await;
    l.accounts.set_disabled(&owner.user_id, true).await.unwrap();
    l.reject(&owner.user_id).await;
    l.reject(&other.user_id).await;
}

#[tokio::test]
async fn late_child_world_seal_access_seal_and_suppressed_seal_failures_roll_back_every_row() {
    for fault in [
        "CREATE TRIGGER workshop_fault BEFORE INSERT ON standalone_card_versions WHEN NEW.card_id='c1710000000040008000000000000006' BEGIN SELECT RAISE(ABORT,'last child'); END",
        "CREATE TRIGGER workshop_fault BEFORE INSERT ON world_revisions BEGIN SELECT RAISE(ABORT,'world seal'); END",
        "CREATE TRIGGER workshop_fault BEFORE INSERT ON world_access_revisions BEGIN SELECT RAISE(ABORT,'last access seal'); END",
        "CREATE TRIGGER workshop_fault BEFORE UPDATE ON card_components WHEN NEW.card_id='c1710000000040008000000000000006' BEGIN SELECT RAISE(IGNORE); END",
    ] {
        let l = Lab::new().await;
        let owner = l.user("owner", Role::Editor).await;
        l.accounts.start_session(&owner.user_id).await.unwrap();
        sqlx::query(fault).execute(&l.pool).await.unwrap();
        l.reject(&owner.user_id).await;
        assert_eq!(l.count("audit_events").await, 0);
        assert_eq!(l.count("worlds").await, 0);
        sqlx::query("DROP TRIGGER workshop_fault")
            .execute(&l.pool)
            .await
            .unwrap();
        assert_eq!(l.workshop().seed(&owner.user_id).await.unwrap().created, 15);
        l.integrity().await;
    }
}

#[tokio::test]
async fn creation_provenance_bytes_components_choices_and_membership_corruption_reject() {
    for corruption in [
        "UPDATE audit_events SET reason='another import' WHERE action='own'",
        "UPDATE audit_events SET actor_user_id=(SELECT id FROM users LIMIT 1),system_key=NULL WHERE id=(SELECT audit_id FROM move_versions)",
        "UPDATE audit_events SET reason=NULL WHERE id=(SELECT audit_id FROM card_field_versions LIMIT 1)",
        "UPDATE audit_events SET action='adopt' WHERE id=(SELECT audit_id FROM standalone_card_versions LIMIT 1)",
        "UPDATE standalone_card_versions SET after_json=after_json||' ' WHERE scope=''",
        "UPDATE card_components SET snapshot_json=snapshot_json||' ' WHERE namespace='move'",
        "UPDATE card_component_fields SET boolean_value=1 WHERE state='boolean'",
        "UPDATE card_field_options SET label='Changed choice' WHERE slot=0",
        "UPDATE card_field_versions SET source_revision=NULL,source_id=NULL WHERE scope<>'' AND source_id IS NOT NULL",
        "UPDATE world_content_snapshots SET after_json=json_set(after_json,'$.content.type_ids',json('[]'))",
        "UPDATE world_access_revisions SET after_json=replace(after_json,'\"collaborator_ids\":[]','\"collaborator_ids\":[\"c1700000000040008000000000000001\"]')",
    ] {
        let l = Lab::new().await;
        let owner = l.user("owner", Role::Editor).await;
        l.workshop().seed(&owner.user_id).await.unwrap();
        l.corrupt(corruption).await;
        l.reject(&owner.user_id).await;
    }
}

#[tokio::test]
async fn partial_batch_and_scoped_slug_and_id_collisions_are_rejected_without_suffix_or_repair() {
    // A single shared creation baseline remains a partial batch even without its world.
    for collision in [
        "partial",
        "shared-card-slug",
        "shared-field-slug",
        "world-slug",
        "id",
    ] {
        let l = Lab::new().await;
        let owner = l.user("owner", Role::Administrator).await;
        let (token, _) = l.accounts.start_session(&owner.user_id).await.unwrap();
        let metadata = |name: &str| game_data::authoring::ContentFields {
            slug: "ignored".into(),
            display_name: name.into(),
            description: "Test-owned collision".into(),
        };
        match collision {
            "partial" => {
                let row = l
                    .cards()
                    .create_field(
                        &token,
                        None,
                        NewField {
                            fields: metadata("Partial definition"),
                            kind: FieldKind::Boolean,
                            source: None,
                            reason: None,
                        },
                    )
                    .await
                    .unwrap();
                // Change the temporary database's identity consistently, with guards restored.
                l.corrupt(&format!("UPDATE card_field_definitions SET id='{SHARED_ROLL_BONUS_ID}' WHERE id='{id}'; UPDATE card_field_versions SET field_id='{SHARED_ROLL_BONUS_ID}',after_json=replace(after_json,'{id}','{SHARED_ROLL_BONUS_ID}') WHERE field_id='{id}'", id=row.id)).await;
            }
            "shared-card-slug" => {
                l.cards()
                    .create(
                        &token,
                        None,
                        NewCard {
                            fields: metadata("Trail Kit example"),
                            component: CardComponent::default(),
                            source: None,
                            reason: None,
                        },
                    )
                    .await
                    .unwrap();
            }
            "shared-field-slug" => {
                l.cards()
                    .create_field(
                        &token,
                        None,
                        NewField {
                            fields: metadata("Roll Bonus"),
                            kind: FieldKind::Boolean,
                            source: None,
                            reason: None,
                        },
                    )
                    .await
                    .unwrap();
            }
            "world-slug" => {
                Authoring::new(l.accounts.clone())
                    .create_world(&token, metadata("Card Workshop"), None)
                    .await
                    .unwrap();
            }
            "id" => {
                let world = Authoring::new(l.accounts.clone())
                    .create_world(&token, metadata("Other world"), None)
                    .await
                    .unwrap();
                l.corrupt(&format!("UPDATE worlds SET id='{WORLD_ID}'; UPDATE world_revisions SET world_id='{WORLD_ID}',after_json=replace(after_json,'{id}','{WORLD_ID}'); UPDATE world_content_snapshots SET world_id='{WORLD_ID}',after_json=replace(after_json,'{id}','{WORLD_ID}'); UPDATE world_access_revisions SET world_id='{WORLD_ID}',after_json=replace(after_json,'{id}','{WORLD_ID}')", id=world.id)).await;
            }
            _ => unreachable!(),
        }
        l.reject(&owner.user_id).await;
    }
}

#[tokio::test]
async fn old_commands_keep_their_baselines_and_workshop_stays_explicit_in_both_orders() {
    for workshop_first in [false, true] {
        let l = Lab::new().await;
        let owner = l.user("owner", Role::Administrator).await;
        if workshop_first {
            l.workshop().seed(&owner.user_id).await.unwrap();
        }
        success(l.cli(&["seed-creature-fixtures"]));
        assert_eq!(l.count("species").await, 3);
        assert_eq!(l.count("forms").await, 3);
        assert_eq!(l.count("worlds").await, i64::from(workshop_first));
        success(l.cli(&["seed-reference-fixtures"]));
        success(l.cli(&["seed-world-fixtures", "--owner-user-id", &owner.user_id]));
        success(l.cli(&["seed-fixtures", "--owner-user-id", &owner.user_id]));
        assert_eq!(
            l.count("audit_events").await,
            69 + if workshop_first { 16 } else { 0 }
        );
        if !workshop_first {
            assert_eq!(l.count("standalone_cards").await, 0);
            assert_eq!(l.count("card_field_definitions").await, 0);
            assert_eq!(l.workshop().seed(&owner.user_id).await.unwrap().created, 15);
        }
        assert_eq!(l.count("audit_events").await, 85);
        let before = l.snapshot().await;
        for args in [
            vec!["seed-creature-fixtures"],
            vec!["seed-reference-fixtures"],
            vec!["seed-world-fixtures", "--owner-user-id", &owner.user_id],
            vec!["seed-fixtures", "--owner-user-id", &owner.user_id],
        ] {
            let out = success(l.cli(&args));
            if args[0] == "seed-fixtures" {
                assert!(out.contains("skipped 65 matching objects"), "{out}");
            }
            assert_eq!(l.snapshot().await, before);
        }
        assert_eq!(l.workshop().seed(&owner.user_id).await.unwrap().skipped, 15);
        assert_eq!(l.snapshot().await, before);
        l.integrity().await;
    }
}

#[tokio::test]
async fn public_inspection_and_v2_offline_export_include_full_original_closure_without_writes() {
    let l = Lab::new().await;
    let owner = l.user("owner", Role::Editor).await;
    let (token, _) = l.accounts.start_session(&owner.user_id).await.unwrap();
    l.workshop().seed(&owner.user_id).await.unwrap();
    let before = l.snapshot().await;
    let inspection = WorldInspection::new(l.accounts.clone())
        .inspect(AddressKey::Id(WORLD_ID.into()), false)
        .await
        .unwrap();
    assert!(
        inspection.capture.is_complete(),
        "{:?}",
        inspection.capture.gaps
    );
    assert!(inspection.fingerprint.is_some());
    assert_eq!(inspection.capture.card_data.cards.len(), 7);
    assert_eq!(inspection.capture.card_data.fields.len(), 7);
    let export = WorldExport::new(l.accounts.clone())
        .preview(&token, AddressKey::Slug("card-workshop".into()))
        .await
        .unwrap();
    let offline = game_data::record_package::RecordPackage::from_bytes(&export.bytes).unwrap();
    assert_eq!(offline.manifest.schema_version, 3);
    assert_eq!(offline.content.cards.cards.len(), 7);
    assert_eq!(offline.content.cards.fields.len(), 7);
    let wire = String::from_utf8(export.bytes).unwrap();
    assert!(wire.contains("10 + 3 + 5 = 18"));
    assert!(!wire.contains(&owner.user_id));
    assert!(!wire.contains("bundled-fixture-loader"));
    assert_eq!(l.snapshot().await, before);
    l.integrity().await;
}
