use super::*;
use crate::{
    authoring::ContentFields,
    card_fields::{ChoiceOption, FieldDefinition, FieldKind},
    card_package::{self, CardPackage, CardPackageContent, native_card_pin},
    cards::{CardComponent, CardIdentity, CardNamespace, CardRecord},
    inspection::{InspectionPayload, PublicWorldContent},
    package::{Package, PackageContent, PackageRecord, payload_key},
    relations::*,
    stats::{NumericConstraints, StatDefinition, StatPin},
    types::TypeRecord,
};

fn id(n: usize) -> String {
    format!("{n:032x}")
}
fn metadata() -> ContentFields {
    ContentFields {
        slug: "sample".into(),
        display_name: "Sample".into(),
        description: "Draft".into(),
    }
}
fn empty() -> RecordPackageContent {
    let payload = InspectionPayload::World(PublicWorldContent {
        id: id(1),
        fields: metadata(),
        revision: 1,
        archived_at: None,
    });
    let key = payload_key(&payload);
    RecordPackageContent {
        schema_version: 3,
        cards: CardPackageContent {
            schema_version: 2,
            typed: PackageContent {
                schema_version: 1,
                root_policy: package::ROOT_POLICY.into(),
                world: key.clone(),
                roots: vec![key.clone()],
                records: vec![PackageRecord { key, payload }],
                bindings: vec![],
            },
            card_roots: vec![],
            field_roots: vec![],
            cards: vec![],
            fields: vec![],
        },
        layout_roots: vec![],
        collection_roots: vec![],
        layouts: vec![],
        collections: vec![],
        relations: vec![],
    }
}
fn card(n: usize) -> CardRecord {
    CardRecord {
        identity: CardIdentity {
            namespace: CardNamespace::Standalone,
            world_id: Some(id(1)),
            id: id(n),
        },
        revision: 1,
        fields: metadata(),
        archived_at: None,
        source: None,
        component: CardComponent::default(),
        legacy_unmodeled: false,
    }
}
fn field(n: usize, kind: FieldKind) -> FieldDefinition {
    FieldDefinition {
        id: id(n),
        world_id: Some(id(1)),
        source: None,
        fields: metadata(),
        revision: 1,
        archived_at: None,
        kind,
    }
}
fn column(n: usize, kind: ColumnKind) -> RecordColumn {
    RecordColumn {
        id: id(n),
        label: format!("Column {n}"),
        kind,
    }
}
fn cell(n: usize, value: RecordValue) -> RecordCell {
    RecordCell {
        column_id: id(n),
        value,
    }
}
fn mixed() -> RecordPackageContent {
    let mut c = empty();
    let a = card(2);
    let mut b = card(3);
    b.archived_at = Some(10);
    let native = InspectionPayload::Type(TypeRecord {
        id: id(4),
        fields: metadata(),
        revision: 2,
        archived_at: Some(10),
    });
    let native_key = payload_key(&native);
    let native_pin = native_card_pin(&native_key).unwrap();
    let n = CardRecord {
        identity: native_pin.identity.clone(),
        revision: 2,
        fields: metadata(),
        archived_at: Some(10),
        source: None,
        component: CardComponent::default(),
        legacy_unmodeled: true,
    };
    c.cards.typed.records.push(PackageRecord {
        key: native_key,
        payload: native,
    });
    let shared_stat = StatDefinition {
        id: id(40),
        world_id: None,
        source: None,
        fields: metadata(),
        revision: 3,
        archived_at: Some(10),
        constraints: NumericConstraints::default(),
    };
    let stat = StatDefinition {
        id: id(41),
        world_id: Some(id(1)),
        source: Some(StatPin {
            stat_id: id(40),
            version: 3,
        }),
        fields: metadata(),
        revision: 2,
        archived_at: Some(10),
        constraints: NumericConstraints::default(),
    };
    for s in [shared_stat, stat] {
        let payload = InspectionPayload::Stat(s);
        c.cards.typed.records.push(PackageRecord {
            key: payload_key(&payload),
            payload,
        });
    }
    let mut shared_field = field(10, FieldKind::Number(NumericConstraints::default()));
    shared_field.world_id = None;
    shared_field.archived_at = Some(10);
    let mut number = field(
        11,
        FieldKind::Number(NumericConstraints {
            whole_only: true,
            minimum: Some("0".parse().unwrap()),
            maximum: Some("5".parse().unwrap()),
        }),
    );
    number.source = Some(shared_field.pin());
    let choice = field(
        12,
        FieldKind::Choice {
            options: vec![ChoiceOption {
                id: id(99),
                label: "Option".into(),
            }],
        },
    );
    let reference = field(13, FieldKind::CardReference);
    let boolean = field(14, FieldKind::Boolean);
    let text = field(15, FieldKind::Text { max_length: 20 });
    let mut referenced_field = field(16, FieldKind::Boolean);
    referenced_field.archived_at = Some(10);
    let layout = LayoutRecord {
        id: id(20),
        world_id: Some(id(1)),
        source: None,
        fields: metadata(),
        revision: 1,
        archived_at: None,
        columns: vec![
            column(
                50,
                ColumnKind::Reference {
                    allowed: vec![
                        ReferenceKind::Card,
                        ReferenceKind::Stat,
                        ReferenceKind::Field,
                    ],
                },
            ),
            column(51, ColumnKind::Field(number.clone())),
            column(52, ColumnKind::Field(choice.clone())),
            column(53, ColumnKind::Field(reference.clone())),
            column(54, ColumnKind::Field(boolean.clone())),
            column(55, ColumnKind::Field(text.clone())),
        ],
    };
    let definition = CollectionDefinition {
        id: id(30),
        world_id: Some(id(1)),
        source: None,
        fields: metadata(),
        revision: 1,
        archived_at: None,
        layout: layout.clone(),
    };
    let entries = vec![
        RecordEntry {
            id: id(60),
            cells: vec![
                cell(51, RecordValue::Number("0".parse().unwrap())),
                cell(50, RecordValue::Reference(DefinitionPin::Card(b.pin()))),
                cell(52, RecordValue::Choice(id(99))),
                cell(54, RecordValue::Boolean(false)),
                cell(55, RecordValue::Text(String::new())),
            ],
        },
        RecordEntry {
            id: id(61),
            cells: vec![
                cell(50, RecordValue::Reference(DefinitionPin::Card(b.pin()))),
                cell(51, RecordValue::Missing),
            ],
        },
        RecordEntry {
            id: id(62),
            cells: vec![
                cell(
                    50,
                    RecordValue::Reference(DefinitionPin::Stat {
                        world_id: Some(id(1)),
                        stat_id: id(41),
                        version: 2,
                    }),
                ),
                cell(51, RecordValue::Unknown),
            ],
        },
        RecordEntry {
            id: id(63),
            cells: vec![
                cell(
                    50,
                    RecordValue::Reference(DefinitionPin::Field(referenced_field.pin())),
                ),
                cell(51, RecordValue::NotApplicable),
            ],
        },
        RecordEntry {
            id: id(64),
            cells: vec![cell(
                53,
                RecordValue::Reference(DefinitionPin::Card(native_pin)),
            )],
        },
        RecordEntry {
            id: id(65),
            cells: vec![],
        },
    ];
    c.relations = vec![
        CardRelationRecord {
            card: a.pin(),
            relations: CardRelations {
                collections: Some(vec![RecordCollection {
                    definition: definition.clone(),
                    value: CollectionValue::Entries(entries),
                }]),
            },
            legacy_unmodeled: false,
        },
        CardRelationRecord {
            card: b.pin(),
            relations: CardRelations::default(),
            legacy_unmodeled: true,
        },
        CardRelationRecord {
            card: n.pin(),
            relations: CardRelations {
                collections: Some(vec![]),
            },
            legacy_unmodeled: false,
        },
    ];
    c.cards.card_roots = vec![a.pin()];
    c.cards.cards = vec![a, b, n];
    c.cards.fields = vec![
        shared_field,
        number,
        choice,
        reference,
        boolean,
        text,
        referenced_field,
    ];
    c.layout_roots.push(layout.pin());
    c.collection_roots.push(definition.pin());
    c.layouts.push(layout);
    c.collections.push(definition);
    c
}
fn rows(c: &mut RecordPackageContent) -> &mut Vec<RecordEntry> {
    match &mut c
        .relations
        .iter_mut()
        .find(|r| r.card.identity.id == id(2))
        .unwrap()
        .relations
        .collections
        .as_mut()
        .unwrap()[0]
        .value
    {
        CollectionValue::Entries(entries) => entries,
        _ => panic!("expected entries"),
    }
}
// Attackers may update the digest and counts. Acceptance still needs the proof.
fn rejects(p: &RecordPackage, change: impl FnOnce(&mut RecordPackageContent)) {
    let mut p = p.clone();
    change(&mut p.content);
    p.manifest.root_count = root_count(&p.content);
    p.manifest.record_count = record_count(&p.content);
    p.manifest.content_sha256 = checksum(&p.content).unwrap();
    assert!(RecordPackage::from_bytes(&bounded_json(&p).unwrap()).is_err());
    assert!(p.to_bytes().is_err());
    assert!(p.diagnostics().is_empty());
}

