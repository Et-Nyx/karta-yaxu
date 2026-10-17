//! Public capture contracts against temporary, test-owned databases only.
use game_data::{
    authoring::{ContentFields, slug_from_name},
    inspection::*,
    moves::NumericValue,
};
use sqlx::{AssertSqlSafe, Connection, SqliteConnection, SqlitePool, sqlite::SqlitePoolOptions};
use std::{
    collections::{BTreeMap, BTreeSet},
    time::Duration,
};
use tempfile::TempDir;
use tokio::{
    sync::oneshot,
    time::{Instant, timeout},
};
use web_app::{
    accounts::Accounts,
    application::{
        authoring::{AddressKey, Authoring, Edit, Error, SourcePin},
        fixtures::{Fixtures, SeedOptions},
        inspection::{WorldInspection, fingerprint},
        learning::{Learning, LearningRecordInput, LearnsetAdoptEdit, LearnsetEdit},
        moves::{MoveEdit, Moves, NewMove},
        types::Types,
    },
    auth::{Role, Subject},
    persistence,
};

const CLASSIC: &str = "da05ae68560e4534ae229ce6139494ae";
const VARIANT: &str = "5d2d9dba132840f3acbaa5ed1e2b0b49";
const SAMPLE_FORM: &str = "31e00000000040008000000000000001";
const SAMPLE_VARIANT: &str = "31f00000000040008000000000000001";
const PROBE: &str = "42b00000000040008000000000000001";
const LEARNSET: &str = "44a00000000040008000000000000002";

#[tokio::test]
async fn public_inspection_admission_is_shared_and_retained_through_usage_without_writes() {
    use game_data::inspection_analysis::{UseView, VersionFilter, WhereUsedQuery};
    let l = Lab::new().await;
    let before = l.snapshot().await;
    let inspector = l.inspect.clone();
    let first = inspector
        .inspect(AddressKey::Id(CLASSIC.into()), false)
        .await
        .unwrap();
    let second = l
        .inspect
        .inspect(AddressKey::Id(VARIANT.into()), true)
        .await
        .unwrap();
    assert!(matches!(
        inspector
            .inspect(AddressKey::Id(CLASSIC.into()), false)
            .await,
        Err(Error::Unavailable)
    ));
    let fp = first.fingerprint.clone();
    let (first, uses) = inspector
        .usages(
            first,
            WhereUsedQuery {
                target: key(DomainKind::Move, Some(CLASSIC), PROBE, 1).identity(),
                versions: VersionFilter::Exact(1),
                view: UseView::DirectUses,
                page: 1,
            },
        )
        .await
        .unwrap();
    assert_eq!(first.fingerprint, fp);
    assert!(uses.complete && !uses.uses.is_empty());
    assert!(matches!(
        l.inspect
            .inspect(AddressKey::Id(CLASSIC.into()), false)
            .await,
        Err(Error::Unavailable)
    ));
    drop(first);
    let third = l
        .inspect
        .inspect(AddressKey::Id(CLASSIC.into()), false)
        .await
        .unwrap();
    assert!(third.capture.is_complete());
    drop(third);
    drop(second);
    assert_eq!(l.snapshot().await, before);
}

fn fields(name: &str) -> ContentFields {
    ContentFields {
        slug: slug_from_name(name),
        display_name: name.into(),
        description: "Synthetic inspection test.".into(),
    }
}
fn key(kind: DomainKind, world: Option<&str>, id: &str, revision: i64) -> DomainNodeKey {
    DomainNodeKey {
        kind,
        world_id: world.map(str::to_owned),
        id: id.into(),
        revision,
    }
}

fn fixture_roots(world: &str) -> BTreeSet<DomainNodeKey> {
    // Independent manifest oracle: no persistence reader or captured graph is
    // used to derive the expected membership identities.
    let original: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/worlds.json")).unwrap();
    let stats: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/stats.json")).unwrap();
    let moves: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/moves.json")).unwrap();
    let charts: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/charts.json")).unwrap();
    let mut roots = BTreeSet::from([key(DomainKind::World, None, world, 2)]);
    let w = original["worlds"]
        .as_array()
        .unwrap()
        .iter()
        .find(|w| w["id"] == world)
        .unwrap();
    for t in w["types"].as_array().unwrap() {
        roots.insert(key(
            DomainKind::Type,
            Some(world),
            t["id"].as_str().unwrap(),
            1,
        ));
    }
    for v in w["variants"].as_array().unwrap() {
        roots.insert(key(
            DomainKind::Variant,
            Some(world),
            v["record"]["id"].as_str().unwrap(),
            1,
        ));
    }
    for (manifest, collection, kind) in [
        (&stats, "definitions", DomainKind::Stat),
        (&stats, "rosters", DomainKind::StatRoster),
        (&stats, "variants", DomainKind::Variant),
        (&moves, "attributes", DomainKind::MoveAttribute),
        (&moves, "moves", DomainKind::Move),
        (&moves, "rulesets", DomainKind::Ruleset),
        (&moves, "learnsets", DomainKind::Learnset),
        (&moves, "presets", DomainKind::Preset),
        (&charts, "charts", DomainKind::Chart),
    ] {
        for row in manifest[collection]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["world_id"] == world)
        {
            roots.insert(key(kind, Some(world), row["id"].as_str().unwrap(), 1));
        }
    }
    roots
}
struct Lab {
    pool: SqlitePool,
    accounts: Accounts,
    inspect: WorldInspection,
    token: String,
    _dir: TempDir,
}
impl Lab {
    async fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let pool = persistence::connect_for_migration(&format!(
            "sqlite://{}/inspection.db",
            dir.path().display()
        ))
        .await
        .unwrap();
        persistence::migrate(&pool).await.unwrap();
        let accounts = Accounts::new(pool.clone(), 3600).unwrap();
        Fixtures::new(accounts.clone())
            .seed(SeedOptions {
                dev_auth: true,
                ..Default::default()
            })
            .await
            .unwrap();
        let actor = accounts
            .sign_in(Subject {
                provider: "placeholder".into(),
                provider_subject: "admin".into(),
                display_name: "admin".into(),
            })
            .await
            .unwrap();
        let (token, _) = accounts.start_session(&actor.user_id).await.unwrap();
        Self {
            inspect: WorldInspection::new(accounts.clone()),
            accounts,
            pool,
            token,
            _dir: dir,
        }
    }
    async fn capture(&self, world: &str) -> CapturedWorld {
        let r = self
            .inspect
            .capture(AddressKey::Id(world.into()))
            .await
            .unwrap();
        assert!(r.is_complete(), "{:?}", r.gaps);
        r
    }
    async fn snapshot(&self) -> BTreeMap<String, Vec<String>> {
        let tables: Vec<String> = sqlx::query_scalar("SELECT name FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name").fetch_all(&self.pool).await.unwrap();
        let mut result = BTreeMap::new();
        for table in tables {
            let columns: Vec<String> =
                sqlx::query_scalar("SELECT name FROM pragma_table_info(?) ORDER BY cid")
                    .bind(&table)
                    .fetch_all(&self.pool)
                    .await
                    .unwrap();
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
            result.insert(table, rows);
        }
        result
    }
}

