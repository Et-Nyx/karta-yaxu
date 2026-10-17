use super::*;
use crate::{
    authoring::ContentFields,
    charts::{ChartCell, ChartRecord, Factor},
    inspection::{
        PublicFormContent, PublicSpeciesContent, PublicVariantContent, PublicWorldContent,
    },
    inspection_analysis::FindingCategory,
    learning::{LearningRecord, LearnsetRecord, MovesetRecord},
    moves::{MoveAttributeDefinition, MoveAttributeValue, MoveRecord, NumericValue},
    rulesets::{AttributeExpectation, RepeatPolicy, RulesetRecord},
    stats::{BaseStats, NumericConstraints, StatDefinition, StatRoster, StatValue},
    types::{TypeAssignment, TypePin, TypeRecord, TypingState, WorldTypeRecord},
};

fn id(n: usize) -> String {
    format!("{n:032x}")
}
fn fields() -> ContentFields {
    ContentFields {
        slug: "sample".into(),
        display_name: "Sample".into(),
        description: "Design draft".into(),
    }
}
fn add(content: &mut PackageContent, payload: InspectionPayload, root: bool) -> DomainNodeKey {
    let key = payload_key(&payload);
    if root {
        content.roots.push(key.clone());
    }
    content.records.push(PackageRecord {
        key: key.clone(),
        payload,
    });
    key
}
fn bind(content: &mut PackageContent, target: &DomainNodeKey) {
    content.bindings.push(IdentityBinding {
        identity: target.identity(),
        target: target.clone(),
    });
}
fn empty() -> PackageContent {
    let world = InspectionPayload::World(PublicWorldContent {
        id: id(1),
        fields: fields(),
        revision: 1,
        archived_at: None,
    });
    let key = payload_key(&world);
    PackageContent {
        schema_version: SCHEMA_VERSION,
        root_policy: ROOT_POLICY.into(),
        world: key.clone(),
        roots: vec![key.clone()],
        records: vec![PackageRecord {
            key,
            payload: world,
        }],
        bindings: vec![],
    }
}
fn variant() -> PublicVariantContent {
    PublicVariantContent {
        id: id(2),
        world_id: id(1),
        source: None,
        fields: fields(),
        revision: 1,
        archived_at: None,
        typing: TypingState::unmodeled(),
        stats: BaseStats::unmodeled(),
    }
}
fn movement() -> MoveRecord {
    MoveRecord {
        id: id(4),
        world_id: Some(id(1)),
        source: None,
        fields: fields(),
        revision: 1,
        archived_at: Some(123),
        attributes: None,
        mappings: vec![],
    }
}
fn learnset(m: &MoveRecord) -> LearnsetRecord {
    LearnsetRecord {
        id: id(5),
        world_id: Some(id(1)),
        form_id: None,
        variant_id: Some(id(2)),
        source: None,
        revision: 1,
        records: Some(vec![
            LearningRecord {
                id: id(101),
                move_record: m.clone(),
                method: None,
                context: None,
                condition: None,
                threshold: NumericValue::Unknown,
            },
            LearningRecord {
                id: id(102),
                move_record: m.clone(),
                method: Some("Synthetic".into()),
                context: None,
                condition: None,
                threshold: NumericValue::NotApplicable,
            },
        ]),
        mappings: vec![],
    }
}
fn lab(policy: i64) -> PackageContent {
    let mut content = empty();
    let owner = add(&mut content, InspectionPayload::Variant(variant()), true);
    bind(&mut content, &owner);
    let definition = MoveAttributeDefinition {
        id: id(3),
        world_id: Some(id(1)),
        source: None,
        fields: fields(),
        unit: "synthetic".into(),
        revision: 1,
        archived_at: None,
        constraints: NumericConstraints {
            whole_only: true,
            minimum: Some("0".parse().unwrap()),
            maximum: Some("0".parse().unwrap()),
        },
    };
    add(
        &mut content,
        InspectionPayload::MoveAttribute(definition.clone()),
        true,
    );
    let mut m = movement();
    m.attributes = Some(vec![MoveAttributeValue {
        definition: definition.clone(),
        value: NumericValue::Known("0".parse().unwrap()),
    }]);
    add(&mut content, InspectionPayload::Move(m.clone()), false);
    let rules = RulesetRecord {
        id: id(6),
        world_id: id(1),
        revision: 1,
        fields: fields(),
        expectations: Some(vec![AttributeExpectation {
            definition,
            required: true,
            allow_not_applicable: false,
            constraints: NumericConstraints {
                whole_only: true,
                minimum: Some("1".parse().unwrap()),
                maximum: Some("2".parse().unwrap()),
            },
        }]),
        stat_roster: None,
        chart: None,
        max_moves: Some(1),
        repeat_policy: RepeatPolicy::Forbid,
        notes: String::new(),
    };
    let learning = learnset(&m);
    let preset = MovesetRecord {
        id: id(7),
        world_id: id(1),
        variant_id: id(2),
        fields: fields(),
        revision: 1,
        archived_at: None,
        learnset: learning.pin(),
        ruleset: rules.pin(),
        entries: vec![id(101), id(102), id(101)],
        validator_version: policy,
    };
    add(&mut content, InspectionPayload::Ruleset(rules), true);
    add(&mut content, InspectionPayload::Learnset(learning), true);
    add(&mut content, InspectionPayload::Preset(preset), true);
    content
}

