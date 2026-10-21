use super::*;
use crate::{
    card_fields::FieldKind,
    cards::{CardIdentity, CardNamespace, component_ref_pins},
    inspection::{DomainKind, DomainNodeKey, InspectionPayload},
};
use std::collections::{BTreeMap, BTreeSet};

fn require(ok: bool, message: &str) -> Result<(), String> {
    if ok { Ok(()) } else { Err(message.into()) }
}
fn field_errors(e: crate::authoring::FieldErrors) -> String {
    e.into_iter()
        .map(|(k, v)| format!("{k}: {v}"))
        .collect::<Vec<_>>()
        .join("; ")
}

/// Map a native exact typed key to its same-identity card pin, without adding
/// variants to the frozen v1 domain enums. Non-card typed records return None.
pub fn native_card_pin(key: &DomainNodeKey) -> Option<CardPin> {
    let namespace = match key.kind {
        DomainKind::Form => CardNamespace::Form,
        DomainKind::Variant => CardNamespace::Variant,
        DomainKind::Type => CardNamespace::Type,
        DomainKind::Move => CardNamespace::Move,
        _ => return None,
    };
    Some(CardPin {
        identity: CardIdentity {
            namespace,
            world_id: key.world_id.clone(),
            id: key.id.clone(),
        },
        version: key.revision,
    })
}

/// Standalone cards have no specialized typed projection.
pub fn native_key(pin: &CardPin) -> Option<DomainNodeKey> {
    let kind = match pin.identity.namespace {
        CardNamespace::Form => DomainKind::Form,
        CardNamespace::Variant => DomainKind::Variant,
        CardNamespace::Type => DomainKind::Type,
        CardNamespace::Move => DomainKind::Move,
        CardNamespace::Standalone => return None,
    };
    Some(DomainNodeKey {
        kind,
        world_id: pin.identity.world_id.clone(),
        id: pin.identity.id.clone(),
        revision: pin.version,
    })
}

pub(super) fn preflight(c: &CardPackageContent) -> Result<(), String> {
    occurrence_count(c).map(|_| ())
}