// Test-owned TEMP views make a real capture SELECT execute a long SQLite VM
// query. The collation sends a rendezvous from its recursive CTE's first step;
// tests never infer "in flight" from a sleep or a zero-duration deadline.
async fn slow_capture_view(
    c: &mut SqliteConnection,
    table: &str,
    iterations: u64,
) -> oneshot::Receiver<()> {
    assert!(matches!(table, "moves" | "worlds"));
    let (sender, started) = oneshot::channel();
    let sender = std::sync::Mutex::new(Some(sender));
    c.lock_handle()
        .await
        .unwrap()
        .create_collation("inspection_step_started", move |_, _| {
            if let Some(sender) = sender.lock().unwrap().take() {
                let _ = sender.send(());
            }
            std::cmp::Ordering::Equal
        })
        .unwrap();
    sqlx::query(AssertSqlSafe(format!("CREATE TEMP VIEW {table} AS SELECT * FROM main.{table} WHERE (WITH RECURSIVE spin(n) AS (SELECT CASE WHEN 'a' COLLATE inspection_step_started = 'b' THEN 0 ELSE 0 END UNION ALL SELECT n+1 FROM spin WHERE n<{iterations}) SELECT max(n) FROM spin)>=0")))
        .execute(&mut *c).await.unwrap();
    started
}

#[tokio::test]
async fn complete_65_object_fixture_has_independently_expected_world_roots_and_explicit_closure() {
    let l = Lab::new().await;
    let before = l.snapshot().await;
    for (world, expected_roots, expected_nodes, local_types, local_stats, variants, shared_forms) in
        [(CLASSIC, 20, 37, 3, 3, 3, 3), (VARIANT, 23, 39, 4, 4, 4, 2)]
    {
        let c = l.capture(world).await;
        assert_eq!(c.roots.len(), expected_roots);
        assert_eq!(
            c.roots.keys().cloned().collect::<BTreeSet<_>>(),
            fixture_roots(world)
        );
        assert_eq!(c.nodes.len(), expected_nodes);
        let expected = BTreeMap::from([
            (DomainKind::World, 1),
            (DomainKind::Type, local_types),
            (DomainKind::Stat, local_stats),
            (DomainKind::Variant, variants),
            (DomainKind::MoveAttribute, 2),
            (DomainKind::Move, 2),
            (DomainKind::Chart, 1),
            (DomainKind::StatRoster, 1),
            (DomainKind::Ruleset, 1),
            (DomainKind::Learnset, 1),
            (DomainKind::Preset, 2),
        ]);
        let mut actual = BTreeMap::new();
        for k in c.roots.keys() {
            *actual.entry(k.kind).or_insert(0) += 1;
        }
        assert_eq!(actual, expected);
        assert_eq!(
            c.nodes
                .keys()
                .filter(|k| k.kind == DomainKind::Form)
                .count(),
            shared_forms
        );
        assert!(!c.nodes.values().any(|p| {
            p.fields()
                .is_some_and(|f| ["Mossback", "Tidefin"].contains(&f.display_name.as_str()))
        }));
        assert!(
            c.nodes
                .keys()
                .all(|k| k.world_id.as_deref().is_none_or(|w| w == world))
        );
        // The Emberling default-form backlink is identity-only. It must not pull
        // an unrelated current shared form into the graph.
        let ember = c
            .nodes
            .iter()
            .find_map(|(_, p)| {
                if let InspectionPayload::Species(r) = p {
                    (r.fields.display_name == "Emberling").then_some(r)
                } else {
                    None
                }
            })
            .unwrap();
        assert!(
            !c.nodes
                .keys()
                .any(|k| k.kind == DomainKind::Form && k.id == ember.default_form_id)
        );
        assert_eq!(
            c.absences
                .iter()
                .filter(|a| a.kind == DomainKind::Learnset)
                .count(),
            variants - 1
        );
        assert_eq!(
            c.absences
                .iter()
                .filter(|a| a.kind == DomainKind::Preset)
                .count(),
            variants - 1
        );
        assert_eq!(
            c.edges
                .iter()
                .filter(|e| e.kind == EdgeKind::Membership)
                .count(),
            expected_roots - 1
        );
        assert_eq!(
            c.edges
                .iter()
                .filter(|e| matches!(e.site, EdgeSite::Mapping { .. }))
                .count(),
            18
        );
        assert_eq!(
            c.edges
                .iter()
                .filter(|e| e.kind == EdgeKind::Ownership && matches!(&e.to, DomainRef::Exact(_)))
                .count(),
            3
        );
        // Full acquisition context includes two distinct records of Probe.
        let local_set = c
            .roots
            .keys()
            .find(|k| k.kind == DomainKind::Learnset)
            .unwrap();
        let acquisitions = c
            .edges
            .iter()
            .filter(|e| &e.from == local_set && matches!(e.site, EdgeSite::Acquisition { .. }))
            .collect::<Vec<_>>();
        assert_eq!(acquisitions.len(), 3);
        assert_eq!(acquisitions[0].to, acquisitions[1].to);
        assert_ne!(acquisitions[0].site, acquisitions[1].site);
        let selections = c
            .edges
            .iter()
            .filter(|e| e.kind == EdgeKind::Selection)
            .collect::<Vec<_>>();
        assert_eq!(selections.len(), 4);
        for e in selections {
            assert!(matches!(
                e.site,
                EdgeSite::PresetSelection { slot: 1..=3, .. }
            ));
        }
        assert!(
            c.nodes
                .values()
                .filter_map(|p| if let InspectionPayload::Preset(r) = p {
                    Some(r.validator_version)
                } else {
                    None
                })
                .all(|v| v == 1)
        );
        assert!(fingerprint(&c).is_some());
    }
    assert_eq!(l.snapshot().await, before);
}

