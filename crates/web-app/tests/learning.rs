use game_data::{
    authoring::{ContentFields, slug_from_name},
    learning::*,
    moves::{MoveRecord, NumericValue},
    rulesets::{RepeatPolicy, RulesetRecord},
};
use serde::Serialize;
use sqlx::{Executor, SqlitePool};
use tempfile::TempDir;
use web_app::{
    accounts::Accounts,
    application::{
        authoring::{AddressKey, Authoring, Error, VariantRecord, WorldRecord},
        learning::{
            Learning, LearningRecordInput, LearnsetAdoptEdit, LearnsetEdit, MovesetEdit,
            PROVISIONAL_ID,
        },
    },
    auth::{Actor, Role, Subject},
    persistence,
};

fn fields(name: &str) -> ContentFields {
    ContentFields {
        slug: slug_from_name(name),
        display_name: name.into(),
        description: "Synthetic independent learning example.".into(),
    }
}
fn snapshot(row: &impl Serialize) -> String {
    serde_json::json!({"snapshot_version":1,"content":row}).to_string()
}
struct User {
    actor: Actor,
    token: String,
}
async fn user(accounts: &Accounts, name: &str, role: Role) -> User {
    accounts.invite("test", name, role, None).await.unwrap();
    let actor = accounts
        .sign_in(Subject {
            provider: "test".into(),
            provider_subject: name.into(),
            display_name: name.into(),
        })
        .await
        .unwrap();
    let (token, _) = accounts.start_session(&actor.user_id).await.unwrap();
    User { actor, token }
}
struct Lab {
    pool: SqlitePool,
    accounts: Accounts,
    author: Authoring,
    learning: Learning,
    admin: User,
    owner: User,
    other: User,
    viewer: User,
    _dir: TempDir,
}
impl Lab {
    async fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let pool = persistence::connect_for_migration(&format!(
            "sqlite://{}/learning.db",
            dir.path().display()
        ))
        .await
        .unwrap();
        persistence::migrate(&pool).await.unwrap();
        let accounts = Accounts::new(pool.clone(), 3600).unwrap();
        let admin = user(&accounts, "Admin", Role::Administrator).await;
        let owner = user(&accounts, "Owner", Role::Editor).await;
        let other = user(&accounts, "Other", Role::Editor).await;
        let viewer = user(&accounts, "Viewer", Role::Viewer).await;
        Self {
            author: Authoring::new(accounts.clone()),
            learning: Learning::new(accounts.clone()),
            pool,
            accounts,
            admin,
            owner,
            other,
            viewer,
            _dir: dir,
        }
    }
    async fn world(&self, name: &str) -> WorldRecord {
        self.author
            .create_world(&self.owner.token, fields(name), None)
            .await
            .unwrap()
    }
    async fn variant(&self, world: &str, name: &str) -> VariantRecord {
        let head = self.author.world_detail(world).await.unwrap();
        self.author
            .create_variant(
                &self.owner.token,
                world,
                head.revision,
                None,
                fields(name),
                None,
            )
            .await
            .unwrap()
    }
    async fn form(&self, name: &str) -> String {
        self.author
            .create_species(&self.admin.token, fields(name), None)
            .await
            .unwrap()
            .default_form_id
    }
    async fn audits(&self) -> i64 {
        sqlx::query_scalar("SELECT count(*) FROM audit_events")
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }
    async fn integrity(&self) {
        let rows: Vec<(String, i64, String, i64)> = sqlx::query_as("PRAGMA foreign_key_check")
            .fetch_all(&self.pool)
            .await
            .unwrap();
        assert!(rows.is_empty(), "{rows:?}");
        let result: String = sqlx::query_scalar("PRAGMA integrity_check")
            .fetch_one(&self.pool)
            .await
            .unwrap();
        assert_eq!(result, "ok");
    }
    // Exact core fixtures use only the agreed typed SQL contract; production
    // learning calls the core's checked readers, never these test helpers.
    async fn move_row(
        &self,
        world: Option<&str>,
        id: &str,
        name: &str,
        source: Option<game_data::moves::MovePin>,
    ) -> MoveRecord {
        let row = MoveRecord {
            id: id.into(),
            world_id: world.map(str::to_owned),
            source,
            fields: fields(name),
            revision: 1,
            archived_at: None,
            attributes: Some(vec![]),
            mappings: vec![],
        };
        self.write_move(&row, None).await;
        row
    }
    async fn write_move(&self, row: &MoveRecord, before: Option<&MoveRecord>) {
        let mut tx = persistence::begin_write(&self.pool).await.unwrap();
        let audit = format!("a{:031x}", self.audits().await);
        sqlx::query(
            "INSERT INTO audit_events(id,system_key,action) VALUES (?,'bundled-fixture-loader',?)",
        )
        .bind(&audit)
        .bind(if before.is_none() { "create" } else { "edit" })
        .execute(&mut *tx)
        .await
        .unwrap();
        if before.is_none() {
            sqlx::query("INSERT INTO moves(id,scope,world_id,slug,revision) VALUES (?,?,?,?,?)")
                .bind(&row.id)
                .bind(row.world_id.as_deref().unwrap_or(""))
                .bind(&row.world_id)
                .bind(&row.fields.slug)
                .bind(row.revision)
                .execute(&mut *tx)
                .await
                .unwrap();
        } else {
            sqlx::query("UPDATE moves SET revision=?,slug=? WHERE id=? AND scope=?")
                .bind(row.revision)
                .bind(&row.fields.slug)
                .bind(&row.id)
                .bind(row.world_id.as_deref().unwrap_or(""))
                .execute(&mut *tx)
                .await
                .unwrap();
        }
        sqlx::query("INSERT INTO move_versions(scope,move_id,revision,source_move_id,source_revision,slug,display_name,description,archived_at,attributes_defined,audit_id,previous_revision,before_json,after_json) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?)")
            .bind(row.world_id.as_deref().unwrap_or("")).bind(&row.id).bind(row.revision).bind(row.source.as_ref().map(|p|&p.move_id)).bind(row.source.as_ref().map(|p|p.version))
            .bind(&row.fields.slug).bind(&row.fields.display_name).bind(&row.fields.description).bind(row.archived_at).bind(i64::from(row.attributes.is_some())).bind(audit).bind(before.map(|b|b.revision)).bind(before.map(snapshot)).bind(snapshot(row)).execute(&mut *tx).await.unwrap();
        tx.commit().await.unwrap();
    }
    async fn rules(&self, world: &str, cap: Option<i64>, policy: RepeatPolicy) -> RulesetRecord {
        let row = RulesetRecord {
            id: format!("b{:031x}", self.audits().await),
            world_id: world.into(),
            revision: 1,
            fields: fields("Learning rules"),
            expectations: Some(vec![]),
            stat_roster: None,
            chart: None,
            max_moves: cap,
            repeat_policy: policy,
            notes: String::new(),
        };
        self.write_rules(&row, None).await;
        row
    }
    async fn write_rules(&self, row: &RulesetRecord, before: Option<&RulesetRecord>) {
        let mut tx = persistence::begin_write(&self.pool).await.unwrap();
        let audit = format!("a{:031x}", self.audits().await);
        sqlx::query(
            "INSERT INTO audit_events(id,system_key,action) VALUES (?,'bundled-fixture-loader',?)",
        )
        .bind(&audit)
        .bind(if before.is_none() { "create" } else { "edit" })
        .execute(&mut *tx)
        .await
        .unwrap();
        if before.is_none() {
            sqlx::query("INSERT INTO world_rulesets(world_id,id,revision) VALUES (?,?,?)")
                .bind(&row.world_id)
                .bind(&row.id)
                .bind(row.revision)
                .execute(&mut *tx)
                .await
                .unwrap();
        } else {
            sqlx::query("UPDATE world_rulesets SET revision=? WHERE world_id=? AND id=?")
                .bind(row.revision)
                .bind(&row.world_id)
                .bind(&row.id)
                .execute(&mut *tx)
                .await
                .unwrap();
        }
        let policy = match row.repeat_policy {
            RepeatPolicy::Allow => "allow",
            RepeatPolicy::Forbid => "forbid",
            RepeatPolicy::Unspecified => "unspecified",
        };
        sqlx::query("INSERT INTO world_ruleset_versions(world_id,ruleset_id,revision,slug,display_name,description,expectations_defined,max_moves,repeat_policy,notes,audit_id,previous_revision,before_json,after_json) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?)")
            .bind(&row.world_id).bind(&row.id).bind(row.revision).bind(&row.fields.slug).bind(&row.fields.display_name).bind(&row.fields.description).bind(i64::from(row.expectations.is_some())).bind(row.max_moves).bind(policy).bind(&row.notes).bind(audit).bind(before.map(|r|r.revision)).bind(before.map(snapshot)).bind(snapshot(row)).execute(&mut *tx).await.unwrap();
        tx.commit().await.unwrap();
    }
    async fn learn(
        &self,
        world: Option<&str>,
        owner: &str,
        moves: &[&MoveRecord],
    ) -> LearnsetRecord {
        self.learning
            .save_learnset(
                if world.is_some() {
                    &self.owner.token
                } else {
                    &self.admin.token
                },
                world,
                owner,
                LearnsetEdit {
                    expected_revision: 0,
                    records: Some(
                        moves
                            .iter()
                            .enumerate()
                            .map(|(i, m)| {
                                input(
                                    m,
                                    if i % 2 == 0 {
                                        NumericValue::Unknown
                                    } else {
                                        NumericValue::NotApplicable
                                    },
                                )
                            })
                            .collect(),
                    ),
                    reason: None,
                },
            )
            .await
            .unwrap()
    }
    async fn legacy_preset(
        &self,
        world: &str,
        variant: &str,
        set: &LearnsetRecord,
        rules: &RulesetRecord,
        entries: Vec<String>,
    ) -> MovesetRecord {
        let row = MovesetRecord {
            id: "cccccccccccccccccccccccccccccccc".into(),
            world_id: world.into(),
            variant_id: variant.into(),
            fields: fields("Legacy policy preset"),
            revision: 1,
            archived_at: None,
            learnset: set.pin(),
            ruleset: rules.pin(),
            entries,
            validator_version: 1,
        };
        let mut tx = persistence::begin_write(&self.pool).await.unwrap();
        assert!(self.learning.seed_preset_in(&mut tx, &row).await.unwrap());
        tx.commit().await.unwrap();
        assert_eq!(
            self.learning.preset(world, variant, &row.id).await.unwrap(),
            row
        );
        row
    }
}
fn input(m: &MoveRecord, threshold: NumericValue) -> LearningRecordInput {
    LearningRecordInput {
        id: String::new(),
        move_pin: m.pin(),
        method: None,
        context: Some("training".into()),
        condition: None,
        threshold,
    }
}
fn learn_edit(row: &LearnsetRecord) -> LearnsetEdit {
    LearnsetEdit {
        expected_revision: row.revision,
        records: row.records.as_ref().map(|rs| {
            rs.iter()
                .map(|r| LearningRecordInput {
                    id: r.id.clone(),
                    move_pin: r.move_record.pin(),
                    method: r.method.clone(),
                    context: r.context.clone(),
                    condition: r.condition.clone(),
                    threshold: r.threshold.clone(),
                })
                .collect()
        }),
        reason: None,
    }
}
fn preset_edit(
    name: &str,
    set: &LearnsetRecord,
    rules: &RulesetRecord,
    entries: Vec<String>,
) -> MovesetEdit {
    MovesetEdit {
        expected_revision: 0,
        fields: fields(name),
        learnset: set.pin(),
        ruleset: rules.pin(),
        entries,
        reason: None,
    }
}
fn saved_edit(row: &MovesetRecord) -> MovesetEdit {
    MovesetEdit {
        expected_revision: row.revision,
        fields: row.fields.clone(),
        learnset: row.learnset.clone(),
        ruleset: row.ruleset.clone(),
        entries: row.entries.clone(),
        reason: None,
    }
}