pub(crate) fn occurrence_count(c: &CardPackageContent) -> Result<usize, String> {
    require(
        c.typed
            .roots
            .len()
            .saturating_add(c.card_roots.len())
            .saturating_add(c.field_roots.len())
            <= MAX_ROOTS,
        "Card package exceeds the combined 2048 root limit.",
    )?;
    require(
        c.typed
            .records
            .len()
            .saturating_add(c.cards.len())
            .saturating_add(c.fields.len())
            <= MAX_RECORDS,
        "Card package exceeds the combined 8192 record limit.",
    )?;
    require(
        c.typed.bindings.len() <= MAX_RECORDS,
        "Too many identity bindings.",
    )?;
    let mut n = root_count(c) + c.typed.bindings.len();
    let mut add = |count: usize| -> Result<(), String> {
        n = n.checked_add(count).ok_or("Occurrence count overflow.")?;
        require(
            n <= MAX_OCCURRENCES,
            "Card package exceeds the combined 65536 occurrence limit.",
        )
    };
    for r in &c.typed.records {
        add(package::payload_occurrences(&r.payload)?)?;
    }
    fn field_size(f: &FieldDefinition) -> usize {
        1 + usize::from(f.source.is_some())
            + match &f.kind {
                FieldKind::Choice { options } => options.len(),
                _ => 0,
            }
    }
    for f in &c.fields {
        add(field_size(f))?;
    }
    for card in &c.cards {
        add(2 + usize::from(card.source.is_some()))?;
        add(card.component.labels.as_ref().map_or(0, Vec::len))?;
        for f in card.component.fields.iter().flatten() {
            add(2 + field_size(&f.definition))?;
        }
    }
    add(0)?;
    bounded_json(c)?;
    Ok(n)
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Node {
    Typed(DomainNodeKey),
    Card(CardPin),
    Field(FieldPin),
}

pub(super) fn validate(c: &CardPackageContent) -> Result<(), String> {
    validate_with_external_seeds(c, &[], &[], &[])
}

/// Seeds extend projection closure only; all intrinsic and root checks still run.
/// The caller must independently prove combined closure from real catalog roots.
pub(crate) fn validate_with_external_seeds(
    c: &CardPackageContent,
    external_cards: &[CardPin],
    external_fields: &[FieldPin],
    external_typed: &[DomainNodeKey],
) -> Result<(), String> {
    preflight(c)?;
    require(
        c.schema_version == SCHEMA_VERSION,
        "Unsupported card content schema.",
    )?;
    require(
        c.card_roots.windows(2).all(|w| w[0] < w[1])
            && c.field_roots.windows(2).all(|w| w[0] < w[1])
            && c.cards.windows(2).all(|w| w[0].pin() < w[1].pin())
            && c.fields.windows(2).all(|w| w[0].pin() < w[1].pin()),
        "Card/field records and roots must be uniquely canonical-sorted.",
    )?;
    let cards: BTreeMap<_, _> = c.cards.iter().map(|r| (r.pin(), r)).collect();
    let fields: BTreeMap<_, _> = c.fields.iter().map(|r| (r.pin(), r)).collect();
    let typed: BTreeMap<_, _> = c
        .typed
        .records
        .iter()
        .map(|r| (r.key.clone(), &r.payload))
        .collect();
    let bindings: BTreeMap<_, _> = c
        .typed
        .bindings
        .iter()
        .map(|b| (&b.identity, &b.target))
        .collect();
    let in_scope = |world: &Option<String>| world.as_ref().is_none_or(|w| w == &c.typed.world.id);
    let mut seeds = vec![];
    let mut graph = BTreeMap::<Node, Vec<Node>>::new();
    for r in &c.typed.records {
        let refs = package::references(&r.payload)?;
        let mut edges: Vec<_> = refs.exact.into_iter().map(Node::Typed).collect();
        for identity in refs.identities {
            edges.push(Node::Typed(
                (*bindings
                    .get(&identity)
                    .ok_or("Missing typed identity binding.")?)
                .clone(),
            ));
        }
        if let Some(pin) = native_card_pin(&r.key) {
            let card = cards
                .get(&pin)
                .ok_or("Missing same-revision native card component.")?;
            require(
                r.payload.fields() == Some(&card.fields)
                    && r.payload.archived_at() == card.archived_at,
                "Native card metadata/archive state differs from its typed payload.",
            )?;
            let source = native_source(&r.payload);
            require(
                card.source == source,
                "Native card source differs from its typed payload.",
            )?;
            edges.push(Node::Card(pin));
        }
        graph.insert(Node::Typed(r.key.clone()), edges);
    }
    for card in &c.cards {
        card.validate().map_err(field_errors)?;
        require(
            in_scope(&card.identity.world_id),
            "Card belongs to another world.",
        )?;
        require(
            !card.legacy_unmodeled || card.identity.namespace != CardNamespace::Standalone,
            "Standalone cards cannot claim legacy unmodeled components.",
        )?;
        let mut edges: Vec<_> = component_ref_pins(&card.component)
            .into_iter()
            .map(Node::Card)
            .collect();
        if let Some(p) = &card.source {
            edges.push(Node::Card(p.clone()));
        }
        for f in card.component.fields.iter().flatten() {
            require(
                fields.get(&f.definition.pin()).copied() == Some(&f.definition),
                "Embedded reusable field differs from its exact record.",
            )?;
            edges.push(Node::Field(f.definition.pin()));
        }
        if let Some(k) = native_key(&card.pin()) {
            require(
                typed.contains_key(&k),
                "Native card is missing its exact typed payload.",
            )?;
            seeds.push(k.clone());
            edges.push(Node::Typed(k));
        }
        graph.insert(Node::Card(card.pin()), edges);
    }
    for f in &c.fields {
        f.validate().map_err(field_errors)?;
        require(in_scope(&f.world_id), "Field belongs to another world.")?;
        graph.insert(
            Node::Field(f.pin()),
            f.source.iter().cloned().map(Node::Field).collect(),
        );
    }
    // This checks ALL native intrinsic/cross-record constraints, original catalog
    // roots, required identity bindings and typed closure. Seeds cannot relax any
    // of these rules. The combined traversal below rejects self-justifying extras.
    seeds.extend_from_slice(external_typed);
    package::validate_with_external_seeds(&c.typed, &seeds)?;
    let expected_native_roots: BTreeSet<_> =
        c.typed.roots.iter().filter_map(native_card_pin).collect();
    let actual_native_roots: BTreeSet<_> = c
        .card_roots
        .iter()
        .filter(|p| p.identity.namespace != CardNamespace::Standalone)
        .cloned()
        .collect();
    require(
        expected_native_roots == actual_native_roots,
        "Native card roots must exactly match the typed active local card roots.",
    )?;
    let mut root_identities = BTreeSet::new();
    for pin in &c.card_roots {
        let r = cards.get(pin).ok_or("Missing card root.")?;
        require(
            r.identity.world_id.as_ref() == Some(&c.typed.world.id)
                && r.archived_at.is_none()
                && root_identities.insert(&r.identity),
            "Card roots must be distinct active local identities.",
        )?;
    }
    let mut field_identities = BTreeSet::new();
    for pin in &c.field_roots {
        let f = fields.get(pin).ok_or("Missing field root.")?;
        require(
            f.world_id.as_ref() == Some(&c.typed.world.id)
                && f.archived_at.is_none()
                && field_identities.insert(&f.id),
            "Field roots must be distinct active local identities.",
        )?;
    }
    let mut pending: BTreeSet<_> = c
        .typed
        .roots
        .iter()
        .cloned()
        .map(Node::Typed)
        .chain(c.card_roots.iter().cloned().map(Node::Card))
        .chain(c.field_roots.iter().cloned().map(Node::Field))
        .chain(external_cards.iter().cloned().map(Node::Card))
        .chain(external_fields.iter().cloned().map(Node::Field))
        .chain(external_typed.iter().cloned().map(Node::Typed))
        .collect();
    let mut visited = BTreeSet::new();
    while let Some(node) = pending.pop_first() {
        if visited.insert(node.clone()) {
            for target in graph
                .get(&node)
                .ok_or("Missing exact card/field/typed dependency.")?
            {
                if !visited.contains(target) {
                    pending.insert(target.clone());
                }
            }
        }
    }
    require(
        visited.len() == graph.len(),
        "Card package contains records outside its combined root closure.",
    )
}

pub(super) fn native_source(payload: &InspectionPayload) -> Option<CardPin> {
    let (namespace, id, version) = match payload {
        InspectionPayload::Variant(r) => r
            .source
            .as_ref()
            .map(|p| (CardNamespace::Form, &p.form_id, p.version))?,
        InspectionPayload::WorldType(r) => r
            .source
            .as_ref()
            .map(|p| (CardNamespace::Type, &p.type_id, p.version))?,
        InspectionPayload::Move(r) => r
            .source
            .as_ref()
            .map(|p| (CardNamespace::Move, &p.move_id, p.version))?,
        _ => return None,
    };
    Some(CardPin {
        identity: CardIdentity {
            namespace,
            world_id: None,
            id: id.clone(),
        },
        version,
    })
}