#[tokio::test]
async fn repeated_selection_sites_and_exact_old_versions_survive_current_renames() {
    let l = Lab::new().await;
    let moves = Moves::new(l.accounts.clone());
    let old = moves.detail(Some(CLASSIC), PROBE).await.unwrap();
    moves
        .save(
            &l.token,
            Some(CLASSIC),
            PROBE,
            MoveEdit {
                expected_revision: 1,
                fields: fields("Probe renamed"),
                attributes: old.attributes.as_ref().map(|v| {
                    v.iter()
                        .map(|a| web_app::application::moves::AttributeValueInput {
                            attribute: a.definition.pin(),
                            value: a.value.clone(),
                        })
                        .collect()
                }),
                reason: None,
            },
        )
        .await
        .unwrap();
    let author = Authoring::new(l.accounts.clone());
    let form = author.form_detail(SAMPLE_FORM).await.unwrap();
    author
        .save_form(
            &l.token,
            SAMPLE_FORM,
            form.species_version,
            Edit {
                expected_revision: form.revision,
                fields: fields("Sample renamed"),
                reason: None,
            },
        )
        .await
        .unwrap();
    let c = l.capture(CLASSIC).await;
    assert!(
        c.nodes
            .contains_key(&key(DomainKind::Move, Some(CLASSIC), PROBE, 1))
    );
    assert!(
        c.nodes
            .contains_key(&key(DomainKind::Move, Some(CLASSIC), PROBE, 2))
    );
    assert!(
        c.nodes
            .contains_key(&key(DomainKind::Form, None, SAMPLE_FORM, 1))
    );
    assert!(
        !c.nodes
            .contains_key(&key(DomainKind::Form, None, SAMPLE_FORM, 2))
    );
    assert_eq!(
        c.heads[&key(DomainKind::Form, None, SAMPLE_FORM, 1).identity()].revision,
        2
    );
    let c = l.capture(VARIANT).await;
    let repeated = key(
        DomainKind::Preset,
        Some(VARIANT),
        "46a00000000040008000000000000004",
        1,
    );
    let selected = c
        .edges
        .iter()
        .filter(|e| e.from == repeated && e.kind == EdgeKind::Selection)
        .collect::<Vec<_>>();
    assert_eq!(selected.len(), 3);
    assert_eq!(selected[0].to, selected[2].to);
    assert!(
        matches!(&selected[0].site, EdgeSite::PresetSelection { slot: 1, record_id, .. } if record_id == "45a00000000040008000000000000001")
    );
    assert!(
        matches!(&selected[2].site, EdgeSite::PresetSelection { slot: 3, record_id, .. } if record_id == "45a00000000040008000000000000001")
    );
}

#[tokio::test]
async fn source_form_does_not_inherit_independent_learnset_and_all_absences_are_explicit() {
    let l = Lab::new().await;
    let author = Authoring::new(l.accounts.clone());
    let w = author
        .create_world(&l.token, fields("Empty context"), None)
        .await
        .unwrap();
    let v = author
        .create_variant(
            &l.token,
            &w.id,
            w.revision,
            Some(SourcePin {
                form_id: SAMPLE_FORM.into(),
                version: 1,
            }),
            fields("Source only"),
            None,
        )
        .await
        .unwrap();
    let c = l.capture(&w.id).await;
    assert!(
        c.nodes
            .contains_key(&key(DomainKind::Form, None, SAMPLE_FORM, 1))
    );
    assert!(!c.nodes.keys().any(|k| k.kind == DomainKind::Learnset));
    assert_eq!(
        c.absences.into_iter().collect::<BTreeSet<_>>(),
        BTreeSet::from([
            InventoryAbsence {
                kind: DomainKind::Chart,
                owner_id: None
            },
            InventoryAbsence {
                kind: DomainKind::StatRoster,
                owner_id: None
            },
            InventoryAbsence {
                kind: DomainKind::Ruleset,
                owner_id: None
            },
            InventoryAbsence {
                kind: DomainKind::Learnset,
                owner_id: Some(v.id.clone())
            },
            InventoryAbsence {
                kind: DomainKind::Preset,
                owner_id: Some(v.id)
            },
        ])
    );
}

#[tokio::test]
async fn explicit_shared_learnset_keeps_its_owner_identity_without_expanding_that_form() {
    let l = Lab::new().await;
    let author = Authoring::new(l.accounts.clone());
    let unrelated = author
        .create_species(&l.token, fields("Separate acquisition owner"), None)
        .await
        .unwrap();
    let learning = Learning::new(l.accounts.clone());
    let shared = learning
        .save_learnset(
            &l.token,
            None,
            &unrelated.default_form_id,
            LearnsetEdit {
                expected_revision: 0,
                records: Some(vec![]),
                reason: None,
            },
        )
        .await
        .unwrap();
    let world = author
        .create_world(&l.token, fields("Explicit learning source"), None)
        .await
        .unwrap();
    let variant = author
        .create_variant(&l.token, &world.id, 1, None, fields("Local owner"), None)
        .await
        .unwrap();
    learning
        .adopt_learnset(
            &l.token,
            &world.id,
            &variant.id,
            LearnsetAdoptEdit {
                expected_revision: 0,
                source: shared.pin(),
                apply_source: true,
                mappings: vec![],
                reason: None,
            },
        )
        .await
        .unwrap();
    let c = l.capture(&world.id).await;
    assert!(
        c.nodes
            .contains_key(&key(DomainKind::Learnset, None, &shared.id, 1))
    );
    assert!(
        !c.nodes
            .keys()
            .any(|k| k.id == unrelated.default_form_id || k.id == unrelated.id)
    );
    assert!(
        c.edges
            .iter()
            .any(|e| e.from == key(DomainKind::Learnset, None, &shared.id, 1)
                && e.to
                    == DomainRef::Identity(DomainIdentity {
                        kind: DomainKind::Form,
                        world_id: None,
                        id: unrelated.default_form_id.clone()
                    }))
    );
}

#[tokio::test]
async fn known_unreferenced_current_head_companions_are_preflighted_before_exact_old_read() {
    let l = Lab::new().await;
    let author = Authoring::new(l.accounts.clone());
    let species = author
        .create_species(&l.token, fields("Companion"), None)
        .await
        .unwrap();
    let world = author
        .create_world(&l.token, fields("Companion context"), None)
        .await
        .unwrap();
    author
        .create_variant(
            &l.token,
            &world.id,
            1,
            Some(SourcePin {
                form_id: species.default_form_id.clone(),
                version: 1,
            }),
            fields("Old exact consumer"),
            None,
        )
        .await
        .unwrap();
    let mut renamed = fields("Companion renamed");
    renamed.description = "Large but valid descriptive metadata.".repeat(50);
    author
        .save_form(
            &l.token,
            &species.default_form_id,
            1,
            Edit {
                expected_revision: 1,
                fields: renamed,
                reason: None,
            },
        )
        .await
        .unwrap();
    let old_size: i64 = sqlx::query_scalar("SELECT length(CAST(after_json AS BLOB)) FROM form_stat_snapshots WHERE form_id=? AND revision=1").bind(&species.default_form_id).fetch_one(&l.pool).await.unwrap();
    let limits = InspectionLimits {
        max_snapshot_bytes: usize::try_from(old_size).unwrap() + 1024,
        ..Default::default()
    };
    let before = l.snapshot().await;
    let c = l
        .inspect
        .capture_with_limits(AddressKey::Id(world.id), limits)
        .await
        .unwrap();
    assert!(c.gaps.iter().any(|g| {
        g.kind == CaptureGapKind::Budget
            && g.subject
                .as_ref()
                .is_some_and(|k| k.id == species.default_form_id && k.revision == 2)
    }));
    assert!(fingerprint(&c).is_none());
    assert_eq!(l.snapshot().await, before);
}