// Compare every row and column, including heads, children, seals and audits.
async fn database_rows(pool: &SqlitePool) -> Vec<Vec<String>> {
    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
    )
    .fetch_all(pool)
    .await
    .unwrap();
    let mut rows = vec![];
    for table in tables {
        let columns: Vec<String> = sqlx::query_scalar("SELECT name FROM pragma_table_info(?)")
            .bind(&table)
            .fetch_all(pool)
            .await
            .unwrap();
        let values = columns
            .iter()
            .map(|name| format!("quote(\"{name}\")"))
            .collect::<Vec<_>>()
            .join("||','||");
        // Identifiers come only from the test-owned migration schema.
        rows.push(
            sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                "SELECT {values} FROM \"{table}\" ORDER BY 1"
            )))
            .fetch_all(pool)
            .await
            .unwrap(),
        );
    }
    rows
}

fn assert_blocked_selection(error: Error) {
    let Error::Invalid(errors) = error else {
        panic!("Expected an invalid new selection, got {error:?}");
    };
    assert!(errors.contains_key("entries"), "{errors:?}");
    assert!(errors["entries"].contains("unarchived"));
}

#[tokio::test]
async fn preset_policy_revalidation_is_explicit_attributed_and_preserves_legacy_history() {
    let l = Lab::new().await;
    let w = l.world("Policy versions").await;
    let v = l.variant(&w.id, "Policy owner").await;
    let set = l.learn(Some(&w.id), &v.id, &[]).await;
    let rules = l.rules(&w.id, None, RepeatPolicy::Unspecified).await;
    assert_eq!(VALIDATOR_VERSION, 2);
    let legacy = l.legacy_preset(&w.id, &v.id, &set, &rules, vec![]).await;
    let old_report = l
        .learning
        .validation(&w.id, &v.id, &legacy.id, Some(1))
        .await
        .unwrap();
    assert_eq!(old_report.validator_version, 1);
    assert_eq!(
        old_report.incomplete,
        vec![
            "The ruleset has no moveset maximum.",
            "The ruleset repeat policy is unspecified.",
        ]
    );
    assert!(
        !old_report
            .not_evaluated
            .iter()
            .any(|m| m.contains("repeat policy"))
    );
    let mut edit = saved_edit(&legacy);
    edit.fields.description = "An ordinary metadata edit retains policy one".into();
    let before = database_rows(&l.pool).await;
    let ordinary = l
        .learning
        .preview_preset(&w.id, &v.id, Some(&legacy.id), &edit)
        .await
        .unwrap();
    assert_eq!(ordinary.proposed.validator_version, 1);
    assert_eq!(ordinary.report.validator_version, 1);
    assert_eq!(ordinary.report.incomplete, old_report.incomplete);
    assert_eq!(database_rows(&l.pool).await, before);
    let edited = l
        .learning
        .save_preset(&l.owner.token, &w.id, &v.id, &legacy.id, edit)
        .await
        .unwrap();
    assert_eq!(edited, ordinary.proposed);
    assert_eq!(edited.revision, 2);
    assert_eq!(edited.validator_version, 1);
    let mut edited_report = old_report.clone();
    edited_report
        .inputs
        .iter_mut()
        .find(|i| i.kind == "preset")
        .unwrap()
        .version = edited.revision;
    assert_eq!(
        l.learning
            .validation(&w.id, &v.id, &legacy.id, None)
            .await
            .unwrap(),
        edited_report
    );

    // Advancing dependency heads must not advance the exact contexts reviewed
    // during policy-only revalidation.
    let mut change = learn_edit(&set);
    change.records = None;
    l.learning
        .save_learnset(&l.owner.token, Some(&w.id), &v.id, change)
        .await
        .unwrap();
    let mut newer_rules = rules.clone();
    newer_rules.revision = 2;
    newer_rules.max_moves = Some(4);
    newer_rules.repeat_policy = RepeatPolicy::Forbid;
    l.write_rules(&newer_rules, Some(&rules)).await;
    let old_history = l
        .learning
        .preset_history(&w.id, &v.id, &legacy.id, None)
        .await
        .unwrap();
    let audits = l.audits().await;
    let before = database_rows(&l.pool).await;
    let mut edit = saved_edit(&edited);
    edit.reason = Some("Explicit current-policy revalidation".into());
    let preview = l
        .learning
        .preview_preset_revalidation(&w.id, &v.id, &legacy.id, &edit)
        .await
        .unwrap();
    let mut expected = edited.clone();
    expected.revision = 3;
    expected.validator_version = VALIDATOR_VERSION;
    assert_eq!(preview.entry, Some(edited));
    assert_eq!(preview.proposed, expected);
    assert_eq!(preview.report.validator_version, VALIDATOR_VERSION);
    assert!(preview.report.incomplete.is_empty());
    assert!(preview.report.contradictions.is_empty());
    assert!(
        preview
            .report
            .not_evaluated
            .iter()
            .any(|m| m.contains("repeat policy is unspecified"))
    );
    assert_eq!(database_rows(&l.pool).await, before);
    let upgraded = l
        .learning
        .revalidate_preset(&l.owner.token, &w.id, &v.id, &legacy.id, edit)
        .await
        .unwrap();
    assert_eq!(upgraded, preview.proposed);
    assert_eq!(upgraded.learnset, set.pin());
    assert_eq!(upgraded.ruleset, rules.pin());
    assert_eq!(l.audits().await, audits + 1);
    assert_eq!(
        l.learning
            .validation(&w.id, &v.id, &legacy.id, None)
            .await
            .unwrap(),
        preview.report
    );
    let history = l
        .learning
        .preset_history(&w.id, &v.id, &legacy.id, None)
        .await
        .unwrap();
    assert_eq!(history.len(), old_history.len() + 1);
    assert_eq!(history[0].revision, 3);
    assert_eq!(history[0].previous_revision, Some(2));
    assert_eq!(history[0].actor_name, "Owner");
    assert_eq!(history[0].action, "edit");
    assert_eq!(
        history[0].reason.as_deref(),
        Some("Explicit current-policy revalidation")
    );
    assert_eq!(
        history[0].before_json.as_ref(),
        Some(&old_history[0].after_json)
    );
    for (retained, old) in history.iter().skip(1).zip(&old_history) {
        assert_eq!(retained.before_json, old.before_json);
        assert_eq!(retained.after_json, old.after_json);
    }
    let audit: (String, Option<String>, String) = sqlx::query_as("SELECT actor_user_id,system_key,action FROM audit_events WHERE id=(SELECT audit_id FROM moveset_preset_versions WHERE preset_id=? AND revision=3)")
        .bind(&legacy.id).fetch_one(&l.pool).await.unwrap();
    assert_eq!(audit, (l.owner.actor.user_id.clone(), None, "edit".into()));
    assert_eq!(
        l.learning
            .preset_version(&w.id, &v.id, &legacy.id, 1)
            .await
            .unwrap(),
        legacy
    );
    assert_eq!(
        l.learning
            .validation(&w.id, &v.id, &legacy.id, Some(1))
            .await
            .unwrap(),
        old_report
    );
    assert_eq!(
        l.learning
            .validation(&w.id, &v.id, &legacy.id, Some(2))
            .await
            .unwrap(),
        edited_report
    );

    let before = database_rows(&l.pool).await;
    let noop = saved_edit(&upgraded);
    assert_eq!(
        l.learning
            .preview_preset_revalidation(&w.id, &v.id, &legacy.id, &noop)
            .await
            .unwrap()
            .proposed,
        upgraded
    );
    assert_eq!(
        l.learning
            .revalidate_preset(&l.owner.token, &w.id, &v.id, &legacy.id, noop)
            .await
            .unwrap(),
        upgraded
    );
    // A policy-one fixture baseline remains accepted, idempotent and byte-exact
    // even after the live head has explicitly advanced to policy two.
    let mut tx = persistence::begin_write(&l.pool).await.unwrap();
    assert!(!l.learning.seed_preset_in(&mut tx, &legacy).await.unwrap());
    tx.commit().await.unwrap();
    assert_eq!(database_rows(&l.pool).await, before);

    let current = l
        .learning
        .create_preset(
            &l.owner.token,
            &w.id,
            &v.id,
            preset_edit("New current policy", &set, &rules, vec![]),
        )
        .await
        .unwrap();
    assert_eq!(current.validator_version, VALIDATOR_VERSION);
    let report = l
        .learning
        .validation(&w.id, &v.id, &current.id, None)
        .await
        .unwrap();
    assert_eq!(report.validator_version, VALIDATOR_VERSION);
    assert!(report.incomplete.is_empty());
    assert!(
        report
            .not_evaluated
            .iter()
            .any(|m| m.contains("repeat policy is unspecified"))
    );
    l.integrity().await;
}