/// Corrupt structural data while producing an otherwise correct manifest and
/// recomputed checksum: checksum-only validation must not pass these inputs.
fn forged(mut content: PackageContent) -> Vec<u8> {
    content.roots.sort();
    content.records.sort_by(|a, b| a.key.cmp(&b.key));
    content.bindings.sort_by(|a, b| a.identity.cmp(&b.identity));
    let package = Package {
        manifest: Manifest {
            format: FORMAT.into(),
            schema_version: SCHEMA_VERSION,
            producer: PRODUCER.into(),
            content_sha256: checksum(&content).unwrap(),
            root_count: content.roots.len(),
            record_count: content.records.len(),
        },
        content,
    };
    serde_json::to_vec(&package).unwrap()
}
fn reject(content: PackageContent) {
    assert!(Package::from_bytes(&forged(content)).is_err());
}

#[test]
fn deterministic_roundtrip_retains_archived_pins_drafts_and_occurrences() {
    for policy in [1, 2] {
        let content = lab(policy);
        let mut shuffled = content.clone();
        shuffled.roots.reverse();
        shuffled.records.reverse();
        shuffled.bindings.reverse();
        let package = Package::build(content).unwrap();
        assert_eq!(package, Package::build(shuffled).unwrap());
        let bytes = package.to_bytes().unwrap();
        assert_eq!(package, Package::from_bytes(&bytes).unwrap());
        let findings = package.diagnostics();
        assert!(
            findings
                .iter()
                .any(|f| f.category == FindingCategory::Contradiction)
        );
        assert!(
            findings
                .iter()
                .any(|f| f.policy == Some(policy) && f.code == "validator.preset.saved")
        );
        assert!(!findings.iter().any(|f| f.code == "dependency.older_pin"));
        let learning = package
            .content
            .records
            .iter()
            .find_map(|r| match &r.payload {
                InspectionPayload::Learnset(r) => Some(r),
                _ => None,
            })
            .unwrap();
        assert_eq!(learning.records.as_ref().unwrap().len(), 2);
        let refs = references(&InspectionPayload::Learnset(learning.clone())).unwrap();
        assert_eq!(refs.exact.len(), 2);
        assert_eq!(refs.exact[0], refs.exact[1]);
        assert_eq!(refs.identities.len(), 1);
        let pretty = serde_json::to_vec_pretty(&package).unwrap();
        assert_eq!(Package::from_bytes(&pretty).unwrap(), package);
    }
}

#[test]
fn independently_rooted_unused_content_is_allowed_but_extras_are_not() {
    let mut content = empty();
    let mut m = movement();
    m.archived_at = None;
    add(&mut content, InspectionPayload::Move(m), true);
    assert!(Package::build(content.clone()).is_ok());
    content.roots.retain(|key| key.kind == DomainKind::World);
    reject(content);
}

