use game_data::{moves::*, rulesets::*};
use sqlx::{Executor, SqlitePool};
use tempfile::TempDir;
use web_app::{
    accounts::Accounts,
    application::{
        authoring::{AddressKey, Authoring, Error, WorldRecord},
        moves::*,
    },
    auth::{Actor, Role, Subject},
    persistence,
};

fn pure_records() -> (MoveRecord, RulesetRecord) {
    let world = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_owned();
    let mut f = fields("Numeric design");
    f.slug = "numeric-design".into();
    let definition = MoveAttributeDefinition {
        id: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into(),
        world_id: Some(world.clone()),
        source: None,
        fields: f.clone(),
        unit: String::new(),
        revision: 1,
        archived_at: None,
        constraints: NumericConstraints::default(),
    };
    let m = MoveRecord {
        id: "cccccccccccccccccccccccccccccccc".into(),
        world_id: Some(world.clone()),
        source: None,
        fields: f.clone(),
        revision: 1,
        archived_at: None,
        attributes: Some(vec![MoveAttributeValue {
            definition: definition.clone(),
            value: NumericValue::Unknown,
        }]),
        mappings: vec![],
    };
    let r = RulesetRecord {
        id: "dddddddddddddddddddddddddddddddd".into(),
        world_id: world,
        revision: 1,
        fields: f,
        expectations: Some(vec![AttributeExpectation {
            definition,
            required: true,
            allow_not_applicable: false,
            constraints: NumericConstraints {
                minimum: Some("-1.25".parse().unwrap()),
                maximum: Some("0".parse().unwrap()),
                whole_only: false,
            },
        }]),
        stat_roster: None,
        chart: None,
        max_moves: None,
        repeat_policy: RepeatPolicy::Unspecified,
        notes: String::new(),
    };
    (m, r)
}

#[test]
fn pure_reports_distinguish_required_unknown_absent_na_and_known_restrictions() {
    let (mut m, mut r) = pure_records();
    for (state, incomplete, contradictions) in [
        (NumericValue::Unknown, 1, 0),
        (NumericValue::NotApplicable, 0, 1),
        (NumericValue::Known("0".parse().unwrap()), 0, 0),
        (NumericValue::Known("0.000001".parse().unwrap()), 0, 1),
        (NumericValue::Known("-1.25".parse().unwrap()), 0, 0),
    ] {
        m.attributes.as_mut().unwrap()[0].value = state;
        let report = validate_move(&m, &r);
        assert_eq!(report.validator_version, CURRENT_VALIDATOR_VERSION);
        assert_eq!(report.incomplete.len(), incomplete);
        assert_eq!(report.contradictions.len(), contradictions);
        assert!(!report.not_evaluated.is_empty());
        assert_eq!(report, validate_move(&m, &r));
        assert_eq!(report.inputs.len(), 3);
    }
    r.expectations.as_mut().unwrap()[0].required = false;
    m.attributes.as_mut().unwrap()[0].value = NumericValue::NotApplicable;
    assert_eq!(validate_move(&m, &r).contradictions.len(), 1);
    r.expectations.as_mut().unwrap()[0].allow_not_applicable = true;
    assert!(validate_move(&m, &r).contradictions.is_empty());
    r.expectations.as_mut().unwrap()[0].required = true;
    assert_eq!(validate_move(&m, &r).contradictions.len(), 1);
    m.attributes = Some(vec![]);
    assert_eq!(validate_move(&m, &r).incomplete.len(), 1);
    m.attributes = None;
    assert_eq!(validate_move(&m, &r).incomplete.len(), 2);
    r.expectations = None;
    assert_eq!(validate_move(&m, &r).incomplete.len(), 2);
    assert_eq!(
        ValidationReport::default().validator_version,
        CURRENT_VALIDATOR_VERSION
    );
}

#[test]
fn reports_identify_exact_contexts_and_never_retarget_mismatched_definitions() {
    let (mut m, mut r) = pure_records();
    m.attributes.as_mut().unwrap()[0].definition.revision = 2;
    m.attributes.as_mut().unwrap()[0].value = NumericValue::Known("100".parse().unwrap());
    let report = validate_move(&m, &r);
    assert!(report.incomplete[0].contains("context mismatch"));
    assert!(report.contradictions.is_empty());
    let versions: Vec<i64> = report
        .inputs
        .iter()
        .filter(|i| i.kind == "move_attribute")
        .map(|i| i.version)
        .collect();
    assert_eq!(versions, vec![1, 2]);
    let legacy = validate_move_with_policy(&m, &r, 1);
    assert_eq!(
        legacy
            .inputs
            .iter()
            .filter(|i| i.kind == "move_attribute")
            .map(|i| i.version)
            .collect::<Vec<_>>(),
        vec![2, 1]
    );
    r.world_id = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee".into();
    assert!(
        validate_move(&m, &r)
            .contradictions
            .iter()
            .any(|s| s.contains("scope"))
    );
}

#[test]
fn numeric_serde_is_tagged_strict_and_preserves_exact_decimal_boundaries() {
    for s in [
        "-9223372036854775808",
        "9223372036854775807",
        "-0.000001",
        "0",
    ] {
        let v = NumericValue::Known(s.parse().unwrap());
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(serde_json::from_str::<NumericValue>(&json).unwrap(), v);
    }
    for v in [NumericValue::Unknown, NumericValue::NotApplicable] {
        let json = serde_json::to_string(&v).unwrap();
        assert!(!json.contains("value"));
        assert_eq!(serde_json::from_str::<NumericValue>(&json).unwrap(), v);
    }
    for json in [
        r#"{"state":"known","value":0}"#,
        r#"{"state":"known","value":"0.0"}"#,
        r#"{"state":"known","value":"NaN"}"#,
        r#"{"state":"unknown","extra":1}"#,
        r#"{"state":"unknown","value":"0"}"#,
        r#"{"state":"not_applicable","value":null}"#,
    ] {
        assert!(
            serde_json::from_str::<NumericValue>(json).is_err(),
            "{json}"
        );
    }
    let (m, r) = pure_records();
    let mut json = serde_json::to_value(&m).unwrap();
    json["attributes"][0]["definition"]["extra"] = true.into();
    assert!(serde_json::from_value::<MoveRecord>(json).is_err());
    let mut json = serde_json::to_value(&r).unwrap();
    json["extra"] = true.into();
    assert!(serde_json::from_value::<RulesetRecord>(json).is_err());
}