#[tokio::test]
async fn preset_policy_revalidation_prioritizes_authorization_and_expected_before_bad_references() {
    let l = Lab::new().await;
    let w = l.world("Revalidation guards").await;
    let v = l.variant(&w.id, "Owner").await;
    let set = l.learn(Some(&w.id), &v.id, &[]).await;
    let rules = l.rules(&w.id, None, RepeatPolicy::Unspecified).await;
    let legacy = l.legacy_preset(&w.id, &v.id, &set, &rules, vec![]).await;
    let before = database_rows(&l.pool).await;
    let mut missing = saved_edit(&legacy);
    missing.learnset.version = 999;
    let mut malformed = saved_edit(&legacy);
    malformed.ruleset.ruleset_id = "not-a-canonical-id".into();
    let mut absent_entry = saved_edit(&legacy);
    absent_entry.entries = vec!["ffffffffffffffffffffffffffffffff".into()];
    for edit in [missing, malformed, absent_entry] {
        assert!(matches!(
            l.learning
                .preview_preset_revalidation(&w.id, &v.id, &legacy.id, &edit)
                .await,
            Err(Error::Invalid(_))
        ));
        assert!(matches!(
            l.learning
                .revalidate_preset(&l.owner.token, &w.id, &v.id, &legacy.id, edit.clone())
                .await,
            Err(Error::Invalid(_))
        ));
        let mut stale = edit;
        stale.expected_revision = 0;
        assert_eq!(
            l.learning
                .preview_preset_revalidation(&w.id, &v.id, &legacy.id, &stale)
                .await
                .unwrap_err(),
            Error::Conflict
        );
        assert_eq!(
            l.learning
                .revalidate_preset(&l.owner.token, &w.id, &v.id, &legacy.id, stale.clone())
                .await
                .unwrap_err(),
            Error::Conflict
        );
        for token in [&l.other.token, &l.viewer.token, &l.admin.token] {
            assert_eq!(
                l.learning
                    .revalidate_preset(token, &w.id, &v.id, &legacy.id, stale.clone())
                    .await
                    .unwrap_err(),
                Error::Forbidden
            );
        }
        assert_eq!(database_rows(&l.pool).await, before);
    }
    assert_eq!(
        l.learning.preset(&w.id, &v.id, &legacy.id).await.unwrap(),
        legacy
    );
    l.accounts
        .set_disabled(&l.owner.actor.user_id, true)
        .await
        .unwrap();
    let before = database_rows(&l.pool).await;
    let mut edit = saved_edit(&legacy);
    edit.expected_revision = 0;
    edit.learnset.version = 999;
    assert_eq!(
        l.learning
            .revalidate_preset(&l.owner.token, &w.id, &v.id, &legacy.id, edit)
            .await
            .unwrap_err(),
        Error::Unauthorized
    );
    assert_eq!(database_rows(&l.pool).await, before);
    l.integrity().await;
}

#[tokio::test]
async fn preset_policy_revalidation_late_version_failure_rolls_back_head_children_and_audit() {
    let l = Lab::new().await;
    let w = l.world("Revalidation rollback").await;
    let v = l.variant(&w.id, "Owner").await;
    let m = l
        .move_row(
            Some(&w.id),
            "11111111111111111111111111111111",
            "Move",
            None,
        )
        .await;
    let set = l.learn(Some(&w.id), &v.id, &[&m]).await;
    let rules = l.rules(&w.id, None, RepeatPolicy::Unspecified).await;
    let entries = vec![set.records.as_ref().unwrap()[0].id.clone(); 2];
    let legacy = l.legacy_preset(&w.id, &v.id, &set, &rules, entries).await;
    let old_report = l
        .learning
        .validation(&w.id, &v.id, &legacy.id, None)
        .await
        .unwrap();
    let before = database_rows(&l.pool).await;
    // The writer stages the audit, mutable head and both child entries before
    // inserting the new policy-two revision. Every normal seal remains enabled.
    l.pool.execute("CREATE TRIGGER revalidation_fail BEFORE INSERT ON moveset_preset_versions WHEN NEW.revision=2 AND NEW.validator_version=2 BEGIN SELECT RAISE(ABORT,'test late revalidation failure'); END").await.unwrap();
    let mut edit = saved_edit(&legacy);
    edit.reason = Some("This upgrade must roll back".into());
    let preview = l
        .learning
        .preview_preset_revalidation(&w.id, &v.id, &legacy.id, &edit)
        .await
        .unwrap();
    assert_eq!(preview.proposed.validator_version, VALIDATOR_VERSION);
    assert_eq!(preview.proposed.entries, legacy.entries);
    assert_eq!(preview.proposed.revision, 2);
    assert!(matches!(
        l.learning
            .revalidate_preset(&l.owner.token, &w.id, &v.id, &legacy.id, edit)
            .await,
        Err(Error::Unavailable)
    ));
    assert_eq!(database_rows(&l.pool).await, before);
    assert_eq!(
        l.learning.preset(&w.id, &v.id, &legacy.id).await.unwrap(),
        legacy
    );
    assert_eq!(
        l.learning
            .validation(&w.id, &v.id, &legacy.id, None)
            .await
            .unwrap(),
        old_report
    );
    assert_eq!(
        l.learning
            .preset_history(&w.id, &v.id, &legacy.id, None)
            .await
            .unwrap()
            .len(),
        1
    );
    l.integrity().await;
}

