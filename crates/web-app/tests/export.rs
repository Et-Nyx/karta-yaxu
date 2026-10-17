//! Frozen-package application regressions; every database is temporary and test-owned.
use game_data::{
    authoring::ContentFields,
    inspection::{DomainKind, DomainNodeKey, InspectionLimits, InspectionPayload},
    inspection_analysis::FindingCategory,
    moves::NumericValue,
    package::Package,
};
use sqlx::{AssertSqlSafe, SqlitePool};
use std::collections::{BTreeMap, BTreeSet};
use web_app::{
    accounts::Accounts,
    application::{
        authoring::{AddressKey, Authoring, Edit, Error, SourcePin},
        export::{Confirmation, ExportResult, WorldExport},
        fixtures::{Fixtures, SeedOptions},
        learning::{Learning, LearnsetAdoptEdit, LearnsetEdit},
        moves::{AttributeValueInput, MoveEdit, Moves, NewMove},
    },
    auth::{Role, Subject},
    persistence,
};

const CLASSIC: &str = "da05ae68560e4534ae229ce6139494ae";
const VARIANT: &str = "5d2d9dba132840f3acbaa5ed1e2b0b49";
const SAMPLE_FORM: &str = "31e00000000040008000000000000001";
const SAMPLE_VARIANT: &str = "31f00000000040008000000000000001";
const PROBE: &str = "42b00000000040008000000000000001";