#[tokio::test]
async fn missing_current_membership_and_corrupt_shared_scope_cannot_pass_as_complete() {
    let l = Lab::new().await;
    for trigger in ["world_revisions_update", "world_content_update"] {
        sqlx::query(AssertSqlSafe(format!("DROP TRIGGER {trigger}")))
            .execute(&l.pool)
            .await
            .unwrap();
    }
    // Make the v1 projection and v2 seal agree while omitting a real member.
    // Existing historical membership checks alone only prove listed IDs exist.
    for table in ["world_revisions", "world_content_snapshots"] {
        sqlx::query(AssertSqlSafe(format!("UPDATE {table} SET after_json=json_set(after_json,'$.content.variant_ids',json('[]')) WHERE world_id=? AND revision=2"))).bind(CLASSIC).execute(&l.pool).await.unwrap();
    }
    assert_eq!(
        l.inspect.capture(AddressKey::Id(CLASSIC.into())).await,
        Err(Error::Unavailable)
    );
    let l = Lab::new().await;
    sqlx::query("DROP TRIGGER move_versions_update")
        .execute(&l.pool)
        .await
        .unwrap();
    // A malformed shared snapshot may otherwise recurse into its own alleged
    // shared source before an older reader reaches its scope comparison.
    sqlx::query("UPDATE move_versions SET after_json=json_set(after_json,'$.content.world_id',?,'$.content.source',json_object('move_id',move_id,'version',1)) WHERE scope='' AND move_id='42a00000000040008000000000000001'").bind(CLASSIC).execute(&l.pool).await.unwrap();
    let c = l
        .inspect
        .capture(AddressKey::Id(CLASSIC.into()))
        .await
        .unwrap();
    assert!(c.gaps.iter().any(|g| g.kind == CaptureGapKind::Integrity));
    assert!(fingerprint(&c).is_none());
}

#[tokio::test]
async fn unused_archived_local_roots_are_inventoried_without_shared_catalog_expansion() {
    let l = Lab::new().await;
    let moves = Moves::new(l.accounts.clone());
    let new = |name: &str| NewMove {
        fields: fields(name),
        source: None,
        attributes: None,
        reason: None,
    };
    let unused = moves
        .create(&l.token, Some(CLASSIC), new("Unused local"))
        .await
        .unwrap();
    let shared = moves
        .create(&l.token, None, new("Unrelated shared"))
        .await
        .unwrap();
    // Consistently archived synthetic initial content: archive/revert authoring
    // itself is deliberately a separate milestone.
    sqlx::query("DROP TRIGGER move_versions_update")
        .execute(&l.pool)
        .await
        .unwrap();
    sqlx::query("UPDATE move_versions SET archived_at=123,after_json=json_set(after_json,'$.content.archived_at',123) WHERE scope=? AND move_id=?").bind(CLASSIC).bind(&unused.id).execute(&l.pool).await.unwrap();
    // This test initializes a coherent archived definition, including its new
    // same-revision card companion. Mismatched companions are covered separately.
    sqlx::query("DROP TRIGGER card_components_sealed_update")
        .execute(&l.pool)
        .await
        .unwrap();
    sqlx::query("UPDATE card_components SET snapshot_json=json_set(snapshot_json,'$.content.archived_at',123) WHERE namespace='move' AND scope=? AND card_id=? AND revision=1").bind(CLASSIC).bind(&unused.id).execute(&l.pool).await.unwrap();
    let c = l.capture(CLASSIC).await;
    let k = key(DomainKind::Move, Some(CLASSIC), &unused.id, 1);
    assert_eq!(c.roots[&k], RootRole::Archived);
    assert_eq!(c.heads[&k.identity()].archived_at, Some(123));
    assert!(!c.nodes.keys().any(|k| k.id == shared.id));
    assert!(
        !c.edges
            .iter()
            .any(|e| e.to == DomainRef::Exact(k.clone()) && e.kind != EdgeKind::Membership)
    );
}

#[tokio::test]
async fn fingerprint_is_canonical_public_only_and_ignores_access_and_account_updates() {
    let l = Lab::new().await;
    let mut c = l.capture(CLASSIC).await;
    let f = fingerprint(&c).unwrap();
    c.edges.reverse();
    c.absences.reverse();
    assert_eq!(fingerprint(&c).as_deref(), Some(f.as_str()));
    l.accounts
        .invite("test", "collaborator", Role::Editor, None)
        .await
        .unwrap();
    let u = l
        .accounts
        .sign_in(Subject {
            provider: "test".into(),
            provider_subject: "collaborator".into(),
            display_name: "PRIVATE_SENTINEL".into(),
        })
        .await
        .unwrap();
    Authoring::new(l.accounts.clone())
        .grant_collaborator(
            &l.token,
            CLASSIC,
            1,
            &u.user_id,
            Some("PRIVATE_ACCESS_REASON"),
        )
        .await
        .unwrap();
    sqlx::query("UPDATE users SET display_name='PRIVATE_RENAMED' WHERE id=?")
        .bind(&u.user_id)
        .execute(&l.pool)
        .await
        .unwrap();
    let next = l.capture(CLASSIC).await;
    assert_eq!(fingerprint(&next).as_deref(), Some(f.as_str()));
    // Serialize the same canonical vectors as the hashing boundary, since JSON
    // cannot represent structured BTreeMap keys as object property names.
    let public = serde_json::to_string(&(
        next.world,
        next.nodes.into_iter().collect::<Vec<_>>(),
        next.heads.into_iter().collect::<Vec<_>>(),
        next.edges,
        next.absences,
    ))
    .unwrap();
    for secret in [
        "owner_user_id",
        "access_revision",
        "actor_user_id",
        "PRIVATE_",
        "token_hash",
        "before_json",
        "after_json",
    ] {
        assert!(!public.contains(secret), "{secret}");
    }
    c.edges.pop();
    assert_ne!(fingerprint(&c).as_deref(), Some(f.as_str()));
}

#[tokio::test]
async fn every_budget_is_partial_identified_and_read_only() {
    let l = Lab::new().await;
    let before = l.snapshot().await;
    for limits in [
        InspectionLimits {
            max_roots: 1,
            ..Default::default()
        },
        InspectionLimits {
            max_nodes: 1,
            ..Default::default()
        },
        InspectionLimits {
            max_edges: 1,
            ..Default::default()
        },
        InspectionLimits {
            max_snapshot_bytes: 1,
            ..Default::default()
        },
        InspectionLimits {
            max_total_snapshot_bytes: 1,
            ..Default::default()
        },
    ] {
        let mut c = l.pool.acquire().await.unwrap();
        sqlx::query("PRAGMA query_only=ON")
            .execute(&mut *c)
            .await
            .unwrap();
        let r = persistence::inspection::capture_world(
            &mut c,
            AddressKey::Id(CLASSIC.into()),
            limits.clone(),
        )
        .await
        .unwrap();
        assert!(!r.is_complete(), "{limits:?}");
        assert!(
            r.gaps
                .iter()
                .any(|g| g.kind == CaptureGapKind::Budget && g.subject.is_some())
        );
        assert!(fingerprint(&r).is_none());
        assert!(r.edges.len() <= limits.max_edges);
        assert!(r.nodes.len() <= limits.max_nodes.max(1));
        sqlx::query("PRAGMA query_only=OFF")
            .execute(&mut *c)
            .await
            .unwrap();
    }
    assert_eq!(l.snapshot().await, before);
}