#[test]
fn recomputed_checksums_do_not_authorize_invalid_roots_bindings_or_closure() {
    let original = lab(2);
    let mut c = original.clone();
    c.bindings.clear();
    reject(c);
    let mut c = original.clone();
    c.bindings[0].target.revision = 9;
    reject(c);
    let mut c = original.clone();
    c.bindings[0].identity.id = id(99);
    reject(c);
    let mut c = original.clone();
    c.bindings.push(c.bindings[0].clone());
    reject(c);
    let mut c = original.clone();
    c.records.push(c.records[0].clone());
    reject(c);
    let mut c = original.clone();
    c.roots.push(c.roots[0].clone());
    reject(c);
    let mut c = original.clone();
    c.records.retain(|r| r.key.kind != DomainKind::Move);
    reject(c);
    let mut c = original.clone();
    c.roots.retain(|r| r.kind != DomainKind::World);
    reject(c);
    let mut c = original.clone();
    let unused = c
        .records
        .iter()
        .find(|r| r.key.kind == DomainKind::Ruleset)
        .unwrap()
        .key
        .clone();
    bind(&mut c, &unused);
    reject(c);
    let mut c = original.clone();
    c.roots.push(
        c.records
            .iter()
            .find(|r| r.key.kind == DomainKind::Move)
            .unwrap()
            .key
            .clone(),
    );
    reject(c); // Archived dependency cannot be promoted to an active root.
    let mut c = original.clone();
    c.roots.retain(|r| r.kind != DomainKind::Variant);
    reject(c); // Child owner must be a root.
    let mut c = original;
    if let InspectionPayload::World(w) = &mut c.records[0].payload {
        w.archived_at = Some(1);
    }
    reject(c);
}

#[test]
fn recomputed_checksums_do_not_hide_embedded_or_selection_corruption() {
    let mut c = lab(2);
    let r = c
        .records
        .iter_mut()
        .find(|r| r.key.kind == DomainKind::Learnset)
        .unwrap();
    if let InspectionPayload::Learnset(r) = &mut r.payload {
        r.records.as_mut().unwrap()[0]
            .move_record
            .fields
            .description = "Fabricated embedded snapshot".into();
    }
    reject(c);
    let mut c = lab(2);
    let r = c
        .records
        .iter_mut()
        .find(|r| r.key.kind == DomainKind::Preset)
        .unwrap();
    if let InspectionPayload::Preset(r) = &mut r.payload {
        r.entries.push(id(999));
    }
    reject(c);
    let mut c = lab(2);
    let r = c
        .records
        .iter_mut()
        .find(|r| r.key.kind == DomainKind::Move)
        .unwrap();
    if let InspectionPayload::Move(r) = &mut r.payload {
        r.attributes.as_mut().unwrap()[0].value = NumericValue::Known("1".parse().unwrap());
    }
    reject(c);
}

#[test]
fn unsupported_policies_scopes_and_key_disagreement_fail() {
    reject(lab(3));
    let mut c = lab(2);
    c.schema_version = 2;
    reject(c);
    let mut c = lab(2);
    c.root_policy = "all-inspection-roots".into();
    reject(c);
    let mut c = lab(2);
    c.records[1].key.revision += 1;
    reject(c);
    let mut c = lab(2);
    if let InspectionPayload::Variant(v) = &mut c.records[1].payload {
        v.world_id = id(99);
        c.records[1].key = payload_key(&c.records[1].payload);
    }
    reject(c);
}

#[test]
fn strict_json_rejects_alternate_numbers_duplicate_fields_and_missing_nulls() {
    let package = Package::build(lab(2)).unwrap();
    let json = String::from_utf8(package.to_bytes().unwrap()).unwrap();
    for corrupted in [
        json.replacen("\"schema_version\":1", "\"schema_version\":1.0", 1),
        json.replacen("\"schema_version\":1", "\"schema_version\":1e0", 1),
        json.replacen(
            "\"schema_version\":1",
            "\"schema_version\":1,\"schema_version\":1",
            1,
        ),
        json.replacen("\"world_id\":null,", "", 1),
        json.replacen("\"value\":\"0\"", "\"value\":0", 1),
        json.replacen("\"value\":\"0\"", "\"value\":\"0.0\"", 1),
        json.replacen("\"manifest\":{", "\"manifest\":{\"secret\":0,", 1),
    ] {
        assert_ne!(corrupted, json);
        assert!(Package::from_bytes(corrupted.as_bytes()).is_err());
    }
    let mut mutated = package.clone();
    mutated.manifest.content_sha256 = "0".repeat(64);
    assert!(mutated.to_bytes().is_err());
    let mut mutated = package;
    mutated.manifest.record_count += 1;
    assert!(mutated.to_bytes().is_err());
}