#[test]
fn mixed_exact_closure_roundtrip_and_old_formats_stay_independent() {
    let p = RecordPackage::build(mixed()).unwrap();
    assert_eq!((p.manifest.root_count, p.manifest.record_count), (4, 19));
    let bytes = p.to_bytes().unwrap();
    assert_eq!(RecordPackage::from_bytes(&bytes).unwrap(), p);
    assert_eq!(
        RecordPackage::from_bytes(&serde_json::to_vec_pretty(&p).unwrap()).unwrap(),
        p
    );
    assert_eq!(card_package::schema_version_from_bytes(&bytes).unwrap(), 3);
    assert!(CardPackage::build(p.content.cards.clone()).is_err());
    assert!(Package::build(p.content.cards.typed.clone()).is_err());
    assert!(CardPackage::from_bytes(&bytes).is_err());
    assert!(Package::from_bytes(&bytes).is_err());
    let v1 = Package::build(empty().cards.typed)
        .unwrap()
        .to_bytes()
        .unwrap();
    let v2 = CardPackage::build(empty().cards)
        .unwrap()
        .to_bytes()
        .unwrap();
    assert!(RecordPackage::from_bytes(&v1).is_err());
    assert!(RecordPackage::from_bytes(&v2).is_err());
    assert!(Package::from_bytes(&v1).is_ok());
    assert!(CardPackage::from_bytes(&v2).is_ok());
    // Sorting outer vectors must leave the authored array order untouched.
    let mut shuffled = p.content.clone();
    shuffled.cards.cards.reverse();
    shuffled.cards.fields.reverse();
    shuffled.cards.typed.records.reverse();
    shuffled.relations.reverse();
    assert_eq!(
        RecordPackage::build(shuffled).unwrap().to_bytes().unwrap(),
        bytes
    );
    assert!(
        p.content
            .relations
            .iter()
            .find(|r| r.card.identity.id == id(3))
            .unwrap()
            .legacy_unmodeled
    );
}

