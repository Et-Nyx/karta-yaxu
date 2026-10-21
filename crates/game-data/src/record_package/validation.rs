use super::*;
use crate::{
    card_fields::{FieldDefinition, FieldKind, FieldPin},
    card_package::{self, native_card_pin, native_key},
    cards::{CardPin, component_ref_pins},
    inspection::{DomainKind, DomainNodeKey},
    relations::{CollectionValue, ColumnKind, DefinitionPin, RecordValue},
};
use std::collections::{BTreeMap, BTreeSet};

fn require(ok: bool, message: &str) -> Result<(), String> {
    if ok { Ok(()) } else { Err(message.into()) }
}
fn domain(result: Result<(), crate::authoring::FieldErrors>) -> Result<(), String> {
    result.map_err(|e| {
        e.into_iter()
            .map(|(k, v)| format!("{k}: {v}"))
            .collect::<Vec<_>>()
            .join("; ")
    })
}

/// Count embedded public evidence too, before cloning graph keys or validating
/// nested values. Wire bytes are bounded by the existing streaming serializer.
pub(super) fn preflight(c: &RecordPackageContent) -> Result<(), String> {
    require(
        root_count(c) <= MAX_ROOTS,
        "Record package exceeds the combined 2048 root limit.",
    )?;
    require(
        record_count(c) <= MAX_RECORDS,
        "Record package exceeds the combined 8192 record limit.",
    )?;
    let mut n = card_package::occurrence_count(&c.cards)?;
    let mut add = |count: usize| -> Result<(), String> {
        n = n.checked_add(count).ok_or("Occurrence count overflow.")?;
        require(
            n <= MAX_OCCURRENCES,
            "Record package exceeds the combined 65536 occurrence limit.",
        )
    };
    fn field(
        f: &FieldDefinition,
        add: &mut impl FnMut(usize) -> Result<(), String>,
    ) -> Result<(), String> {
        add(1 + usize::from(f.source.is_some()))?;
        if let FieldKind::Choice { options } = &f.kind {
            add(options.len())?;
        }
        Ok(())
    }
    fn layout(
        l: &LayoutRecord,
        add: &mut impl FnMut(usize) -> Result<(), String>,
    ) -> Result<(), String> {
        add(1 + usize::from(l.source.is_some()))?;
        for col in &l.columns {
            add(1)?;
            match &col.kind {
                ColumnKind::Field(f) => field(f, add)?,
                ColumnKind::Reference { allowed } => add(allowed.len())?,
            }
        }
        Ok(())
    }
    fn collection(
        d: &CollectionDefinition,
        add: &mut impl FnMut(usize) -> Result<(), String>,
    ) -> Result<(), String> {
        add(1 + usize::from(d.source.is_some()))?;
        layout(&d.layout, add)
    }
    add(c.layout_roots.len())?;
    add(c.collection_roots.len())?;
    for l in &c.layouts {
        layout(l, &mut add)?;
    }
    for d in &c.collections {
        collection(d, &mut add)?;
    }
    for r in &c.relations {
        add(1)?;
        for a in r.relations.collections.iter().flatten() {
            add(1)?;
            collection(&a.definition, &mut add)?;
            if let CollectionValue::Entries(entries) = &a.value {
                for e in entries {
                    add(1)?;
                    add(e.cells.len())?;
                    for cell in &e.cells {
                        if matches!(cell.value, RecordValue::Reference(_)) {
                            add(1)?;
                        }
                    }
                }
            }
        }
    }
    bounded_json(c)?;
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Node {
    Typed(DomainNodeKey),
    Card(CardPin),
    Field(FieldPin),
    Relation(CardPin),
    Layout(LayoutPin),
    Collection(CollectionPin),
}
fn target(p: &DefinitionPin) -> Node {
    match p {
        DefinitionPin::Card(p) => Node::Card(p.clone()),
        DefinitionPin::Field(p) => Node::Field(p.clone()),
        DefinitionPin::Stat {
            world_id,
            stat_id,
            version,
        } => Node::Typed(DomainNodeKey {
            kind: DomainKind::Stat,
            world_id: world_id.clone(),
            id: stat_id.clone(),
            revision: *version,
        }),
    }
}

pub(super) fn validate(c: &RecordPackageContent) -> Result<(), String> {
    preflight(c)?;
    require(
        c.schema_version == SCHEMA_VERSION,
        "Unsupported record content schema.",
    )?;
    require(
        c.layout_roots.windows(2).all(|w| w[0] < w[1])
            && c.collection_roots.windows(2).all(|w| w[0] < w[1])
            && c.layouts.windows(2).all(|w| w[0].pin() < w[1].pin())
            && c.collections.windows(2).all(|w| w[0].pin() < w[1].pin())
            && c.relations.windows(2).all(|w| w[0].pin() < w[1].pin()),
        "Record roots and keyed records must be uniquely canonical-sorted.",
    )?;
    let cards = &c.cards;
    require(
        c.relations.len() == cards.cards.len()
            && c.relations
                .iter()
                .zip(&cards.cards)
                .all(|(r, card)| r.card == card.pin()),
        "Every card must have exactly one same-pin relation component.",
    )?;
    let layouts: BTreeMap<_, _> = c.layouts.iter().map(|l| (l.pin(), l)).collect();
    let collections: BTreeMap<_, _> = c.collections.iter().map(|d| (d.pin(), d)).collect();
    let fields: BTreeMap<_, _> = cards.fields.iter().map(|f| (f.pin(), f)).collect();
    let in_scope = |w: &Option<String>| w.as_ref().is_none_or(|w| w == &cards.typed.world.id);
    let mut graph = BTreeMap::<Node, Vec<Node>>::new();
    // Independently derive the complete old projection edges as well as new edges.
    // Projection validator seeds below are NOT roots of this proof.
    let bindings: BTreeMap<_, _> = cards
        .typed
        .bindings
        .iter()
        .map(|b| (&b.identity, &b.target))
        .collect();
    for r in &cards.typed.records {
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
            edges.push(Node::Card(pin));
        }
        graph.insert(Node::Typed(r.key.clone()), edges);
    }
    for card in &cards.cards {
        let mut edges: Vec<_> = component_ref_pins(&card.component)
            .into_iter()
            .map(Node::Card)
            .collect();
        edges.extend(card.source.iter().cloned().map(Node::Card));
        edges.extend(
            card.component
                .fields
                .iter()
                .flatten()
                .map(|f| Node::Field(f.definition.pin())),
        );
        if let Some(key) = native_key(&card.pin()) {
            edges.push(Node::Typed(key));
        }
        edges.push(Node::Relation(card.pin()));
        graph.insert(Node::Card(card.pin()), edges);
    }
    for f in &cards.fields {
        graph.insert(
            Node::Field(f.pin()),
            f.source.iter().cloned().map(Node::Field).collect(),
        );
    }
    let mut external_cards = BTreeSet::new();
    let mut external_fields = BTreeSet::new();
    let mut external_typed = BTreeSet::new();
    for l in &c.layouts {
        domain(l.validate())?;
        require(in_scope(&l.world_id), "Layout belongs to another world.")?;
        let mut edges: Vec<_> = l.source.iter().cloned().map(Node::Layout).collect();
        for col in &l.columns {
            if let ColumnKind::Field(f) = &col.kind {
                require(
                    fields.get(&f.pin()).copied() == Some(f),
                    "Layout scalar field differs from its exact indexed field.",
                )?;
                edges.push(Node::Field(f.pin()));
                external_fields.insert(f.pin());
            }
        }
        graph.insert(Node::Layout(l.pin()), edges);
    }
    for d in &c.collections {
        domain(d.validate())?;
        require(
            in_scope(&d.world_id),
            "Collection belongs to another world.",
        )?;
        require(
            layouts.get(&d.layout.pin()).copied() == Some(&d.layout),
            "Embedded collection layout differs from its exact indexed layout.",
        )?;
        let mut edges: Vec<_> = d.source.iter().cloned().map(Node::Collection).collect();
        edges.push(Node::Layout(d.layout.pin()));
        graph.insert(Node::Collection(d.pin()), edges);
    }
    for r in &c.relations {
        domain(r.validate())?;
        let mut edges = vec![];
        for a in r.relations.collections.iter().flatten() {
            require(
                collections.get(&a.definition.pin()).copied() == Some(&a.definition),
                "Relation collection differs from its exact indexed collection.",
            )?;
            edges.push(Node::Collection(a.definition.pin()));
            if let CollectionValue::Entries(entries) = &a.value {
                for e in entries {
                    for cell in &e.cells {
                        if let RecordValue::Reference(p) = &cell.value {
                            let node = target(p);
                            match &node {
                                Node::Card(p) => {
                                    external_cards.insert(p.clone());
                                }
                                Node::Field(p) => {
                                    external_fields.insert(p.clone());
                                }
                                Node::Typed(p) => {
                                    external_typed.insert(p.clone());
                                }
                                _ => unreachable!(),
                            }
                            edges.push(node);
                        }
                    }
                }
            }
        }
        graph.insert(Node::Relation(r.pin()), edges);
    }
    card_package::validate_with_external_seeds(
        cards,
        &external_cards.into_iter().collect::<Vec<_>>(),
        &external_fields.into_iter().collect::<Vec<_>>(),
        &external_typed.into_iter().collect::<Vec<_>>(),
    )?;
    let mut identities = BTreeSet::new();
    for pin in &c.layout_roots {
        let l = layouts.get(pin).ok_or("Missing layout root.")?;
        require(
            l.world_id.as_ref() == Some(&cards.typed.world.id)
                && l.archived_at.is_none()
                && identities.insert(&l.id),
            "Layout roots must be distinct active local identities.",
        )?;
    }
    let mut identities = BTreeSet::new();
    for pin in &c.collection_roots {
        let d = collections.get(pin).ok_or("Missing collection root.")?;
        require(
            d.world_id.as_ref() == Some(&cards.typed.world.id)
                && d.archived_at.is_none()
                && identities.insert(&d.id),
            "Collection roots must be distinct active local identities.",
        )?;
    }
    let mut pending: BTreeSet<_> = cards
        .typed
        .roots
        .iter()
        .cloned()
        .map(Node::Typed)
        .chain(cards.card_roots.iter().cloned().map(Node::Card))
        .chain(cards.field_roots.iter().cloned().map(Node::Field))
        .chain(c.layout_roots.iter().cloned().map(Node::Layout))
        .chain(c.collection_roots.iter().cloned().map(Node::Collection))
        .collect();
    let mut visited = BTreeSet::new();
    while let Some(node) = pending.pop_first() {
        if visited.insert(node.clone()) {
            for edge in graph
                .get(&node)
                .ok_or("Missing exact record package dependency.")?
            {
                if !visited.contains(edge) {
                    pending.insert(edge.clone());
                }
            }
        }
    }
    require(
        visited.len() == graph.len(),
        "Record package contains evidence outside its combined root closure.",
    )
}
