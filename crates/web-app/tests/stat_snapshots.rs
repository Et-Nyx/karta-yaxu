//! Compatibility and integrity at the creature snapshot boundary, on test-owned databases.
use game_data::{
    authoring::ContentFields,
    stats::{BaseStats, NumericConstraints, StatDefinition, StatPin, StatRoster},
};
use serde_json::{Value, json};
use sqlx::{Row, SqlSafeStr, SqliteConnection, SqlitePool};
use tempfile::TempDir;
use web_app::{
    accounts::Accounts,
    application::{
        authoring::{Authoring, Edit, Error, HistoryKind, SourcePin},
        stat_values::{FormStatsEdit, StatValues, VariantStatsEdit},
        types::Types,
    },
    auth::{Role, Subject},
    persistence,
};

struct Lab {
    pool: SqlitePool,
    accounts: Accounts,
    author: Authoring,
    types: Types,
    values: StatValues,
    token: String,
    user: String,
    _dir: TempDir,
}
fn fields(name: &str) -> ContentFields {
    ContentFields {
        slug: name.to_lowercase().replace(' ', "-"),
        display_name: name.into(),
        description: "Snapshot boundary test.".into(),
    }
}
fn edit(revision: i64, name: &str) -> Edit {
    Edit {
        expected_revision: revision,
        fields: fields(name),
        reason: None,
    }
}
fn envelope(value: &impl serde::Serialize) -> String {
    serde_json::to_string_pretty(&json!({"snapshot_version":1,"content":value})).unwrap()
}
impl Lab {
    async fn new(through: i64) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let pool = persistence::connect_for_migration(&format!(
            "sqlite://{}/snapshots.db",
            dir.path().display()
        ))
        .await
        .unwrap();
        migrate_through(&pool, through).await;
        let accounts = Accounts::new(pool.clone(), 3600).unwrap();
        accounts
            .invite("test", "Admin", Role::Administrator, None)
            .await
            .unwrap();
        let actor = accounts
            .sign_in(Subject {
                provider: "test".into(),
                provider_subject: "Admin".into(),
                display_name: "Admin".into(),
            })
            .await
            .unwrap();
        let (token, _) = accounts.start_session(&actor.user_id).await.unwrap();
        Self {
            pool,
            author: Authoring::new(accounts.clone()),
            types: Types::new(accounts.clone()),
            values: StatValues::new(accounts.clone()),
            accounts,
            token,
            user: actor.user_id,
            _dir: dir,
        }
    }
    async fn integrity(&self) {
        let rows = sqlx::query("PRAGMA foreign_key_check")
            .fetch_all(&self.pool)
            .await
            .unwrap();
        assert!(rows.is_empty());
        let result: String = sqlx::query_scalar("PRAGMA integrity_check")
            .fetch_one(&self.pool)
            .await
            .unwrap();
        assert_eq!(result, "ok");
    }
    async fn count(&self, table: &str) -> i64 {
        sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {table}")))
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }
}
async fn migrate_through(pool: &SqlitePool, through: i64) {
    let scripts = [
        (1, "auth", include_str!("../migrations/0001_auth.sql")),
        (
            2,
            "world authoring",
            include_str!("../migrations/0002_world_authoring.sql"),
        ),
        (3, "types", include_str!("../migrations/0003_types.sql")),
        (4, "charts", include_str!("../migrations/0004_charts.sql")),
        (5, "stats", include_str!("../migrations/0005_stats.sql")),
        (
            6,
            "stat content",
            include_str!("../migrations/0006_stat_content.sql"),
        ),
    ];
    let migrations = scripts
        .into_iter()
        .filter(|(n, _, _)| *n <= through)
        .map(|(n, name, sql)| {
            sqlx::migrate::Migration::new(
                n,
                name.into(),
                sqlx::migrate::MigrationType::Simple,
                sql.into_sql_str(),
                false,
            )
        })
        .collect();
    sqlx::migrate::Migrator {
        migrations: std::borrow::Cow::Owned(migrations),
        ..sqlx::migrate::Migrator::DEFAULT
    }
    .run(pool)
    .await
    .unwrap();
}
async fn audit(c: &mut SqliteConnection, n: i64) -> String {
    let id = format!("aa{n:030x}");
    sqlx::query("INSERT INTO audit_events(id,system_key,action) VALUES (?,'bundled-fixture-loader','create')").bind(&id).execute(c).await.unwrap();
    id
}