#[test]
fn missing_extras_and_self_justifying_cycles_fail_with_fresh_checksums() {
    let p = RecordPackage::build(mixed()).unwrap();
    rejects(&p, |c| {
        c.relations.pop();
    });
    rejects(&p, |c| {
        c.relations.push(c.relations[0].clone());
    });
    rejects(&p, |c| {
        c.cards.fields.pop();
    });
    rejects(&p, |c| {
        c.cards.typed.records.retain(|r| r.key.id != id(40));
    });
    rejects(&p, |c| {
        c.layouts.clear();
    });
    rejects(&p, |c| {
        c.collections.clear();
    });
    rejects(&p, |c| {
        c.cards.cards.retain(|r| r.identity.id != id(4));
    });
    rejects(&p, |c| {
        let mut extra = card(90);
        extra.archived_at = Some(1);
        let d = c.collections[0].clone();
        c.relations.push(CardRelationRecord {
            card: extra.pin(),
            legacy_unmodeled: false,
            relations: CardRelations {
                collections: Some(vec![RecordCollection {
                    definition: d,
                    value: CollectionValue::Entries(vec![RecordEntry {
                        id: id(91),
                        cells: vec![cell(
                            50,
                            RecordValue::Reference(DefinitionPin::Card(extra.pin())),
                        )],
                    }]),
                }]),
            },
        });
        c.cards.cards.push(extra);
        c.cards.cards.sort_by_key(CardRecord::pin);
        c.relations.sort_by_key(CardRelationRecord::pin);
    });
    rejects(&p, |c| {
        c.layout_roots.clear();
        c.collection_roots.clear();
        c.cards.card_roots.clear();
    });
    rejects(&p, |c| {
        c.cards.typed.records.push(PackageRecord {
            key: payload_key(&InspectionPayload::Stat(StatDefinition {
                id: id(100),
                world_id: Some(id(1)),
                source: None,
                fields: metadata(),
                revision: 1,
                archived_at: None,
                constraints: NumericConstraints::default(),
            })),
            payload: InspectionPayload::Stat(StatDefinition {
                id: id(100),
                world_id: Some(id(1)),
                source: None,
                fields: metadata(),
                revision: 1,
                archived_at: None,
                constraints: NumericConstraints::default(),
            }),
        });
        c.cards.typed.records.sort_by(|a, b| a.key.cmp(&b.key));
    });
}