struct User {
    actor: Actor,
    token: String,
}
struct Lab {
    pool: SqlitePool,
    accounts: Accounts,
    moves: Moves,
    author: Authoring,
    admin: User,
    owner: User,
    other: User,
    viewer: User,
    _dir: TempDir,
}
fn fields(name: &str) -> ContentFields {
    ContentFields {
        slug: "ignored / slug".into(),
        display_name: name.into(),
        description: "Synthetic move design.".into(),
    }
}
fn new_attribute(name: &str) -> NewAttribute {
    NewAttribute {
        fields: fields(name),
        unit: "points".into(),
        constraints: NumericConstraints::default(),
        source: None,
        reason: None,
    }
}
fn attribute_edit(a: &MoveAttributeDefinition) -> AttributeEdit {
    AttributeEdit {
        expected_revision: a.revision,
        fields: a.fields.clone(),
        unit: a.unit.clone(),
        constraints: a.constraints.clone(),
        reason: None,
    }
}
fn new_move(name: &str, attributes: Option<Vec<AttributeValueInput>>) -> NewMove {
    NewMove {
        fields: fields(name),
        source: None,
        attributes,
        reason: None,
    }
}
fn value(a: &MoveAttributeDefinition, v: NumericValue) -> AttributeValueInput {
    AttributeValueInput {
        attribute: a.pin(),
        value: v,
    }
}
fn move_edit(m: &MoveRecord) -> MoveEdit {
    MoveEdit {
        expected_revision: m.revision,
        fields: m.fields.clone(),
        attributes: m.attributes.as_ref().map(|a| {
            a.iter()
                .map(|a| value(&a.definition, a.value.clone()))
                .collect()
        }),
        reason: None,
    }
}
fn ruleset_edit() -> RulesetEdit {
    RulesetEdit {
        expected_revision: 0,
        fields: fields("World configuration"),
        expectations: None,
        stat_roster: None,
        chart: None,
        max_moves: None,
        repeat_policy: RepeatPolicy::Unspecified,
        notes: String::new(),
        reason: None,
    }
}
async fn user(a: &Accounts, name: &str, role: Role) -> User {
    a.invite("test", name, role, None).await.unwrap();
    let actor = a
        .sign_in(Subject {
            provider: "test".into(),
            provider_subject: name.into(),
            display_name: name.into(),
        })
        .await
        .unwrap();
    let (token, _) = a.start_session(&actor.user_id).await.unwrap();
    User { actor, token }
}
impl Lab {
    async fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let pool = persistence::connect_for_migration(&format!(
            "sqlite://{}/moves.db",
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
            moves: Moves::new(accounts.clone()),
            author: Authoring::new(accounts.clone()),
            accounts,
            pool,
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
    async fn attribute(&self, world: Option<&str>, name: &str) -> MoveAttributeDefinition {
        self.moves
            .create_attribute(
                if world.is_some() {
                    &self.owner.token
                } else {
                    &self.admin.token
                },
                world,
                new_attribute(name),
            )
            .await
            .unwrap()
    }
    async fn audits(&self) -> i64 {
        sqlx::query_scalar("SELECT count(*) FROM audit_events")
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }
    async fn archive_attribute(&self, row: &MoveAttributeDefinition) -> MoveAttributeDefinition {
        let world = row.world_id.as_deref();
        assert_eq!(self.moves.attribute(world, &row.id).await.unwrap(), *row);
        let mut archived = row.clone();
        archived.revision += 1;
        archived.archived_at = Some(1_800_000_000);
        let after = serde_json::json!({"snapshot_version":1,"content":archived}).to_string();
        let audit = format!("archive-{}-{}", row.id, archived.revision);
        let mut tx = persistence::begin_write(&self.pool).await.unwrap();
        sqlx::query("INSERT INTO audit_events(id,actor_user_id,action) VALUES (?,?,'edit')")
            .bind(&audit)
            .bind(if world.is_some() {
                &self.owner.actor.user_id
            } else {
                &self.admin.actor.user_id
            })
            .execute(&mut *tx)
            .await
            .unwrap();
        assert_eq!(sqlx::query("UPDATE move_attribute_definitions SET revision=? WHERE scope=? AND id=? AND revision=?")
            .bind(archived.revision).bind(world.unwrap_or("")).bind(&row.id).bind(row.revision)
            .execute(&mut *tx).await.unwrap().rows_affected(), 1);
        // Copy typed metadata and the exact predecessor bytes; the normal seals,
        // immutable-history triggers and deferred FKs all remain enabled.
        sqlx::query("INSERT INTO move_attribute_versions(scope,attribute_id,revision,source_scope,source_attribute_id,source_revision,slug,display_name,description,unit,archived_at,whole_only,minimum_coefficient,minimum_scale,maximum_coefficient,maximum_scale,audit_id,previous_revision,before_json,after_json) SELECT scope,attribute_id,?,source_scope,source_attribute_id,source_revision,slug,display_name,description,unit,?,whole_only,minimum_coefficient,minimum_scale,maximum_coefficient,maximum_scale,?,revision,after_json,? FROM move_attribute_versions WHERE scope=? AND attribute_id=? AND revision=?")
            .bind(archived.revision).bind(archived.archived_at).bind(audit).bind(after)
            .bind(world.unwrap_or("")).bind(&row.id).bind(row.revision).execute(&mut *tx).await.unwrap();
        tx.commit().await.unwrap();
        assert_eq!(
            self.moves.attribute(world, &row.id).await.unwrap(),
            archived
        );
        assert_eq!(
            self.moves
                .attribute_version(world, &row.id, row.revision)
                .await
                .unwrap(),
            *row
        );
        archived
    }
    async fn archive_empty_chart(
        &self,
        row: &game_data::charts::ChartRecord,
    ) -> game_data::charts::ChartRecord {
        assert!(row.roster.is_empty() && row.cells.is_empty() && row.mapping.is_empty());
        let mut archived = row.clone();
        archived.revision += 1;
        archived.archived_at = Some(1_800_000_000);
        let audit = format!("archive-chart-{}-{}", row.id, archived.revision);
        let after = serde_json::json!({"snapshot_version":1,"content":archived}).to_string();
        let mut tx = persistence::begin_write(&self.pool).await.unwrap();
        sqlx::query("INSERT INTO audit_events(id,actor_user_id,action) VALUES (?,?,'edit')")
            .bind(&audit)
            .bind(&self.owner.actor.user_id)
            .execute(&mut *tx)
            .await
            .unwrap();
        assert_eq!(
            sqlx::query("UPDATE charts SET revision=? WHERE scope=? AND id=? AND revision=?")
                .bind(archived.revision)
                .bind(row.world_id.as_deref().unwrap_or(""))
                .bind(&row.id)
                .bind(row.revision)
                .execute(&mut *tx)
                .await
                .unwrap()
                .rows_affected(),
            1
        );
        // The empty children are sealed normally; copy the predecessor's exact
        // bytes, typed metadata and source pin without changing any guards.
        sqlx::query("INSERT INTO chart_versions(scope,chart_id,revision,source_scope,source_chart_id,source_revision,slug,display_name,description,archived_at,audit_id,previous_revision,before_json,after_json) SELECT scope,chart_id,?,source_scope,source_chart_id,source_revision,slug,display_name,description,?,?,revision,after_json,? FROM chart_versions WHERE scope=? AND chart_id=? AND revision=?")
            .bind(archived.revision).bind(archived.archived_at).bind(audit).bind(after)
            .bind(row.world_id.as_deref().unwrap_or("")).bind(&row.id).bind(row.revision)
            .execute(&mut *tx).await.unwrap();
        tx.commit().await.unwrap();
        let charts = web_app::application::charts::Charts::new(self.accounts.clone());
        assert_eq!(
            charts
                .detail(row.world_id.as_deref(), &row.id)
                .await
                .unwrap(),
            archived
        );
        assert_eq!(
            charts
                .version(row.world_id.as_deref(), &row.id, row.revision)
                .await
                .unwrap(),
            *row
        );
        archived
    }
    async fn integrity(&self) {
        let rows: Vec<(String, i64, String, i64)> = sqlx::query_as("PRAGMA foreign_key_check")
            .fetch_all(&self.pool)
            .await
            .unwrap();
        assert!(rows.is_empty(), "{rows:?}");
    }
}

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

#[tokio::test]
async fn numeric_states_unmodeled_empty_and_exact_versions_survive_roundtrips() {
    let l = Lab::new().await;
    let w = l.world("States").await;
    let mut n = new_attribute("  Signed   energy  ");
    n.unit = " point ".into();
    n.constraints = NumericConstraints {
        minimum: Some("-1.25".parse().unwrap()),
        maximum: Some("0".parse().unwrap()),
        whole_only: false,
    };
    let a = l
        .moves
        .create_attribute(&l.owner.token, Some(&w.id), n)
        .await
        .unwrap();
    assert_eq!(a.fields.slug, "signed-energy");
    assert_eq!(a.unit, "point");
    let mut m = l
        .moves
        .create(&l.owner.token, Some(&w.id), new_move("Stateful", None))
        .await
        .unwrap();
    let baseline = m.clone();
    for state in [
        Some(vec![]),
        Some(vec![value(&a, NumericValue::Unknown)]),
        Some(vec![value(&a, NumericValue::Known("0".parse().unwrap()))]),
        Some(vec![value(&a, NumericValue::NotApplicable)]),
        None,
    ] {
        let mut e = move_edit(&m);
        e.attributes = state;
        m = l
            .moves
            .save(&l.owner.token, Some(&w.id), &m.id, e)
            .await
            .unwrap();
        assert_eq!(l.moves.detail(Some(&w.id), &m.id).await.unwrap(), m);
    }
    assert_eq!(m.revision, 6);
    assert_eq!(
        l.moves.version(Some(&w.id), &m.id, 1).await.unwrap(),
        baseline
    );
    let before = l.audits().await;
    assert_eq!(
        l.moves
            .save(&l.owner.token, Some(&w.id), &m.id, move_edit(&m))
            .await
            .unwrap(),
        m
    );
    assert_eq!(l.audits().await, before);
    let mut e = attribute_edit(&a);
    e.constraints.whole_only = true;
    let a2 = l
        .moves
        .save_attribute(&l.owner.token, Some(&w.id), &a.id, e)
        .await
        .unwrap();
    assert_eq!(a2.revision, 2);
    assert_eq!(
        l.moves
            .attribute_version(Some(&w.id), &a.id, 1)
            .await
            .unwrap(),
        a
    );
    let historical = l.moves.version(Some(&w.id), &m.id, 3).await.unwrap();
    assert_eq!(historical.attributes.unwrap()[0].definition, a);
    let history = l.moves.history(Some(&w.id), &m.id, None).await.unwrap();
    assert_eq!(history.len(), 6);
    assert_eq!(history[0].actor_name, "Owner");
    assert_eq!(
        history[0].before_json.as_ref(),
        Some(&history[1].after_json)
    );
    assert_eq!(
        l.moves
            .history(Some(&w.id), &m.id, Some(3))
            .await
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        l.moves
            .history_entry(Some(&w.id), &m.id, 2)
            .await
            .unwrap()
            .revision,
        2
    );
    assert_eq!(
        l.moves
            .resolve_move(Some(&w.id), AddressKey::Slug(m.fields.slug.clone()))
            .await
            .unwrap(),
        m
    );
    assert_eq!(
        l.moves
            .resolve_attribute(Some(&w.id), AddressKey::Slug(a2.fields.slug.clone()))
            .await
            .unwrap(),
        a2
    );
    l.integrity().await;
}

#[tokio::test]
async fn intrinsic_numeric_constraints_and_scope_are_errors_with_no_partial_audit() {
    let l = Lab::new().await;
    let w = l.world("Scope").await;
    let w2 = l.world("Other scope").await;
    let shared = l.attribute(None, "Number").await;
    let mut n = new_attribute("Local whole");
    n.constraints = NumericConstraints {
        whole_only: true,
        minimum: Some("-2".parse().unwrap()),
        maximum: Some("2".parse().unwrap()),
    };
    let local = l
        .moves
        .create_attribute(&l.owner.token, Some(&w.id), n)
        .await
        .unwrap();
    let before = l.audits().await;
    for (world, a, v) in [
        (Some(w.id.as_str()), &local, "2.000001"),
        (Some(w.id.as_str()), &local, "0.5"),
        (Some(w2.id.as_str()), &local, "0"),
        (None, &local, "0"),
        (Some(w.id.as_str()), &shared, "0"),
    ] {
        let result = l
            .moves
            .create(
                if world.is_none() {
                    &l.admin.token
                } else {
                    &l.owner.token
                },
                world,
                new_move(
                    "Invalid",
                    Some(vec![value(a, NumericValue::Known(v.parse().unwrap()))]),
                ),
            )
            .await;
        assert!(matches!(result, Err(Error::Invalid(_))), "{result:?}");
    }
    let mut bad = local.pin();
    bad.version = 99;
    assert!(matches!(
        l.moves
            .create(
                &l.owner.token,
                Some(&w.id),
                new_move(
                    "Broken",
                    Some(vec![AttributeValueInput {
                        attribute: bad,
                        value: NumericValue::Unknown
                    }])
                )
            )
            .await,
        Err(Error::Invalid(_))
    ));
    assert!(
        l.moves
            .create(
                &l.owner.token,
                Some(&w.id),
                new_move(
                    "Duplicate",
                    Some(vec![
                        value(&local, NumericValue::Unknown),
                        value(&local, NumericValue::NotApplicable)
                    ])
                )
            )
            .await
            .is_err()
    );
    assert_eq!(l.audits().await, before);
    assert!(l.moves.list(Some(&w.id)).await.unwrap().is_empty());
    l.integrity().await;
}

#[tokio::test]
async fn policy_precedes_stale_and_dependencies_and_revocation_is_checked_under_lock() {
    let l = Lab::new().await;
    let w = l.world("Policy").await;
    let a = l.attribute(Some(&w.id), "A").await;
    let m = l
        .moves
        .create(&l.owner.token, Some(&w.id), new_move("M", None))
        .await
        .unwrap();
    let mut e = move_edit(&m);
    e.expected_revision = 0;
    e.attributes = Some(vec![AttributeValueInput {
        attribute: AttributePin {
            attribute_id: "bad".into(),
            version: -1,
        },
        value: NumericValue::Unknown,
    }]);
    for token in [&l.admin.token, &l.other.token, &l.viewer.token] {
        assert_eq!(
            l.moves.save(token, Some(&w.id), &m.id, e.clone()).await,
            Err(Error::Forbidden)
        );
        assert_eq!(
            l.moves
                .create_attribute(token, Some(&w.id), new_attribute("No access"))
                .await,
            Err(Error::Forbidden)
        );
        assert_eq!(
            l.moves.save_ruleset(token, &w.id, ruleset_edit()).await,
            Err(Error::Forbidden)
        );
    }
    assert_eq!(
        l.moves.save(&l.owner.token, Some(&w.id), &m.id, e).await,
        Err(Error::Conflict)
    );
    assert_eq!(
        l.moves
            .create_attribute(&l.owner.token, None, new_attribute("Shared"))
            .await,
        Err(Error::Forbidden)
    );
    let mut e = attribute_edit(&a);
    e.expected_revision = 0;
    e.constraints.minimum = Some("3".parse().unwrap());
    e.constraints.maximum = Some("1".parse().unwrap());
    assert_eq!(
        l.moves
            .save_attribute(&l.owner.token, Some(&w.id), &a.id, e)
            .await,
        Err(Error::Conflict)
    );
    let before = l.audits().await;
    l.accounts
        .set_disabled(&l.owner.actor.user_id, true)
        .await
        .unwrap();
    assert_eq!(
        l.moves
            .save(&l.owner.token, Some(&w.id), &m.id, move_edit(&m))
            .await,
        Err(Error::Unauthorized)
    );
    assert_eq!(l.audits().await, before);
}

#[tokio::test]
async fn source_adoption_is_explicit_injective_and_preserves_all_states() {
    let l = Lab::new().await;
    let w = l.world("Adoption").await;
    let a = l.attribute(None, "Shared attribute").await;
    let b = l.attribute(None, "Unmapped source").await;
    let mut n = new_attribute("Local");
    n.source = Some(a.pin());
    let local = l
        .moves
        .create_attribute(&l.owner.token, Some(&w.id), n)
        .await
        .unwrap();
    let mut n = new_attribute("Duplicate provenance");
    n.source = Some(a.pin());
    let duplicate = l
        .moves
        .create_attribute(&l.owner.token, Some(&w.id), n)
        .await
        .unwrap();
    let source = l
        .moves
        .create(
            &l.admin.token,
            None,
            new_move(
                "Source",
                Some(vec![
                    value(&a, NumericValue::Unknown),
                    value(&b, NumericValue::Known("0".parse().unwrap())),
                ]),
            ),
        )
        .await
        .unwrap();
    let mut n = new_move(
        "Local move",
        Some(vec![value(&duplicate, NumericValue::NotApplicable)]),
    );
    n.source = Some(source.pin());
    let m = l
        .moves
        .create(&l.owner.token, Some(&w.id), n)
        .await
        .unwrap();
    let mut edit = MoveAdoptEdit {
        expected_revision: 1,
        target_version: 1,
        fields: m.fields.clone(),
        apply_source: false,
        mappings: vec![AttributeMapping {
            shared_attribute_id: a.id.clone(),
            local_attribute: local.pin(),
        }],
        reason: None,
    };
    let p = l.moves.adoption_preview(&w.id, &m.id, &edit).await.unwrap();
    assert_eq!(p.proposed.attributes, m.attributes);
    assert_eq!(p.unmapped.len(), 1);
    assert!(p.removed.is_empty());
    edit.apply_source = true;
    let p = l.moves.adoption_preview(&w.id, &m.id, &edit).await.unwrap();
    assert_eq!(p.removed.len(), 1);
    assert_eq!(p.unmapped[0].definition, b);
    let adopted = l
        .moves
        .adopt(&l.owner.token, &w.id, &m.id, edit.clone())
        .await
        .unwrap();
    assert_eq!(adopted, p.proposed);
    assert_eq!(
        adopted.attributes.as_ref().unwrap()[0].value,
        NumericValue::Unknown
    );
    edit.expected_revision = 2;
    let before = l.audits().await;
    assert_eq!(
        l.moves
            .adopt(&l.owner.token, &w.id, &m.id, edit.clone())
            .await
            .unwrap(),
        adopted
    );
    assert_eq!(l.audits().await, before);
    edit.mappings.push(AttributeMapping {
        shared_attribute_id: a.id.clone(),
        local_attribute: duplicate.pin(),
    });
    assert!(matches!(
        l.moves
            .adopt(&l.owner.token, &w.id, &m.id, edit.clone())
            .await,
        Err(Error::Invalid(_))
    ));
    edit.mappings.truncate(1);
    for state in [
        NumericValue::NotApplicable,
        NumericValue::Known("0".parse().unwrap()),
    ] {
        let current = l.moves.detail(None, &source.id).await.unwrap();
        let mut e = move_edit(&current);
        e.attributes = Some(vec![value(&a, state.clone())]);
        let source = l
            .moves
            .save(&l.admin.token, None, &source.id, e)
            .await
            .unwrap();
        edit.target_version = source.revision;
        let local_move = l
            .moves
            .adopt(&l.owner.token, &w.id, &m.id, edit.clone())
            .await
            .unwrap();
        assert_eq!(local_move.attributes.as_ref().unwrap()[0].value, state);
        edit.expected_revision = local_move.revision;
    }
    let current = l.moves.detail(None, &source.id).await.unwrap();
    let mut e = move_edit(&current);
    e.attributes = None;
    let source = l
        .moves
        .save(&l.admin.token, None, &source.id, e)
        .await
        .unwrap();
    edit.target_version = source.revision;
    edit.mappings.clear();
    let adopted = l
        .moves
        .adopt(&l.owner.token, &w.id, &m.id, edit)
        .await
        .unwrap();
    assert!(adopted.attributes.is_none());
    assert_eq!(l.moves.version(Some(&w.id), &m.id, 1).await.unwrap(), m);
    l.integrity().await;
}

#[tokio::test]
async fn adoption_checks_exact_local_constraints_and_definition_adoption_does_not_retarget_moves() {
    let l = Lab::new().await;
    let w = l.world("Constraints").await;
    let a = l.attribute(None, "A").await;
    let mut n = new_attribute("Local A");
    n.source = Some(a.pin());
    n.constraints.maximum = Some("1".parse().unwrap());
    let local = l
        .moves
        .create_attribute(&l.owner.token, Some(&w.id), n)
        .await
        .unwrap();
    let source = l
        .moves
        .create(
            &l.admin.token,
            None,
            new_move(
                "Source",
                Some(vec![value(&a, NumericValue::Known("2".parse().unwrap()))]),
            ),
        )
        .await
        .unwrap();
    let mut n = new_move(
        "Local move",
        Some(vec![value(&local, NumericValue::Unknown)]),
    );
    n.source = Some(source.pin());
    let m = l
        .moves
        .create(&l.owner.token, Some(&w.id), n)
        .await
        .unwrap();
    let before = l.audits().await;
    assert!(matches!(
        l.moves
            .adopt(
                &l.owner.token,
                &w.id,
                &m.id,
                MoveAdoptEdit {
                    expected_revision: 1,
                    target_version: 1,
                    fields: m.fields.clone(),
                    apply_source: true,
                    mappings: vec![AttributeMapping {
                        shared_attribute_id: a.id.clone(),
                        local_attribute: local.pin()
                    }],
                    reason: None
                }
            )
            .await,
        Err(Error::Invalid(_))
    ));
    assert_eq!(l.audits().await, before);
    let mut e = attribute_edit(&a);
    e.unit = "turns".into();
    let a2 = l
        .moves
        .save_attribute(&l.admin.token, None, &a.id, e)
        .await
        .unwrap();
    let p = l
        .moves
        .preview_attribute_adoption(&w.id, &local.id, 2, &attribute_edit(&local))
        .await
        .unwrap();
    assert_eq!(p.old_source, a);
    assert_eq!(p.new_source, a2);
    let adopted = l
        .moves
        .adopt_attribute(&l.owner.token, &w.id, &local.id, 2, attribute_edit(&local))
        .await
        .unwrap();
    assert_eq!(adopted.unit, local.unit);
    assert_eq!(adopted.source, Some(a2.pin()));
    assert_eq!(l.moves.detail(Some(&w.id), &m.id).await.unwrap(), m);
    assert_eq!(
        l.moves
            .attribute_history(Some(&w.id), &local.id, None)
            .await
            .unwrap()
            .len(),
        2
    );
}

#[tokio::test]
async fn singleton_rulesets_are_independent_and_pin_exact_contexts() {
    let l = Lab::new().await;
    let w = l.world("Rules").await;
    let other = l.world("Other").await;
    let a = l.attribute(Some(&w.id), "A").await;
    let elsewhere = l.attribute(Some(&other.id), "Elsewhere").await;
    assert!(l.moves.ruleset(&w.id).await.unwrap().is_none());
    let mut e = ruleset_edit();
    e.expectations = Some(vec![ExpectationInput {
        attribute: a.pin(),
        required: true,
        allow_not_applicable: false,
        constraints: NumericConstraints {
            maximum: Some("5".parse().unwrap()),
            ..Default::default()
        },
    }]);
    e.max_moves = Some(7);
    e.repeat_policy = RepeatPolicy::Forbid;
    e.notes = "Conditions are inert.".into();
    let first = l
        .moves
        .save_ruleset(&l.owner.token, &w.id, e.clone())
        .await
        .unwrap();
    assert_eq!(
        l.author.world_detail(&w.id).await.unwrap().revision,
        w.revision
    );
    e.expected_revision = 1;
    let before = l.audits().await;
    assert_eq!(
        l.moves
            .save_ruleset(&l.owner.token, &w.id, e.clone())
            .await
            .unwrap(),
        first
    );
    assert_eq!(l.audits().await, before);
    e.expected_revision = 0;
    e.expectations.as_mut().unwrap()[0].attribute = elsewhere.pin();
    assert_eq!(
        l.moves.save_ruleset(&l.owner.token, &w.id, e.clone()).await,
        Err(Error::Conflict)
    );
    e.expected_revision = 1;
    assert!(matches!(
        l.moves.save_ruleset(&l.owner.token, &w.id, e.clone()).await,
        Err(Error::Invalid(_))
    ));
    e.expectations = Some(vec![]);
    e.max_moves = Some(0);
    assert!(matches!(
        l.moves.save_ruleset(&l.owner.token, &w.id, e.clone()).await,
        Err(Error::Invalid(_))
    ));
    e.max_moves = None;
    let second = l
        .moves
        .save_ruleset(&l.owner.token, &w.id, e)
        .await
        .unwrap();
    assert_eq!(second.id, first.id);
    assert_eq!(second.revision, 2);
    assert_eq!(
        l.moves.ruleset_version(&w.id, &first.id, 1).await.unwrap(),
        first
    );
    assert_eq!(l.moves.ruleset_history(&w.id, None).await.unwrap().len(), 2);
    assert!(matches!(
        l.moves.ruleset_version(&other.id, &first.id, 1).await,
        Err(Error::NotFound)
    ));
    l.integrity().await;
}

#[tokio::test]
async fn late_failure_rolls_back_move_head_children_and_audit() {
    let l = Lab::new().await;
    let a = l.attribute(None, "A").await;
    let m = l
        .moves
        .create(&l.admin.token, None, new_move("M", None))
        .await
        .unwrap();
    let before = l.audits().await;
    l.pool.execute("CREATE TRIGGER reject_move_seal BEFORE INSERT ON move_versions WHEN NEW.revision=2 BEGIN SELECT RAISE(ABORT,'test late failure'); END").await.unwrap();
    let mut e = move_edit(&m);
    e.attributes = Some(vec![value(&a, NumericValue::Known("0".parse().unwrap()))]);
    assert!(l.moves.save(&l.admin.token, None, &m.id, e).await.is_err());
    assert_eq!(l.audits().await, before);
    assert_eq!(l.moves.detail(None, &m.id).await.unwrap(), m);
    let children: i64 = sqlx::query_scalar("SELECT count(*) FROM move_attribute_bindings")
        .fetch_one(&l.pool)
        .await
        .unwrap();
    assert_eq!(children, 0);
    l.integrity().await;
}

#[tokio::test]
async fn sealed_children_and_versions_reject_changes_and_corruption_is_not_unknown() {
    let l = Lab::new().await;
    let a = l.attribute(None, "A").await;
    let m = l
        .moves
        .create(
            &l.admin.token,
            None,
            new_move("M", Some(vec![value(&a, NumericValue::Unknown)])),
        )
        .await
        .unwrap();
    assert!(
        sqlx::query("UPDATE move_attribute_bindings SET state='not_applicable' WHERE move_id=?")
            .bind(&m.id)
            .execute(&l.pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("DELETE FROM move_versions WHERE move_id=?")
            .bind(&m.id)
            .execute(&l.pool)
            .await
            .is_err()
    );
    assert!(sqlx::query("INSERT INTO move_attribute_bindings(scope,move_id,revision,slot,attribute_id,attribute_version,state) VALUES ('',?,1,2,?,1,'unknown')").bind(&m.id).bind(&a.id).execute(&l.pool).await.is_err());
    l.pool
        .execute("DROP TRIGGER move_attribute_bindings_update")
        .await
        .unwrap();
    sqlx::query("UPDATE move_attribute_bindings SET state='not_applicable' WHERE move_id=?")
        .bind(&m.id)
        .execute(&l.pool)
        .await
        .unwrap();
    assert_eq!(l.moves.detail(None, &m.id).await, Err(Error::Unavailable));
    assert_eq!(
        l.moves.version(None, &m.id, 1).await,
        Err(Error::Unavailable)
    );
    assert!(l.moves.history(None, &m.id, None).await.is_err());
}

#[tokio::test]
async fn head_snapshot_predecessor_audit_and_dependencies_are_verified() {
    for corruption in ["head", "snapshot", "predecessor", "audit", "dependency"] {
        let l = Lab::new().await;
        let a = l.attribute(None, "A").await;
        let m = l
            .moves
            .create(
                &l.admin.token,
                None,
                new_move("M", Some(vec![value(&a, NumericValue::Unknown)])),
            )
            .await
            .unwrap();
        let mut e = move_edit(&m);
        e.fields.display_name = "M two".into();
        let m2 = l.moves.save(&l.admin.token, None, &m.id, e).await.unwrap();
        match corruption {
            "head" => {
                sqlx::query("UPDATE moves SET slug='wrong' WHERE id=?")
                    .bind(&m.id)
                    .execute(&l.pool)
                    .await
                    .unwrap();
            }
            "snapshot" => {
                l.pool
                    .execute("DROP TRIGGER move_versions_update")
                    .await
                    .unwrap();
                sqlx::query("UPDATE move_versions SET after_json=json_set(after_json,'$.content.extra',1) WHERE move_id=? AND revision=2").bind(&m.id).execute(&l.pool).await.unwrap();
            }
            "predecessor" => {
                l.pool
                    .execute("DROP TRIGGER move_versions_update")
                    .await
                    .unwrap();
                sqlx::query("UPDATE move_versions SET before_json=after_json WHERE move_id=? AND revision=2").bind(&m.id).execute(&l.pool).await.unwrap();
            }
            "audit" => {
                l.pool.execute("DROP TRIGGER audit_update").await.unwrap();
                sqlx::query("UPDATE audit_events SET action='create' WHERE id=(SELECT audit_id FROM move_versions WHERE move_id=? AND revision=2)").bind(&m.id).execute(&l.pool).await.unwrap();
            }
            _ => {
                l.pool
                    .execute("DROP TRIGGER move_attribute_versions_update")
                    .await
                    .unwrap();
                sqlx::query(
                    "UPDATE move_attribute_versions SET unit='corrupt' WHERE attribute_id=?",
                )
                .bind(&a.id)
                .execute(&l.pool)
                .await
                .unwrap();
            }
        }
        assert_eq!(
            l.moves.detail(None, &m2.id).await,
            Err(Error::Unavailable),
            "{corruption}"
        );
        assert_eq!(
            l.moves.version(None, &m2.id, 1).await,
            Err(Error::Unavailable),
            "{corruption}"
        );
    }
}

#[path = "support/legacy.rs"]
mod legacy;

#[tokio::test]
async fn populated_0006_upgrade_preserves_existing_bytes_and_seeds_are_atomic_idempotent() {
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
    let source_path = dir.path().join("source.db");
    let source = persistence::connect_for_migration(&format!("sqlite://{}", source_path.display()))
        .await
        .unwrap();
    persistence::migrate(&source).await.unwrap();
    let accounts = Accounts::new(source.clone(), 3600).unwrap();
    let owner = user(&accounts, "Owner", Role::Editor).await;
    let author = Authoring::new(accounts.clone());
    let world = author
        .create_world(&owner.token, fields("Existing world"), None)
        .await
        .unwrap();
    legacy::copy_core(&source_path, &pool).await;
    let accounts = Accounts::new(pool.clone(), 3600).unwrap();
    let author = Authoring::new(accounts.clone());
    let bytes: String =
        sqlx::query_scalar("SELECT after_json FROM world_revisions WHERE world_id=?")
            .bind(&world.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    sqlx::raw_sql(include_str!("../migrations/0007_moves_rulesets.sql"))
        .execute(&pool)
        .await
        .unwrap();
    for sql in [
        include_str!("../migrations/0008_learning.sql"),
        include_str!("../migrations/0009_cards.sql"),
        include_str!("../migrations/0010_records.sql"),
    ] {
        sqlx::raw_sql(sql).execute(&pool).await.unwrap();
    }
    assert_eq!(author.world_detail(&world.id).await.unwrap(), world);
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT after_json FROM world_revisions WHERE world_id=?")
            .bind(&world.id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        bytes
    );
    let moves = Moves::new(accounts);
    let mut f = fields("Seed attribute");
    f.slug = "seed-attribute".into();
    let a = MoveAttributeDefinition {
        id: "11111111111111111111111111111111".into(),
        world_id: Some(world.id.clone()),
        source: None,
        fields: f.clone(),
        unit: String::new(),
        revision: 1,
        archived_at: None,
        constraints: NumericConstraints::default(),
    };
    f.slug = "seed-move".into();
    let m = MoveRecord {
        id: "22222222222222222222222222222222".into(),
        world_id: Some(world.id.clone()),
        source: None,
        fields: f.clone(),
        revision: 1,
        archived_at: None,
        attributes: Some(vec![MoveAttributeValue {
            definition: a.clone(),
            value: NumericValue::NotApplicable,
        }]),
        mappings: vec![],
    };
    f.slug = "seed-rules".into();
    let r = RulesetRecord {
        id: "33333333333333333333333333333333".into(),
        world_id: world.id.clone(),
        revision: 1,
        fields: f,
        expectations: Some(vec![]),
        stat_roster: None,
        chart: None,
        max_moves: None,
        repeat_policy: RepeatPolicy::Unspecified,
        notes: String::new(),
    };
    let mut tx = persistence::begin_write(&pool).await.unwrap();
    assert!(moves.seed_attribute_in(&mut tx, &a).await.unwrap());
    assert!(moves.seed_move_in(&mut tx, &m).await.unwrap());
    assert!(moves.seed_ruleset_in(&mut tx, &r).await.unwrap());
    tx.rollback().await.unwrap();
    assert!(moves.list(Some(&world.id)).await.unwrap().is_empty());
    assert!(moves.attributes(Some(&world.id)).await.unwrap().is_empty());
    assert!(moves.ruleset(&world.id).await.unwrap().is_none());
    let mut tx = persistence::begin_write(&pool).await.unwrap();
    moves.seed_attribute_in(&mut tx, &a).await.unwrap();
    moves.seed_move_in(&mut tx, &m).await.unwrap();
    moves.seed_ruleset_in(&mut tx, &r).await.unwrap();
    tx.commit().await.unwrap();
    let mut e = move_edit(&m);
    e.attributes = None;
    let edited = moves
        .save(&owner.token, Some(&world.id), &m.id, e)
        .await
        .unwrap();
    let mut tx = persistence::begin_write(&pool).await.unwrap();
    assert!(!moves.seed_attribute_in(&mut tx, &a).await.unwrap());
    assert!(!moves.seed_move_in(&mut tx, &m).await.unwrap());
    assert!(!moves.seed_ruleset_in(&mut tx, &r).await.unwrap());
    let mut collision = m.clone();
    collision.attributes = None;
    assert!(moves.seed_move_in(&mut tx, &collision).await.is_err());
    tx.commit().await.unwrap();
    assert_eq!(moves.detail(Some(&world.id), &m.id).await.unwrap(), edited);
    // Archive via the same typed writer protocol, retaining exact historical consumers.
    let mut archived = a.clone();
    archived.revision = 2;
    archived.archived_at = Some(1_800_000_000);
    let mut tx = persistence::begin_write(&pool).await.unwrap();
    // Seed helpers cannot masquerade as an archive mutation.
    assert!(moves.seed_attribute_in(&mut tx, &archived).await.is_err());
    tx.rollback().await.unwrap();
    assert_eq!(moves.version(Some(&world.id), &m.id, 1).await.unwrap(), m);
}

#[tokio::test]
async fn archived_heads_are_not_selectable_but_exact_consumers_and_history_remain_readable() {
    let l = Lab::new().await;
    let a = l.attribute(None, "Archive A").await;
    let m = l
        .moves
        .create(
            &l.admin.token,
            None,
            new_move("M", Some(vec![value(&a, NumericValue::Unknown)])),
        )
        .await
        .unwrap();
    let archived = l.archive_attribute(&a).await;
    assert_eq!(l.moves.attribute(None, &a.id).await.unwrap(), archived);
    assert_eq!(l.moves.attribute_version(None, &a.id, 1).await.unwrap(), a);
    assert_eq!(l.moves.detail(None, &m.id).await.unwrap(), m);
    assert_eq!(
        l.moves
            .attribute_history(None, &a.id, None)
            .await
            .unwrap()
            .len(),
        2
    );
    assert!(matches!(
        l.moves
            .create(
                &l.admin.token,
                None,
                new_move("No selection", Some(vec![value(&a, NumericValue::Unknown)]))
            )
            .await,
        Err(Error::Invalid(_))
    ));
    let world = l.world("Archive source").await;
    let mut n = new_attribute("Cannot adopt archived");
    n.source = Some(a.pin());
    assert!(matches!(
        l.moves
            .create_attribute(&l.owner.token, Some(&world.id), n)
            .await,
        Err(Error::Invalid(_))
    ));
    l.integrity().await;
}

#[tokio::test]
async fn retained_archived_attribute_pins_allow_edits_but_not_new_selections() {
    let l = Lab::new().await;
    let w = l.world("Retained pins").await;
    for world in [None, Some(w.id.as_str())] {
        let token = if world.is_some() {
            &l.owner.token
        } else {
            &l.admin.token
        };
        let mut n = new_attribute("Pinned limits");
        n.constraints.maximum = Some("10".parse().unwrap());
        let a = l.moves.create_attribute(token, world, n).await.unwrap();
        let b = l.attribute(world, "Other binding").await;
        let m = l
            .moves
            .create(
                token,
                world,
                new_move(
                    "Retained",
                    Some(vec![
                        value(&a, NumericValue::Unknown),
                        value(&b, NumericValue::Unknown),
                    ]),
                ),
            )
            .await
            .unwrap();
        let other = l
            .moves
            .create(
                token,
                world,
                new_move(
                    "Different consumer",
                    Some(vec![value(&b, NumericValue::Unknown)]),
                ),
            )
            .await
            .unwrap();
        let mut e = attribute_edit(&a);
        e.fields.display_name = "New head metadata".into();
        e.unit = "new unit".into();
        e.constraints = NumericConstraints {
            whole_only: true,
            minimum: Some("100".parse().unwrap()),
            maximum: Some("200".parse().unwrap()),
        };
        let newer = l
            .moves
            .save_attribute(token, world, &a.id, e)
            .await
            .unwrap();
        let archived = l.archive_attribute(&newer).await;
        assert_eq!(archived.revision, 3);
        assert_eq!(l.moves.version(world, &m.id, 1).await.unwrap(), m);
        let audits = l.audits().await;
        assert_eq!(
            l.moves
                .save(token, world, &m.id, move_edit(&m))
                .await
                .unwrap(),
            m
        );
        assert_eq!(l.audits().await, audits);

        let mut e = move_edit(&m);
        e.fields.description = "Unrelated metadata change".into();
        let edited = l.moves.save(token, world, &m.id, e).await.unwrap();
        let mut e = move_edit(&edited);
        e.attributes.as_mut().unwrap()[0].value = NumericValue::Known("6.25".parse().unwrap());
        e.attributes.as_mut().unwrap()[1].value = NumericValue::NotApplicable;
        let edited = l.moves.save(token, world, &m.id, e).await.unwrap();
        assert_eq!(edited.attributes.as_ref().unwrap()[0].definition, a);
        let mut e = move_edit(&edited);
        e.attributes.as_mut().unwrap().remove(1);
        let edited = l.moves.save(token, world, &m.id, e).await.unwrap();
        assert_eq!(edited.attributes.as_ref().unwrap()[0].definition, a);

        let audits = l.audits().await;
        let mut invalid = move_edit(&edited);
        invalid.attributes.as_mut().unwrap()[0].value = NumericValue::Known("150".parse().unwrap());
        assert!(matches!(
            l.moves.save(token, world, &m.id, invalid).await,
            Err(Error::Invalid(_))
        ));
        for pin in [newer.pin(), archived.pin()] {
            let mut retarget = move_edit(&edited);
            retarget.attributes.as_mut().unwrap()[0].attribute = pin;
            retarget.attributes.as_mut().unwrap()[0].value = NumericValue::Unknown;
            assert!(matches!(
                l.moves.save(token, world, &m.id, retarget).await,
                Err(Error::Invalid(_))
            ));
        }
        assert!(matches!(
            l.moves
                .create(
                    token,
                    world,
                    new_move(
                        "New selection",
                        Some(vec![value(&a, NumericValue::Unknown)])
                    )
                )
                .await,
            Err(Error::Invalid(_))
        ));
        let mut retarget = move_edit(&other);
        retarget.attributes = Some(vec![value(&a, NumericValue::Unknown)]);
        assert!(matches!(
            l.moves.save(token, world, &other.id, retarget).await,
            Err(Error::Invalid(_))
        ));
        let mut stale = move_edit(&m);
        stale.attributes = Some(vec![value(&archived, NumericValue::Unknown)]);
        assert_eq!(
            l.moves.save(token, world, &m.id, stale).await,
            Err(Error::Conflict)
        );
        assert_eq!(
            l.moves
                .save(&l.other.token, world, &m.id, move_edit(&edited))
                .await,
            Err(Error::Forbidden)
        );
        assert_eq!(l.audits().await, audits);
        assert_eq!(l.moves.detail(world, &m.id).await.unwrap(), edited);
    }
    l.integrity().await;
}

#[tokio::test]
async fn retained_archived_expectations_allow_policy_edits_and_removals() {
    let l = Lab::new().await;
    let w = l.world("Retained expectations").await;
    let a = l.attribute(Some(&w.id), "Archived expectation").await;
    let b = l.attribute(Some(&w.id), "Other expectation").await;
    let mut e = ruleset_edit();
    e.expectations = Some(
        vec![&a, &b]
            .into_iter()
            .map(|a| ExpectationInput {
                attribute: a.pin(),
                required: false,
                allow_not_applicable: false,
                constraints: NumericConstraints::default(),
            })
            .collect(),
    );
    let first = l
        .moves
        .save_ruleset(&l.owner.token, &w.id, e.clone())
        .await
        .unwrap();
    let mut definition = attribute_edit(&a);
    definition.unit = "Changed head unit".into();
    definition.constraints.whole_only = true;
    let newer = l
        .moves
        .save_attribute(&l.owner.token, Some(&w.id), &a.id, definition)
        .await
        .unwrap();
    let archived = l.archive_attribute(&newer).await;
    e.expected_revision = first.revision;
    let audits = l.audits().await;
    assert_eq!(
        l.moves
            .save_ruleset(&l.owner.token, &w.id, e.clone())
            .await
            .unwrap(),
        first
    );
    assert_eq!(l.audits().await, audits);
    e.notes = "Unrelated notes".into();
    let second = l
        .moves
        .save_ruleset(&l.owner.token, &w.id, e.clone())
        .await
        .unwrap();
    assert_eq!(second.expectations.as_ref().unwrap()[0].definition, a);
    e.expected_revision = second.revision;
    e.expectations.as_mut().unwrap()[0].required = true;
    e.expectations.as_mut().unwrap()[0].constraints.maximum = Some("6.25".parse().unwrap());
    e.expectations.as_mut().unwrap()[1].allow_not_applicable = true;
    let third = l
        .moves
        .save_ruleset(&l.owner.token, &w.id, e.clone())
        .await
        .unwrap();
    assert_eq!(third.expectations.as_ref().unwrap()[0].definition, a);
    e.expected_revision = third.revision;
    e.expectations.as_mut().unwrap().remove(1);
    let fourth = l
        .moves
        .save_ruleset(&l.owner.token, &w.id, e.clone())
        .await
        .unwrap();
    assert_eq!(fourth.expectations.as_ref().unwrap()[0].definition, a);
    let audits = l.audits().await;
    for pin in [newer.pin(), archived.pin()] {
        let mut retarget = e.clone();
        retarget.expected_revision = fourth.revision;
        retarget.expectations.as_mut().unwrap()[0].attribute = pin;
        assert!(matches!(
            l.moves.save_ruleset(&l.owner.token, &w.id, retarget).await,
            Err(Error::Invalid(_))
        ));
    }
    assert_eq!(l.audits().await, audits);
    let mut remove = e.clone();
    remove.expected_revision = fourth.revision;
    remove.expectations = Some(vec![]);
    let empty = l
        .moves
        .save_ruleset(&l.owner.token, &w.id, remove)
        .await
        .unwrap();
    e.expected_revision = empty.revision;
    assert!(matches!(
        l.moves.save_ruleset(&l.owner.token, &w.id, e).await,
        Err(Error::Invalid(_))
    ));
    assert_eq!(
        l.moves.ruleset_version(&w.id, &first.id, 1).await.unwrap(),
        first
    );
    l.integrity().await;
}

#[tokio::test]
async fn source_attribute_copy_still_requires_selectable_pins_even_if_already_bound() {
    let l = Lab::new().await;
    let w = l.world("Copy archive checks").await;
    let shared = l.attribute(None, "Shared source").await;
    let mut n = new_attribute("Local source");
    n.source = Some(shared.pin());
    let local = l
        .moves
        .create_attribute(&l.owner.token, Some(&w.id), n)
        .await
        .unwrap();
    let source = l
        .moves
        .create(
            &l.admin.token,
            None,
            new_move("Source", Some(vec![value(&shared, NumericValue::Unknown)])),
        )
        .await
        .unwrap();
    let mut n = new_move("Local", Some(vec![value(&local, NumericValue::Unknown)]));
    n.source = Some(source.pin());
    let m = l
        .moves
        .create(&l.owner.token, Some(&w.id), n)
        .await
        .unwrap();
    l.archive_attribute(&local).await;
    assert_eq!(
        l.moves
            .save(&l.owner.token, Some(&w.id), &m.id, move_edit(&m))
            .await
            .unwrap(),
        m
    );
    let copy = MoveAdoptEdit {
        expected_revision: m.revision,
        target_version: source.revision,
        fields: m.fields.clone(),
        apply_source: true,
        mappings: vec![AttributeMapping {
            shared_attribute_id: shared.id,
            local_attribute: local.pin(),
        }],
        reason: None,
    };
    let audits = l.audits().await;
    assert!(matches!(
        l.moves.adoption_preview(&w.id, &m.id, &copy).await,
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        l.moves.adopt(&l.owner.token, &w.id, &m.id, copy).await,
        Err(Error::Invalid(_))
    ));
    assert_eq!(l.audits().await, audits);
    assert_eq!(l.moves.detail(Some(&w.id), &m.id).await.unwrap(), m);
    l.integrity().await;
}

#[tokio::test]
async fn revision_last_seal_rejects_incomplete_children_before_commit() {
    let l = Lab::new().await;
    let a = l.attribute(None, "A").await;
    let m = l
        .moves
        .create(
            &l.admin.token,
            None,
            new_move(
                "M",
                Some(vec![value(&a, NumericValue::Known("0".parse().unwrap()))]),
            ),
        )
        .await
        .unwrap();
    let before = l
        .moves
        .history_entry(None, &m.id, 1)
        .await
        .unwrap()
        .after_json;
    let mut second = m.clone();
    second.revision = 2;
    let after = serde_json::json!({"snapshot_version":1,"content":second}).to_string();
    let mut tx = persistence::begin_write(&l.pool).await.unwrap();
    sqlx::query("INSERT INTO audit_events(id,actor_user_id,action) VALUES ('bad-seal',?,'edit')")
        .bind(&l.admin.actor.user_id)
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("UPDATE moves SET revision=2 WHERE id=?")
        .bind(&m.id)
        .execute(&mut *tx)
        .await
        .unwrap();
    let result=sqlx::query("INSERT INTO move_versions(scope,move_id,revision,slug,display_name,description,attributes_defined,audit_id,previous_revision,before_json,after_json) SELECT scope,move_id,2,slug,display_name,description,attributes_defined,'bad-seal',1,?,? FROM move_versions WHERE move_id=? AND revision=1").bind(before).bind(after).bind(&m.id).execute(&mut *tx).await;
    assert!(result.is_err());
    tx.rollback().await.unwrap();
    assert_eq!(l.moves.detail(None, &m.id).await.unwrap(), m);
    l.integrity().await;
}

#[tokio::test]
async fn rulesets_pin_roster_and_chart_scope_without_advancing_them_or_the_world() {
    use web_app::application::{
        charts::{Charts, NewChart},
        stats::Stats,
    };
    let l = Lab::new().await;
    let w = l.world("Contexts").await;
    let other = l.world("Other contexts").await;
    let stats = Stats::new(l.accounts.clone());
    let charts = Charts::new(l.accounts.clone());
    let roster = stats
        .save_roster(&l.owner.token, &w.id, 0, vec![], None)
        .await
        .unwrap();
    let foreign = stats
        .save_roster(&l.owner.token, &other.id, 0, vec![], None)
        .await
        .unwrap();
    let chart = charts
        .create(
            &l.owner.token,
            Some(&w.id),
            NewChart {
                fields: fields("Chart"),
                roster: vec![],
                source: None,
                mapping: vec![],
                apply_source: false,
                reason: None,
            },
        )
        .await
        .unwrap();
    let mut e = ruleset_edit();
    e.stat_roster = Some(roster.pin());
    e.chart = Some(game_data::charts::ChartPin {
        chart_id: chart.id.clone(),
        version: chart.revision,
    });
    let first = l
        .moves
        .save_ruleset(&l.owner.token, &w.id, e.clone())
        .await
        .unwrap();
    e.expected_revision = 1;
    e.stat_roster = Some(foreign.pin());
    assert!(matches!(
        l.moves.save_ruleset(&l.owner.token, &w.id, e.clone()).await,
        Err(Error::Invalid(_))
    ));
    e.stat_roster = Some(roster.pin());
    e.chart.as_mut().unwrap().version = 99;
    assert!(matches!(
        l.moves.save_ruleset(&l.owner.token, &w.id, e.clone()).await,
        Err(Error::Invalid(_))
    ));
    e.chart.as_mut().unwrap().version = 1;
    assert!(matches!(
        l.moves.save_ruleset(&l.owner.token, &other.id, e).await,
        Err(Error::Conflict)
    ));
    assert_eq!(l.moves.ruleset(&w.id).await.unwrap(), Some(first));
    assert_eq!(
        l.author.world_detail(&w.id).await.unwrap().revision,
        w.revision
    );
    l.integrity().await;
}

#[tokio::test]
async fn rulesets_retain_archived_chart_pins_but_reject_new_exact_selections() {
    use web_app::application::charts::{ChartPin, Charts, NewChart};
    let l = Lab::new().await;
    let charts = Charts::new(l.accounts.clone());
    for archived_version in [false, true] {
        let w = l
            .world(if archived_version {
                "Archived exact version"
            } else {
                "Archived chart head"
            })
            .await;
        let chart = charts
            .create(
                &l.owner.token,
                Some(&w.id),
                NewChart {
                    fields: fields("Retained chart"),
                    roster: vec![],
                    source: None,
                    mapping: vec![],
                    apply_source: false,
                    reason: None,
                },
            )
            .await
            .unwrap();
        let pin = ChartPin {
            chart_id: chart.id.clone(),
            version: if archived_version { 2 } else { 1 },
        };
        let mut edit = ruleset_edit();
        edit.chart = Some(pin.clone());
        let first = if archived_version {
            l.archive_empty_chart(&chart).await;
            // Trusted historical content may already retain an archived exact
            // version. This adapter writes normal typed rows, seals and audit.
            let row = RulesetRecord {
                id: "33333333333333333333333333333333".into(),
                world_id: w.id.clone(),
                revision: 1,
                fields: chart.fields.clone(),
                expectations: None,
                stat_roster: None,
                chart: Some(pin.clone()),
                max_moves: None,
                repeat_policy: RepeatPolicy::Unspecified,
                notes: String::new(),
            };
            let mut tx = persistence::begin_write(&l.pool).await.unwrap();
            assert!(l.moves.seed_ruleset_in(&mut tx, &row).await.unwrap());
            tx.commit().await.unwrap();
            edit.fields = row.fields.clone();
            row
        } else {
            let row = l
                .moves
                .save_ruleset(&l.owner.token, &w.id, edit.clone())
                .await
                .unwrap();
            l.archive_empty_chart(&chart).await;
            row
        };
        assert_eq!(l.moves.ruleset(&w.id).await.unwrap(), Some(first.clone()));
        assert_eq!(
            l.moves.ruleset_version(&w.id, &first.id, 1).await.unwrap(),
            first
        );
        edit.expected_revision = first.revision;
        let before = database_rows(&l.pool).await;
        assert_eq!(
            l.moves
                .save_ruleset(&l.owner.token, &w.id, edit.clone())
                .await
                .unwrap(),
            first
        );
        assert_eq!(database_rows(&l.pool).await, before);
        edit.notes = "Unrelated notes with a retained archived chart".into();
        edit.max_moves = Some(3);
        let updated = l
            .moves
            .save_ruleset(&l.owner.token, &w.id, edit.clone())
            .await
            .unwrap();
        assert_eq!(updated.chart, Some(pin.clone()));
        assert_eq!(updated.notes, edit.notes);
        assert_eq!(updated.max_moves, Some(3));
        assert_eq!(updated.revision, first.revision + 1);
        edit.expected_revision = updated.revision;
        let before = database_rows(&l.pool).await;
        let mut changed = edit.clone();
        changed.chart.as_mut().unwrap().version = if archived_version { 1 } else { 2 };
        let Error::Invalid(errors) = l
            .moves
            .save_ruleset(&l.owner.token, &w.id, changed.clone())
            .await
            .unwrap_err()
        else {
            panic!("Expected the changed archived chart pin to be invalid");
        };
        assert!(errors.contains_key("chart"));
        changed.expected_revision = first.revision;
        assert_eq!(
            l.moves
                .save_ruleset(&l.owner.token, &w.id, changed.clone())
                .await
                .unwrap_err(),
            Error::Conflict
        );
        assert_eq!(
            l.moves
                .save_ruleset(&l.other.token, &w.id, changed)
                .await
                .unwrap_err(),
            Error::Forbidden
        );
        let mut invalid_max = edit.clone();
        invalid_max.max_moves = Some(-1);
        assert!(matches!(
            l.moves
                .save_ruleset(&l.owner.token, &w.id, invalid_max)
                .await,
            Err(Error::Invalid(_))
        ));
        assert_eq!(database_rows(&l.pool).await, before);

        let foreign = l.world("Other chart scope").await;
        let before = database_rows(&l.pool).await;
        let mut wrong_world = ruleset_edit();
        wrong_world.chart = Some(pin.clone());
        let Error::Invalid(errors) = l
            .moves
            .save_ruleset(&l.owner.token, &foreign.id, wrong_world)
            .await
            .unwrap_err()
        else {
            panic!("Expected wrong-world chart reference to be invalid");
        };
        assert!(errors.contains_key("chart"));
        assert_eq!(database_rows(&l.pool).await, before);

        // A valid retained-pin edit still rolls back every row on late failure.
        l.pool.execute("CREATE TRIGGER retained_chart_ruleset_fail BEFORE INSERT ON world_ruleset_versions WHEN NEW.revision=3 BEGIN SELECT RAISE(ABORT,'test retained chart failure'); END").await.unwrap();
        let mut failed = edit.clone();
        failed.notes = "Must roll back".into();
        assert!(
            l.moves
                .save_ruleset(&l.owner.token, &w.id, failed)
                .await
                .is_err()
        );
        assert_eq!(database_rows(&l.pool).await, before);
        l.pool
            .execute("DROP TRIGGER retained_chart_ruleset_fail")
            .await
            .unwrap();

        edit.chart = None;
        let removed = l
            .moves
            .save_ruleset(&l.owner.token, &w.id, edit.clone())
            .await
            .unwrap();
        edit.expected_revision = removed.revision;
        edit.chart = Some(pin);
        let before = database_rows(&l.pool).await;
        assert!(matches!(
            l.moves.save_ruleset(&l.owner.token, &w.id, edit).await,
            Err(Error::Invalid(_))
        ));
        assert_eq!(database_rows(&l.pool).await, before);
        assert_eq!(l.moves.ruleset(&w.id).await.unwrap(), Some(removed));
        assert_eq!(
            l.moves.ruleset_version(&w.id, &first.id, 1).await.unwrap(),
            first
        );
        assert_eq!(
            charts.version(Some(&w.id), &chart.id, 1).await.unwrap(),
            chart
        );
        assert_eq!(l.author.world_detail(&w.id).await.unwrap(), w);
    }
    l.integrity().await;
}

#[tokio::test]
async fn collaborator_revocation_is_effective_for_existing_sessions_and_all_aggregate_writes() {
    let l = Lab::new().await;
    let w = l.world("Collaborative").await;
    l.author
        .grant_collaborator(
            &l.owner.token,
            &w.id,
            w.access_revision,
            &l.other.actor.user_id,
            None,
        )
        .await
        .unwrap();
    let a = l
        .moves
        .create_attribute(&l.other.token, Some(&w.id), new_attribute("Collaborator A"))
        .await
        .unwrap();
    let m = l
        .moves
        .create(
            &l.other.token,
            Some(&w.id),
            new_move(
                "Collaborator M",
                Some(vec![value(&a, NumericValue::Unknown)]),
            ),
        )
        .await
        .unwrap();
    l.moves
        .save_ruleset(&l.other.token, &w.id, ruleset_edit())
        .await
        .unwrap();
    let access = l.author.world_detail(&w.id).await.unwrap().access_revision;
    l.author
        .revoke_collaborator(&l.owner.token, &w.id, access, &l.other.actor.user_id, None)
        .await
        .unwrap();
    let before = l.audits().await;
    assert_eq!(
        l.moves
            .save_attribute(&l.other.token, Some(&w.id), &a.id, attribute_edit(&a))
            .await,
        Err(Error::Forbidden)
    );
    assert_eq!(
        l.moves
            .save(&l.other.token, Some(&w.id), &m.id, move_edit(&m))
            .await,
        Err(Error::Forbidden)
    );
    let mut e = ruleset_edit();
    e.expected_revision = 1;
    assert_eq!(
        l.moves.save_ruleset(&l.other.token, &w.id, e).await,
        Err(Error::Forbidden)
    );
    assert_eq!(l.audits().await, before);
}