#[tokio::test]
async fn preset_archived_move_selections_retain_exact_acquisitions_and_multiplicity() {
    let l = Lab::new().await;
    let w = l.world("Preset archives").await;
    let v = l.variant(&w.id, "Owner").await;
    let first = l
        .move_row(
            Some(&w.id),
            "11111111111111111111111111111111",
            "Move",
            None,
        )
        .await;
    let mut second = first.clone();
    second.revision = 2;
    second.fields = fields("Move revised");
    l.write_move(&second, Some(&first)).await;
    let live = l
        .move_row(
            Some(&w.id),
            "22222222222222222222222222222222",
            "Live Move",
            None,
        )
        .await;
    let set = l
        .learn(Some(&w.id), &v.id, &[&first, &second, &first, &live])
        .await;
    let records = set.records.as_ref().unwrap();
    let a = records[0].id.clone();
    let b = records[1].id.clone();
    let unselected = records[2].id.clone();
    let live_id = records[3].id.clone();
    let rules = l.rules(&w.id, None, RepeatPolicy::Allow).await;
    let preset = l
        .learning
        .create_preset(
            &l.owner.token,
            &w.id,
            &v.id,
            preset_edit(
                "Retained selections",
                &set,
                &rules,
                vec![a.clone(), a.clone(), b.clone(), live_id.clone()],
            ),
        )
        .await
        .unwrap();
    assert_eq!(preset.validator_version, VALIDATOR_VERSION);
    let original_report = l
        .learning
        .validation(&w.id, &v.id, &preset.id, Some(1))
        .await
        .unwrap();

    // Prepare two exact contexts while the Move is selectable. The first
    // retargets the same acquisition to a pin already used by another entry.
    let mut retarget = learn_edit(&set);
    retarget.records.as_mut().unwrap()[0].move_pin = second.pin();
    let retargeted = l
        .learning
        .save_learnset(&l.owner.token, Some(&w.id), &v.id, retarget)
        .await
        .unwrap();
    let mut restore = learn_edit(&retargeted);
    restore.records.as_mut().unwrap()[0].move_pin = first.pin();
    restore.records.as_mut().unwrap()[0].method = Some("New inert metadata".into());
    let restored = l
        .learning
        .save_learnset(&l.owner.token, Some(&w.id), &v.id, restore)
        .await
        .unwrap();
    let mut archived = second.clone();
    archived.revision = 3;
    archived.archived_at = Some(1_800_000_000);
    l.write_move(&archived, Some(&second)).await;
    assert_eq!(
        web_app::application::moves::Moves::new(l.accounts.clone())
            .detail(Some(&w.id), &first.id)
            .await
            .unwrap(),
        archived
    );
    assert_eq!(
        l.learning
            .preset_version(&w.id, &v.id, &preset.id, 1)
            .await
            .unwrap(),
        preset
    );
    assert_eq!(
        l.learning
            .validation(&w.id, &v.id, &preset.id, None)
            .await
            .unwrap(),
        original_report
    );
    let before = database_rows(&l.pool).await;
    let new = preset_edit("New archived selection", &set, &rules, vec![a.clone()]);
    assert_blocked_selection(
        l.learning
            .preview_preset(&w.id, &v.id, None, &new)
            .await
            .unwrap_err(),
    );
    assert_blocked_selection(
        l.learning
            .create_preset(&l.owner.token, &w.id, &v.id, new)
            .await
            .unwrap_err(),
    );
    let mut extra = saved_edit(&preset);
    extra.entries.push(a.clone());
    let mut distinct = saved_edit(&preset);
    distinct.entries.push(unselected);
    let mut changed_pin = saved_edit(&preset);
    changed_pin.learnset = retargeted.pin();
    for edit in [extra, distinct, changed_pin] {
        assert_blocked_selection(
            l.learning
                .preview_preset(&w.id, &v.id, Some(&preset.id), &edit)
                .await
                .unwrap_err(),
        );
        assert_blocked_selection(
            l.learning
                .save_preset(&l.owner.token, &w.id, &v.id, &preset.id, edit.clone())
                .await
                .unwrap_err(),
        );
        let mut stale = edit.clone();
        stale.expected_revision = 0;
        assert_eq!(
            l.learning
                .save_preset(&l.owner.token, &w.id, &v.id, &preset.id, stale.clone())
                .await
                .unwrap_err(),
            Error::Conflict
        );
        assert_eq!(
            l.learning
                .save_preset(&l.other.token, &w.id, &v.id, &preset.id, stale)
                .await
                .unwrap_err(),
            Error::Forbidden
        );
    }
    assert_eq!(database_rows(&l.pool).await, before);
    assert_eq!(
        l.learning
            .save_preset(
                &l.owner.token,
                &w.id,
                &v.id,
                &preset.id,
                saved_edit(&preset)
            )
            .await
            .unwrap(),
        preset
    );
    assert_eq!(database_rows(&l.pool).await, before);

    // Reordering and a newer learnset context retain both copies of the same
    // exact acquisition. The old context, not the current head, defines retention.
    let mut edit = saved_edit(&preset);
    edit.learnset = restored.pin();
    edit.fields.description = "Unrelated preset metadata".into();
    edit.entries = vec![b.clone(), a.clone(), live_id, a.clone()];
    let preview = l
        .learning
        .preview_preset(&w.id, &v.id, Some(&preset.id), &edit)
        .await
        .unwrap();
    let reordered = l
        .learning
        .save_preset(&l.owner.token, &w.id, &v.id, &preset.id, edit)
        .await
        .unwrap();
    assert_eq!(reordered, preview.proposed);
    let mut edit = saved_edit(&reordered);
    edit.entries = vec![b, a.clone()];
    let removed = l
        .learning
        .save_preset(&l.owner.token, &w.id, &v.id, &preset.id, edit)
        .await
        .unwrap();
    let before = database_rows(&l.pool).await;
    let mut extra = saved_edit(&removed);
    extra.entries.push(a);
    assert_blocked_selection(
        l.learning
            .save_preset(&l.owner.token, &w.id, &v.id, &preset.id, extra)
            .await
            .unwrap_err(),
    );
    assert_eq!(database_rows(&l.pool).await, before);

    // A late write failure on an otherwise valid retained selection must leave
    // no advanced head, child row, version or attributed audit behind.
    l.pool.execute("CREATE TRIGGER retained_preset_fail BEFORE INSERT ON moveset_preset_versions WHEN NEW.revision=4 BEGIN SELECT RAISE(ABORT,'test retained failure'); END").await.unwrap();
    let mut edit = saved_edit(&removed);
    edit.fields.description = "Must roll back".into();
    assert!(
        l.learning
            .save_preset(&l.owner.token, &w.id, &v.id, &preset.id, edit)
            .await
            .is_err()
    );
    assert_eq!(database_rows(&l.pool).await, before);
    assert_eq!(
        l.learning
            .validation(&w.id, &v.id, &preset.id, Some(1))
            .await
            .unwrap(),
        original_report
    );
    l.integrity().await;
}

#[tokio::test]
async fn repeated_acquisitions_named_presets_and_exact_draft_reports() {
    let l = Lab::new().await;
    let w = l.world("World").await;
    let v = l.variant(&w.id, "Variant").await;
    let m = l
        .move_row(
            Some(&w.id),
            "11111111111111111111111111111111",
            "Move",
            None,
        )
        .await;
    let mut newer = m.clone();
    newer.revision = 2;
    newer.fields = fields("Move revised");
    l.write_move(&newer, Some(&m)).await;
    let set = l.learn(Some(&w.id), &v.id, &[&m, &newer]).await;
    let records = set.records.as_ref().unwrap();
    assert_eq!(records.len(), 2);
    assert_ne!(records[0].id, records[1].id);
    let rules = l.rules(&w.id, Some(1), RepeatPolicy::Forbid).await;
    let edit = preset_edit(
        "Repeated draft",
        &set,
        &rules,
        vec![
            records[0].id.clone(),
            records[1].id.clone(),
            records[0].id.clone(),
        ],
    );
    let preview = l
        .learning
        .preview_preset(&w.id, &v.id, None, &edit)
        .await
        .unwrap();
    assert_eq!(preview.proposed.id, PROVISIONAL_ID);
    assert_eq!(preview.report.validator_version, VALIDATOR_VERSION);
    assert!(
        preview
            .report
            .contradictions
            .iter()
            .any(|s| s.contains("maximum"))
    );
    assert!(
        preview
            .report
            .contradictions
            .iter()
            .any(|s| s.contains("repeats are forbidden"))
    );
    let first = l
        .learning
        .create_preset(&l.owner.token, &w.id, &v.id, edit)
        .await
        .unwrap();
    assert_eq!(
        l.learning
            .validation(&w.id, &v.id, &first.id, None)
            .await
            .unwrap()
            .contradictions,
        preview.report.contradictions
    );
    let mut allow = rules.clone();
    allow.revision = 2;
    allow.max_moves = None;
    allow.repeat_policy = RepeatPolicy::Allow;
    l.write_rules(&allow, Some(&rules)).await;
    let second = l
        .learning
        .create_preset(
            &l.owner.token,
            &w.id,
            &v.id,
            preset_edit("Unbounded draft", &set, &allow, first.entries.clone()),
        )
        .await
        .unwrap();
    assert!(
        l.learning
            .validation(&w.id, &v.id, &second.id, None)
            .await
            .unwrap()
            .contradictions
            .is_empty()
    );
    assert_eq!(l.learning.presets(&w.id, &v.id).await.unwrap().len(), 2);
    assert_eq!(
        l.learning
            .resolve_preset(&w.id, &v.id, AddressKey::Slug(first.fields.slug.clone()))
            .await
            .unwrap(),
        first
    );
    let report = l
        .learning
        .validation(&w.id, &v.id, &first.id, Some(1))
        .await
        .unwrap();
    assert!(
        report
            .inputs
            .iter()
            .any(|i| i.kind == "ruleset" && i.id == rules.id && i.version == 1)
    );
    assert!(
        report
            .inputs
            .iter()
            .any(|i| i.kind == "move" && i.id == m.id && i.version == 1)
    );
    assert!(
        report
            .inputs
            .iter()
            .any(|i| i.kind == "move" && i.id == m.id && i.version == 2)
    );
    assert_eq!(
        l.learning
            .validation(&w.id, &v.id, &first.id, Some(1))
            .await
            .unwrap(),
        report
    );
    assert_eq!(l.author.variant_detail(&w.id, &v.id).await.unwrap(), v);
    l.integrity().await;
}