#[tokio::test]
async fn read_only_connection_and_query_only_success_need_no_write_privileges() {
    let l = Lab::new().await;
    let before = l.snapshot().await;
    let pool = persistence::connect(&format!(
        "sqlite://{}/inspection.db?mode=ro",
        l._dir.path().display()
    ))
    .await
    .unwrap();
    let service = WorldInspection::new(Accounts::new(pool.clone(), 3600).unwrap());
    assert!(
        service
            .capture(AddressKey::Slug("classiclab".into()))
            .await
            .unwrap()
            .is_complete()
    );
    let mut c = l.pool.acquire().await.unwrap();
    sqlx::query("PRAGMA query_only=ON")
        .execute(&mut *c)
        .await
        .unwrap();
    let changes: i64 = sqlx::query_scalar("SELECT total_changes()")
        .fetch_one(&mut *c)
        .await
        .unwrap();
    assert!(
        persistence::inspection::capture_world(
            &mut c,
            AddressKey::Id(CLASSIC.into()),
            InspectionLimits::default()
        )
        .await
        .unwrap()
        .is_complete()
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT total_changes()")
            .fetch_one(&mut *c)
            .await
            .unwrap(),
        changes
    );
    sqlx::query("PRAGMA query_only=OFF")
        .execute(&mut *c)
        .await
        .unwrap();
    assert_eq!(l.snapshot().await, before);
    pool.close().await;
}

#[tokio::test]
async fn valid_v1_v2_v3_creatures_remain_readable_without_invented_extensions() {
    let l = Lab::new().await;
    let author = Authoring::new(l.accounts.clone());
    let world = author
        .create_world(&l.token, fields("Legacy inspection"), None)
        .await
        .unwrap();
    let mut variants = vec![];
    for n in 1..=3 {
        variants.push(
            author
                .create_variant(
                    &l.token,
                    &world.id,
                    world.revision + n - 1,
                    None,
                    fields(&format!("Legacy {n}")),
                    None,
                )
                .await
                .unwrap(),
        );
    }
    for trigger in [
        "variant_revisions_update",
        "variant_stat_delete",
        "variant_content_delete",
    ] {
        sqlx::query(AssertSqlSafe(format!("DROP TRIGGER {trigger}")))
            .execute(&l.pool)
            .await
            .unwrap();
    }
    let mut tx = l.pool.begin().await.unwrap();
    for (i, v) in variants.iter().take(2).enumerate() {
        sqlx::query("DELETE FROM variant_stat_snapshots WHERE world_id=? AND variant_id=?")
            .bind(&world.id)
            .bind(&v.id)
            .execute(&mut *tx)
            .await
            .unwrap();
        sqlx::query("UPDATE variant_revisions SET stat_snapshot_version=0,content_snapshot_version=? WHERE world_id=? AND variant_id=?").bind(if i == 0 { 1 } else { 2 }).bind(&world.id).bind(&v.id).execute(&mut *tx).await.unwrap();
        if i == 0 {
            sqlx::query("DELETE FROM variant_content_snapshots WHERE world_id=? AND variant_id=?")
                .bind(&world.id)
                .bind(&v.id)
                .execute(&mut *tx)
                .await
                .unwrap();
        }
    }
    tx.commit().await.unwrap();
    let c = l.capture(&world.id).await;
    for v in variants {
        let InspectionPayload::Variant(row) =
            &c.nodes[&key(DomainKind::Variant, Some(&world.id), &v.id, 1)]
        else {
            panic!("variant payload");
        };
        assert!(row.stats.assignments.is_none());
        assert!(row.typing.assignments.is_none());
    }
}

#[tokio::test]
async fn orphan_learning_heads_are_not_hidden_by_owner_joins() {
    for (head, trigger, kind) in [
        (
            "learnsets",
            "learnset_owner_immutable",
            DomainKind::Learnset,
        ),
        (
            "moveset_presets",
            "preset_owner_immutable",
            DomainKind::Preset,
        ),
    ] {
        let l = Lab::new().await;
        sqlx::query(AssertSqlSafe(format!("DROP TRIGGER {trigger}")))
            .execute(&l.pool)
            .await
            .unwrap();
        let mut c = l.pool.acquire().await.unwrap();
        sqlx::query("PRAGMA foreign_keys=OFF")
            .execute(&mut *c)
            .await
            .unwrap();
        sqlx::query(AssertSqlSafe(format!(
            "UPDATE {head} SET variant_id='ffffffffffffffffffffffffffffffff' WHERE world_id=?"
        )))
        .bind(CLASSIC)
        .execute(&mut *c)
        .await
        .unwrap();
        sqlx::query("PRAGMA foreign_keys=ON")
            .execute(&mut *c)
            .await
            .unwrap();
        drop(c);
        let before = l.snapshot().await;
        let c = l
            .inspect
            .capture(AddressKey::Id(CLASSIC.into()))
            .await
            .unwrap();
        assert!(c.gaps.iter().any(|g| g.kind == CaptureGapKind::Integrity
            && g.subject.as_ref().is_some_and(|s| s.kind == kind)));
        assert!(fingerprint(&c).is_none());
        assert_eq!(l.snapshot().await, before);
    }
}