#[test]
fn embedded_equality_native_metadata_and_scope_are_mandatory() {
    let p = RecordPackage::build(mixed()).unwrap();
    rejects(&p, |c| {
        c.collections[0].layout.fields.display_name = "Different".into()
    });
    rejects(&p, |c| {
        c.relations
            .iter_mut()
            .find(|r| r.card.identity.id == id(2))
            .unwrap()
            .relations
            .collections
            .as_mut()
            .unwrap()[0]
            .definition
            .fields
            .description = "Different".into()
    });
    rejects(&p, |c| {
        c.cards
            .fields
            .iter_mut()
            .find(|f| f.id == id(11))
            .unwrap()
            .fields
            .description = "Different".into()
    });
    rejects(&p, |c| {
        c.cards
            .cards
            .iter_mut()
            .find(|r| r.identity.id == id(4))
            .unwrap()
            .fields
            .description = "Different".into()
    });
    rejects(&p, |c| {
        c.cards
            .cards
            .iter_mut()
            .find(|r| r.identity.id == id(4))
            .unwrap()
            .source = Some(c.cards.cards[0].pin())
    });
    rejects(&p, |c| {
        rows(c)[0].cells[1].value = RecordValue::Reference(DefinitionPin::Stat {
            world_id: Some(id(999)),
            stat_id: id(41),
            version: 2,
        })
    });
    rejects(&p, |c| {
        c.layouts[0].source = Some(LayoutPin {
            world_id: Some(id(1)),
            layout_id: id(20),
            version: 1,
        })
    });
    rejects(&p, |c| {
        c.collections[0].source = Some(c.collections[0].pin())
    });
    rejects(&p, |c| {
        c.cards
            .cards
            .iter_mut()
            .find(|r| r.identity.id == id(2))
            .unwrap()
            .legacy_unmodeled = true
    }); // scalar v2 Standalone rule stays frozen
}

#[test]
fn rows_validate_allowlists_scalar_constraints_stable_ids_and_legacy_shape() {
    let p = RecordPackage::build(mixed()).unwrap();
    for value in [
        RecordValue::Number("6".parse().unwrap()),
        RecordValue::Number("0.5".parse().unwrap()),
        RecordValue::Boolean(false),
    ] {
        rejects(&p, |c| rows(c)[0].cells[0].value = value);
    }
    rejects(&p, |c| {
        rows(c)[0].cells[2].value = RecordValue::Choice(id(100))
    });
    rejects(&p, |c| {
        rows(c)[4].cells[0].value =
            RecordValue::Reference(DefinitionPin::Field(c.cards.fields[0].pin()))
    });
    rejects(&p, |c| {
        let duplicate = rows(c)[0].clone();
        rows(c).push(duplicate);
    });
    rejects(&p, |c| {
        let duplicate = rows(c)[0].cells[0].clone();
        rows(c)[0].cells.push(duplicate);
    });
    rejects(&p, |c| rows(c)[0].cells[0].column_id = id(999));
    rejects(&p, |c| c.relations[0].legacy_unmodeled = true);
    // Change every embedded copy so this rejection is the domain allowlist check.
    rejects(&p, |c| {
        for l in &mut c.layouts {
            l.columns[0].kind = ColumnKind::Reference {
                allowed: vec![ReferenceKind::Stat],
            };
        }
        c.collections[0].layout = c.layouts[0].clone();
        c.relations
            .iter_mut()
            .find(|r| r.card.identity.id == id(2))
            .unwrap()
            .relations
            .collections
            .as_mut()
            .unwrap()[0]
            .definition = c.collections[0].clone();
    });
    for value in [
        CollectionValue::Missing,
        CollectionValue::Unknown,
        CollectionValue::NotApplicable,
        CollectionValue::Entries(vec![]),
    ] {
        // Retain exact dependencies as active roots for this state-only experiment.
        let mut c = mixed();
        c.cards.cards.retain(|r| r.identity.id == id(2));
        c.relations.truncate(1);
        c.cards.typed.records.truncate(1);
        c.cards.fields.retain(|f| f.id != id(16));
        c.relations[0].relations.collections.as_mut().unwrap()[0].value = value.clone();
        let roundtrip = RecordPackage::build(c).unwrap();
        assert_eq!(
            roundtrip.content.relations[0]
                .relations
                .collections
                .as_ref()
                .unwrap()[0]
                .value,
            value
        );
    }
}