#[tokio::test]
async fn explicit_source_copy_is_deterministic_preserves_each_record_and_never_retargets() {
    let l = Lab::new().await;
    let form = l.form("Shared").await;
    let w = l.world("World").await;
    let v = l.variant(&w.id, "Variant").await;
    let shared = l
        .move_row(
            None,
            "11111111111111111111111111111111",
            "Shared move",
            None,
        )
        .await;
    let local = l
        .move_row(
            Some(&w.id),
            "22222222222222222222222222222222",
            "Local move",
            Some(shared.pin()),
        )
        .await;
    let _ambiguous = l
        .move_row(
            Some(&w.id),
            "33333333333333333333333333333333",
            "Another copy",
            Some(shared.pin()),
        )
        .await;
    let source = l.learn(None, &form, &[&shared, &shared]).await;
    let mut edit = LearnsetAdoptEdit {
        expected_revision: 0,
        source: source.pin(),
        apply_source: true,
        mappings: vec![],
        reason: None,
    };
    assert!(matches!(
        l.learning.preview_adoption(&w.id, &v.id, &edit).await,
        Err(Error::Invalid(_))
    ));
    edit.mappings = vec![MoveMapping {
        shared_move_id: shared.id.clone(),
        local_move: local.pin(),
    }];
    let a = l
        .learning
        .preview_adoption(&w.id, &v.id, &edit)
        .await
        .unwrap();
    let b = l
        .learning
        .preview_adoption(&w.id, &v.id, &edit)
        .await
        .unwrap();
    assert_eq!(a.proposed, b.proposed);
    assert_eq!(a.proposed.id, PROVISIONAL_ID);
    let adopted = l
        .learning
        .adopt_learnset(&l.owner.token, &w.id, &v.id, edit)
        .await
        .unwrap();
    let source_records = source.records.as_ref().unwrap();
    let local_records = adopted.records.as_ref().unwrap();
    assert_eq!(local_records.len(), 2);
    assert_eq!(local_records[0].id, source_records[0].id);
    assert_eq!(local_records[1].id, source_records[1].id);
    assert_eq!(local_records[0].threshold, NumericValue::Unknown);
    assert_eq!(local_records[1].threshold, NumericValue::NotApplicable);
    assert!(local_records.iter().all(|r| r.move_record == local));
    assert_eq!(
        l.author.variant_detail(&w.id, &v.id).await.unwrap().source,
        None
    );
    let rules = l.rules(&w.id, None, RepeatPolicy::Allow).await;
    let preset = l
        .learning
        .create_preset(
            &l.owner.token,
            &w.id,
            &v.id,
            preset_edit(
                "Pinned",
                &adopted,
                &rules,
                vec![local_records[1].id.clone()],
            ),
        )
        .await
        .unwrap();
    let report = l
        .learning
        .validation(&w.id, &v.id, &preset.id, None)
        .await
        .unwrap();
    let mut change = learn_edit(&source);
    change.records.as_mut().unwrap().clear();
    let next = l
        .learning
        .save_learnset(&l.admin.token, None, &form, change)
        .await
        .unwrap();
    let adopted_next = l
        .learning
        .adopt_learnset(
            &l.owner.token,
            &w.id,
            &v.id,
            LearnsetAdoptEdit {
                expected_revision: adopted.revision,
                source: next.pin(),
                apply_source: true,
                mappings: vec![],
                reason: Some("Replace with known empty".into()),
            },
        )
        .await
        .unwrap();
    assert_eq!(adopted_next.records, Some(vec![]));
    assert_eq!(
        l.learning.preset(&w.id, &v.id, &preset.id).await.unwrap(),
        preset
    );
    assert_eq!(
        l.learning
            .validation(&w.id, &v.id, &preset.id, None)
            .await
            .unwrap(),
        report
    );
    assert_eq!(
        l.learning
            .learnset_version(Some(&w.id), &adopted.id, 1)
            .await
            .unwrap(),
        adopted
    );
    assert_eq!(l.author.variant_detail(&w.id, &v.id).await.unwrap(), v);
    l.integrity().await;
}

#[tokio::test]
async fn ownership_scope_and_broken_exact_references_are_save_errors() {
    let l = Lab::new().await;
    let w = l.world("World").await;
    let other = l.world("Other").await;
    let v = l.variant(&w.id, "Variant").await;
    let v2 = l.variant(&w.id, "Second").await;
    let m = l
        .move_row(
            Some(&w.id),
            "11111111111111111111111111111111",
            "Move",
            None,
        )
        .await;
    let foreign = l
        .move_row(
            Some(&other.id),
            "22222222222222222222222222222222",
            "Foreign",
            None,
        )
        .await;
    assert!(matches!(
        l.learning
            .save_learnset(
                &l.owner.token,
                Some(&w.id),
                &v.id,
                LearnsetEdit {
                    expected_revision: 0,
                    records: Some(vec![input(&foreign, NumericValue::Unknown)]),
                    reason: None
                }
            )
            .await,
        Err(Error::Invalid(_))
    ));
    let set = l.learn(Some(&w.id), &v.id, &[&m]).await;
    let rules = l.rules(&w.id, None, RepeatPolicy::Unspecified).await;
    let wrong_owner = preset_edit("Wrong owner", &set, &rules, vec![]);
    assert!(matches!(
        l.learning
            .create_preset(&l.owner.token, &w.id, &v2.id, wrong_owner)
            .await,
        Err(Error::Invalid(_))
    ));
    let other_rules = l.rules(&other.id, None, RepeatPolicy::Allow).await;
    assert!(matches!(
        l.learning
            .create_preset(
                &l.owner.token,
                &w.id,
                &v.id,
                preset_edit("Wrong world", &set, &other_rules, vec![])
            )
            .await,
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        l.learning
            .create_preset(
                &l.owner.token,
                &w.id,
                &v.id,
                preset_edit(
                    "Broken record",
                    &set,
                    &rules,
                    vec!["ffffffffffffffffffffffffffffffff".into()]
                )
            )
            .await,
        Err(Error::Invalid(_))
    ));
    let mut missing = preset_edit("Missing revision", &set, &rules, vec![]);
    missing.learnset.version = 99;
    assert!(matches!(
        l.learning
            .create_preset(&l.owner.token, &w.id, &v.id, missing)
            .await,
        Err(Error::Invalid(_))
    ));
    assert!(l.learning.presets(&w.id, &v.id).await.unwrap().is_empty());
    l.integrity().await;
}

