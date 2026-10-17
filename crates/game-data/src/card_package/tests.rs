use super::*;
use crate::{
    authoring::ContentFields,
    card_fields::{CardField, ChoiceOption, FieldKind, FieldValue},
    cards::{CardComponent, CardIdentity, CardNamespace},
    inspection::{InspectionPayload, PublicWorldContent},
    package::{IdentityBinding, Package, PackageRecord, payload_key},
    types::{TypeRecord, WorldTypeRecord},
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
fn empty() -> CardPackageContent {
    let payload = InspectionPayload::World(PublicWorldContent {
        id: id(1),
        fields: metadata(),
        revision: 1,
        archived_at: None,
    });
    let key = payload_key(&payload);
    CardPackageContent {
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
    }
}
fn standalone(n: usize) -> CardRecord {
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
        component: CardComponent {
            fields: Some(vec![]),
            rules_text: Some("Inert".into()),
            labels: Some(vec![]),
        },
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
fn add_native(c: &mut CardPackageContent, payload: InspectionPayload, root: bool) -> CardPin {
    let key = payload_key(&payload);
    let pin = native_card_pin(&key).unwrap();
    c.cards.push(CardRecord {
        identity: pin.identity.clone(),
        revision: pin.version,
        fields: payload.fields().unwrap().clone(),
        archived_at: payload.archived_at(),
        source: validation::native_source(&payload),
        component: CardComponent::default(),
        legacy_unmodeled: true,
    });
    c.typed.records.push(PackageRecord {
        key: key.clone(),
        payload,
    });
    if root {
        c.typed.roots.push(key);
        c.card_roots.push(pin.clone());
    }
    pin
}
fn cyclic() -> CardPackageContent {
    let mut c = empty();
    let f = field(10, FieldKind::CardReference);
    let mut a = standalone(2);
    let mut b = standalone(3);
    b.archived_at = Some(123);
    a.component.fields = Some(vec![CardField {
        definition: f.clone(),
        value: FieldValue::CardReference(b.pin()),
    }]);
    b.component.fields = Some(vec![CardField {
        definition: f.clone(),
        value: FieldValue::CardReference(a.pin()),
    }]);
    c.card_roots.push(a.pin());
    c.cards = vec![b, a];
    c.fields.push(f);
    c
}
// Simulate an attacker who recomputes the checksum; failures must come from
// structural validation, not merely digest mismatch.
fn rejects(mut p: CardPackage, change: impl FnOnce(&mut CardPackageContent)) {
    change(&mut p.content);
    p.manifest.root_count = root_count(&p.content);
    p.manifest.record_count = record_count(&p.content);
    p.manifest.content_sha256 = checksum(&p.content).unwrap();
    assert!(CardPackage::from_bytes(&bounded_json(&p).unwrap()).is_err());
}

#[test]
fn canonical_cycles_roundtrip_and_typed_v1_remains_independent() {
    let p = CardPackage::build(cyclic()).unwrap();
    let bytes = p.to_bytes().unwrap();
    assert_eq!(CardPackage::from_bytes(&bytes).unwrap(), p);
    assert_eq!(
        CardPackage::from_bytes(&serde_json::to_vec_pretty(&p).unwrap()).unwrap(),
        p
    );
    let mut shuffled = p.content.clone();
    shuffled.cards.reverse();
    assert_eq!(
        CardPackage::build(shuffled).unwrap().to_bytes().unwrap(),
        bytes
    );
    assert!(Package::from_bytes(&bytes).is_err());
    let v1 = Package::build(empty().typed).unwrap();
    assert_eq!(Package::from_bytes(&v1.to_bytes().unwrap()).unwrap(), v1);
    assert!(CardPackage::from_bytes(&v1.to_bytes().unwrap()).is_err());
    assert_eq!(schema_version_from_bytes(&bytes).unwrap(), 2);
    assert_eq!(
        schema_version_from_bytes(&v1.to_bytes().unwrap()).unwrap(),
        1
    );
}

#[test]
fn native_custom_references_extend_projection_without_promoting_old_roots() {
    let mut c = cyclic();
    let target = add_native(
        &mut c,
        InspectionPayload::WorldType(WorldTypeRecord {
            id: id(20),
            world_id: id(1),
            source: None,
            fields: metadata(),
            revision: 3,
            archived_at: Some(123),
        }),
        false,
    );
    c.cards
        .iter_mut()
        .find(|r| r.identity.id == id(3))
        .unwrap()
        .component
        .fields
        .as_mut()
        .unwrap()[0]
        .value = FieldValue::CardReference(target);
    let p = CardPackage::build(c).unwrap();
    assert!(Package::build(p.content.typed.clone()).is_err());
    assert_eq!(CardPackage::from_bytes(&p.to_bytes().unwrap()).unwrap(), p);
    rejects(p.clone(), |c| {
        c.cards
            .retain(|r| r.identity.namespace != CardNamespace::Type)
    });
    rejects(p.clone(), |c| {
        c.cards
            .iter_mut()
            .find(|r| r.identity.namespace == CardNamespace::Type)
            .unwrap()
            .fields
            .display_name = "Different".into()
    });
    rejects(p, |c| {
        c.typed
            .records
            .retain(|r| r.key.kind != crate::inspection::DomainKind::Type)
    });
}

#[test]
fn native_sources_and_field_sources_are_exact_transitive_dependencies() {
    let mut c = empty();
    let shared = add_native(
        &mut c,
        InspectionPayload::Type(TypeRecord {
            id: id(20),
            fields: metadata(),
            revision: 2,
            archived_at: Some(8),
        }),
        false,
    );
    let local = add_native(
        &mut c,
        InspectionPayload::WorldType(WorldTypeRecord {
            id: id(21),
            world_id: id(1),
            source: Some(crate::types::TypePin {
                type_id: id(20),
                version: 2,
            }),
            fields: metadata(),
            revision: 1,
            archived_at: None,
        }),
        true,
    );
    let mut sf = field(30, FieldKind::Boolean);
    sf.world_id = None;
    let mut lf = field(31, FieldKind::Boolean);
    lf.source = Some(sf.pin());
    c.cards
        .iter_mut()
        .find(|r| r.pin() == local)
        .unwrap()
        .legacy_unmodeled = false;
    c.cards
        .iter_mut()
        .find(|r| r.pin() == local)
        .unwrap()
        .component
        .fields = Some(vec![CardField {
        definition: lf.clone(),
        value: FieldValue::Boolean(false),
    }]);
    c.field_roots.push(lf.pin());
    c.fields = vec![lf, sf];
    let p = CardPackage::build(c).unwrap();
    rejects(p.clone(), |c| {
        c.cards
            .iter_mut()
            .find(|r| r.pin() == local)
            .unwrap()
            .source = None
    });
    rejects(p.clone(), |c| c.cards.retain(|r| r.pin() != shared));
    rejects(p.clone(), |c| c.fields.retain(|f| f.world_id.is_some()));
    rejects(p, |c| {
        c.cards
            .iter_mut()
            .find(|r| r.pin() == local)
            .unwrap()
            .component
            .fields
            .as_mut()
            .unwrap()[0]
            .definition
            .fields
            .description = "Mismatch".into()
    });
}

#[test]
fn unrelated_cycles_bindings_and_duplicate_keys_are_rejected() {
    let p = CardPackage::build(cyclic()).unwrap();
    rejects(p.clone(), |c| c.card_roots.clear());
    rejects(p.clone(), |c| c.cards.push(c.cards[0].clone()));
    rejects(p.clone(), |c| c.fields.push(c.fields[0].clone()));
    rejects(p.clone(), |c| c.card_roots.push(c.card_roots[0].clone()));
    rejects(p.clone(), |c| c.cards.push(standalone(99)));
    rejects(p.clone(), |c| c.fields.push(field(99, FieldKind::Boolean)));
    rejects(p.clone(), |c| {
        c.typed.bindings.push(IdentityBinding {
            identity: c.typed.world.identity(),
            target: c.typed.world.clone(),
        })
    });
    let text = String::from_utf8(p.to_bytes().unwrap()).unwrap();
    assert!(
        CardPackage::from_bytes(
            text.replacen(
                "\"schema_version\":2",
                "\"schema_version\":2,\"schema_version\":2",
                1
            )
            .as_bytes()
        )
        .is_err()
    );
    rejects(p, |c| c.schema_version = 3);
}

#[test]
fn values_scopes_legacy_and_roots_fail_closed() {
    let p = CardPackage::build(cyclic()).unwrap();
    rejects(p.clone(), |c| {
        c.cards[0].component.fields.as_mut().unwrap()[0].value = FieldValue::Boolean(false)
    });
    rejects(p.clone(), |c| c.cards[0].identity.world_id = Some(id(999)));
    rejects(p.clone(), |c| c.cards[0].legacy_unmodeled = true);
    rejects(p.clone(), |c| {
        c.card_roots = vec![
            c.cards
                .iter()
                .find(|r| r.archived_at.is_some())
                .unwrap()
                .pin(),
        ]
    });
    rejects(p, |c| {
        c.cards[0].component.fields.as_mut().unwrap()[0].value =
            FieldValue::CardReference(CardPin {
                identity: CardIdentity {
                    namespace: CardNamespace::Standalone,
                    world_id: Some(id(999)),
                    id: id(3),
                },
                version: 1,
            })
    });
}

#[test]
fn exact_numeric_choice_and_distinct_authored_states() {
    let mut c = empty();
    let f = field(
        10,
        FieldKind::Number(crate::stats::NumericConstraints {
            whole_only: true,
            minimum: Some("0".parse().unwrap()),
            maximum: Some("2".parse().unwrap()),
        }),
    );
    let mut card = standalone(2);
    card.component.fields = Some(vec![CardField {
        definition: f.clone(),
        value: FieldValue::Number("0".parse().unwrap()),
    }]);
    c.card_roots.push(card.pin());
    c.cards.push(card);
    c.fields.push(f);
    let p = CardPackage::build(c).unwrap();
    for v in [
        FieldValue::Missing,
        FieldValue::Unknown,
        FieldValue::NotApplicable,
        FieldValue::Number("0".parse().unwrap()),
    ] {
        let mut c = p.content.clone();
        c.cards[0].component.fields.as_mut().unwrap()[0].value = v;
        CardPackage::build(c).unwrap();
    }
    rejects(p.clone(), |c| {
        c.cards[0].component.fields.as_mut().unwrap()[0].value =
            FieldValue::Number("3".parse().unwrap())
    });
    rejects(p.clone(), |c| {
        c.cards[0].component.fields.as_mut().unwrap()[0].value =
            FieldValue::Number("0.5".parse().unwrap())
    });
    let mut c = p.content;
    c.fields[0].kind = FieldKind::Choice {
        options: vec![ChoiceOption {
            id: id(50),
            label: "One".into(),
        }],
    };
    c.cards[0].component.fields.as_mut().unwrap()[0] = CardField {
        definition: c.fields[0].clone(),
        value: FieldValue::Choice(id(50)),
    };
    let p = CardPackage::build(c).unwrap();
    rejects(p, |c| {
        c.cards[0].component.fields.as_mut().unwrap()[0].value = FieldValue::Choice("One".into())
    });
}

#[test]
fn operational_limits_and_untrusted_mutation() {
    assert!(
        CardPackage::from_bytes(&vec![b' '; MAX_PACKAGE_BYTES + 1])
            .unwrap_err()
            .contains("32 MiB")
    );
    let mut c = empty();
    c.card_roots = vec![standalone(2).pin(); MAX_ROOTS];
    assert!(CardPackage::build(c).unwrap_err().contains("root limit"));
    let mut c = empty();
    c.cards = vec![standalone(2); MAX_RECORDS];
    assert!(CardPackage::build(c).unwrap_err().contains("record limit"));
    let mut p = CardPackage::build(cyclic()).unwrap();
    p.content.cards[0].component.rules_text = Some("x".repeat(MAX_PACKAGE_BYTES));
    assert!(p.to_bytes().is_err());
    assert!(p.diagnostics().is_empty());
}

#[test]
fn referenced_form_expands_species_default_binding_and_its_component() {
    use crate::inspection::{PublicFormContent, PublicSpeciesContent};
    let mut c = cyclic();
    let species = InspectionPayload::Species(PublicSpeciesContent {
        id: id(40),
        fields: metadata(),
        revision: 1,
        default_form_id: id(41),
        archived_at: None,
    });
    c.typed.records.push(PackageRecord {
        key: payload_key(&species),
        payload: species,
    });
    let form = |n, is_default| {
        InspectionPayload::Form(PublicFormContent {
            id: id(n),
            species_id: id(40),
            species_version: 1,
            fields: metadata(),
            revision: 1,
            is_default,
            archived_at: Some(123),
            typing: crate::types::TypingState::unmodeled(),
            stats: crate::stats::BaseStats::unmodeled(),
        })
    };
    let default = add_native(&mut c, form(41, true), false);
    let referenced = add_native(&mut c, form(42, false), false);
    let target = native_key(&default).unwrap();
    c.typed.bindings.push(IdentityBinding {
        identity: target.identity(),
        target,
    });
    c.cards
        .iter_mut()
        .find(|r| r.identity.id == id(3))
        .unwrap()
        .component
        .fields
        .as_mut()
        .unwrap()[0]
        .value = FieldValue::CardReference(referenced);
    let p = CardPackage::build(c).unwrap();
    assert_eq!(p.content.typed.roots.len(), 1);
    assert_eq!(CardPackage::from_bytes(&p.to_bytes().unwrap()).unwrap(), p);
    rejects(p.clone(), |c| c.cards.retain(|r| r.pin() != default));
    rejects(p.clone(), |c| c.typed.bindings.clear());
    rejects(p, |c| {
        c.cards
            .iter_mut()
            .find(|r| r.pin() == default)
            .unwrap()
            .source = Some(standalone(90).pin())
    });
}

#[test]
fn active_native_root_requires_exact_card_root_but_shared_forms_are_not_roots() {
    let mut c = empty();
    add_native(
        &mut c,
        InspectionPayload::WorldType(WorldTypeRecord {
            id: id(20),
            world_id: id(1),
            source: None,
            fields: metadata(),
            revision: 2,
            archived_at: None,
        }),
        true,
    );
    let p = CardPackage::build(c).unwrap();
    rejects(p.clone(), |c| c.card_roots.clear());
    rejects(p, |c| c.cards[0].archived_at = Some(123));
}