#[tokio::test]
async fn corrupt_children_never_become_unknown_and_anchor_corruption_is_unavailable() {
    for (trigger, statement) in [
        (
            Some("species_versions_update"),
            "UPDATE species_versions SET after_json=json_set(after_json,'$.content.fields.description','different exact projection') WHERE species_id='6a64df7c88604b3f9381536710449a8b'",
        ),
        (
            None,
            "UPDATE forms SET description='different head projection' WHERE id='31e00000000040008000000000000001'",
        ),
        (
            Some("type_versions_update"),
            "UPDATE type_versions SET after_json=json_set(after_json,'$.snapshot_version',99) WHERE type_id='6b4c13114c374eb6891469592b1743de'",
        ),
        (
            Some("variant_stat_update"),
            "UPDATE variant_stat_snapshots SET after_json=json_set(after_json,'$.snapshot_version',99) WHERE variant_id='31f00000000040008000000000000001'",
        ),
        (
            Some("variant_binding_update"),
            "UPDATE variant_type_bindings SET type_id='04d6c4508dd642da9e414ce264302b5f' WHERE variant_id='3df3f58dcbee43ecb1ba4c35bbc8384f' AND slot=1",
        ),
        (
            Some("variant_content_update"),
            "UPDATE variant_content_snapshots SET after_json=json_set(after_json,'$.content.typing.assignments',json('[]')) WHERE variant_id='3df3f58dcbee43ecb1ba4c35bbc8384f'",
        ),
    ] {
        let l = Lab::new().await;
        if let Some(trigger) = trigger {
            sqlx::query(AssertSqlSafe(format!("DROP TRIGGER {trigger}")))
                .execute(&l.pool)
                .await
                .unwrap();
        }
        let mut c = l.pool.acquire().await.unwrap();
        sqlx::query("PRAGMA ignore_check_constraints=ON")
            .execute(&mut *c)
            .await
            .unwrap();
        sqlx::query(statement).execute(&mut *c).await.unwrap();
        sqlx::query("PRAGMA ignore_check_constraints=OFF")
            .execute(&mut *c)
            .await
            .unwrap();
        drop(c);
        let before = l.snapshot().await;
        let r = l
            .inspect
            .capture(AddressKey::Id(CLASSIC.into()))
            .await
            .unwrap();
        assert!(
            r.gaps.iter().any(|g| g.kind == CaptureGapKind::Integrity),
            "{statement}"
        );
        assert!(fingerprint(&r).is_none());
        assert_eq!(l.snapshot().await, before);
    }
    let l = Lab::new().await;
    sqlx::query("DROP TRIGGER world_content_update")
        .execute(&l.pool)
        .await
        .unwrap();
    sqlx::query("UPDATE world_content_snapshots SET after_json=json_set(after_json,'$.content.fields.description','bad current anchor') WHERE world_id=? AND revision=2").bind(CLASSIC).execute(&l.pool).await.unwrap();
    let before = l.snapshot().await;
    let mut c = l.pool.acquire().await.unwrap();
    sqlx::query("PRAGMA query_only=ON")
        .execute(&mut *c)
        .await
        .unwrap();
    assert_eq!(
        persistence::inspection::capture_world(
            &mut c,
            AddressKey::Id(CLASSIC.into()),
            InspectionLimits::default()
        )
        .await,
        Err(Error::Unavailable)
    );
    sqlx::query("PRAGMA query_only=OFF")
        .execute(&mut *c)
        .await
        .unwrap();
    assert_eq!(l.snapshot().await, before);
}

#[tokio::test]
async fn independent_learning_edits_change_capture_without_advancing_world_or_variant() {
    let l = Lab::new().await;
    let old = l.capture(CLASSIC).await;
    let learning = Learning::new(l.accounts.clone());
    let set = learning
        .learnset(Some(CLASSIC), SAMPLE_VARIANT)
        .await
        .unwrap()
        .unwrap();
    learning
        .save_learnset(
            &l.token,
            Some(CLASSIC),
            SAMPLE_VARIANT,
            LearnsetEdit {
                expected_revision: set.revision,
                records: Some(
                    set.records
                        .unwrap()
                        .into_iter()
                        .map(|r| LearningRecordInput {
                            id: r.id,
                            move_pin: r.move_record.pin(),
                            method: Some("Changed descriptive method".into()),
                            context: r.context,
                            condition: r.condition,
                            threshold: NumericValue::Unknown,
                        })
                        .collect(),
                ),
                reason: None,
            },
        )
        .await
        .unwrap();
    let next = l.capture(CLASSIC).await;
    assert_eq!(old.world, next.world);
    let variant = key(DomainKind::Variant, Some(CLASSIC), SAMPLE_VARIANT, 1);
    assert_eq!(old.nodes[&variant], next.nodes[&variant]);
    assert!(
        next.nodes
            .contains_key(&key(DomainKind::Learnset, Some(CLASSIC), LEARNSET, 1))
    );
    assert!(
        next.nodes
            .contains_key(&key(DomainKind::Learnset, Some(CLASSIC), LEARNSET, 2))
    );
    assert_ne!(fingerprint(&old), fingerprint(&next));
}