#[tokio::test]
async fn archived_moves_can_be_retained_only_on_their_existing_acquisition_exact_pins() {
    use web_app::application::moves::Moves;
    let l = Lab::new().await;
    let w = l.world("Retained acquisitions").await;
    let variant = l.variant(&w.id, "Owner").await;
    let form = l.form("Shared owner").await;
    let core = Moves::new(l.accounts.clone());
    for (world, owner, token) in [
        (None, form.as_str(), l.admin.token.as_str()),
        (
            Some(w.id.as_str()),
            variant.id.as_str(),
            l.owner.token.as_str(),
        ),
    ] {
        let first = l
            .move_row(
                world,
                if world.is_some() {
                    "11111111111111111111111111111111"
                } else {
                    "22222222222222222222222222222222"
                },
                "Old move metadata",
                None,
            )
            .await;
        let mut second = first.clone();
        second.revision = 2;
        second.fields = fields("Newer move metadata");
        l.write_move(&second, Some(&first)).await;
        let other = l
            .move_row(
                world,
                if world.is_some() {
                    "33333333333333333333333333333333"
                } else {
                    "44444444444444444444444444444444"
                },
                "Other Move",
                None,
            )
            .await;
        let set = l.learn(world, owner, &[&first, &second, &other]).await;
        let mut archived = second.clone();
        archived.revision = 3;
        archived.archived_at = Some(1_800_000_000);
        // The existing typed writer stages a real head advance, edit audit and
        // immutable version with its exact predecessor. Every seal stays enabled.
        l.write_move(&archived, Some(&second)).await;
        assert_eq!(core.detail(world, &first.id).await.unwrap(), archived);
        assert_eq!(core.version(world, &first.id, 1).await.unwrap(), first);
        assert_eq!(core.version(world, &first.id, 2).await.unwrap(), second);
        assert_eq!(
            l.learning
                .learnset_version(world, &set.id, 1)
                .await
                .unwrap(),
            set
        );
        let audits = l.audits().await;
        assert_eq!(
            l.learning
                .save_learnset(token, world, owner, learn_edit(&set))
                .await
                .unwrap(),
            set
        );
        assert_eq!(l.audits().await, audits);
        let mut edit = learn_edit(&set);
        edit.records.as_mut().unwrap()[2].method = Some("Edited other acquisition".into());
        let edited = l
            .learning
            .save_learnset(token, world, owner, edit)
            .await
            .unwrap();
        assert_eq!(edited.records.as_ref().unwrap()[0].move_record, first);
        assert_eq!(edited.records.as_ref().unwrap()[1].move_record, second);
        let audits = l.audits().await;
        let mut add = learn_edit(&edited);
        add.records
            .as_mut()
            .unwrap()
            .push(input(&first, NumericValue::Unknown));
        assert!(matches!(
            l.learning.save_learnset(token, world, owner, add).await,
            Err(Error::Invalid(_))
        ));
        for pin in [second.pin(), archived.pin()] {
            let mut retarget = learn_edit(&edited);
            retarget.records.as_mut().unwrap()[0].move_pin = pin;
            assert!(matches!(
                l.learning
                    .save_learnset(token, world, owner, retarget)
                    .await,
                Err(Error::Invalid(_))
            ));
        }
        let mut retarget = learn_edit(&edited);
        retarget.records.as_mut().unwrap()[2].move_pin = first.pin();
        assert!(matches!(
            l.learning
                .save_learnset(token, world, owner, retarget)
                .await,
            Err(Error::Invalid(_))
        ));
        let mut stale = learn_edit(&set);
        stale.records.as_mut().unwrap()[0].move_pin = archived.pin();
        assert_eq!(
            l.learning
                .save_learnset(token, world, owner, stale)
                .await
                .unwrap_err(),
            Error::Conflict
        );
        assert_eq!(
            l.learning
                .save_learnset(&l.other.token, world, owner, learn_edit(&edited))
                .await
                .unwrap_err(),
            Error::Forbidden
        );
        assert_eq!(l.audits().await, audits);
        let mut remove = learn_edit(&edited);
        remove.records.as_mut().unwrap().remove(2);
        remove.records.as_mut().unwrap()[0].threshold =
            NumericValue::Known("-0.125".parse().unwrap());
        let retained = l
            .learning
            .save_learnset(token, world, owner, remove)
            .await
            .unwrap();
        assert_eq!(retained.records.as_ref().unwrap().len(), 2);
        assert_eq!(retained.records.as_ref().unwrap()[0].move_record, first);
        assert_eq!(retained.records.as_ref().unwrap()[1].move_record, second);
        assert_eq!(
            retained.records.as_ref().unwrap()[0].threshold,
            NumericValue::Known("-0.125".parse().unwrap())
        );
        assert_eq!(
            l.learning
                .learnset_version(world, &set.id, 1)
                .await
                .unwrap(),
            set
        );
    }
    l.integrity().await;
}

#[tokio::test]
async fn learnset_source_copy_still_selects_moves_even_when_the_same_pin_is_already_retained() {
    let l = Lab::new().await;
    let w = l.world("Copy archive checks").await;
    let variant = l.variant(&w.id, "Owner").await;
    let form = l.form("Source owner").await;
    let shared = l
        .move_row(None, "11111111111111111111111111111111", "Shared", None)
        .await;
    let local = l
        .move_row(
            Some(&w.id),
            "22222222222222222222222222222222",
            "Local",
            Some(shared.pin()),
        )
        .await;
    let source = l.learn(None, &form, &[&shared]).await;
    let set = l.learn(Some(&w.id), &variant.id, &[&local]).await;
    let mut archived = local.clone();
    archived.revision = 2;
    archived.archived_at = Some(1_800_000_000);
    l.write_move(&archived, Some(&local)).await;
    assert_eq!(
        l.learning
            .save_learnset(&l.owner.token, Some(&w.id), &variant.id, learn_edit(&set))
            .await
            .unwrap(),
        set
    );
    let copy = LearnsetAdoptEdit {
        expected_revision: set.revision,
        source: source.pin(),
        apply_source: true,
        mappings: vec![MoveMapping {
            shared_move_id: shared.id,
            local_move: local.pin(),
        }],
        reason: None,
    };
    let audits = l.audits().await;
    assert!(matches!(
        l.learning.preview_adoption(&w.id, &variant.id, &copy).await,
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        l.learning
            .adopt_learnset(&l.owner.token, &w.id, &variant.id, copy)
            .await,
        Err(Error::Invalid(_))
    ));
    assert_eq!(l.audits().await, audits);
    assert_eq!(
        l.learning
            .learnset(Some(&w.id), &variant.id)
            .await
            .unwrap()
            .unwrap(),
        set
    );
    l.integrity().await;
}

#[tokio::test]
async fn expected_first_authorization_revocation_and_noops() {
    let l = Lab::new().await;
    let w = l.world("World").await;
    let v = l.variant(&w.id, "Variant").await;
    let form = l.form("Shared").await;
    let m = l
        .move_row(
            Some(&w.id),
            "11111111111111111111111111111111",
            "Move",
            None,
        )
        .await;
    let set = l.learn(Some(&w.id), &v.id, &[&m]).await;
    let audits = l.audits().await;
    assert_eq!(
        l.learning
            .save_learnset(&l.owner.token, Some(&w.id), &v.id, learn_edit(&set))
            .await
            .unwrap(),
        set
    );
    assert_eq!(l.audits().await, audits);
    let mut stale = learn_edit(&set);
    stale.expected_revision = 0;
    stale.records.as_mut().unwrap()[0].move_pin.version = 999;
    assert_eq!(
        l.learning
            .save_learnset(&l.owner.token, Some(&w.id), &v.id, stale)
            .await
            .unwrap_err(),
        Error::Conflict
    );
    for token in [&l.other.token, &l.viewer.token, &l.admin.token] {
        assert_eq!(
            l.learning
                .save_learnset(token, Some(&w.id), &v.id, learn_edit(&set))
                .await
                .unwrap_err(),
            Error::Forbidden
        );
    }
    assert_eq!(
        l.learning
            .save_learnset(
                &l.owner.token,
                None,
                &form,
                LearnsetEdit {
                    expected_revision: 0,
                    records: None,
                    reason: None
                }
            )
            .await
            .unwrap_err(),
        Error::Forbidden
    );
    let rules = l.rules(&w.id, None, RepeatPolicy::Allow).await;
    let preset = l
        .learning
        .create_preset(
            &l.owner.token,
            &w.id,
            &v.id,
            preset_edit("Draft", &set, &rules, vec![]),
        )
        .await
        .unwrap();
    let audits = l.audits().await;
    assert_eq!(
        l.learning
            .save_preset(
                &l.owner.token,
                &w.id,
                &v.id,
                &preset.id,
                saved_edit(&preset)
            )
            .await
            .unwrap(),
        preset
    );
    assert_eq!(l.audits().await, audits);
    let mut stale = saved_edit(&preset);
    stale.expected_revision = 0;
    stale.learnset.version = 999;
    assert_eq!(
        l.learning
            .save_preset(&l.owner.token, &w.id, &v.id, &preset.id, stale)
            .await
            .unwrap_err(),
        Error::Conflict
    );
    l.accounts
        .set_disabled(&l.owner.actor.user_id, true)
        .await
        .unwrap();
    assert_eq!(
        l.learning
            .save_learnset(&l.owner.token, Some(&w.id), &v.id, learn_edit(&set))
            .await
            .unwrap_err(),
        Error::Unauthorized
    );
    assert_eq!(
        l.learning
            .save_preset(
                &l.owner.token,
                &w.id,
                &v.id,
                &preset.id,
                saved_edit(&preset)
            )
            .await
            .unwrap_err(),
        Error::Unauthorized
    );
    assert_eq!(l.audits().await, audits);
    l.integrity().await;
}