fn forms(content: &mut PackageContent) -> DomainNodeKey {
    let species = PublicSpeciesContent {
        id: id(20),
        fields: fields(),
        revision: 1,
        default_form_id: id(21),
        archived_at: None,
    };
    let form = PublicFormContent {
        id: id(21),
        species_id: species.id.clone(),
        species_version: 1,
        fields: fields(),
        revision: 1,
        is_default: true,
        archived_at: Some(123),
        typing: TypingState::unmodeled(),
        stats: BaseStats::unmodeled(),
    };
    add(content, InspectionPayload::Species(species), false);
    let pinned = add(content, InspectionPayload::Form(form.clone()), false);
    let mut current = form;
    current.revision = 2;
    current.fields.description = "Later exact owner observation".into();
    let head = add(content, InspectionPayload::Form(current), false);
    bind(content, &head);
    pinned
}

#[test]
fn identity_binding_cycles_are_self_contained_and_do_not_import_reverse_children() {
    let mut content = empty();
    let pinned = forms(&mut content);
    let mut v = variant();
    v.source = Some(crate::inspection::InspectionFormPin {
        form_id: pinned.id,
        version: 1,
    });
    add(&mut content, InspectionPayload::Variant(v), true);
    let package = Package::build(content.clone()).unwrap();
    assert_eq!(
        Package::from_bytes(&package.to_bytes().unwrap()).unwrap(),
        package
    );
    let mut m = movement();
    m.world_id = None;
    let mut learning = learnset(&m);
    learning.world_id = None;
    learning.form_id = Some(id(21));
    learning.variant_id = None;
    add(&mut content, InspectionPayload::Move(m), false);
    add(&mut content, InspectionPayload::Learnset(learning), false);
    reject(content); // A source Form never implicitly imports its Learnset.
}

#[test]
fn explicit_source_learnset_is_independent_of_variant_source_and_preserves_shared_repetitions() {
    let mut content = lab(2);
    forms(&mut content);
    // This source owner is bound to Form revision 2; revision 1 is not pinned by
    // a Variant in this example and must not be an unexplained extra record.
    content
        .records
        .retain(|r| r.key.kind != DomainKind::Form || r.key.revision != 1);
    let mut shared_move = movement();
    shared_move.world_id = None;
    add(
        &mut content,
        InspectionPayload::Move(shared_move.clone()),
        false,
    );
    let mut source = learnset(&shared_move);
    let mut later_move = shared_move.clone();
    later_move.revision = 2;
    later_move.fields.description = "A different exact revision of the repeated Move".into();
    source.records.as_mut().unwrap()[1].move_record = later_move.clone();
    add(&mut content, InspectionPayload::Move(later_move), false);
    source.id = id(30);
    source.world_id = None;
    source.form_id = Some(id(21));
    source.variant_id = None;
    add(
        &mut content,
        InspectionPayload::Learnset(source.clone()),
        false,
    );
    let local = content
        .records
        .iter_mut()
        .find(|r| r.key.kind == DomainKind::Learnset && r.key.world_id.is_some())
        .unwrap();
    if let InspectionPayload::Learnset(r) = &mut local.payload {
        r.source = Some(source.pin());
        r.mappings = vec![crate::learning::MoveMapping {
            shared_move_id: shared_move.id.clone(),
            local_move: movement().pin(),
        }];
    }
    // An explicit mapped local source may pin revision 1 even though the source
    // Learnset also contains revision 2 of that canonical shared Move identity.
    for record in &mut content.records {
        match &mut record.payload {
            InspectionPayload::Move(m) if m.world_id.is_some() => {
                m.source = Some(shared_move.pin());
            }
            InspectionPayload::Learnset(l) if l.world_id.is_some() => {
                for acquisition in l.records.iter_mut().flatten() {
                    acquisition.move_record.source = Some(shared_move.pin());
                }
            }
            _ => {}
        }
    }
    // No Variant source Form exists, but the explicit shared Learnset is valid.
    Package::build(content.clone()).unwrap();
    let local = content
        .records
        .iter_mut()
        .find(|r| r.key.kind == DomainKind::Learnset && r.key.world_id.is_some())
        .unwrap();
    if let InspectionPayload::Learnset(r) = &mut local.payload {
        r.mappings[0].shared_move_id = id(999);
    }
    reject(content);
}