#[test]
fn domain_kind_urls_and_edge_sites_have_strict_stable_contracts() {
    for (kind, alias) in [
        (DomainKind::World, "world"),
        (DomainKind::Species, "species"),
        (DomainKind::Form, "form"),
        (DomainKind::Variant, "variant"),
        (DomainKind::Type, "type"),
        (DomainKind::Stat, "stat"),
        (DomainKind::StatRoster, "stat-roster"),
        (DomainKind::MoveAttribute, "move-attribute"),
        (DomainKind::Move, "move"),
        (DomainKind::Chart, "chart"),
        (DomainKind::Ruleset, "ruleset"),
        (DomainKind::Learnset, "learnset"),
        (DomainKind::Preset, "preset"),
    ] {
        assert_eq!(kind.as_str(), alias);
        assert_eq!(alias.parse(), Ok(kind));
        assert_eq!(
            serde_json::to_string(&kind).unwrap(),
            format!("\"{alias}\"")
        );
    }
    assert!("stat_roster".parse::<DomainKind>().is_err());
    assert!(serde_json::from_str::<EdgeSite>(r#"{"typing":{"slot":1,"surprise":true}}"#).is_err());
    assert!(serde_json::from_str::<PublicWorldContent>(r#"{"id":"w","fields":{"slug":"w","display_name":"W","description":""},"revision":1,"archived_at":null,"owner_user_id":"private"}"#).is_err());
}

#[tokio::test]
async fn unreferenced_legacy_predecessors_verify_their_own_typed_metadata_and_headers() {
    for (kind, damage) in [
        (DomainKind::Type, "local typed"),
        (DomainKind::Type, "shared typed"),
        (DomainKind::Species, "typed"),
        (DomainKind::Type, "local header"),
        (DomainKind::Type, "shared header"),
        (DomainKind::Species, "header"),
        (DomainKind::World, "header"),
        (DomainKind::Form, "header"),
        (DomainKind::Form, "previous header"),
    ] {
        let l = Lab::new().await;
        let author = Authoring::new(l.accounts.clone());
        let types = Types::new(l.accounts.clone());
        let world = author
            .create_world(&l.token, fields("Companion integrity"), None)
            .await
            .unwrap();
        let rename = || Edit {
            expected_revision: 1,
            fields: fields("Legitimate historical rename"),
            reason: None,
        };
        let (current, table, column, trigger) = match kind {
            DomainKind::Type if damage.starts_with("local") => {
                let t = types
                    .create_world_type(&l.token, &world.id, 1, None, fields("Old local type"), None)
                    .await
                    .unwrap();
                types
                    .save_world_type(&l.token, &world.id, &t.id, rename())
                    .await
                    .unwrap();
                (
                    key(kind, Some(&world.id), &t.id, 2),
                    "world_type_versions",
                    "type_id",
                    "world_type_versions_update",
                )
            }
            DomainKind::Type => {
                let t = types
                    .create_type(&l.token, fields("Old shared type"), None)
                    .await
                    .unwrap();
                types.save_type(&l.token, &t.id, rename()).await.unwrap();
                types
                    .create_world_type(
                        &l.token,
                        &world.id,
                        1,
                        Some(game_data::types::TypePin {
                            type_id: t.id.clone(),
                            version: 2,
                        }),
                        fields("Exact current source"),
                        None,
                    )
                    .await
                    .unwrap();
                (
                    key(kind, None, &t.id, 2),
                    "type_versions",
                    "type_id",
                    "type_versions_update",
                )
            }
            DomainKind::Species | DomainKind::Form => {
                let species = author
                    .create_species(&l.token, fields("Old species"), None)
                    .await
                    .unwrap();
                if kind == DomainKind::Species {
                    author
                        .save_species(&l.token, &species.id, rename())
                        .await
                        .unwrap();
                }
                author
                    .save_form(
                        &l.token,
                        &species.default_form_id,
                        if kind == DomainKind::Species { 2 } else { 1 },
                        rename(),
                    )
                    .await
                    .unwrap();
                author
                    .create_variant(
                        &l.token,
                        &world.id,
                        1,
                        Some(SourcePin {
                            form_id: species.default_form_id.clone(),
                            version: 2,
                        }),
                        fields("Current exact form"),
                        None,
                    )
                    .await
                    .unwrap();
                if kind == DomainKind::Species {
                    (
                        key(kind, None, &species.id, 2),
                        "species_versions",
                        "species_id",
                        "species_versions_update",
                    )
                } else {
                    (
                        key(kind, None, &species.default_form_id, 2),
                        "form_versions",
                        "form_id",
                        "form_versions_update",
                    )
                }
            }
            DomainKind::World => {
                author
                    .create_variant(
                        &l.token,
                        &world.id,
                        1,
                        None,
                        fields("Membership observation"),
                        None,
                    )
                    .await
                    .unwrap();
                (
                    key(kind, None, &world.id, 2),
                    "world_revisions",
                    "world_id",
                    "world_revisions_update",
                )
            }
            _ => unreachable!(),
        };
        let baseline = l.capture(&world.id).await;
        assert!(baseline.nodes.contains_key(&current), "{kind:?}/{damage}");
        let mut previous = current.clone();
        previous.revision = 1;
        assert!(
            !baseline.nodes.contains_key(&previous),
            "predecessor leaked into public graph"
        );
        sqlx::query(AssertSqlSafe(format!("DROP TRIGGER {trigger}")))
            .execute(&l.pool)
            .await
            .unwrap();
        let mut c = l.pool.acquire().await.unwrap();
        sqlx::query("PRAGMA foreign_keys=OFF")
            .execute(&mut *c)
            .await
            .unwrap();
        sqlx::query("PRAGMA ignore_check_constraints=ON")
            .execute(&mut *c)
            .await
            .unwrap();
        let change = if damage.starts_with("previous") {
            "previous_revision=7"
        } else if damage.ends_with("typed") {
            "description='Corrupt typed predecessor only'"
        } else {
            "audit_id='ffffffffffffffffffffffffffffffff'"
        };
        sqlx::query(AssertSqlSafe(format!(
            "UPDATE {table} SET {change} WHERE {column}=? AND revision=1"
        )))
        .bind(&current.id)
        .execute(&mut *c)
        .await
        .unwrap();
        sqlx::query("PRAGMA foreign_keys=ON")
            .execute(&mut *c)
            .await
            .unwrap();
        sqlx::query("PRAGMA ignore_check_constraints=OFF")
            .execute(&mut *c)
            .await
            .unwrap();
        drop(c);
        let before = l.snapshot().await;
        let captured = l.inspect.capture(AddressKey::Id(world.id)).await.unwrap();
        assert!(
            captured
                .gaps
                .iter()
                .any(|g| g.kind == CaptureGapKind::Integrity
                    && g.subject.as_ref() == Some(&previous)),
            "{kind:?}/{damage}: {:?}",
            captured.gaps
        );
        assert!(!captured.nodes.contains_key(&previous));
        assert!(fingerprint(&captured).is_none());
        assert_eq!(l.snapshot().await, before);
    }
}

#[tokio::test]
async fn legacy_companion_verification_stops_at_the_immediate_predecessor() {
    let l = Lab::new().await;
    let author = Authoring::new(l.accounts.clone());
    let types = Types::new(l.accounts.clone());
    let world = author
        .create_world(&l.token, fields("Bounded predecessor chain"), None)
        .await
        .unwrap();
    let t = types
        .create_type(&l.token, fields("Original shared metadata"), None)
        .await
        .unwrap();
    for revision in 1..=2 {
        types
            .save_type(
                &l.token,
                &t.id,
                Edit {
                    expected_revision: revision,
                    fields: fields(&format!("Metadata revision {}", revision + 1)),
                    reason: None,
                },
            )
            .await
            .unwrap();
    }
    types
        .create_world_type(
            &l.token,
            &world.id,
            1,
            Some(game_data::types::TypePin {
                type_id: t.id.clone(),
                version: 3,
            }),
            fields("Exact current type"),
            None,
        )
        .await
        .unwrap();
    sqlx::query("DROP TRIGGER type_versions_update")
        .execute(&l.pool)
        .await
        .unwrap();
    sqlx::query("UPDATE type_versions SET description='Outside requested and immediate-companion scope' WHERE type_id=? AND revision=1").bind(&t.id).execute(&l.pool).await.unwrap();
    let c = l.capture(&world.id).await;
    assert!(c.nodes.contains_key(&key(DomainKind::Type, None, &t.id, 3)));
    assert!(!c.nodes.contains_key(&key(DomainKind::Type, None, &t.id, 2)));
    assert!(!c.nodes.contains_key(&key(DomainKind::Type, None, &t.id, 1)));
    assert!(fingerprint(&c).is_some());
}

#[tokio::test]
async fn zero_deadline_has_no_usable_anchor_and_leaves_the_connection_usable() {
    let l = Lab::new().await;
    let mut c = l.pool.acquire().await.unwrap();
    assert_eq!(
        persistence::inspection::capture_world(
            &mut c,
            AddressKey::Id(CLASSIC.into()),
            InspectionLimits {
                deadline_ms: 0,
                ..Default::default()
            }
        )
        .await,
        Err(Error::Unavailable)
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT 42")
            .fetch_one(&mut *c)
            .await
            .unwrap(),
        42
    );
    assert_eq!(
        persistence::inspection::capture_world(
            &mut c,
            AddressKey::Id("ffffffffffffffffffffffffffffffff".into()),
            InspectionLimits {
                deadline_ms: 2000,
                ..Default::default()
            }
        )
        .await,
        Err(Error::NotFound)
    );
    tokio::time::sleep(Duration::from_millis(2050)).await;
    assert_eq!(sqlx::query_scalar::<_, i64>("WITH RECURSIVE n(x) AS (VALUES(0) UNION ALL SELECT x+1 FROM n WHERE x<10000) SELECT max(x) FROM n").fetch_one(&mut *c).await.unwrap(), 10000);
    assert!(!c.is_in_transaction());
}

#[tokio::test]
async fn nonzero_inflight_capture_deadlines_interrupt_and_clean_up_before_connection_reuse() {
    let l = Lab::new().await;
    let before = l.snapshot().await;
    for table in ["moves", "worlds"] {
        let mut c = l.pool.acquire().await.unwrap();
        let started = slow_capture_view(&mut c, table, 1_000_000_000).await;
        let start = Instant::now();
        let result = timeout(
            Duration::from_secs(5),
            persistence::inspection::capture_world(
                &mut c,
                AddressKey::Id(CLASSIC.into()),
                InspectionLimits {
                    deadline_ms: 1000,
                    ..Default::default()
                },
            ),
        )
        .await
        .expect("SQLite progress deadline must stop the VM and allow cleanup");
        started
            .await
            .expect("the recursive SQLite step actually started");
        assert!(start.elapsed() < Duration::from_secs(5));
        if table == "worlds" {
            assert_eq!(
                result,
                Err(Error::Unavailable),
                "anchor read is inside the deadline"
            );
        } else {
            let captured = result.unwrap();
            assert!(!captured.gaps.is_empty());
            assert!(
                captured
                    .gaps
                    .iter()
                    .all(|g| g.kind == CaptureGapKind::Budget),
                "{:?}",
                captured.gaps
            );
            assert!(fingerprint(&captured).is_none());
        }
        // Cross the old deadline, then execute enough VM operations to detect
        // any expired handler accidentally left installed on an ordinary exit.
        assert_eq!(sqlx::query_scalar::<_, i64>("WITH RECURSIVE n(x) AS (VALUES(0) UNION ALL SELECT x+1 FROM n WHERE x<10000) SELECT max(x) FROM n").fetch_one(&mut *c).await.unwrap(), 10000);
        assert!(!c.is_in_transaction());
        let tx = c.begin().await.unwrap();
        tx.rollback().await.unwrap();
        sqlx::query(AssertSqlSafe(format!("DROP VIEW temp.{table}")))
            .execute(&mut *c)
            .await
            .unwrap();
        let captured = persistence::inspection::capture_world(
            &mut c,
            AddressKey::Id(CLASSIC.into()),
            InspectionLimits::default(),
        )
        .await
        .unwrap();
        assert!(captured.is_complete(), "{:?}", captured.gaps);
    }
    assert_eq!(l.snapshot().await, before);
}

#[tokio::test]
async fn pool_acquisition_uses_the_same_inspection_deadline() {
    let l = Lab::new().await;
    let mut occupied = vec![];
    for _ in 0..5 {
        occupied.push(l.pool.acquire().await.unwrap());
    }
    let start = Instant::now();
    let result = timeout(
        Duration::from_secs(2),
        l.inspect.capture_with_limits(
            AddressKey::Id(CLASSIC.into()),
            InspectionLimits {
                deadline_ms: 30,
                ..Default::default()
            },
        ),
    )
    .await
    .unwrap();
    assert_eq!(result, Err(Error::Unavailable));
    assert!(start.elapsed() < Duration::from_secs(2));
    drop(occupied);
    assert!(l.capture(CLASSIC).await.is_complete());
}

#[tokio::test]
async fn cancelling_a_borrowed_capture_makes_its_callback_inert_and_queues_rollback() {
    let l = Lab::new().await;
    let before = l.snapshot().await;
    let mut c = l.pool.acquire().await.unwrap();
    // Borrowed cancellation cannot own/discard the caller's connection. Its
    // finite pending step may finish before the queued rollback, but the callback
    // must never poison subsequent queries after the original deadline expires.
    let started = slow_capture_view(&mut c, "moves", 2_000_000).await;
    let mut capture = Box::pin(persistence::inspection::capture_world(
        &mut c,
        AddressKey::Id(CLASSIC.into()),
        InspectionLimits {
            deadline_ms: 10_000,
            ..Default::default()
        },
    ));
    tokio::select! {
        result = &mut capture => panic!("capture finished before in-flight rendezvous: {result:?}"),
        result = started => result.unwrap(),
    }
    drop(capture);
    tokio::time::sleep(Duration::from_millis(10_050)).await;
    let max = timeout(Duration::from_secs(5), sqlx::query_scalar::<_, i64>("WITH RECURSIVE n(x) AS (VALUES(0) UNION ALL SELECT x+1 FROM n WHERE x<10000) SELECT max(x) FROM n").fetch_one(&mut *c)).await.unwrap().unwrap();
    assert_eq!(max, 10000);
    assert!(!c.is_in_transaction());
    let tx = c.begin().await.unwrap();
    tx.rollback().await.unwrap();
    sqlx::query("DROP VIEW temp.moves")
        .execute(&mut *c)
        .await
        .unwrap();
    assert!(
        persistence::inspection::capture_world(
            &mut c,
            AddressKey::Id(CLASSIC.into()),
            InspectionLimits::default()
        )
        .await
        .unwrap()
        .is_complete()
    );
    assert_eq!(l.snapshot().await, before);
}

#[tokio::test]
async fn cancelling_the_service_discards_its_inflight_connection_instead_of_returning_a_handler() {
    let l = Lab::new().await;
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&format!(
            "sqlite://{}/inspection.db",
            l._dir.path().display()
        ))
        .await
        .unwrap();
    let service = WorldInspection::new(Accounts::new(pool.clone(), 3600).unwrap());
    let before = l.snapshot().await;
    let mut c = pool.acquire().await.unwrap();
    sqlx::query("CREATE TEMP TABLE inspection_lease_marker(n INTEGER)")
        .execute(&mut *c)
        .await
        .unwrap();
    let started = slow_capture_view(&mut c, "moves", 1_000_000_000).await;
    drop(c);
    let task = tokio::spawn(async move {
        service
            .capture_with_limits(
                AddressKey::Id(CLASSIC.into()),
                InspectionLimits {
                    deadline_ms: 10_000,
                    ..Default::default()
                },
            )
            .await
    });
    timeout(Duration::from_secs(5), started)
        .await
        .unwrap()
        .unwrap();
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    // Cancellation interrupts at a VM progress boundary; it does not wait for
    // the original ten-second deadline or let the poisoned lease back in.
    let mut next = timeout(Duration::from_secs(5), pool.acquire())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM sqlite_temp_master WHERE name='inspection_lease_marker'"
        )
        .fetch_one(&mut *next)
        .await
        .unwrap(),
        0
    );
    assert_eq!(sqlx::query_scalar::<_, i64>("WITH RECURSIVE n(x) AS (VALUES(0) UNION ALL SELECT x+1 FROM n WHERE x<10000) SELECT max(x) FROM n").fetch_one(&mut *next).await.unwrap(), 10000);
    assert!(!next.is_in_transaction());
    let tx = next.begin().await.unwrap();
    tx.rollback().await.unwrap();
    assert_eq!(l.snapshot().await, before);
    drop(next);
    pool.close().await;
}