#[tokio::test]
async fn unknown_empty_zero_and_explicit_historical_reconciliation() {
    let l = Lab::new().await;
    let w = l.world("World").await;
    let v = l.variant(&w.id, "Variant").await;
    assert_eq!(l.learning.learnset(Some(&w.id), &v.id).await.unwrap(), None);
    let first = l
        .learning
        .save_learnset(
            &l.owner.token,
            Some(&w.id),
            &v.id,
            LearnsetEdit {
                expected_revision: 0,
                records: None,
                reason: None,
            },
        )
        .await
        .unwrap();
    let rules = l.rules(&w.id, None, RepeatPolicy::Unspecified).await;
    let preset = l
        .learning
        .create_preset(
            &l.owner.token,
            &w.id,
            &v.id,
            preset_edit("Unknown", &first, &rules, vec![]),
        )
        .await
        .unwrap();
    assert!(
        l.learning
            .validation(&w.id, &v.id, &preset.id, None)
            .await
            .unwrap()
            .incomplete
            .iter()
            .any(|s| s.contains("learnset is unmodeled"))
    );
    let empty = l
        .learning
        .save_learnset(
            &l.owner.token,
            Some(&w.id),
            &v.id,
            LearnsetEdit {
                expected_revision: 1,
                records: Some(vec![]),
                reason: None,
            },
        )
        .await
        .unwrap();
    assert_ne!(first.records, empty.records);
    let m = l
        .move_row(
            Some(&w.id),
            "11111111111111111111111111111111",
            "Move",
            None,
        )
        .await;
    let next = l
        .learning
        .save_learnset(
            &l.owner.token,
            Some(&w.id),
            &v.id,
            LearnsetEdit {
                expected_revision: 2,
                records: Some(vec![input(&m, NumericValue::Known("0".parse().unwrap()))]),
                reason: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(
        next.records.as_ref().unwrap()[0].threshold,
        NumericValue::Known("0".parse().unwrap())
    );
    let mut edit = saved_edit(&preset);
    edit.learnset = empty.pin();
    let preview = l
        .learning
        .preview_preset(&w.id, &v.id, Some(&preset.id), &edit)
        .await
        .unwrap();
    assert!(
        !preview
            .report
            .incomplete
            .iter()
            .any(|s| s.contains("learnset is unmodeled"))
    );
    let revised = l
        .learning
        .save_preset(&l.owner.token, &w.id, &v.id, &preset.id, edit)
        .await
        .unwrap();
    assert_eq!(revised.learnset, empty.pin());
    assert_ne!(revised.learnset, next.pin());
    assert_eq!(
        l.learning
            .preset_version(&w.id, &v.id, &preset.id, 1)
            .await
            .unwrap(),
        preset
    );
    assert_eq!(
        l.learning
            .learnset_history(Some(&w.id), &v.id, None)
            .await
            .unwrap()
            .len(),
        3
    );
    assert_eq!(
        l.learning
            .preset_history(&w.id, &v.id, &preset.id, None)
            .await
            .unwrap()
            .len(),
        2
    );
    l.integrity().await;
}

#[tokio::test]
async fn late_failure_rolls_back_head_children_and_audit() {
    let l = Lab::new().await;
    let w = l.world("World").await;
    let v = l.variant(&w.id, "Variant").await;
    let m = l
        .move_row(
            Some(&w.id),
            "11111111111111111111111111111111",
            "Move",
            None,
        )
        .await;
    let set = l.learn(Some(&w.id), &v.id, &[&m]).await;
    let rules = l.rules(&w.id, None, RepeatPolicy::Allow).await;
    let preset = l
        .learning
        .create_preset(
            &l.owner.token,
            &w.id,
            &v.id,
            preset_edit("Draft", &set, &rules, vec![]),
        )
        .await
        .unwrap();
    let audits = l.audits().await;
    l.pool.execute("CREATE TRIGGER learning_fail BEFORE INSERT ON learnset_versions WHEN NEW.revision=2 BEGIN SELECT RAISE(ABORT,'test late failure'); END").await.unwrap();
    let mut edit = learn_edit(&set);
    edit.records.as_mut().unwrap()[0].method = Some("changed".into());
    assert!(
        l.learning
            .save_learnset(&l.owner.token, Some(&w.id), &v.id, edit)
            .await
            .is_err()
    );
    assert_eq!(l.audits().await, audits);
    assert_eq!(
        l.learning.learnset(Some(&w.id), &v.id).await.unwrap(),
        Some(set.clone())
    );
    let children: i64 =
        sqlx::query_scalar("SELECT count(*) FROM learnset_records WHERE revision=2")
            .fetch_one(&l.pool)
            .await
            .unwrap();
    assert_eq!(children, 0);
    l.pool.execute("CREATE TRIGGER preset_fail BEFORE INSERT ON moveset_preset_versions WHEN NEW.revision=2 BEGIN SELECT RAISE(ABORT,'test late failure'); END").await.unwrap();
    let mut edit = saved_edit(&preset);
    edit.entries = vec![set.records.as_ref().unwrap()[0].id.clone()];
    assert!(
        l.learning
            .save_preset(&l.owner.token, &w.id, &v.id, &preset.id, edit)
            .await
            .is_err()
    );
    assert_eq!(l.audits().await, audits);
    assert_eq!(
        l.learning.preset(&w.id, &v.id, &preset.id).await.unwrap(),
        preset
    );
    let children: i64 =
        sqlx::query_scalar("SELECT count(*) FROM moveset_preset_entries WHERE revision=2")
            .fetch_one(&l.pool)
            .await
            .unwrap();
    assert_eq!(children, 0);
    l.integrity().await;
}

#[tokio::test]
async fn immutable_seals_and_corruption_fail_closed() {
    let l = Lab::new().await;
    let w = l.world("World").await;
    let v = l.variant(&w.id, "Variant").await;
    let m = l
        .move_row(
            Some(&w.id),
            "11111111111111111111111111111111",
            "Move",
            None,
        )
        .await;
    let set = l.learn(Some(&w.id), &v.id, &[&m]).await;
    let rules = l.rules(&w.id, None, RepeatPolicy::Allow).await;
    let preset = l
        .learning
        .create_preset(
            &l.owner.token,
            &w.id,
            &v.id,
            preset_edit(
                "Draft",
                &set,
                &rules,
                vec![set.records.as_ref().unwrap()[0].id.clone()],
            ),
        )
        .await
        .unwrap();
    assert!(
        l.pool
            .execute("UPDATE learnset_records SET method='forged'")
            .await
            .is_err()
    );
    assert!(
        l.pool
            .execute("DELETE FROM learnset_records")
            .await
            .is_err()
    );
    assert!(
        l.pool
            .execute("UPDATE moveset_preset_entries SET slot=20")
            .await
            .is_err()
    );
    assert!(
        l.pool
            .execute("DELETE FROM moveset_preset_versions")
            .await
            .is_err()
    );
    assert!(l.pool.execute("INSERT INTO learnset_records SELECT scope,learnset_id,revision,2,'ffffffffffffffffffffffffffffffff',move_id,move_version,method,context,condition,threshold_state,coefficient,scale FROM learnset_records").await.is_err());
    l.pool
        .execute("DROP TRIGGER learnset_records_sealed_update")
        .await
        .unwrap();
    l.pool
        .execute("UPDATE learnset_records SET method='forged'")
        .await
        .unwrap();
    assert_eq!(
        l.learning.learnset(Some(&w.id), &v.id).await.unwrap_err(),
        Error::Unavailable
    );
    assert_eq!(
        l.learning
            .validation(&w.id, &v.id, &preset.id, None)
            .await
            .unwrap_err(),
        Error::Unavailable
    );
}

#[tokio::test]
async fn trusted_creation_baselines_preserve_edits_and_share_the_setup_transaction() {
    let l = Lab::new().await;
    let w = l.world("World").await;
    let v = l.variant(&w.id, "Variant").await;
    let rules = l.rules(&w.id, None, RepeatPolicy::Allow).await;
    let baseline = LearnsetRecord {
        id: "44444444444444444444444444444444".into(),
        world_id: Some(w.id.clone()),
        form_id: None,
        variant_id: Some(v.id.clone()),
        source: None,
        revision: 1,
        records: Some(vec![]),
        mappings: vec![],
    };
    let preset = MovesetRecord {
        id: "55555555555555555555555555555555".into(),
        world_id: w.id.clone(),
        variant_id: v.id.clone(),
        fields: fields("Seed draft"),
        revision: 1,
        archived_at: None,
        learnset: baseline.pin(),
        ruleset: rules.pin(),
        entries: vec![],
        validator_version: 1,
    };
    let audits = l.audits().await;
    let mut tx = persistence::begin_write(&l.pool).await.unwrap();
    assert!(
        l.learning
            .seed_learnset_in(&mut tx, &baseline)
            .await
            .unwrap()
    );
    assert!(l.learning.seed_preset_in(&mut tx, &preset).await.unwrap());
    tx.rollback().await.unwrap();
    assert_eq!(l.audits().await, audits);
    assert!(
        l.learning
            .learnset(Some(&w.id), &v.id)
            .await
            .unwrap()
            .is_none()
    );
    let mut tx = persistence::begin_write(&l.pool).await.unwrap();
    l.learning
        .seed_learnset_in(&mut tx, &baseline)
        .await
        .unwrap();
    l.learning.seed_preset_in(&mut tx, &preset).await.unwrap();
    tx.commit().await.unwrap();
    let revised = l
        .learning
        .save_learnset(
            &l.owner.token,
            Some(&w.id),
            &v.id,
            LearnsetEdit {
                expected_revision: 1,
                records: None,
                reason: None,
            },
        )
        .await
        .unwrap();
    let mut edit = saved_edit(&preset);
    edit.fields.display_name = "Edited draft".into();
    let revised_preset = l
        .learning
        .save_preset(&l.owner.token, &w.id, &v.id, &preset.id, edit)
        .await
        .unwrap();
    let audits = l.audits().await;
    let mut tx = persistence::begin_write(&l.pool).await.unwrap();
    assert!(
        !l.learning
            .seed_learnset_in(&mut tx, &baseline)
            .await
            .unwrap()
    );
    assert!(!l.learning.seed_preset_in(&mut tx, &preset).await.unwrap());
    let mut forged = baseline.clone();
    forged.records = None;
    assert_eq!(
        l.learning
            .seed_learnset_in(&mut tx, &forged)
            .await
            .unwrap_err(),
        Error::Unavailable
    );
    tx.commit().await.unwrap();
    assert_eq!(l.audits().await, audits);
    assert_eq!(
        l.learning.learnset(Some(&w.id), &v.id).await.unwrap(),
        Some(revised)
    );
    assert_eq!(
        l.learning.preset(&w.id, &v.id, &preset.id).await.unwrap(),
        revised_preset
    );
    l.integrity().await;
}

#[tokio::test]
async fn keep_local_source_attachment_and_same_pin_copy_are_explicit() {
    let l = Lab::new().await;
    let w = l.world("World").await;
    let v = l.variant(&w.id, "Variant").await;
    let form = l.form("Source").await;
    let other_form = l.form("Other source").await;
    let shared = l
        .move_row(None, "11111111111111111111111111111111", "Shared", None)
        .await;
    let local = l
        .move_row(
            Some(&w.id),
            "22222222222222222222222222222222",
            "Local",
            Some(shared.pin()),
        )
        .await;
    let source = l.learn(None, &form, &[&shared]).await;
    let other = l.learn(None, &other_form, &[&shared]).await;
    let original = l.learn(Some(&w.id), &v.id, &[&local, &local]).await;
    let edit = LearnsetAdoptEdit {
        expected_revision: 1,
        source: source.pin(),
        apply_source: false,
        mappings: vec![],
        reason: None,
    };
    let preview = l
        .learning
        .preview_adoption(&w.id, &v.id, &edit)
        .await
        .unwrap();
    assert!(preview.old_source.is_none());
    assert_eq!(preview.proposed.records, original.records);
    let attached = l
        .learning
        .adopt_learnset(&l.owner.token, &w.id, &v.id, edit)
        .await
        .unwrap();
    assert_eq!(attached.records, original.records);
    assert_eq!(attached.source, Some(source.pin()));
    let edit = LearnsetAdoptEdit {
        expected_revision: attached.revision,
        source: other.pin(),
        apply_source: false,
        mappings: vec![],
        reason: None,
    };
    assert!(matches!(
        l.learning.preview_adoption(&w.id, &v.id, &edit).await,
        Err(Error::Invalid(_))
    ));
    let copy = LearnsetAdoptEdit {
        expected_revision: attached.revision,
        source: source.pin(),
        apply_source: true,
        mappings: vec![MoveMapping {
            shared_move_id: shared.id.clone(),
            local_move: local.pin(),
        }],
        reason: None,
    };
    let copied = l
        .learning
        .adopt_learnset(&l.owner.token, &w.id, &v.id, copy.clone())
        .await
        .unwrap();
    assert_eq!(copied.records.as_ref().unwrap().len(), 1);
    let audits = l.audits().await;
    let mut again = copy;
    again.expected_revision = copied.revision;
    assert_eq!(
        l.learning
            .adopt_learnset(&l.owner.token, &w.id, &v.id, again)
            .await
            .unwrap(),
        copied
    );
    assert_eq!(l.audits().await, audits);
    l.integrity().await;
}

#[tokio::test]
async fn missing_snapshot_fields_are_corruption_not_unknown_and_future_policy_is_not_reinterpreted()
{
    let l = Lab::new().await;
    let w = l.world("World").await;
    let v = l.variant(&w.id, "Variant").await;
    let m = l
        .move_row(
            Some(&w.id),
            "11111111111111111111111111111111",
            "Move",
            None,
        )
        .await;
    let set = l.learn(Some(&w.id), &v.id, &[&m]).await;
    let rules = l.rules(&w.id, Some(1), RepeatPolicy::Forbid).await;
    let preset = l
        .learning
        .create_preset(
            &l.owner.token,
            &w.id,
            &v.id,
            preset_edit(
                "Draft",
                &set,
                &rules,
                vec![set.records.as_ref().unwrap()[0].id.clone(); 2],
            ),
        )
        .await
        .unwrap();
    let mut future = preset.clone();
    future.validator_version = 99;
    let report = validate_preset(&future, &set, &rules);
    assert_eq!(report.validator_version, 99);
    assert!(report.contradictions.is_empty());
    assert!(
        report
            .not_evaluated
            .iter()
            .any(|s| s.contains("unsupported"))
    );
    l.pool
        .execute("DROP TRIGGER preset_versions_sealed_update")
        .await
        .unwrap();
    l.pool.execute("UPDATE moveset_preset_versions SET after_json=json_remove(after_json,'$.content.archived_at')").await.unwrap();
    assert_eq!(
        l.learning
            .preset(&w.id, &v.id, &preset.id)
            .await
            .unwrap_err(),
        Error::Unavailable
    );
    l.pool
        .execute("DROP TRIGGER learnset_versions_sealed_update")
        .await
        .unwrap();
    l.pool
        .execute(
            "UPDATE learnset_versions SET after_json=json_remove(after_json,'$.content.records')",
        )
        .await
        .unwrap();
    assert_eq!(
        l.learning.learnset(Some(&w.id), &v.id).await.unwrap_err(),
        Error::Unavailable
    );
}

#[tokio::test]
async fn populated_0006_upgrade_preserves_creature_snapshots_users_sessions_and_audit() {
    let dir = tempfile::tempdir().unwrap();
    let pool = persistence::connect_for_migration(&format!(
        "sqlite://{}/upgrade.db",
        dir.path().display()
    ))
    .await
    .unwrap();
    for sql in [
        include_str!("../migrations/0001_auth.sql"),
        include_str!("../migrations/0002_world_authoring.sql"),
        include_str!("../migrations/0003_types.sql"),
        include_str!("../migrations/0004_charts.sql"),
        include_str!("../migrations/0005_stats.sql"),
        include_str!("../migrations/0006_stat_content.sql"),
    ] {
        sqlx::raw_sql(sql).execute(&pool).await.unwrap();
    }
    let accounts = Accounts::new(pool.clone(), 3600).unwrap();
    let admin = user(&accounts, "Admin", Role::Administrator).await;
    let owner = user(&accounts, "Owner", Role::Editor).await;
    let author = Authoring::new(accounts.clone());
    let species = author
        .create_species(&admin.token, fields("Existing species"), None)
        .await
        .unwrap();
    let world = author
        .create_world(&owner.token, fields("Existing world"), None)
        .await
        .unwrap();
    let variant = author
        .create_variant(
            &owner.token,
            &world.id,
            world.revision,
            None,
            fields("Existing variant"),
            None,
        )
        .await
        .unwrap();
    let queries = [
        "SELECT after_json FROM world_revisions ORDER BY world_id,revision",
        "SELECT after_json FROM form_versions ORDER BY form_id,revision",
        "SELECT after_json FROM form_content_snapshots ORDER BY form_id,revision",
        "SELECT after_json FROM form_stat_snapshots ORDER BY form_id,revision",
        "SELECT after_json FROM variant_revisions ORDER BY world_id,variant_id,revision",
        "SELECT after_json FROM variant_content_snapshots ORDER BY world_id,variant_id,revision",
        "SELECT after_json FROM variant_stat_snapshots ORDER BY world_id,variant_id,revision",
        "SELECT json_array(id,actor_user_id,system_key,occurred_at,action,reason) FROM audit_events ORDER BY id",
        "SELECT json_array(id,provider,provider_subject,display_name,role,is_disabled,created_at,updated_at) FROM users ORDER BY id",
        "SELECT json_array(token_hash,user_id,created_at,expires_at,revoked_at) FROM sessions ORDER BY token_hash",
    ];
    let mut retained = vec![];
    for query in queries {
        retained.push((
            query,
            sqlx::query_scalar::<_, String>(query)
                .fetch_all(&pool)
                .await
                .unwrap(),
        ));
    }
    sqlx::raw_sql(include_str!("../migrations/0007_moves_rulesets.sql"))
        .execute(&pool)
        .await
        .unwrap();
    sqlx::raw_sql(include_str!("../migrations/0008_learning.sql"))
        .execute(&pool)
        .await
        .unwrap();
    for (query, old) in retained {
        assert_eq!(
            sqlx::query_scalar::<_, String>(query)
                .fetch_all(&pool)
                .await
                .unwrap(),
            old,
            "{query}"
        );
    }
    assert!(accounts.authenticate(&owner.token).await.unwrap().is_some());
    assert_eq!(
        author.variant_detail(&world.id, &variant.id).await.unwrap(),
        variant
    );
    let learning = Learning::new(accounts);
    assert!(
        learning
            .learnset(None, &species.default_form_id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        learning
            .learnset(Some(&world.id), &variant.id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        learning
            .presets(&world.id, &variant.id)
            .await
            .unwrap()
            .is_empty()
    );
    let set = learning
        .save_learnset(
            &owner.token,
            Some(&world.id),
            &variant.id,
            LearnsetEdit {
                expected_revision: 0,
                records: Some(vec![]),
                reason: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(set.revision, 1);
    assert_eq!(
        author.variant_detail(&world.id, &variant.id).await.unwrap(),
        variant
    );
    let issues: Vec<(String, i64, String, i64)> = sqlx::query_as("PRAGMA foreign_key_check")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert!(issues.is_empty());
}