#[test]
fn mapping_requires_membership_in_exact_source_move_and_explicit_local_provenance() {
    use crate::moves::{AttributeMapping, AttributePin, MovePin};
    let mut c = lab(2);
    let mut shared_attribute = c
        .records
        .iter()
        .find_map(|r| match &r.payload {
            InspectionPayload::MoveAttribute(a) => Some(a.clone()),
            _ => None,
        })
        .unwrap();
    shared_attribute.id = id(70);
    shared_attribute.world_id = None;
    add(
        &mut c,
        InspectionPayload::MoveAttribute(shared_attribute.clone()),
        false,
    );
    let mut source = movement();
    source.id = id(71);
    source.world_id = None;
    source.attributes = Some(vec![MoveAttributeValue {
        definition: shared_attribute.clone(),
        value: NumericValue::Unknown,
    }]);
    add(&mut c, InspectionPayload::Move(source.clone()), false);
    // Update every occurrence of the local definition and Move consistently.
    for record in &mut c.records {
        let source_attribute = Some(AttributePin {
            attribute_id: shared_attribute.id.clone(),
            version: 1,
        });
        match &mut record.payload {
            InspectionPayload::MoveAttribute(a) if a.world_id.is_some() => {
                a.source = source_attribute
            }
            InspectionPayload::Ruleset(r) => {
                r.expectations.as_mut().unwrap()[0].definition.source = source_attribute
            }
            InspectionPayload::Move(m) if m.world_id.is_some() => {
                m.attributes.as_mut().unwrap()[0].definition.source = source_attribute;
            }
            InspectionPayload::Learnset(l) => {
                for a in l.records.iter_mut().flatten() {
                    a.move_record.attributes.as_mut().unwrap()[0]
                        .definition
                        .source = source_attribute.clone();
                }
            }
            _ => {}
        }
    }
    let mapped = c
        .records
        .iter()
        .find_map(|r| match &r.payload {
            InspectionPayload::Move(m) if m.world_id.is_some() => {
                Some(m.attributes.as_ref().unwrap()[0].definition.pin())
            }
            _ => None,
        })
        .unwrap();
    for record in &mut c.records {
        let set_source = |m: &mut MoveRecord| {
            m.source = Some(MovePin {
                move_id: source.id.clone(),
                version: 1,
            });
            m.mappings = vec![AttributeMapping {
                shared_attribute_id: shared_attribute.id.clone(),
                local_attribute: mapped.clone(),
            }];
        };
        match &mut record.payload {
            InspectionPayload::Move(m) if m.world_id.is_some() => set_source(m),
            InspectionPayload::Learnset(l) => {
                for a in l.records.iter_mut().flatten() {
                    set_source(&mut a.move_record);
                }
            }
            _ => {}
        }
    }
    Package::build(c.clone()).unwrap();
    // The source attribute still exists in the package through local provenance,
    // but removing it from the pinned source Move must reject the mapping.
    let row = c
        .records
        .iter_mut()
        .find(|r| r.key.kind == DomainKind::Move && r.key.world_id.is_none())
        .unwrap();
    if let InspectionPayload::Move(m) = &mut row.payload {
        m.attributes = Some(vec![]);
    }
    reject(c);
}

#[test]
fn saved_policy_one_optional_configuration_is_not_silently_upgraded() {
    for policy in [1, 2] {
        let mut c = lab(policy);
        let row = c
            .records
            .iter_mut()
            .find(|r| r.key.kind == DomainKind::Ruleset)
            .unwrap();
        if let InspectionPayload::Ruleset(r) = &mut row.payload {
            r.max_moves = None;
            r.repeat_policy = RepeatPolicy::Unspecified;
        }
        let findings = Package::build(c).unwrap().diagnostics();
        assert_eq!(
            findings.iter().any(|f| f.code == "validator.preset.saved"
                && f.category == FindingCategory::Incomplete
                && f.message.contains("maximum")),
            policy == 1
        );
        assert_eq!(
            findings.iter().any(|f| f.code == "validator.preset.saved"
                && f.category == FindingCategory::NotEvaluated
                && f.message.contains("repeat policy")),
            policy == 2
        );
    }
}