// Seed exact independent dependencies directly, so these tests do not depend on
// definition/roster use-case API choices being implemented in parallel.
async fn definition(l: &Lab, world: Option<&str>, n: i64) -> StatDefinition {
    let row = StatDefinition {
        id: format!("bb{n:030x}"),
        world_id: world.map(str::to_owned),
        source: None,
        fields: fields(&format!("Stat {n}")),
        revision: 1,
        archived_at: None,
        constraints: NumericConstraints {
            whole_only: false,
            minimum: Some("-10.5".parse().unwrap()),
            maximum: Some("10.5".parse().unwrap()),
        },
    };
    let mut tx = persistence::begin_write(&l.pool).await.unwrap();
    let a = audit(&mut tx, n).await;
    sqlx::query("INSERT INTO stat_definitions(id,scope,world_id,slug,revision) VALUES (?,?,?,?,1)")
        .bind(&row.id)
        .bind(world.unwrap_or(""))
        .bind(world)
        .bind(&row.fields.slug)
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("INSERT INTO stat_versions(scope,stat_id,revision,slug,display_name,description,whole_only,minimum_coefficient,minimum_scale,maximum_coefficient,maximum_scale,audit_id,after_json) VALUES (?,?,1,?,?,?,0,-105,1,105,1,?,?)")
        .bind(world.unwrap_or("")).bind(&row.id).bind(&row.fields.slug).bind(&row.fields.display_name).bind(&row.fields.description).bind(a).bind(envelope(&row)).execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
    row
}
async fn roster(l: &Lab, world: &str, entries: Vec<StatDefinition>, n: i64) -> StatRoster {
    let row = StatRoster {
        id: format!("cc{n:030x}"),
        world_id: world.into(),
        revision: 1,
        entries,
    };
    let mut tx = persistence::begin_write(&l.pool).await.unwrap();
    let a = audit(&mut tx, n).await;
    sqlx::query("INSERT INTO stat_rosters(world_id,id,revision) VALUES (?,?,1)")
        .bind(world)
        .bind(&row.id)
        .execute(&mut *tx)
        .await
        .unwrap();
    for (i, stat) in row.entries.iter().enumerate() {
        sqlx::query("INSERT INTO stat_roster_entries(world_id,roster_id,revision,slot,scope,stat_id,stat_version) VALUES (?,?,1,?,?,?,1)").bind(world).bind(&row.id).bind((i+1) as i64).bind(world).bind(&stat.id).execute(&mut *tx).await.unwrap();
    }
    sqlx::query("INSERT INTO stat_roster_versions(world_id,roster_id,revision,audit_id,after_json) VALUES (?,?,1,?,?)").bind(world).bind(&row.id).bind(a).bind(envelope(&row)).execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
    row
}
async fn archive_definition(l: &Lab, old: &StatDefinition, n: i64) -> StatDefinition {
    let mut row = old.clone();
    row.revision = 2;
    row.archived_at = Some(1);
    let mut tx = persistence::begin_write(&l.pool).await.unwrap();
    let a = audit(&mut tx, n).await;
    sqlx::query("UPDATE stat_definitions SET revision=2 WHERE id=?")
        .bind(&row.id)
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("INSERT INTO stat_versions(scope,stat_id,revision,slug,display_name,description,archived_at,whole_only,minimum_coefficient,minimum_scale,maximum_coefficient,maximum_scale,audit_id,previous_revision,before_json,after_json) SELECT scope,stat_id,2,slug,display_name,description,1,whole_only,minimum_coefficient,minimum_scale,maximum_coefficient,maximum_scale,?,1,after_json,? FROM stat_versions WHERE stat_id=? AND revision=1")
        .bind(a).bind(envelope(&row)).bind(&row.id).execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
    row
}
async fn save_form(
    l: &Lab,
    id: &str,
    revision: i64,
    values: Option<Vec<(StatPin, Option<game_data::stats::Decimal>)>>,
) {
    l.values
        .save_form(
            &l.token,
            id,
            FormStatsEdit {
                expected_revision: revision,
                values,
                reason: None,
            },
        )
        .await
        .unwrap();
}
async fn chains(l: &Lab, kind: HistoryKind, id: &str) {
    let (legacy, typing, stats, key) = match kind {
        HistoryKind::Form => (
            "form_versions",
            "form_content_snapshots",
            "form_stat_snapshots",
            "form_id",
        ),
        HistoryKind::Variant => (
            "variant_revisions",
            "variant_content_snapshots",
            "variant_stat_snapshots",
            "variant_id",
        ),
        _ => unreachable!(),
    };
    let history = l.author.history(kind, id, None).await.unwrap();
    for p in history.windows(2) {
        assert_eq!(p[0].before_json.as_deref(), Some(p[1].after_json.as_str()));
    }
    for (table, predecessor) in [(legacy, legacy), (typing, typing), (stats, stats)] {
        let sql = format!(
            "SELECT a.before_json, b.after_json FROM {table} a JOIN {predecessor} b ON a.{key}=b.{key} AND a.revision=b.revision+1 WHERE a.{key}=?"
        );
        let rows: Vec<(String, String)> = sqlx::query_as(sqlx::AssertSqlSafe(sql))
            .bind(id)
            .fetch_all(&l.pool)
            .await
            .unwrap();
        for (before, after) in rows {
            assert_eq!(before, after, "{table}");
        }
    }
}