#[test]
fn unused_active_roots_and_transitive_archived_sources_are_valid() {
    let mut c = empty();
    let shared = LayoutRecord {
        id: id(20),
        world_id: None,
        source: None,
        fields: metadata(),
        revision: 2,
        archived_at: Some(1),
        columns: vec![],
    };
    let local = LayoutRecord {
        id: id(21),
        world_id: Some(id(1)),
        source: Some(shared.pin()),
        fields: metadata(),
        revision: 1,
        archived_at: None,
        columns: vec![],
    };
    let shared_collection = CollectionDefinition {
        id: id(30),
        world_id: None,
        source: None,
        fields: metadata(),
        revision: 2,
        archived_at: Some(1),
        layout: shared.clone(),
    };
    let local_collection = CollectionDefinition {
        id: id(31),
        world_id: Some(id(1)),
        source: Some(shared_collection.pin()),
        fields: metadata(),
        revision: 1,
        archived_at: None,
        layout: local.clone(),
    };
    c.layout_roots.push(local.pin());
    c.collection_roots.push(local_collection.pin());
    c.layouts = vec![local, shared];
    c.collections = vec![local_collection, shared_collection];
    let p = RecordPackage::build(c).unwrap();
    assert_eq!(p.manifest.record_count, 5);
    rejects(&p, |c| {
        c.layouts.retain(|l| l.world_id.is_some());
    });
    rejects(&p, |c| {
        c.collections.retain(|d| d.world_id.is_some());
    });
    rejects(&p, |c| c.layout_roots.push(c.layouts[0].pin()));
}

#[test]
fn strict_wire_markers_order_tokens_and_numeric_canonicality() {
    let p = RecordPackage::build(mixed()).unwrap();
    let text = String::from_utf8(p.to_bytes().unwrap()).unwrap();
    for (a, b) in [
        ("\"schema_version\":3", "\"schema_version\":2"),
        ("\"schema_version\":2", "\"schema_version\":3"),
        ("\"schema_version\":1", "\"schema_version\":3"),
        (
            "\"schema_version\":3",
            "\"schema_version\":3,\"schema_version\":3",
        ),
        (
            "\"state\":\"number\",\"value\":\"0\"",
            "\"state\":\"number\",\"value\":\"0.0\"",
        ),
        ("\"producer\":", "\"extra\":null,\"producer\":"),
    ] {
        assert!(text.contains(a));
        assert!(RecordPackage::from_bytes(text.replacen(a, b, 1).as_bytes()).is_err());
    }
    rejects(&p, |c| c.cards.fields.reverse());
    rejects(&p, |c| c.relations.reverse());
    rejects(&p, |c| c.cards.typed.records.reverse());
    let mut v = serde_json::to_value(&p).unwrap();
    v["manifest"]["schema_version"] = 4.into();
    assert!(RecordPackage::from_bytes(&serde_json::to_vec(&v).unwrap()).is_err());
}

#[test]
fn combined_budgets_precede_graph_work_and_wire_is_bounded() {
    let mut c = empty();
    c.relations = vec![
        CardRelationRecord {
            card: card(2).pin(),
            relations: CardRelations::default(),
            legacy_unmodeled: false
        };
        MAX_RECORDS
    ];
    assert!(RecordPackage::build(c).unwrap_err().contains("8192"));
    let mut c = empty();
    c.layout_roots = vec![
        LayoutPin {
            world_id: Some(id(1)),
            layout_id: id(2),
            version: 1
        };
        MAX_ROOTS
    ];
    assert!(RecordPackage::build(c).unwrap_err().contains("2048"));
    let mut c = mixed();
    let row = rows(&mut c)[0].clone();
    rows(&mut c).extend(std::iter::repeat_n(row, MAX_OCCURRENCES));
    assert!(RecordPackage::build(c).unwrap_err().contains("65536"));
    let mut c = empty();
    c.cards.typed.records[0].payload = InspectionPayload::World(PublicWorldContent {
        id: id(1),
        fields: ContentFields {
            description: "x".repeat(MAX_PACKAGE_BYTES),
            ..metadata()
        },
        revision: 1,
        archived_at: None,
    });
    assert!(RecordPackage::build(c).unwrap_err().contains("32 MiB"));
    assert!(
        RecordPackage::from_bytes(&vec![b' '; MAX_PACKAGE_BYTES + 1])
            .unwrap_err()
            .contains("32 MiB")
    );
}
