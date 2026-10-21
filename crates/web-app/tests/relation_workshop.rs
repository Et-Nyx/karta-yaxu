//! Temporary-database coverage for the independent, opt-in ADR0018 batch.
use game_data::{cards::*, relations::*};
use sqlx::{AssertSqlSafe, SqlitePool};
use std::{
    collections::BTreeMap,
    process::{Command, Output},
};
use tempfile::TempDir;
use web_app::{
    accounts::Accounts,
    application::{
        authoring::{Authoring, Edit},
        card_workshop::CardWorkshop,
        cards::{CardEdit, CardMetadataEdit, Cards},
        relation_workshop::*,
        relations::{CollectionMapping, ColumnMapping, Relations},
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
        let url = format!("sqlite://{}/relations.db", dir.path().display());
        let pool = persistence::connect_for_migration(&url).await.unwrap();
        persistence::migrate(&pool).await.unwrap();
        Self {
            accounts: Accounts::new(pool.clone(), 3600).unwrap(),
            pool,
            url,
            _dir: dir,
        }
    }
    fn workshop(&self) -> RelationWorkshop {
        RelationWorkshop::new(self.accounts.clone())
    }
    fn relations(&self) -> Relations {
        Relations::new(self.accounts.clone())
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
    async fn snapshot(&self) -> BTreeMap<String, Vec<String>> {
        let tables:Vec<String>=sqlx::query_scalar("SELECT name FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name").fetch_all(&self.pool).await.unwrap();
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
    async fn count(&self, table: &str) -> i64 {
        sqlx::query_scalar(AssertSqlSafe(format!("SELECT count(*) FROM {table}")))
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }
    async fn corrupt(&self, sql: &str) {
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
        sqlx::raw_sql(AssertSqlSafe(sql.to_owned()))
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
    async fn card(&self, id: &str, ns: CardNamespace, shared: bool) -> CardRecord {
        self.cards()
            .detail(&identity(id, ns, shared))
            .await
            .unwrap()
    }
}
fn identity(id: &str, namespace: CardNamespace, shared: bool) -> CardIdentity {
    CardIdentity {
        namespace,
        id: id.into(),
        world_id: (!shared).then(|| WORLD_ID.into()),
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
fn entries(r: &CardRelationRecord, index: usize) -> &[RecordEntry] {
    let CollectionValue::Entries(es) = &r.relations.collections.as_ref().unwrap()[index].value
    else {
        panic!("not known")
    };
    es
}

#[test]
fn cli_parser_and_help_precede_config_and_database_access() {
    let d = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}/absent.db", d.path().display());
    for args in [vec!["--help"], vec!["seed-card-relations", "--help"]] {
        assert!(
            success(
                command(&url, &args)
                    .env("KARTA_SESSION_TTL_SECONDS", "invalid")
                    .output()
                    .unwrap()
            )
            .contains("seed-card-relations --owner-user-id <id>")
        );
    }
    for args in [
        vec!["seed-card-relations"],
        vec!["seed-card-relations", "--owner-user-id"],
        vec!["seed-card-relations", "--owner-user-id", "bad"],
        vec![
            "seed-card-relations",
            "--owner-user-id",
            "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        ],
        vec![
            "seed-card-relations",
            "--owner-user-id",
            WORLD_ID,
            "--owner-user-id",
            WORLD_ID,
        ],
        vec![
            "seed-card-relations",
            "--owner-user-id",
            WORLD_ID,
            "--admin-subject",
            "admin",
        ],
        vec!["seed-card-relations", "--owner-user-id", WORLD_ID, "extra"],
    ] {
        let o = command(&url, &args)
            .env("KARTA_SESSION_TTL_SECONDS", "invalid")
            .output()
            .unwrap();
        assert!(!o.status.success());
        assert!(!String::from_utf8_lossy(&o.stderr).contains("KARTA_SESSION"));
    }
    assert!(
        !command(&url, &["seed-card-relations", "--owner-user-id", WORLD_ID])
            .output()
            .unwrap()
            .status
            .success()
    );
    assert!(!d.path().join("absent.db").exists());
}

#[tokio::test]
async fn owner_and_schema_ten_are_required_without_creating_accounts() {
    let l = Lab::new().await;
    l.reject("bad").await;
    l.reject(WORLD_ID).await;
    assert_eq!(l.count("users").await, 0);
    for (name, role, disabled) in [
        ("viewer", Role::Viewer, false),
        ("disabled-editor", Role::Editor, true),
        ("disabled-admin", Role::Administrator, true),
    ] {
        let a = l.user(name, role).await;
        if disabled {
            l.accounts.set_disabled(&a.user_id, true).await.unwrap();
        }
        l.reject(&a.user_id).await;
    }
    let a = l.user("editor", Role::Editor).await;
    sqlx::query("DELETE FROM _sqlx_migrations WHERE version=10")
        .execute(&l.pool)
        .await
        .unwrap();
    l.reject(&a.user_id).await;
}

#[tokio::test]
async fn original_batch_has_exact_native_pins_records_and_one_system_audit_per_creation() {
    let l = Lab::new().await;
    let a = l.user("owner", Role::Editor).await;
    let (token, _) = l.accounts.start_session(&a.user_id).await.unwrap();
    let before = l.snapshot().await;
    let r = l.workshop().seed(&a.user_id).await.unwrap();
    assert_eq!(
        (r.cards, r.fields, r.layouts, r.collections, r.stats),
        (10, 6, 4, 6, 1)
    );
    assert_eq!(
        r.created,
        1 + r.cards + r.fields + r.layouts + r.collections + r.stats
    );
    assert_eq!(r.skipped, 0);
    for (table, count) in [
        ("audit_events", r.created + 1),
        ("card_components", r.cards),
        ("card_relation_versions", r.cards),
        ("record_layouts", r.layouts),
        ("record_collection_definitions", r.collections),
        ("stat_definitions", r.stats),
        ("card_field_definitions", r.fields),
        ("variants", 1),
        ("world_types", 1),
        ("types", 1),
        ("standalone_cards", 7),
        ("worlds", 1),
        ("moves", 0),
        ("forms", 0),
        ("stat_rosters", 0),
    ] {
        assert_eq!(l.count(table).await, count as i64, "{table}");
    }
    let after = l.snapshot().await;
    for t in ["users", "invites", "sessions"] {
        assert_eq!(before[t], after[t]);
    }
    let audits:i64=sqlx::query_scalar("SELECT count(*) FROM audit_events WHERE actor_user_id IS NULL AND system_key='bundled-fixture-loader' AND reason=?").bind(REASON).fetch_one(&l.pool).await.unwrap();
    assert_eq!(audits, (r.created + 1) as i64);
    let mismatched:i64=sqlx::query_scalar("SELECT count(*) FROM card_components c JOIN card_relation_versions r USING(namespace,scope,card_id,revision) WHERE c.audit_id IS NULL OR r.audit_id IS NULL OR c.audit_id<>r.audit_id OR c.revision<>1").fetch_one(&l.pool).await.unwrap();
    assert_eq!(mismatched, 0);
    let relations = l.relations();
    let wind = l
        .card(SECOND_WIND_ID, CardNamespace::Standalone, false)
        .await;
    assert!(wind.component.fields.is_none());
    let costs = relations.card(&wind.pin()).await.unwrap();
    let es = entries(&costs, 0);
    assert_eq!(es.len(), 2);
    assert_eq!(
        es[0].cells[1].value,
        RecordValue::Number("2".parse().unwrap())
    );
    assert_eq!(
        es[1].cells[1].value,
        RecordValue::Number("1".parse().unwrap())
    );
    assert!(es[0].cells.iter().all(|c| c.column_id != CONDITION_COLUMN));
    assert_eq!(es[1].cells[2].value, RecordValue::Text(String::new()));
    let focus = l.card(FOCUS_ID, CardNamespace::Standalone, false).await;
    let produces = relations.card(&focus.pin()).await.unwrap();
    let es = entries(&produces, 0);
    assert_eq!(es[0].cells[0].value, es[1].cells[0].value);
    assert_ne!(es[0].id, es[1].id);
    assert_eq!(
        es[0].cells[1].value,
        RecordValue::Number("0".parse().unwrap())
    );
    assert_eq!(es[1].cells[1].value, RecordValue::Unknown);
    assert_eq!(es[2].cells[0].value, RecordValue::Missing);
    assert_eq!(es[2].cells[1].value, RecordValue::NotApplicable);
    let patient = l.card(PATIENT_ID, CardNamespace::Standalone, false).await;
    let influences = relations.card(&patient.pin()).await.unwrap();
    let es = entries(&influences, 0);
    assert!(
        matches!(&es[0].cells[0].value,RecordValue::Reference(DefinitionPin::Stat {stat_id,..}) if stat_id==RESOLVE_ID)
    );
    assert_eq!(
        es[1].cells[1].value,
        RecordValue::Number("0.5".parse().unwrap())
    );
    assert!(
        matches!(&es[1].cells[0].value,RecordValue::Reference(DefinitionPin::Field(p)) if p.field_id==QUANTITY_ID)
    );
    let runner = l.card(FERNSTRIDER_ID, CardNamespace::Variant, false).await;
    assert!(runner.source.is_none());
    assert_eq!(
        entries(&relations.card(&runner.pin()).await.unwrap(), 0).len(),
        2
    );
    let satchel = l
        .card(FIELD_SATCHEL_ID, CardNamespace::Standalone, false)
        .await;
    assert_eq!(
        entries(&relations.card(&satchel.pin()).await.unwrap(), 0)[0].cells[0].value,
        RecordValue::Reference(DefinitionPin::Card(wind.pin()))
    );
    assert_eq!(
        relations
            .collection(Some(WORLD_ID), PRODUCES_ID)
            .await
            .unwrap()
            .layout
            .pin(),
        relations
            .collection(Some(WORLD_ID), RESOURCE_COSTS_ID)
            .await
            .unwrap()
            .layout
            .pin()
    );
    assert_eq!(
        relations
            .layout(Some(WORLD_ID), RESOURCE_AMOUNT_ID)
            .await
            .unwrap()
            .source
            .unwrap()
            .layout_id,
        SHARED_RESOURCE_AMOUNT_ID
    );
    assert!(
        sqlx::query("PRAGMA foreign_key_check")
            .fetch_all(&l.pool)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(l.accounts.authenticate(&token).await.unwrap().is_some());
}

#[tokio::test]
async fn explicit_source_mapping_preserves_shared_targets_and_rerun_preserves_all_later_state() {
    let l = Lab::new().await;
    let owner = l.user("owner", Role::Administrator).await;
    let other = l.user("collaborator", Role::Editor).await;
    let (token, _) = l.accounts.start_session(&owner.user_id).await.unwrap();
    let (revoked, _) = l.accounts.start_session(&owner.user_id).await.unwrap();
    l.accounts.sign_out(&revoked).await.unwrap();
    let initial = l.workshop().seed(&owner.user_id).await.unwrap();
    let wind = l
        .card(SECOND_WIND_ID, CardNamespace::Standalone, false)
        .await;
    let mapping = CollectionMapping {
        shared_collection_id: SHARED_RESOURCE_COSTS_ID.into(),
        local_collection: l
            .relations()
            .collection(Some(WORLD_ID), RESOURCE_COSTS_ID)
            .await
            .unwrap()
            .pin(),
        columns: [RESOURCE_COLUMN, AMOUNT_COLUMN, CONDITION_COLUMN]
            .iter()
            .map(|id| ColumnMapping {
                source_column_id: (*id).into(),
                local_column_id: (*id).into(),
            })
            .collect(),
    };
    let preview = l
        .relations()
        .preview_source_collections(&wind.identity, 1, &[mapping])
        .await
        .unwrap();
    l.relations()
        .confirm_card(&token, &preview, Some("Explicit example mapping"))
        .await
        .unwrap();
    let current = l
        .card(SECOND_WIND_ID, CardNamespace::Standalone, false)
        .await;
    let copied = l.relations().card(&current.pin()).await.unwrap();
    assert!(
        matches!(&entries(&copied,0)[0].cells[0].value,RecordValue::Reference(DefinitionPin::Card(p)) if p.identity.world_id.is_none() && p.identity.id==SHARED_EMBER_ID)
    );
    let patient = l.card(PATIENT_ID, CardNamespace::Standalone, false).await;
    let mut component = patient.component.clone();
    component.rules_text = Some("Keep later personality prose".into());
    l.cards()
        .save_card(
            &token,
            &patient.identity,
            CardEdit {
                expected_revision: 1,
                component,
                reason: None,
            },
        )
        .await
        .unwrap();
    let restless = l.card(RESTLESS_ID, CardNamespace::Standalone, false).await;
    l.cards()
        .save_metadata(
            &token,
            &restless.identity,
            CardMetadataEdit {
                expected_revision: 1,
                fields: restless.fields,
                archived_at: Some(1),
                reason: None,
            },
        )
        .await
        .unwrap();
    let author = Authoring::new(l.accounts.clone());
    author
        .grant_collaborator(&token, WORLD_ID, 1, &other.user_id, None)
        .await
        .unwrap();
    let mut metadata = author.world_detail(WORLD_ID).await.unwrap().fields;
    metadata.display_name = "Authored Relations".into();
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
    let rerun = l.workshop().seed(&owner.user_id).await.unwrap();
    assert_eq!((rerun.created, rerun.skipped), (0, initial.created));
    assert_eq!(l.snapshot().await, before);
    assert!(l.accounts.authenticate(&token).await.unwrap().is_some());
    assert!(l.accounts.authenticate(&revoked).await.unwrap().is_none());
    l.reject(&other.user_id).await;
}

#[tokio::test]
async fn late_failures_and_suppressed_seals_roll_back_the_entire_batch() {
    for fault in [
        "CREATE TRIGGER fault BEFORE UPDATE ON card_relation_versions WHEN NEW.card_id='d1810000000040008000000000000001' BEGIN SELECT RAISE(IGNORE); END",
        "CREATE TRIGGER fault BEFORE INSERT ON world_revisions BEGIN SELECT RAISE(ABORT,'world seal'); END",
        "CREATE TRIGGER fault BEFORE INSERT ON world_access_revisions BEGIN SELECT RAISE(ABORT,'access seal'); END",
        "CREATE TRIGGER fault BEFORE UPDATE ON card_components WHEN NEW.card_id='d1810000000040008000000000000001' BEGIN SELECT RAISE(IGNORE); END",
    ] {
        let l = Lab::new().await;
        let a = l.user("owner", Role::Editor).await;
        sqlx::query(fault).execute(&l.pool).await.unwrap();
        l.reject(&a.user_id).await;
        assert_eq!(l.count("audit_events").await, 0);
        assert_eq!(l.count("worlds").await, 0);
        sqlx::query("DROP TRIGGER fault")
            .execute(&l.pool)
            .await
            .unwrap();
        assert!(l.workshop().seed(&a.user_id).await.is_ok());
    }
}

#[tokio::test]
async fn corrupt_creation_rows_sources_choices_membership_and_provenance_are_rejected() {
    for corruption in [
        "UPDATE audit_events SET reason='changed' WHERE action='own'",
        "UPDATE card_relation_cells SET coefficient=3 WHERE state='number' AND coefficient=2",
        "UPDATE card_relation_versions SET snapshot_json=snapshot_json||' ' WHERE namespace='variant'",
        "UPDATE card_components SET snapshot_json=snapshot_json||' ' WHERE namespace='variant'",
        "UPDATE record_layout_columns SET label='changed' WHERE label='Amount'",
        "UPDATE card_field_options SET label='changed' WHERE slot=0",
        "UPDATE record_collection_versions SET source_id=NULL,source_revision=NULL WHERE source_id IS NOT NULL",
        "UPDATE stat_versions SET whole_only=1",
        "UPDATE world_content_snapshots SET after_json=json_set(after_json,'$.content.type_ids',json('[]'))",
        "UPDATE world_access_revisions SET after_json=after_json||' '",
    ] {
        let l = Lab::new().await;
        let a = l.user("owner", Role::Editor).await;
        l.workshop().seed(&a.user_id).await.unwrap();
        l.corrupt(corruption).await;
        l.reject(&a.user_id).await;
    }
}

#[tokio::test]
async fn partial_batch_and_world_or_shared_definition_slug_collisions_reject() {
    for mode in [
        "partial",
        "world-slug",
        "layout-slug",
        "collection-slug",
        "field-slug",
    ] {
        let l = Lab::new().await;
        let a = l.user("owner", Role::Administrator).await;
        let (token, _) = l.accounts.start_session(&a.user_id).await.unwrap();
        let metadata = |name: &str| game_data::authoring::ContentFields {
            slug: "ignored".into(),
            display_name: name.into(),
            description: "Test collision".into(),
        };
        match mode {
            "partial" => {
                l.workshop().seed(&a.user_id).await.unwrap();
                // Remove required unmodeled record evidence without breaking
                // unrelated ownership FKs; this remains an incomplete batch.
                l.corrupt("DELETE FROM card_relation_versions WHERE card_id='d1820000000040008000000000000002'").await;
            }
            "world-slug" => {
                Authoring::new(l.accounts.clone())
                    .create_world(&token, metadata("Card Relations Workshop"), None)
                    .await
                    .unwrap();
            }
            "field-slug" => {
                l.cards()
                    .create_field(
                        &token,
                        None,
                        web_app::application::cards::NewField {
                            fields: metadata("Quantity"),
                            kind: game_data::card_fields::FieldKind::Boolean,
                            source: None,
                            reason: None,
                        },
                    )
                    .await
                    .unwrap();
            }
            _ => {
                let layout = l
                    .relations()
                    .create_layout(
                        &token,
                        None,
                        web_app::application::relations::NewLayout {
                            fields: metadata(if mode == "layout-slug" {
                                "Resource Amount"
                            } else {
                                "Other"
                            }),
                            source: None,
                            columns: vec![],
                            reason: None,
                        },
                    )
                    .await
                    .unwrap();
                if mode == "collection-slug" {
                    l.relations()
                        .create_collection(
                            &token,
                            None,
                            web_app::application::relations::NewCollection {
                                fields: metadata("Resource Costs"),
                                source: None,
                                layout: layout.pin(),
                                reason: None,
                            },
                        )
                        .await
                        .unwrap();
                }
            }
        }
        l.reject(&a.user_id).await;
    }
}

#[tokio::test]
async fn old_sixty_five_and_fifteen_object_batches_remain_independent_in_both_orders() {
    for new_first in [false, true] {
        let l = Lab::new().await;
        let a = l.user("owner", Role::Administrator).await;
        let mut new_count = 0;
        if new_first {
            new_count = l.workshop().seed(&a.user_id).await.unwrap().created;
        }
        let out = success(
            command(&l.url, &["seed-fixtures", "--owner-user-id", &a.user_id])
                .output()
                .unwrap(),
        );
        assert!(out.contains("skipped 0 matching objects"));
        assert_eq!(
            l.count("audit_events").await,
            69 + if new_first { new_count as i64 + 1 } else { 0 }
        );
        assert_eq!(
            CardWorkshop::new(l.accounts.clone())
                .seed(&a.user_id)
                .await
                .unwrap()
                .created,
            15
        );
        assert_eq!(
            l.count("audit_events").await,
            85 + if new_first { new_count as i64 + 1 } else { 0 }
        );
        if !new_first {
            new_count = l.workshop().seed(&a.user_id).await.unwrap().created;
        }
        let before = l.snapshot().await;
        assert_eq!(
            l.workshop().seed(&a.user_id).await.unwrap().skipped,
            new_count
        );
        assert_eq!(
            CardWorkshop::new(l.accounts.clone())
                .seed(&a.user_id)
                .await
                .unwrap()
                .skipped,
            15
        );
        assert!(
            success(
                command(&l.url, &["seed-fixtures", "--owner-user-id", &a.user_id])
                    .output()
                    .unwrap()
            )
            .contains("skipped 65 matching objects")
        );
        assert!(
            success(
                command(
                    &l.url,
                    &["seed-card-relations", "--owner-user-id", &a.user_id]
                )
                .output()
                .unwrap()
            )
            .contains(&format!("skipped {new_count} matching objects"))
        );
        assert_eq!(l.snapshot().await, before);
    }
}