#[tokio::test]
async fn stats_writers_preserve_typing_and_all_other_writers_preserve_exact_stats() {
    let l = Lab::new(6).await;
    let s = l
        .author
        .create_species(&l.token, fields("Creature"), None)
        .await
        .unwrap();
    let f = &s.default_form_id;
    let t = l
        .types
        .create_type(&l.token, fields("Type"), None)
        .await
        .unwrap();
    l.types
        .save_form_typing(
            &l.token,
            f,
            1,
            Some(vec![game_data::types::TypePin {
                type_id: t.id,
                version: 1,
            }]),
            None,
        )
        .await
        .unwrap();
    let typing = l.types.form_typing(f, None).await.unwrap();
    let d = definition(&l, None, 1).await;
    save_form(
        &l,
        f,
        2,
        Some(vec![(d.pin(), Some("-1.25".parse().unwrap()))]),
    )
    .await;
    let stats = l.values.form(f).await.unwrap();
    assert_eq!(l.types.form_typing(f, None).await.unwrap(), typing);
    l.author
        .save_form(&l.token, f, 1, edit(3, "Metadata"))
        .await
        .unwrap();
    assert_eq!(l.values.form(f).await.unwrap(), stats);
    l.types
        .save_form_typing(&l.token, f, 4, Some(vec![]), None)
        .await
        .unwrap();
    assert_eq!(l.values.form(f).await.unwrap(), stats);
    let w = l
        .author
        .create_world(&l.token, fields("World"), None)
        .await
        .unwrap();
    let v = l
        .author
        .create_variant(
            &l.token,
            &w.id,
            1,
            Some(SourcePin {
                form_id: f.clone(),
                version: 3,
            }),
            fields("Local"),
            None,
        )
        .await
        .unwrap();
    let local = definition(&l, Some(&w.id), 2).await;
    let r = roster(&l, &w.id, vec![local.clone()], 3).await;
    l.values
        .reconcile(
            &l.token,
            &w.id,
            &v.id,
            VariantStatsEdit {
                expected_revision: 1,
                expected_roster_revision: 1,
                values: vec![(local.id.clone(), Some("0".parse().unwrap()))],
                reason: None,
            },
        )
        .await
        .unwrap();
    let expected = l.values.variant(&w.id, &v.id).await.unwrap();
    assert_eq!(expected.roster, Some(r.pin()));
    l.types
        .save_variant_typing(&l.token, &w.id, &v.id, 2, Some(vec![]), None)
        .await
        .unwrap();
    l.author
        .save_variant(&l.token, &w.id, &v.id, edit(3, "Local metadata"))
        .await
        .unwrap();
    l.author
        .adopt_variant(&l.token, &w.id, &v.id, 5, edit(4, "Adopt metadata"))
        .await
        .unwrap();
    assert_eq!(l.values.variant(&w.id, &v.id).await.unwrap(), expected);
    assert_eq!(
        l.types
            .variant_typing(&w.id, &v.id, None)
            .await
            .unwrap()
            .assignments,
        Some(vec![])
    );
    // Stat catalog and roster authoring never changes the world's content snapshot.
    let world: Value = serde_json::from_str(
        &l.author
            .history_entry(HistoryKind::World, &w.id, 2)
            .await
            .unwrap()
            .after_json,
    )
    .unwrap();
    assert_eq!(world["snapshot_version"], 2);
    assert!(world["content"].get("stats").is_none());
    assert_eq!(l.author.world_detail(&w.id).await.unwrap().revision, 2);
    chains(&l, HistoryKind::Form, f).await;
    chains(&l, HistoryKind::Variant, &v.id).await;
    l.integrity().await;
}