fn address(id: &str) -> AddressKey {
    AddressKey::Id(id.into())
}
fn key(kind: DomainKind, world: Option<&str>, id: &str, revision: i64) -> DomainNodeKey {
    DomainNodeKey {
        kind,
        world_id: world.map(str::to_owned),
        id: id.into(),
        revision,
    }
}
fn fields(name: &str) -> ContentFields {
    ContentFields {
        slug: String::new(),
        display_name: name.into(),
        description: "Synthetic frozen-package regression.".into(),
    }
}
struct Lab {
    accounts: Accounts,
    export: WorldExport,
    pool: SqlitePool,
    owner: String,
    token: String,
    _dir: tempfile::TempDir,
}
impl Lab {
    async fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let pool = persistence::connect_for_migration(&format!(
            "sqlite://{}/export.db",
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
        let owner = accounts
            .sign_in(Subject {
                provider: "placeholder".into(),
                provider_subject: "admin".into(),
                display_name: "admin".into(),
            })
            .await
            .unwrap()
            .user_id;
        let token = accounts.start_session(&owner).await.unwrap().0;
        Self {
            export: WorldExport::new(accounts.clone()),
            accounts,
            pool,
            owner,
            token,
            _dir: dir,
        }
    }
    async fn user(&self, name: &str, role: Role) -> (String, String) {
        self.accounts
            .invite("test", name, role, None)
            .await
            .unwrap();
        let actor = self
            .accounts
            .sign_in(Subject {
                provider: "test".into(),
                provider_subject: name.into(),
                display_name: format!("private-export-{name}"),
            })
            .await
            .unwrap();
        let token = self.accounts.start_session(&actor.user_id).await.unwrap().0;
        (actor.user_id, token)
    }
    async fn preview(&self, world: &str) -> ExportResult {
        self.export
            .preview(&self.token, address(world))
            .await
            .unwrap()
    }
    // Compare actual values in every table, including sessions, grants, audits
    // and immutable snapshots; counts alone would miss in-place writes.
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
    async fn denied(&self, token: &str, world: &str, unauthorized: bool) {
        let before = self.snapshot().await;
        let entry = self.export.entry(token, address(world)).await.unwrap_err();
        let preview = self
            .export
            .preview(token, address(world))
            .await
            .unwrap_err();
        let confirm = self
            .export
            .confirm(token, address(world), &"0".repeat(64))
            .await
            .unwrap_err();
        for error in [entry, preview, confirm] {
            assert!(
                if unauthorized {
                    matches!(error, Error::Unauthorized)
                } else {
                    matches!(error, Error::Forbidden)
                },
                "unexpected permission result: {error:?}"
            );
        }
        assert_eq!(self.snapshot().await, before);
    }
}

// Independent root oracle from committed fixture manifests, not capture output
// or the package's own reference walker.
fn fixture_roots(world: &str) -> BTreeSet<DomainNodeKey> {
    let worlds: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/worlds.json")).unwrap();
    let mut roots = BTreeSet::from([key(DomainKind::World, None, world, 2)]);
    let w = worlds["worlds"]
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
    for (text, collections) in [
        (
            include_str!("../../../fixtures/stats.json"),
            vec![
                ("definitions", DomainKind::Stat),
                ("rosters", DomainKind::StatRoster),
                ("variants", DomainKind::Variant),
            ],
        ),
        (
            include_str!("../../../fixtures/moves.json"),
            vec![
                ("attributes", DomainKind::MoveAttribute),
                ("moves", DomainKind::Move),
                ("rulesets", DomainKind::Ruleset),
                ("learnsets", DomainKind::Learnset),
                ("presets", DomainKind::Preset),
            ],
        ),
        (
            include_str!("../../../fixtures/charts.json"),
            vec![("charts", DomainKind::Chart)],
        ),
    ] {
        let manifest: serde_json::Value = serde_json::from_str(text).unwrap();
        for (collection, kind) in collections {
            for row in manifest[collection]
                .as_array()
                .unwrap()
                .iter()
                .filter(|r| r["world_id"] == world)
            {
                roots.insert(key(kind, Some(world), row["id"].as_str().unwrap(), 1));
            }
        }
    }
    roots
}

fn fixture_records(world: &str, emberling_default: &str) -> BTreeSet<DomainNodeKey> {
    let mut records = fixture_roots(world);
    for (kind, ids) in [
        (
            DomainKind::Species,
            vec![
                "6a64df7c88604b3f9381536710449a8b",
                "77756539fdd648fc9aa5cbf69d6e04c5",
            ],
        ),
        (
            DomainKind::Form,
            vec![
                emberling_default,
                SAMPLE_FORM,
                "45c1900ed0a04324adb4f7a2ffd50bd8",
            ],
        ),
        (
            DomainKind::Type,
            vec![
                "6b4c13114c374eb6891469592b1743de",
                "2effa2714dbb46d3a00c9061bd6f659d",
                "0b89bdbbf95b42f0b7e31c8b75667650",
            ],
        ),
        (
            DomainKind::Stat,
            vec![
                "31a00000000040008000000000000001",
                "31a00000000040008000000000000002",
                "31a00000000040008000000000000003",
            ],
        ),
        (
            DomainKind::MoveAttribute,
            vec![
                "41a00000000040008000000000000001",
                "41a00000000040008000000000000002",
            ],
        ),
        (
            DomainKind::Move,
            vec![
                "42a00000000040008000000000000001",
                "42a00000000040008000000000000002",
            ],
        ),
        (DomainKind::Chart, vec!["90f11300e76a4eafba390307cf61c5fb"]),
        (
            DomainKind::Learnset,
            vec!["44a00000000040008000000000000001"],
        ),
    ] {
        records.extend(ids.into_iter().map(|id| key(kind, None, id, 1)));
    }
    if world == CLASSIC {
        records.insert(key(
            DomainKind::Form,
            None,
            "d4dfc82fd1bb40dbb8ee52947696f24f",
            1,
        ));
    }
    records
}

#[tokio::test]
async fn both_full_labs_roundtrip_exact_roots_bindings_drafts_and_numeric_states_without_writes() {
    let l = Lab::new().await;
    let before = l.snapshot().await;
    let learning = Learning::new(l.accounts.clone());
    let moves = Moves::new(l.accounts.clone());
    // Creature bootstrap allocates this immutable default Form ID. Obtain it
    // independently of package capture, then pin it in the expected closure.
    let emberling_default = Authoring::new(l.accounts.clone())
        .species_detail("6a64df7c88604b3f9381536710449a8b")
        .await
        .unwrap()
        .default_form_id;
    for (world, variant, probe, expected_roots) in [
        (CLASSIC, SAMPLE_VARIANT, PROBE, 20),
        (
            VARIANT,
            "31f00000000040008000000000000002",
            "42c00000000040008000000000000001",
            23,
        ),
    ] {
        let result = l.preview(world).await;
        let offline = Package::from_bytes(&result.bytes)
            .expect("real fixture capture must pass the offline reader");
        assert_eq!(offline, result.package);
        assert_eq!(offline.to_bytes().unwrap(), result.bytes);
        assert_eq!(offline.content.roots.len(), expected_roots);
        assert_eq!(
            offline
                .content
                .roots
                .iter()
                .cloned()
                .collect::<BTreeSet<_>>(),
            fixture_roots(world)
        );
        assert_eq!(
            offline
                .content
                .records
                .iter()
                .map(|r| r.key.clone())
                .collect::<BTreeSet<_>>(),
            fixture_records(world, &emberling_default),
            "only the exact transitive dependencies and capture-time owner/default bindings belong in the package"
        );
        assert_eq!(
            offline
                .content
                .bindings
                .iter()
                .map(|b| b.target.clone())
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([
                key(DomainKind::Form, None, &emberling_default, 1),
                key(
                    DomainKind::Form,
                    None,
                    "45c1900ed0a04324adb4f7a2ffd50bd8",
                    1
                ),
                key(DomainKind::Form, None, SAMPLE_FORM, 1),
                key(DomainKind::Variant, Some(world), variant, 1),
            ])
        );
        assert!(
            offline.content.records.iter().all(|r| r
                .key
                .world_id
                .as_deref()
                .is_none_or(|w| w == world))
        );
        assert!(
            offline
                .content
                .records
                .windows(2)
                .all(|w| w[0].key < w[1].key)
        );
        assert!(
            offline
                .content
                .bindings
                .windows(2)
                .all(|w| w[0].identity < w[1].identity)
        );
        assert!(!offline.content.bindings.is_empty());
        for binding in &offline.content.bindings {
            assert_eq!(binding.identity, binding.target.identity());
            assert!(
                offline
                    .content
                    .records
                    .iter()
                    .any(|r| r.key == binding.target)
            );
        }
        for record in &offline.content.records {
            let owner = match &record.payload {
                InspectionPayload::Species(r) => {
                    Some(key(DomainKind::Form, None, &r.default_form_id, 1))
                }
                InspectionPayload::Learnset(r) => Some(if let Some(form) = &r.form_id {
                    key(DomainKind::Form, None, form, 1)
                } else {
                    key(
                        DomainKind::Variant,
                        Some(world),
                        r.variant_id.as_deref().unwrap(),
                        1,
                    )
                }),
                InspectionPayload::Preset(r) => {
                    Some(key(DomainKind::Variant, Some(world), &r.variant_id, 1))
                }
                _ => None,
            };
            if let Some(owner) = owner {
                assert!(
                    offline
                        .content
                        .bindings
                        .iter()
                        .any(|b| b.identity == owner.identity() && b.target == owner)
                );
            }
        }
        let saved = learning
            .learnset(Some(world), variant)
            .await
            .unwrap()
            .unwrap();
        assert!(offline.content.records.iter().any(
            |r| matches!(&r.payload, InspectionPayload::Learnset(record) if record == &saved)
        ));
        let acquisitions = saved.records.as_ref().unwrap();
        assert_eq!(acquisitions.len(), 3);
        assert_ne!(acquisitions[0].id, acquisitions[1].id);
        assert_eq!(acquisitions[0].move_record, acquisitions[1].move_record);
        assert_eq!(acquisitions[0].threshold, NumericValue::Unknown);
        assert_eq!(
            acquisitions[1].threshold,
            NumericValue::Known("0".parse().unwrap())
        );
        assert_eq!(acquisitions[2].threshold, NumericValue::NotApplicable);
        let exact_move = moves.detail(Some(world), probe).await.unwrap();
        assert!(offline.content.records.iter().any(
            |r| matches!(&r.payload, InspectionPayload::Move(record) if record == &exact_move)
        ));
        let attributes = exact_move.attributes.as_ref().unwrap();
        assert_eq!(
            attributes[0].value,
            NumericValue::Known("0".parse().unwrap())
        );
        assert_eq!(attributes[1].value, NumericValue::Unknown);
        for preset in learning.presets(world, variant).await.unwrap() {
            assert_eq!(
                preset.validator_version, 1,
                "export must not revalidate saved policies"
            );
            assert!(offline.content.records.iter().any(
                |r| matches!(&r.payload, InspectionPayload::Preset(record) if record == &preset)
            ));
            if world == VARIANT && preset.entries.len() == 3 {
                assert_eq!(
                    preset.entries[0], preset.entries[2],
                    "repeated selected acquisitions survive"
                );
            }
        }
        assert!(
            result
                .findings
                .iter()
                .any(|f| f.category == FindingCategory::Contradiction)
        );
        assert!(
            result
                .findings
                .iter()
                .any(|f| f.category == FindingCategory::Incomplete)
        );
        assert!(result.findings.iter().any(|f| f.policy == Some(1)));
        let repeated = l.preview(world).await;
        assert_eq!(
            result.bytes, repeated.bytes,
            "unchanged input has deterministic bytes"
        );
        drop(repeated);
        let Confirmation::Download(download) = l
            .export
            .confirm(
                &l.token,
                address(world),
                &result.package.manifest.content_sha256,
            )
            .await
            .unwrap()
        else {
            panic!("unchanged reviewed content should download")
        };
        assert_eq!(download.bytes, result.bytes);
        let text = std::str::from_utf8(&download.bytes).unwrap();
        for private in [
            &l.owner,
            &l.token,
            "owner_user_id",
            "access_revision",
            "collaborator_ids",
            "provider_subject",
            "token_hash",
            "before_json",
            "after_json",
        ] {
            assert!(
                !text.contains(private),
                "private field/value leaked: {private}"
            );
        }
    }
    assert_eq!(l.snapshot().await, before);
}

#[tokio::test]
async fn permissions_are_enforced_by_every_use_case_and_private_access_changes_do_not_change_bytes()
{
    let l = Lab::new().await;
    let (viewer_id, viewer) = l.user("viewer", Role::Viewer).await;
    let (_, admin) = l.user("nonmember-admin", Role::Administrator).await;
    let (editor_id, editor) = l.user("collaborator", Role::Editor).await;
    l.denied("", CLASSIC, true).await;
    l.denied(&viewer, CLASSIC, false).await;
    l.denied(&admin, CLASSIC, false).await;
    l.denied(&editor, CLASSIC, false).await;
    let bytes = l.preview(CLASSIC).await.bytes.clone();
    let author = Authoring::new(l.accounts.clone());
    author
        .grant_collaborator(
            &l.token,
            CLASSIC,
            1,
            &editor_id,
            Some("private grant reason"),
        )
        .await
        .unwrap();
    let before = l.snapshot().await;
    assert!(l.export.entry(&editor, address(CLASSIC)).await.is_ok());
    let shared = l.export.preview(&editor, address(CLASSIC)).await.unwrap();
    assert_eq!(
        shared.bytes, bytes,
        "access history and requesting actor are not package content"
    );
    let digest = shared.package.manifest.content_sha256.clone();
    drop(shared);
    assert!(matches!(
        l.export
            .confirm(&editor, address(CLASSIC), &digest)
            .await
            .unwrap(),
        Confirmation::Download(_)
    ));
    assert_eq!(l.snapshot().await, before);
    author
        .revoke_collaborator(&l.token, CLASSIC, 2, &editor_id, None)
        .await
        .unwrap();
    l.denied(&editor, CLASSIC, false).await;
    assert!(matches!(
        l.export.confirm(&editor, address(CLASSIC), &digest).await,
        Err(Error::Forbidden)
    ));
    l.accounts.set_disabled(&viewer_id, true).await.unwrap();
    l.denied(&viewer, CLASSIC, true).await;
    l.accounts.sign_out(&l.token).await.unwrap();
    l.denied(&l.token, CLASSIC, true).await;

    // An editor owner is eligible too; administrator status is not required.
    let w = author
        .create_world(&editor, fields("Editor-owned package"), None)
        .await
        .unwrap();
    let before = l.snapshot().await;
    assert!(l.export.entry(&editor, address(&w.id)).await.is_ok());
    let result = l.export.preview(&editor, address(&w.id)).await.unwrap();
    assert_eq!(
        result.package.content.roots,
        vec![key(DomainKind::World, None, &w.id, 1)]
    );
    assert_eq!(result.package.content.records.len(), 1);
    assert!(result.package.content.bindings.is_empty());
    drop(result);
    assert_eq!(l.snapshot().await, before);
    l.accounts.set_disabled(&editor_id, true).await.unwrap();
    l.denied(&editor, &w.id, true).await;
}

#[tokio::test]
async fn source_form_does_not_import_a_learnset_but_independent_explicit_adoption_freezes_its_owner()
 {
    let l = Lab::new().await;
    let author = Authoring::new(l.accounts.clone());
    let learning = Learning::new(l.accounts.clone());
    let w = author
        .create_world(&l.token, fields("Independent learning package"), None)
        .await
        .unwrap();
    let v = author
        .create_variant(
            &l.token,
            &w.id,
            1,
            Some(SourcePin {
                form_id: SAMPLE_FORM.into(),
                version: 1,
            }),
            fields("Explicit source only"),
            None,
        )
        .await
        .unwrap();
    let before = l.snapshot().await;
    let first = l.preview(&w.id).await;
    assert!(
        first
            .package
            .content
            .records
            .iter()
            .any(|r| r.key == key(DomainKind::Form, None, SAMPLE_FORM, 1))
    );
    assert!(
        !first
            .package
            .content
            .records
            .iter()
            .any(|r| r.key.kind == DomainKind::Learnset)
    );
    assert_eq!(
        first
            .package
            .content
            .roots
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([
            key(DomainKind::World, None, &w.id, 2),
            key(DomainKind::Variant, Some(&w.id), &v.id, 1)
        ])
    );
    assert_eq!(l.snapshot().await, before);
    drop(first);
    let unrelated = author
        .create_species(&l.token, fields("Independent shared learning owner"), None)
        .await
        .unwrap();
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
    let owner_form = author
        .form_detail(&unrelated.default_form_id)
        .await
        .unwrap();
    author
        .save_form(
            &l.token,
            &owner_form.id,
            owner_form.species_version,
            Edit {
                expected_revision: owner_form.revision,
                fields: fields("Current independent learning owner"),
                reason: None,
            },
        )
        .await
        .unwrap();
    let adopted = learning
        .adopt_learnset(
            &l.token,
            &w.id,
            &v.id,
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
    let before = l.snapshot().await;
    let result = l.preview(&w.id).await;
    let offline = Package::from_bytes(&result.bytes).unwrap();
    for set in [shared, adopted] {
        assert_eq!(
            set.records,
            Some(vec![]),
            "defined empty stays distinct from unmodeled"
        );
        assert!(
            offline.content.records.iter().any(
                |r| matches!(&r.payload, InspectionPayload::Learnset(record) if record == &set)
            )
        );
    }
    let target = key(DomainKind::Form, None, &unrelated.default_form_id, 2);
    assert!(
        offline
            .content
            .bindings
            .iter()
            .any(|b| b.identity == target.identity() && b.target == target)
    );
    assert!(offline.content.records.iter().any(|r| r.key == target));
    assert!(
        offline
            .content
            .records
            .iter()
            .any(|r| r.key == key(DomainKind::Species, None, &unrelated.id, 1))
    );
    assert!(
        !offline
            .content
            .roots
            .iter()
            .any(|r| r.world_id.is_none() && r.kind != DomainKind::World)
    );
    assert_eq!(l.snapshot().await, before);
}

#[tokio::test]
async fn unused_active_moves_are_catalog_roots_with_distinct_unmodeled_and_defined_empty_attributes()
 {
    let l = Lab::new().await;
    let world = Authoring::new(l.accounts.clone())
        .create_world(&l.token, fields("Unused active catalog"), None)
        .await
        .unwrap();
    let moves = Moves::new(l.accounts.clone());
    let mut created = Vec::new();
    for (name, attributes) in [
        ("Unmodeled unused Move", None),
        ("Defined empty unused Move", Some(vec![])),
    ] {
        created.push(
            moves
                .create(
                    &l.token,
                    Some(&world.id),
                    NewMove {
                        fields: fields(name),
                        source: None,
                        attributes,
                        reason: None,
                    },
                )
                .await
                .unwrap(),
        );
    }
    let before = l.snapshot().await;
    let result = l.preview(&world.id).await;
    let offline = Package::from_bytes(&result.bytes).unwrap();
    assert_eq!(offline.content.roots.len(), 3);
    assert_eq!(offline.content.records.len(), 3);
    assert!(offline.content.bindings.is_empty());
    assert_eq!(
        offline.content.world.revision, world.revision,
        "independent definitions do not revise the world"
    );
    assert!(created[0].attributes.is_none());
    assert_eq!(created[1].attributes, Some(vec![]));
    for row in created {
        assert!(offline.content.roots.contains(&key(
            DomainKind::Move,
            Some(&world.id),
            &row.id,
            1
        )));
        assert!(
            offline
                .content
                .records
                .iter()
                .any(|r| matches!(&r.payload, InspectionPayload::Move(record) if record == &row))
        );
    }
    assert_eq!(l.snapshot().await, before);
}

#[tokio::test]
async fn archived_unused_move_is_excluded_before_corrupt_or_oversized_snapshot_reads() {
    let l = Lab::new().await;
    let world = Authoring::new(l.accounts.clone())
        .create_world(&l.token, fields("Archived unused catalog"), None)
        .await
        .unwrap();
    let empty = l.preview(&world.id).await;
    let digest = empty.package.manifest.content_sha256.clone();
    let empty_bytes = empty.bytes.clone();
    drop(empty);
    let moves = Moves::new(l.accounts.clone());
    let unused = moves
        .create(
            &l.token,
            Some(&world.id),
            NewMove {
                fields: fields("Unused archive candidate"),
                source: None,
                attributes: None,
                reason: None,
            },
        )
        .await
        .unwrap();
    let active = l.preview(&world.id).await;
    let move_key = key(DomainKind::Move, Some(&world.id), &unused.id, 1);
    assert!(active.package.content.roots.contains(&move_key));
    assert!(
        active
            .package
            .content
            .records
            .iter()
            .any(|r| r.key == move_key)
    );
    drop(active);

    // Synthetic initialization in this temporary database, following the
    // inspection tests: no archival use case exists yet. Match the immutable
    // typed archive metadata and its JSON before injecting excluded corruption.
    sqlx::query("DROP TRIGGER move_versions_update")
        .execute(&l.pool)
        .await
        .unwrap();
    assert_eq!(sqlx::query("UPDATE move_versions SET archived_at=123,after_json=json_set(after_json,'$.content.archived_at',123) WHERE scope=? AND move_id=? AND revision=1")
        .bind(&world.id).bind(&unused.id).execute(&l.pool).await.unwrap().rows_affected(), 1);
    let mut archived = unused;
    archived.archived_at = Some(123);
    assert_eq!(
        moves.detail(Some(&world.id), &archived.id).await.unwrap(),
        archived
    );
    let before = l.snapshot().await;
    assert_eq!(l.preview(&world.id).await.bytes, empty_bytes);
    assert_eq!(l.snapshot().await, before);

    for size in [64, InspectionLimits::default().max_snapshot_bytes + 1] {
        // Deliberate corruption only after demonstrating a consistently archived
        // row. The first case disagrees with typed metadata; the second exceeds
        // the normal per-snapshot budget. Neither is in the selected closure.
        sqlx::query("UPDATE move_versions SET after_json=json_set(after_json,'$.content.fields.description',?) WHERE scope=? AND move_id=? AND revision=1")
            .bind("x".repeat(size)).bind(&world.id).bind(&archived.id)
            .execute(&l.pool).await.unwrap();
        let before = l.snapshot().await;
        let result = l.preview(&world.id).await;
        assert_eq!(
            result.bytes, empty_bytes,
            "excluded archived data must not affect capture or its digest"
        );
        assert!(!result.package.content.roots.contains(&move_key));
        assert!(
            !result
                .package
                .content
                .records
                .iter()
                .any(|r| r.key.id == archived.id)
        );
        assert_eq!(Package::from_bytes(&result.bytes).unwrap(), result.package);
        drop(result);
        let Confirmation::Download(download) = l
            .export
            .confirm(&l.token, address(&world.id), &digest)
            .await
            .unwrap()
        else {
            panic!("excluded archived corruption must not invalidate review")
        };
        assert_eq!(download.bytes, empty_bytes);
        assert_eq!(l.snapshot().await, before);
    }
}

#[tokio::test]
async fn required_older_move_survives_as_dependency_when_its_current_head_is_archived() {
    let l = Lab::new().await;
    let initial = l.preview(CLASSIC).await;
    let mut expected_roots = initial.package.content.roots.clone();
    let expected_records = initial.package.content.records.clone();
    let expected_bindings = initial.package.content.bindings.clone();
    drop(initial);
    let moves = Moves::new(l.accounts.clone());
    let old = moves.detail(Some(CLASSIC), PROBE).await.unwrap();
    let head = moves
        .save(
            &l.token,
            Some(CLASSIC),
            PROBE,
            MoveEdit {
                expected_revision: old.revision,
                fields: fields("Current archived Probe"),
                attributes: old.attributes.as_ref().map(|a| {
                    a.iter()
                        .map(|a| AttributeValueInput {
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
    assert_eq!(head.revision, 2);
    // Only synthesize archive state on the newly authored current revision.
    // The genuine revision-1 bytes and embedded learnset snapshots stay intact.
    sqlx::query("DROP TRIGGER move_versions_update")
        .execute(&l.pool)
        .await
        .unwrap();
    assert_eq!(sqlx::query("UPDATE move_versions SET archived_at=123,after_json=json_set(after_json,'$.content.archived_at',123) WHERE scope=? AND move_id=? AND revision=2")
        .bind(CLASSIC).bind(PROBE).execute(&l.pool).await.unwrap().rows_affected(), 1);
    let mut archived = head;
    archived.archived_at = Some(123);
    assert_eq!(moves.detail(Some(CLASSIC), PROBE).await.unwrap(), archived);
    assert_eq!(moves.version(Some(CLASSIC), PROBE, 1).await.unwrap(), old);
    let old_key = key(DomainKind::Move, Some(CLASSIC), PROBE, 1);
    expected_roots.retain(|k| k != &old_key);
    let before = l.snapshot().await;
    let result = l.preview(CLASSIC).await;
    let offline = Package::from_bytes(&result.bytes).unwrap();
    assert_eq!(offline.content.roots, expected_roots);
    assert_eq!(
        offline.content.records, expected_records,
        "required old exact content is preserved, but the archived current head is not imported"
    );
    assert_eq!(offline.content.bindings, expected_bindings);
    assert!(offline.content.records.iter().any(|r| r.key == old_key));
    assert!(
        !offline
            .content
            .records
            .iter()
            .any(|r| r.key == key(DomainKind::Move, Some(CLASSIC), PROBE, 2))
    );
    let learnset = Learning::new(l.accounts.clone())
        .learnset(Some(CLASSIC), SAMPLE_VARIANT)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(learnset.records.as_ref().unwrap()[0].move_record, old);
    assert!(
        offline.content.records.iter().any(
            |r| matches!(&r.payload, InspectionPayload::Learnset(record) if record == &learnset)
        )
    );
    assert_eq!(l.snapshot().await, before);
}

#[tokio::test]
async fn archived_variant_excludes_its_learnset_and_presets_without_expanding_world_membership() {
    use web_app::application::{inspection::WorldInspection, learning::MovesetEdit};

    let l = Lab::new().await;
    let author = Authoring::new(l.accounts.clone());
    let learning = Learning::new(l.accounts.clone());
    let variant = author
        .create_variant(
            &l.token,
            CLASSIC,
            2,
            None,
            fields("Archived learning owner"),
            None,
        )
        .await
        .unwrap();
    let set = learning
        .save_learnset(
            &l.token,
            Some(CLASSIC),
            &variant.id,
            LearnsetEdit {
                expected_revision: 0,
                records: Some(vec![]),
                reason: None,
            },
        )
        .await
        .unwrap();
    let ruleset = Moves::new(l.accounts.clone())
        .ruleset(CLASSIC)
        .await
        .unwrap()
        .unwrap();
    let preset = learning
        .create_preset(
            &l.token,
            CLASSIC,
            &variant.id,
            MovesetEdit {
                expected_revision: 0,
                fields: fields("Archived owner's preset"),
                learnset: set.pin(),
                ruleset: ruleset.pin(),
                entries: vec![],
                reason: None,
            },
        )
        .await
        .unwrap();
    let excluded = BTreeSet::from([
        key(DomainKind::Variant, Some(CLASSIC), &variant.id, 1),
        key(DomainKind::Learnset, Some(CLASSIC), &set.id, 1),
        key(DomainKind::Preset, Some(CLASSIC), &preset.id, 1),
    ]);
    let active = l.preview(CLASSIC).await;
    assert!(
        excluded
            .iter()
            .all(|k| active.package.content.roots.contains(k))
    );
    let mut expected = active.package.content.clone();
    expected.roots.retain(|k| !excluded.contains(k));
    expected.records.retain(|r| !excluded.contains(&r.key));
    expected.bindings.retain(|b| {
        b.identity != key(DomainKind::Variant, Some(CLASSIC), &variant.id, 1).identity()
    });
    drop(active);

    // Consistent synthetic initial archive metadata across the head and all
    // v1/v2/v3 companion snapshots, never a production archival mutation.
    let mut tx = l.pool.begin().await.unwrap();
    for sql in [
        "DROP TRIGGER variant_revisions_update",
        "DROP TRIGGER variant_content_update",
        "DROP TRIGGER variant_stat_update",
    ] {
        sqlx::query(sql).execute(&mut *tx).await.unwrap();
    }
    assert_eq!(
        sqlx::query("UPDATE variants SET archived_at=123 WHERE world_id=? AND id=? AND revision=1")
            .bind(CLASSIC)
            .bind(&variant.id)
            .execute(&mut *tx)
            .await
            .unwrap()
            .rows_affected(),
        1
    );
    for sql in [
        "UPDATE variant_revisions SET after_json=json_set(after_json,'$.content.archived_at',123) WHERE world_id=? AND variant_id=? AND revision=1",
        "UPDATE variant_content_snapshots SET after_json=json_set(after_json,'$.content.archived_at',123) WHERE world_id=? AND variant_id=? AND revision=1",
        "UPDATE variant_stat_snapshots SET after_json=json_set(after_json,'$.content.archived_at',123) WHERE world_id=? AND variant_id=? AND revision=1",
    ] {
        assert_eq!(
            sqlx::query(sql)
                .bind(CLASSIC)
                .bind(&variant.id)
                .execute(&mut *tx)
                .await
                .unwrap()
                .rows_affected(),
            1
        );
    }
    tx.commit().await.unwrap();
    let before = l.snapshot().await;
    // Inspection deliberately reads archived inventory. Its checked readers
    // establish that this setup is coherent rather than merely skipped damage.
    let capture = WorldInspection::new(l.accounts.clone())
        .capture(address(CLASSIC))
        .await
        .unwrap();
    assert!(capture.is_complete(), "{:?}", capture.gaps);
    assert_eq!(
        capture.nodes[&key(DomainKind::Variant, Some(CLASSIC), &variant.id, 1)].archived_at(),
        Some(123)
    );
    assert!(excluded.iter().all(|k| capture.nodes.contains_key(k)));
    let result = l.preview(CLASSIC).await;
    let offline = Package::from_bytes(&result.bytes).unwrap();
    assert_eq!(
        offline.content, expected,
        "catalog membership must not resurrect an archived variant or its independent children"
    );
    assert!(
        offline
            .content
            .records
            .iter()
            .any(|r| r.key == key(DomainKind::Move, Some(CLASSIC), PROBE, 1)),
        "the existing active owner's exact Move dependencies remain intact"
    );
    assert_eq!(l.snapshot().await, before);
}

#[tokio::test]
async fn independent_move_edit_and_world_rename_require_new_review_and_keep_exact_old_pins() {
    let l = Lab::new().await;
    let initial = l.preview(CLASSIC).await;
    let digest = initial.package.manifest.content_sha256.clone();
    drop(initial);
    let author = Authoring::new(l.accounts.clone());
    let world_before = author.world_detail(CLASSIC).await.unwrap();
    let moves = Moves::new(l.accounts.clone());
    let old = moves.detail(Some(CLASSIC), PROBE).await.unwrap();
    moves
        .save(
            &l.token,
            Some(CLASSIC),
            PROBE,
            MoveEdit {
                expected_revision: old.revision,
                fields: fields("Independently renamed Probe"),
                attributes: old.attributes.as_ref().map(|a| {
                    a.iter()
                        .map(|a| AttributeValueInput {
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
    assert_eq!(
        author.world_detail(CLASSIC).await.unwrap().revision,
        world_before.revision
    );
    let before = l.snapshot().await;
    let Confirmation::Changed(changed) = l
        .export
        .confirm(&l.token, address(CLASSIC), &digest)
        .await
        .unwrap()
    else {
        panic!("independent root changes must invalidate review")
    };
    let old_key = key(DomainKind::Move, Some(CLASSIC), PROBE, 1);
    let new_key = key(DomainKind::Move, Some(CLASSIC), PROBE, 2);
    assert!(
        changed
            .package
            .content
            .records
            .iter()
            .any(|r| r.key == old_key
                && matches!(&r.payload, InspectionPayload::Move(record) if record == &old))
    );
    assert!(
        changed
            .package
            .content
            .records
            .iter()
            .any(|r| r.key == new_key)
    );
    assert!(!changed.package.content.roots.contains(&old_key));
    assert!(changed.package.content.roots.contains(&new_key));
    let digest = changed.package.manifest.content_sha256.clone();
    drop(changed);
    assert_eq!(l.snapshot().await, before);
    author
        .save_world(
            &l.token,
            CLASSIC,
            Edit {
                expected_revision: world_before.revision,
                fields: fields("Renamed frozen world"),
                reason: None,
            },
        )
        .await
        .unwrap();
    let before = l.snapshot().await;
    let Confirmation::Changed(changed) = l
        .export
        .confirm(&l.token, address(CLASSIC), &digest)
        .await
        .unwrap()
    else {
        panic!("world rename must invalidate review")
    };
    let reviewed = changed.package.manifest.content_sha256.clone();
    let bytes = changed.bytes.clone();
    drop(changed);
    let Confirmation::Download(download) = l
        .export
        .confirm(&l.token, address(CLASSIC), &reviewed)
        .await
        .unwrap()
    else {
        panic!("fresh review should download")
    };
    assert_eq!(download.bytes, bytes);
    assert_eq!(l.snapshot().await, before);
}

#[tokio::test]
async fn admission_is_shared_across_clones_and_held_by_preview_and_confirmation_results() {
    let l = Lab::new().await;
    let before = l.snapshot().await;
    let clone = l.export.clone();
    let first = l.preview(CLASSIC).await;
    let digest = first.package.manifest.content_sha256.clone();
    let second = clone.preview(&l.token, address(VARIANT)).await.unwrap();
    assert!(matches!(
        l.export.preview(&l.token, address(CLASSIC)).await,
        Err(Error::Unavailable)
    ));
    assert!(
        clone.entry(&l.token, address(CLASSIC)).await.is_ok(),
        "entry is not an export job"
    );
    drop(first);
    let confirmed = clone
        .confirm(&l.token, address(CLASSIC), &digest)
        .await
        .unwrap();
    assert!(matches!(&confirmed, Confirmation::Download(_)));
    assert!(matches!(
        l.export.preview(&l.token, address(CLASSIC)).await,
        Err(Error::Unavailable)
    ));
    drop(confirmed);
    drop(second);
    assert!(l.export.preview(&l.token, address(CLASSIC)).await.is_ok());
    assert_eq!(l.snapshot().await, before);
}

#[tokio::test]
async fn invalid_reviews_and_capture_budget_failures_release_admission_without_partial_results_or_writes()
 {
    let l = Lab::new().await;
    let before = l.snapshot().await;
    for digest in [
        String::new(),
        "a".repeat(63),
        "a".repeat(65),
        "G".repeat(64),
        "A".repeat(64),
    ] {
        assert!(matches!(
            l.export.confirm(&l.token, address(CLASSIC), &digest).await,
            Err(Error::Invalid(_))
        ));
    }
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
        InspectionLimits {
            deadline_ms: 0,
            ..Default::default()
        },
    ] {
        assert!(matches!(
            l.export
                .preview_with_limits(&l.token, address(CLASSIC), limits)
                .await,
            Err(Error::Unavailable)
        ));
    }
    let first = l.preview(CLASSIC).await;
    let second = l.preview(VARIANT).await;
    drop((first, second));
    assert_eq!(l.snapshot().await, before);
}

#[tokio::test]
async fn corrupt_required_source_blocks_preview_and_confirmation_without_releasing_prior_bytes() {
    let l = Lab::new().await;
    let digest = l
        .preview(CLASSIC)
        .await
        .package
        .manifest
        .content_sha256
        .clone();
    // Dedicated corruption injection, confined to this temporary database. A
    // required shared Form's head no longer agrees with its immutable snapshot.
    sqlx::query("UPDATE forms SET description='corrupt required projection' WHERE id=?")
        .bind(SAMPLE_FORM)
        .execute(&l.pool)
        .await
        .unwrap();
    let before = l.snapshot().await;
    assert!(matches!(
        l.export.preview(&l.token, address(CLASSIC)).await,
        Err(Error::Unavailable)
    ));
    assert!(matches!(
        l.export.confirm(&l.token, address(CLASSIC), &digest).await,
        Err(Error::Unavailable)
    ));
    assert_eq!(l.snapshot().await, before);
}