#[test]
fn exact_stat_rosters_require_complete_ordered_embedded_definitions() {
    let mut c = empty();
    let stat = StatDefinition {
        id: id(50),
        world_id: Some(id(1)),
        source: None,
        fields: fields(),
        revision: 1,
        archived_at: None,
        constraints: NumericConstraints::default(),
    };
    add(&mut c, InspectionPayload::Stat(stat.clone()), true);
    let roster = StatRoster {
        id: id(51),
        world_id: id(1),
        revision: 1,
        entries: vec![stat.clone()],
    };
    add(&mut c, InspectionPayload::StatRoster(roster.clone()), true);
    let mut v = variant();
    v.stats = BaseStats {
        roster: Some(roster.pin()),
        assignments: Some(vec![StatValue {
            definition: stat,
            value: None,
        }]),
    };
    add(&mut c, InspectionPayload::Variant(v), true);
    assert!(Package::build(c.clone()).is_ok());
    let record = c
        .records
        .iter_mut()
        .find(|r| r.key.kind == DomainKind::Variant)
        .unwrap();
    if let InspectionPayload::Variant(v) = &mut record.payload {
        v.stats.assignments = Some(vec![]);
    }
    reject(c);
}

#[test]
fn chart_mapping_checks_exact_source_context_and_explicit_canonical_identity() {
    let mut c = empty();
    let shared = TypeRecord {
        id: id(60),
        fields: fields(),
        revision: 1,
        archived_at: None,
    };
    let local = WorldTypeRecord {
        id: id(61),
        world_id: id(1),
        source: Some(TypePin {
            type_id: shared.id.clone(),
            version: 1,
        }),
        fields: fields(),
        revision: 1,
        archived_at: None,
    };
    add(&mut c, InspectionPayload::Type(shared.clone()), false);
    add(&mut c, InspectionPayload::WorldType(local.clone()), true);
    let source = ChartRecord {
        id: id(62),
        world_id: None,
        fields: fields(),
        revision: 1,
        source: None,
        roster: vec![TypeAssignment {
            pin: TypePin {
                type_id: shared.id,
                version: 1,
            },
            fields: shared.fields,
        }],
        cells: vec![],
        mapping: vec![],
        archived_at: None,
    };
    add(&mut c, InspectionPayload::Chart(source.clone()), false);
    let chart = ChartRecord {
        id: id(63),
        world_id: Some(id(1)),
        fields: fields(),
        revision: 1,
        source: Some(crate::charts::ChartPin {
            chart_id: source.id,
            version: 1,
        }),
        roster: vec![TypeAssignment {
            pin: TypePin {
                type_id: local.id.clone(),
                version: 1,
            },
            fields: local.fields,
        }],
        cells: vec![ChartCell {
            attacker_type_id: local.id.clone(),
            defender_type_id: local.id.clone(),
            factor: Factor::new(0, 1).unwrap(),
        }],
        mapping: vec![crate::charts::ChartMapping {
            source_type_id: id(60),
            local_type_id: local.id,
        }],
        archived_at: None,
    };
    add(&mut c, InspectionPayload::Chart(chart), true);
    assert!(Package::build(c.clone()).is_ok());
    let record = c
        .records
        .iter_mut()
        .find(|r| r.key.kind == DomainKind::Chart && r.key.world_id.is_some())
        .unwrap();
    if let InspectionPayload::Chart(r) = &mut record.payload {
        r.mapping[0].source_type_id = id(999);
    }
    reject(c);
}

#[test]
fn operational_limits_and_noncanonical_outer_order_are_rejected() {
    let mut c = empty();
    c.roots = vec![c.world.clone(); MAX_ROOTS + 1];
    assert!(Package::build(c).unwrap_err().contains("root limit"));
    let mut c = lab(2);
    let record = c
        .records
        .iter_mut()
        .find(|r| r.key.kind == DomainKind::Preset)
        .unwrap();
    if let InspectionPayload::Preset(r) = &mut record.payload {
        r.entries = vec![id(101); MAX_OCCURRENCES + 1];
    }
    assert!(Package::build(c).unwrap_err().contains("occurrence limit"));
    let package = Package::build(lab(2)).unwrap();
    let mut unsorted = package.clone();
    unsorted.content.records.reverse();
    unsorted.manifest.content_sha256 = checksum(&unsorted.content).unwrap();
    assert!(Package::from_bytes(&serde_json::to_vec(&unsorted).unwrap()).is_err());
    assert!(
        Package::from_bytes(&vec![b' '; MAX_PACKAGE_BYTES + 1])
            .unwrap_err()
            .contains("32 MiB")
    );
}