#[tokio::test]
async fn empty_unmodeled_and_missing_values_are_sealed_and_late_orphan_writes_fail() {
    let l = Lab::new(6).await;
    let s = l
        .author
        .create_species(&l.token, fields("Creature"), None)
        .await
        .unwrap();
    let f = &s.default_form_id;
    assert_eq!(l.values.form(f).await.unwrap(), BaseStats::unmodeled());
    assert_eq!(l.count("form_stat_snapshots").await, 1);
    save_form(&l, f, 1, Some(vec![])).await;
    assert_eq!(l.values.form(f).await.unwrap().assignments, Some(vec![]));
    let d = definition(&l, None, 1).await;
    save_form(&l, f, 2, Some(vec![(d.pin(), None)])).await;
    for sql in [
        "UPDATE form_stat_bindings SET coefficient=1 WHERE form_id=?",
        "DELETE FROM form_stat_bindings WHERE form_id=?",
        "UPDATE form_stat_snapshots SET stats_defined=0 WHERE form_id=?",
        "DELETE FROM form_stat_snapshots WHERE form_id=?",
    ] {
        assert!(sqlx::query(sql).bind(f).execute(&l.pool).await.is_err());
    }
    for revision in [1, 2, 3] {
        assert!(sqlx::query("INSERT INTO form_stat_bindings(form_id,revision,slot,stat_id,stat_version) VALUES (?,?,2,?,1)").bind(f).bind(revision).bind(&d.id).execute(&l.pool).await.is_err());
    }
    let mut tx = persistence::begin_write(&l.pool).await.unwrap();
    sqlx::query("INSERT INTO form_stat_bindings(form_id,revision,slot,stat_id,stat_version) VALUES (?,99,1,?,1)").bind(f).bind(&d.id).execute(&mut *tx).await.unwrap();
    assert!(tx.commit().await.is_err());
    assert_eq!(l.count("form_stat_bindings").await, 1);
    // Empty world roster still requires a typed roster pin and a v3 seal.
    let w = l
        .author
        .create_world(&l.token, fields("World"), None)
        .await
        .unwrap();
    let v = l
        .author
        .create_variant(&l.token, &w.id, 1, None, fields("Local"), None)
        .await
        .unwrap();
    let r = roster(&l, &w.id, vec![], 2).await;
    l.values
        .reconcile(
            &l.token,
            &w.id,
            &v.id,
            VariantStatsEdit {
                expected_revision: 1,
                expected_roster_revision: 1,
                values: vec![],
                reason: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(
        l.values.variant(&w.id, &v.id).await.unwrap(),
        BaseStats {
            roster: Some(r.pin()),
            assignments: Some(vec![])
        }
    );
    assert_eq!(l.count("variant_stat_snapshots").await, 2);
    l.integrity().await;
}

#[tokio::test]
async fn stat_failures_roll_back_heads_audits_projections_bindings_and_seals() {
    for table in ["form_stat_snapshots", "form_versions"] {
        let l = Lab::new(6).await;
        let s = l
            .author
            .create_species(&l.token, fields("Creature"), None)
            .await
            .unwrap();
        let d = definition(&l, None, 1).await;
        let before = l.count("audit_events").await;
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE TRIGGER fault BEFORE INSERT ON {table} WHEN NEW.revision=2 BEGIN SELECT RAISE(ABORT,'test'); END"))).execute(&l.pool).await.unwrap();
        assert_eq!(
            l.values
                .save_form(
                    &l.token,
                    &s.default_form_id,
                    FormStatsEdit {
                        expected_revision: 1,
                        values: Some(vec![(d.pin(), Some("1.5".parse().unwrap()))]),
                        reason: None
                    }
                )
                .await
                .unwrap_err(),
            Error::Unavailable
        );
        assert_eq!(l.count("audit_events").await, before);
        assert_eq!(l.count("form_stat_bindings").await, 0);
        for table in [
            "form_versions",
            "form_content_snapshots",
            "form_stat_snapshots",
        ] {
            assert_eq!(l.count(table).await, 1);
        }
        assert_eq!(
            l.author
                .form_detail(&s.default_form_id)
                .await
                .unwrap()
                .revision,
            1
        );
        l.integrity().await;
    }
}

#[tokio::test]
async fn malformed_canonical_values_constraints_dependencies_and_projection_chains_fail_closed() {
    let l = Lab::new(6).await;
    let s = l
        .author
        .create_species(&l.token, fields("Creature"), None)
        .await
        .unwrap();
    let f = &s.default_form_id;
    let d = definition(&l, None, 1).await;
    save_form(
        &l,
        f,
        1,
        Some(vec![(d.pin(), Some("1.25".parse().unwrap()))]),
    )
    .await;
    let good = l
        .author
        .history_entry(HistoryKind::Form, f, 2)
        .await
        .unwrap()
        .after_json;
    sqlx::query("DROP TRIGGER form_stat_update")
        .execute(&l.pool)
        .await
        .unwrap();
    let baseline: Value = serde_json::from_str(&good).unwrap();
    let mut cases = vec![];
    for raw in [
        json!(1.25),
        json!("1.250"),
        json!("+1.25"),
        json!("1e0"),
        json!("-0"),
        json!("9223372036854775808"),
        json!("11"),
    ] {
        let mut bad = baseline.clone();
        bad["content"]["stats"]["assignments"][0]["value"] = raw;
        cases.push(bad);
    }
    for (path, value) in [
        ("revision", json!(99)),
        ("world_id", json!("ffffffffffffffffffffffffffffffff")),
        ("archived_at", json!(7)),
    ] {
        let mut bad = baseline.clone();
        bad["content"]["stats"]["assignments"][0]["definition"][path] = value;
        cases.push(bad);
    }
    let mut bad = baseline.clone();
    bad["content"]["stats"]["assignments"][0]["definition"]["constraints"]["maximum"] =
        json!("100");
    cases.push(bad);
    let mut bad = baseline.clone();
    bad["content"]["stats"]
        .as_object_mut()
        .unwrap()
        .remove("roster");
    cases.push(bad);
    let mut bad = baseline.clone();
    bad["content"]["extra"] = true.into();
    cases.push(bad);
    for bad in cases {
        sqlx::query("UPDATE form_stat_snapshots SET after_json=? WHERE form_id=? AND revision=2")
            .bind(bad.to_string())
            .bind(f)
            .execute(&l.pool)
            .await
            .unwrap();
        assert_eq!(
            l.values.form(f).await.unwrap_err(),
            Error::Unavailable,
            "{bad}"
        );
        assert_eq!(
            l.author
                .history_entry(HistoryKind::Form, f, 2)
                .await
                .unwrap_err(),
            Error::Unavailable
        );
    }
    sqlx::query("UPDATE form_stat_snapshots SET after_json=? WHERE form_id=? AND revision=2")
        .bind(&good)
        .bind(f)
        .execute(&l.pool)
        .await
        .unwrap();
    l.author
        .save_form(&l.token, f, 1, edit(2, "Next"))
        .await
        .unwrap();
    // Each projection's predecessor must use its own bytes, never authoritative v3.
    sqlx::query("DROP TRIGGER form_content_update")
        .execute(&l.pool)
        .await
        .unwrap();
    // A schema-valid v2 before with altered whitespace is still the wrong byte chain.
    sqlx::query("UPDATE form_content_snapshots SET before_json=' '||before_json WHERE form_id=? AND revision=3").bind(f).execute(&l.pool).await.unwrap();
    assert_eq!(l.values.form(f).await.unwrap_err(), Error::Unavailable);
}

// Genuine pre-extension content, inserted under the schema that originally owned it.
async fn legacy_creature(l: &Lab, n: i64, v2: bool) -> (String, String, String) {
    use web_app::application::authoring::{FormRecord, SpeciesRecord, VariantRecord, WorldRecord};
    let s = SpeciesRecord {
        id: format!("dd{n:030x}"),
        fields: fields(&format!("Legacy {n}")),
        revision: 1,
        default_form_id: format!("ee{n:030x}"),
        archived_at: None,
    };
    let f = FormRecord {
        id: s.default_form_id.clone(),
        species_id: s.id.clone(),
        species_version: 1,
        fields: s.fields.clone(),
        revision: 1,
        is_default: true,
        archived_at: None,
    };
    let w = WorldRecord {
        id: format!("ff{n:030x}"),
        owner_user_id: l.user.clone(),
        fields: fields(&format!("World {n}")),
        revision: 1,
        access_revision: 1,
        archived_at: None,
    };
    let v = VariantRecord {
        id: format!("ab{n:030x}"),
        world_id: w.id.clone(),
        source: Some(SourcePin {
            form_id: f.id.clone(),
            version: 1,
        }),
        fields: fields(&format!("Variant {n}")),
        revision: 1,
        archived_at: None,
    };
    let world =
        json!({"id":w.id,"fields":w.fields,"revision":1,"archived_at":null,"variant_ids":[v.id]});
    let mut tx = persistence::begin_write(&l.pool).await.unwrap();
    let mut audits = vec![];
    for i in 0..5 {
        audits.push(audit(&mut tx, 1000 + n * 10 + i).await);
    }
    sqlx::query("INSERT INTO species(id,slug,display_name,description,revision,default_form_id) VALUES (?,?,?,?,1,?)").bind(&s.id).bind(&s.fields.slug).bind(&s.fields.display_name).bind(&s.fields.description).bind(&f.id).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO species_versions(species_id,revision,slug,display_name,description,default_form_id,audit_id,after_json) VALUES (?,1,?,?,?,?,?,?)").bind(&s.id).bind(&s.fields.slug).bind(&s.fields.display_name).bind(&s.fields.description).bind(&f.id).bind(&audits[0]).bind(envelope(&s)).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO forms(id,species_id,species_version,slug,display_name,description,revision,is_default) VALUES (?,?,1,?,?,?,1,1)").bind(&f.id).bind(&s.id).bind(&f.fields.slug).bind(&f.fields.display_name).bind(&f.fields.description).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO worlds(id,owner_user_id,slug,display_name,description,revision,access_revision) VALUES (?,?,?,?,?,1,1)").bind(&w.id).bind(&l.user).bind(&w.fields.slug).bind(&w.fields.display_name).bind(&w.fields.description).execute(&mut *tx).await.unwrap();
    if v2 {
        let mut form_full = json!({"snapshot_version":2,"content":f});
        form_full["content"]["typing"] = json!({"assignments":[]});
        sqlx::query("INSERT INTO form_content_snapshots(form_id,revision,typing_defined,after_json) VALUES (?,1,1,?)").bind(&f.id).bind(form_full.to_string()).execute(&mut *tx).await.unwrap();
        let mut variant_full = json!({"snapshot_version":2,"content":v});
        variant_full["content"]["typing"] = json!({"assignments":[]});
        sqlx::query("INSERT INTO variant_content_snapshots(world_id,variant_id,revision,typing_defined,after_json) VALUES (?,?,1,1,?)").bind(&w.id).bind(&v.id).bind(variant_full.to_string()).execute(&mut *tx).await.unwrap();
        let mut world_full = json!({"snapshot_version":2,"content":world});
        world_full["content"]["type_ids"] = json!([]);
        sqlx::query(
            "INSERT INTO world_content_snapshots(world_id,revision,after_json) VALUES (?,1,?)",
        )
        .bind(&w.id)
        .bind(world_full.to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    }
    let (column, value) = if v2 {
        (",content_snapshot_version", ",2")
    } else {
        ("", "")
    };
    sqlx::query(sqlx::AssertSqlSafe(format!("INSERT INTO form_versions(form_id,species_id,species_version,revision,slug,display_name,description,is_default,audit_id,after_json{column}) VALUES (?,?,1,1,?,?,?,1,?,?{value})")))
        .bind(&f.id).bind(&s.id).bind(&f.fields.slug).bind(&f.fields.display_name).bind(&f.fields.description).bind(&audits[1]).bind(envelope(&f)).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO variants(id,world_id,source_form_id,source_form_version,slug,display_name,description,revision) VALUES (?,?,?,1,?,?,?,1)").bind(&v.id).bind(&w.id).bind(&f.id).bind(&v.fields.slug).bind(&v.fields.display_name).bind(&v.fields.description).execute(&mut *tx).await.unwrap();
    sqlx::query(sqlx::AssertSqlSafe(format!("INSERT INTO world_revisions(world_id,revision,audit_id,after_json{column}) VALUES (?,1,?,?{value})"))).bind(&w.id).bind(&audits[2]).bind(envelope(&world)).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO world_access_revisions(world_id,revision,audit_id,after_json) VALUES (?,1,?,?)").bind(&w.id).bind(&audits[3]).bind(envelope(&json!({"world_id":w.id,"owner_user_id":l.user,"access_revision":1,"collaborator_ids":[]}))).execute(&mut *tx).await.unwrap();
    sqlx::query(sqlx::AssertSqlSafe(format!("INSERT INTO variant_revisions(world_id,variant_id,revision,audit_id,after_json{column}) VALUES (?,?,1,?,?{value})"))).bind(&w.id).bind(&v.id).bind(&audits[4]).bind(envelope(&v)).execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
    (f.id, w.id, v.id)
}

// Preserve all old columns in all old tables, including binary session hashes and
// migration checksums. Additive columns are intentionally absent from saved queries.
async fn retained(pool: &SqlitePool) -> Vec<(String, Vec<String>)> {
    let tables: Vec<String> = sqlx::query_scalar("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name").fetch_all(pool).await.unwrap();
    let mut result = vec![];
    for table in tables {
        let columns = sqlx::query(sqlx::AssertSqlSafe(format!(
            "PRAGMA table_info(\"{table}\")"
        )))
        .fetch_all(pool)
        .await
        .unwrap();
        let expressions = columns
            .iter()
            .map(|r| format!("quote(\"{}\")", r.get::<String, _>("name")))
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!("SELECT json_array({expressions}) FROM \"{table}\" ORDER BY 1");
        let rows = sqlx::query_scalar(sqlx::AssertSqlSafe(sql.clone()))
            .fetch_all(pool)
            .await
            .unwrap();
        result.push((sql, rows));
    }
    result
}

#[tokio::test]
async fn populated_0004_upgrade_preserves_every_old_column_and_v1_v2_to_v3_chains() {
    use web_app::application::charts::{Charts, NewChart};
    let l = Lab::new(2).await;
    let v1 = legacy_creature(&l, 1, false).await;
    migrate_through(&l.pool, 4).await;
    let v2 = legacy_creature(&l, 2, true).await;
    let charts = Charts::new(l.accounts.clone());
    let chart = charts
        .create(
            &l.token,
            None,
            NewChart {
                fields: fields("Legacy chart"),
                roster: vec![],
                source: None,
                mapping: vec![],
                apply_source: false,
                reason: None,
            },
        )
        .await
        .unwrap();
    l.accounts
        .invite("test", "Pending", Role::Editor, None)
        .await
        .unwrap();
    let before = retained(&l.pool).await;
    assert!(!persistence::schema_ready(&l.pool).await.unwrap());
    persistence::migrate(&l.pool).await.unwrap();
    assert!(persistence::schema_ready(&l.pool).await.unwrap());
    for (sql, old) in before {
        let sql = if sql.contains("FROM \"_sqlx_migrations\"") {
            sql.replace(" ORDER BY 1", " WHERE version<=4 ORDER BY 1")
        } else {
            sql
        };
        let actual: Vec<String> = sqlx::query_scalar(sqlx::AssertSqlSafe(sql.clone()))
            .fetch_all(&l.pool)
            .await
            .unwrap();
        assert_eq!(actual, old, "{sql}");
    }
    assert!(l.accounts.authenticate(&l.token).await.unwrap().is_some());
    assert_eq!(charts.version(None, &chart.id, 1).await.unwrap(), chart);
    assert_eq!(l.count("form_stat_snapshots").await, 0);
    assert_eq!(l.count("variant_stat_snapshots").await, 0);
    for (form, world, variant) in [v1, v2] {
        for (kind, id) in [(HistoryKind::Form, &form), (HistoryKind::Variant, &variant)] {
            let old = l
                .author
                .history_entry(kind, id, 1)
                .await
                .unwrap()
                .after_json;
            assert_eq!(
                serde_json::from_str::<Value>(&old).unwrap()["snapshot_version"],
                if form.ends_with('1') { 1 } else { 2 }
            );
            // A sealed legacy revision can never acquire even an unmodeled extension.
            let mut extension: Value = serde_json::from_str(&old).unwrap();
            extension["snapshot_version"] = 3.into();
            extension["content"]["stats"] = json!({"roster":null,"assignments":null});
            let sql = if matches!(kind, HistoryKind::Form) {
                "INSERT INTO form_stat_snapshots(form_id,revision,stats_defined,after_json) VALUES (?1,1,0,?2)"
            } else {
                "INSERT INTO variant_stat_snapshots(variant_id,revision,stats_defined,after_json,world_id) VALUES (?1,1,0,?2,?3)"
            };
            let q = sqlx::query(sql).bind(id).bind(extension.to_string());
            let q = if matches!(kind, HistoryKind::Variant) {
                q.bind(&world)
            } else {
                q
            };
            assert!(q.execute(&l.pool).await.is_err());
            for revision in [1, 2] {
                if matches!(kind, HistoryKind::Form) {
                    l.author
                        .save_form(&l.token, id, 1, edit(revision, &format!("Form {revision}")))
                        .await
                        .unwrap();
                } else {
                    l.author
                        .save_variant(
                            &l.token,
                            &world,
                            id,
                            edit(revision, &format!("Updated variant {revision}")),
                        )
                        .await
                        .unwrap();
                }
            }
            let h = l.author.history(kind, id, None).await.unwrap();
            assert_eq!(h[2].after_json, old);
            assert_eq!(h[1].before_json.as_deref(), Some(old.as_str()));
            assert_eq!(h[0].before_json.as_deref(), Some(h[1].after_json.as_str()));
            assert_eq!(
                serde_json::from_str::<Value>(&h[0].after_json).unwrap()["snapshot_version"],
                3
            );
        }
        assert_eq!(l.values.form(&form).await.unwrap(), BaseStats::unmodeled());
        assert_eq!(
            l.values.variant(&world, &variant).await.unwrap(),
            BaseStats::unmodeled()
        );
    }
    l.integrity().await;
}

#[tokio::test]
async fn variant_scope_roster_order_and_typed_values_are_exact_and_missing_v3_never_falls_back() {
    let l = Lab::new(6).await;
    let w = l
        .author
        .create_world(&l.token, fields("World"), None)
        .await
        .unwrap();
    let other = l
        .author
        .create_world(&l.token, fields("Other"), None)
        .await
        .unwrap();
    let a = definition(&l, Some(&w.id), 1).await;
    let b = definition(&l, Some(&w.id), 2).await;
    let foreign = definition(&l, Some(&other.id), 3).await;
    let r = roster(&l, &w.id, vec![a.clone(), b], 4).await;
    let foreign_roster = roster(&l, &other.id, vec![foreign.clone()], 5).await;
    let v = l
        .author
        .create_variant(&l.token, &w.id, 1, None, fields("Local"), None)
        .await
        .unwrap();
    l.values
        .reconcile(
            &l.token,
            &w.id,
            &v.id,
            VariantStatsEdit {
                expected_revision: 1,
                expected_roster_revision: 1,
                values: vec![(a.id.clone(), Some("0".parse().unwrap()))],
                reason: None,
            },
        )
        .await
        .unwrap();
    let good = l
        .author
        .history_entry(HistoryKind::Variant, &v.id, 2)
        .await
        .unwrap()
        .after_json;
    let baseline: Value = serde_json::from_str(&good).unwrap();
    assert_eq!(baseline["content"]["stats"]["roster"], json!(r.pin()));
    assert!(baseline["content"]["stats"]["assignments"][1]["value"].is_null());
    // Even a real exact definition or roster cannot cross the owning world.
    let mut tx = persistence::begin_write(&l.pool).await.unwrap();
    assert!(sqlx::query("INSERT INTO variant_stat_bindings(world_id,variant_id,revision,slot,scope,stat_id,stat_version) VALUES (?,?,3,1,?,?,1)").bind(&w.id).bind(&v.id).bind(&w.id).bind(&foreign.id).execute(&mut *tx).await.is_err());
    assert!(sqlx::query("INSERT INTO variant_stat_bindings(world_id,variant_id,revision,slot,scope,stat_id,stat_version) VALUES (?,?,3,1,?,?,1)").bind(&w.id).bind(&v.id).bind(&other.id).bind(&foreign.id).execute(&mut *tx).await.is_err());
    let mut next = baseline.clone();
    next["content"]["revision"] = 3.into();
    next["content"]["stats"]["roster"] = json!(foreign_roster.pin());
    // Bindings must exist before the seal; empty arrays let this isolate roster FK.
    next["content"]["stats"]["assignments"] = json!([]);
    assert!(sqlx::query("INSERT INTO variant_stat_snapshots(world_id,variant_id,revision,stats_defined,roster_id,roster_version,before_json,after_json) VALUES (?,?,3,1,?,1,?,?)").bind(&w.id).bind(&v.id).bind(&foreign_roster.id).bind(&good).bind(next.to_string()).execute(&mut *tx).await.is_err());
    tx.rollback().await.unwrap();
    sqlx::query("DROP TRIGGER variant_stat_update")
        .execute(&l.pool)
        .await
        .unwrap();
    let mut cases = vec![];
    let mut bad = baseline.clone();
    bad["content"]["stats"]["roster"] = json!(foreign_roster.pin());
    cases.push(bad);
    let mut bad = baseline.clone();
    bad["content"]["stats"]["assignments"]
        .as_array_mut()
        .unwrap()
        .reverse();
    cases.push(bad);
    let mut bad = baseline.clone();
    bad["content"]["stats"]["assignments"]
        .as_array_mut()
        .unwrap()
        .pop();
    cases.push(bad);
    let mut bad = baseline.clone();
    bad["content"]["stats"]["assignments"][0]["definition"]["constraints"]["whole_only"] =
        true.into();
    cases.push(bad);
    let mut bad = baseline.clone();
    bad["content"]["stats"]["assignments"][0]["value"] = json!("1");
    cases.push(bad);
    for bad in cases {
        sqlx::query(
            "UPDATE variant_stat_snapshots SET after_json=? WHERE variant_id=? AND revision=2",
        )
        .bind(bad.to_string())
        .bind(&v.id)
        .execute(&l.pool)
        .await
        .unwrap();
        assert_eq!(
            l.values.variant(&w.id, &v.id).await.unwrap_err(),
            Error::Unavailable
        );
    }
    sqlx::query("UPDATE variant_stat_snapshots SET after_json=? WHERE variant_id=? AND revision=2")
        .bind(&good)
        .bind(&v.id)
        .execute(&l.pool)
        .await
        .unwrap();
    // Current archive state must never invalidate exact historical dependencies.
    let archived = archive_definition(&l, &a, 6).await;
    assert_eq!(
        l.values.variant(&w.id, &v.id).await.unwrap().roster,
        Some(r.pin())
    );
    // Match JSON to a real, same-world typed revision, but not the roster's pin.
    // This isolates roster equality from ordinary JSON/binding equality checks.
    sqlx::query("DROP TRIGGER variant_stat_binding_update")
        .execute(&l.pool)
        .await
        .unwrap();
    let mut bad = baseline.clone();
    bad["content"]["stats"]["assignments"][0]["definition"] = json!(archived);
    sqlx::query("UPDATE variant_stat_bindings SET stat_version=2 WHERE variant_id=? AND slot=1")
        .bind(&v.id)
        .execute(&l.pool)
        .await
        .unwrap();
    sqlx::query("UPDATE variant_stat_snapshots SET after_json=? WHERE variant_id=? AND revision=2")
        .bind(bad.to_string())
        .bind(&v.id)
        .execute(&l.pool)
        .await
        .unwrap();
    assert_eq!(
        l.values.variant(&w.id, &v.id).await.unwrap_err(),
        Error::Unavailable
    );
    sqlx::query("UPDATE variant_stat_bindings SET stat_version=1 WHERE variant_id=? AND slot=1")
        .bind(&v.id)
        .execute(&l.pool)
        .await
        .unwrap();
    sqlx::query("UPDATE variant_stat_snapshots SET after_json=? WHERE variant_id=? AND revision=2")
        .bind(&good)
        .bind(&v.id)
        .execute(&l.pool)
        .await
        .unwrap();
    sqlx::query("CREATE TRIGGER variant_stat_binding_update BEFORE UPDATE ON variant_stat_bindings BEGIN SELECT RAISE(ABORT,'immutable stats'); END").execute(&l.pool).await.unwrap();
    assert_eq!(
        l.values.variant(&w.id, &v.id).await.unwrap().roster,
        Some(r.pin())
    );
    for sql in [
        "UPDATE variant_stat_bindings SET slot=3 WHERE variant_id=?",
        "DELETE FROM variant_stat_bindings WHERE variant_id=?",
    ] {
        assert!(sqlx::query(sql).bind(&v.id).execute(&l.pool).await.is_err());
    }
    assert!(sqlx::query("INSERT INTO variant_stat_bindings(world_id,variant_id,revision,slot,scope,stat_id,stat_version) VALUES (?,?,2,3,?,?,1)").bind(&w.id).bind(&v.id).bind(&w.id).bind(&a.id).execute(&l.pool).await.is_err());
    let mut c = l.pool.acquire().await.unwrap();
    sqlx::query("DROP TRIGGER variant_stat_delete")
        .execute(&mut *c)
        .await
        .unwrap();
    sqlx::query("PRAGMA foreign_keys=OFF")
        .execute(&mut *c)
        .await
        .unwrap();
    sqlx::query("DELETE FROM variant_stat_snapshots WHERE variant_id=? AND revision=2")
        .bind(&v.id)
        .execute(&mut *c)
        .await
        .unwrap();
    sqlx::query("PRAGMA foreign_keys=ON")
        .execute(&mut *c)
        .await
        .unwrap();
    drop(c);
    assert_eq!(
        l.values.variant(&w.id, &v.id).await.unwrap_err(),
        Error::Unavailable
    );
    assert_eq!(
        l.types
            .variant_typing(&w.id, &v.id, None)
            .await
            .unwrap_err(),
        Error::Unavailable
    );
    assert_eq!(
        l.author
            .history_entry(HistoryKind::Variant, &v.id, 2)
            .await
            .unwrap_err(),
        Error::Unavailable
    );
    let audits = l.count("audit_events").await;
    assert_eq!(
        l.author
            .save_variant(&l.token, &w.id, &v.id, edit(2, "Must fail"))
            .await
            .unwrap_err(),
        Error::Unavailable
    );
    assert_eq!(l.count("audit_events").await, audits);
}

#[tokio::test]
async fn v3_guards_and_composite_deferred_fks_require_exact_owner_markers_even_for_empty_seals() {
    let l = Lab::new(6).await;
    let s = l
        .author
        .create_species(&l.token, fields("Creature"), None)
        .await
        .unwrap();
    let f = &s.default_form_id;
    let old = l
        .author
        .history_entry(HistoryKind::Form, f, 1)
        .await
        .unwrap()
        .after_json;
    let mut v3: Value = serde_json::from_str(&old).unwrap();
    v3["content"]["revision"] = 2.into();
    let mut v2 = v3.clone();
    v2["snapshot_version"] = 2.into();
    v2["content"].as_object_mut().unwrap().remove("stats");
    // A seal with no owner cannot commit, even with no child bindings.
    let mut tx = persistence::begin_write(&l.pool).await.unwrap();
    sqlx::query("INSERT INTO form_stat_snapshots(form_id,revision,stats_defined,before_json,after_json) VALUES (?,2,0,?,?)").bind(f).bind(&old).bind(v3.to_string()).execute(&mut *tx).await.unwrap();
    assert!(tx.commit().await.is_err());
    let mut tx = persistence::begin_write(&l.pool).await.unwrap();
    let a = audit(&mut tx, 99).await;
    let p1: String =
        sqlx::query_scalar("SELECT after_json FROM form_versions WHERE form_id=? AND revision=1")
            .bind(f)
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    let p2: String = sqlx::query_scalar(
        "SELECT after_json FROM form_content_snapshots WHERE form_id=? AND revision=1",
    )
    .bind(f)
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    sqlx::query("INSERT INTO form_content_snapshots(form_id,revision,typing_defined,before_json,after_json) VALUES (?,2,0,?,?)").bind(f).bind(&p2).bind(v2.to_string()).execute(&mut *tx).await.unwrap();
    let sql = "INSERT INTO form_versions(form_id,species_id,species_version,slug,display_name,description,revision,is_default,archived_at,audit_id,previous_revision,before_json,after_json,content_snapshot_version,stat_snapshot_version) SELECT form_id,species_id,species_version,slug,display_name,description,2,is_default,archived_at,?2,1,after_json,json_set(after_json,'$.content.revision',2),2,?3 FROM form_versions WHERE form_id=?1 AND revision=1";
    // v2 alone cannot authorize a new revision under either extension discriminator.
    for marker in [0, 3] {
        assert!(
            sqlx::query(sql)
                .bind(f)
                .bind(&a)
                .bind(marker)
                .execute(&mut *tx)
                .await
                .is_err()
        );
    }
    tx.rollback().await.unwrap();
    // Independently test the deferred marker FK by bypassing only its insert guard.
    sqlx::query("DROP TRIGGER form_stat_guard")
        .execute(&l.pool)
        .await
        .unwrap();
    let mut tx = persistence::begin_write(&l.pool).await.unwrap();
    let a = audit(&mut tx, 99).await;
    sqlx::query("INSERT INTO form_content_snapshots(form_id,revision,typing_defined,before_json,after_json) VALUES (?,2,0,?,?)").bind(f).bind(p2).bind(v2.to_string()).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO form_stat_snapshots(form_id,revision,stats_defined,before_json,after_json) VALUES (?,2,0,?,?)").bind(f).bind(old).bind(v3.to_string()).execute(&mut *tx).await.unwrap();
    sqlx::query(sql)
        .bind(f)
        .bind(a)
        .bind(0_i64)
        .execute(&mut *tx)
        .await
        .unwrap();
    assert!(tx.commit().await.is_err());
    assert_eq!(l.count("form_versions").await, 1);
    assert_eq!(l.count("form_stat_snapshots").await, 1);
    assert_eq!(
        serde_json::from_str::<Value>(&p1).unwrap()["snapshot_version"],
        1
    );
    l.integrity().await;
}

#[tokio::test]
async fn stat_bindings_do_not_inherit_the_type_cardinality_limit() {
    let l = Lab::new(6).await;
    let w = l
        .author
        .create_world(&l.token, fields("World"), None)
        .await
        .unwrap();
    let v = l
        .author
        .create_variant(&l.token, &w.id, 1, None, fields("Local"), None)
        .await
        .unwrap();
    let mut entries = vec![];
    for n in 100..136 {
        entries.push(definition(&l, Some(&w.id), n).await);
    }
    roster(&l, &w.id, entries, 200).await;
    l.values
        .reconcile(
            &l.token,
            &w.id,
            &v.id,
            VariantStatsEdit {
                expected_revision: 1,
                expected_roster_revision: 1,
                values: vec![],
                reason: None,
            },
        )
        .await
        .unwrap();
    let state = l.values.variant(&w.id, &v.id).await.unwrap();
    assert_eq!(state.assignments.unwrap().len(), 36);
    assert_eq!(l.count("variant_stat_bindings").await, 36);
    l.integrity().await;
}
