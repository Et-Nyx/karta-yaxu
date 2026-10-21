//! Public card evidence captured in the same read snapshot as typed inspection.
//! Keyed maps are internal DTOs: fingerprint consumers serialize iterated vectors,
//! never JSON objects with structured keys. Heads are observations, not exact pins.
use crate::{
    card_fields::{FieldDefinition, FieldPin},
    cards::{CardIdentity, CardPin, CardRecord},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapturedCards {
    #[serde(default)]
    pub records: crate::record_inspection::CapturedRecords,
    pub cards: BTreeMap<CardPin, CardRecord>,
    pub fields: BTreeMap<FieldPin, FieldDefinition>,
    pub card_roots: BTreeSet<CardPin>,
    pub field_roots: BTreeSet<FieldPin>,
    pub heads: BTreeMap<CardIdentity, CardPin>,
    /// Sorted by (world_id, field_id), independently of revision.
    pub field_heads: Vec<FieldPin>,
}

use crate::{
    card_fields::FieldValue,
    cards::CardNamespace,
    inspection::{
        CapturedWorld, DomainIdentity, DomainKind, DomainNodeKey, DomainRef, EdgeKind,
        InspectionEdge, RootRole,
    },
};
use std::collections::VecDeque;

/// Native definitions and their card components are one node, not two identities.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Node {
    Card(CardPin),
    Field(FieldPin),
    Typed(DomainNodeKey),
    Layout(crate::relations::LayoutPin),
    Collection(crate::relations::CollectionPin),
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Identity {
    Card(CardIdentity),
    Field {
        world_id: Option<String>,
        id: String,
    },
    Typed(DomainIdentity),
    Layout {
        world_id: Option<String>,
        id: String,
    },
    Collection {
        world_id: Option<String>,
        id: String,
    },
}
fn native_identity(key: &DomainIdentity) -> Identity {
    let namespace = match key.kind {
        DomainKind::Form => CardNamespace::Form,
        DomainKind::Variant => CardNamespace::Variant,
        DomainKind::Type => CardNamespace::Type,
        DomainKind::Move => CardNamespace::Move,
        _ => return Identity::Typed(key.clone()),
    };
    Identity::Card(CardIdentity {
        namespace,
        world_id: key.world_id.clone(),
        id: key.id.clone(),
    })
}
impl Node {
    pub fn native(key: &DomainNodeKey) -> Self {
        match native_identity(&key.identity()) {
            Identity::Card(identity) => Self::Card(CardPin {
                identity,
                version: key.revision,
            }),
            _ => Self::Typed(key.clone()),
        }
    }
    pub fn identity(&self) -> Identity {
        match self {
            Self::Card(p) => Identity::Card(p.identity.clone()),
            Self::Field(p) => Identity::Field {
                world_id: p.world_id.clone(),
                id: p.field_id.clone(),
            },
            Self::Typed(p) => Identity::Typed(p.identity()),
            Self::Layout(p) => Identity::Layout {
                world_id: p.world_id.clone(),
                id: p.layout_id.clone(),
            },
            Self::Collection(p) => Identity::Collection {
                world_id: p.world_id.clone(),
                id: p.collection_id.clone(),
            },
        }
    }
    pub fn version(&self) -> i64 {
        match self {
            Self::Card(p) => p.version,
            Self::Field(p) => p.version,
            Self::Typed(p) => p.revision,
            Self::Layout(p) => p.version,
            Self::Collection(p) => p.version,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Site {
    Native(Box<InspectionEdge>),
    CardSource,
    FieldSource,
    Assignment {
        slot: usize,
    },
    CardReference {
        slot: usize,
        field: FieldPin,
    },
    RecordCollection {
        slot: usize,
    },
    CollectionLayout,
    LayoutField {
        slot: usize,
        column_id: String,
    },
    LayoutSource,
    CollectionSource,
    RecordReference {
        collection: crate::relations::CollectionPin,
        entry_id: String,
        column_id: String,
        row_slot: usize,
        cell_slot: usize,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Edge {
    pub from: Node,
    /// Identity-only native observations remain direct all-version evidence only.
    pub to: Option<Node>,
    pub identity: Identity,
    pub site: Site,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Path {
    pub root: Node,
    pub edges: Vec<Edge>,
}
#[derive(Clone, Debug)]
pub struct Query {
    pub target: Identity,
    pub version: Option<i64>,
    pub paths: bool,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Usage {
    pub captured: Vec<Node>,
    pub direct: Vec<Edge>,
    pub paths: Vec<Path>,
    pub total: usize,
    pub truncated: bool,
    pub complete: bool,
}
const RESULTS: usize = 256;
const DEPTH: usize = 32;
const WORK: usize = 65_536;

fn local(identity: &Identity, world: &str) -> bool {
    let scope = match identity {
        Identity::Card(i) => &i.world_id,
        Identity::Field { world_id, .. } => world_id,
        Identity::Typed(i) => &i.world_id,
        Identity::Layout { world_id, .. } | Identity::Collection { world_id, .. } => world_id,
    };
    scope.as_deref().is_none_or(|w| w == world)
}

/// Pure bounded reverse BFS over the union of native and custom evidence.
/// One deterministic shortest representative per root; membership/ownership never
/// become uses, and identity-only observations cannot invent historical paths.
pub fn where_used(capture: &CapturedWorld, query: &Query) -> Usage {
    let mut result = Usage {
        complete: capture.is_complete(),
        ..Usage::default()
    };
    if !local(&query.target, &capture.world.id) {
        return result;
    }
    let mut nodes: BTreeSet<Node> = capture.nodes.keys().map(Node::native).collect();
    nodes.extend(capture.card_data.cards.keys().cloned().map(Node::Card));
    nodes.extend(capture.card_data.fields.keys().cloned().map(Node::Field));
    nodes.extend(
        capture
            .card_data
            .records
            .layouts
            .keys()
            .cloned()
            .map(Node::Layout),
    );
    nodes.extend(
        capture
            .card_data
            .records
            .collections
            .keys()
            .cloned()
            .map(Node::Collection),
    );
    result.captured = nodes
        .into_iter()
        .filter(|n| n.identity() == query.target && query.version.is_none_or(|v| n.version() == v))
        .collect();
    let mut roots: BTreeSet<_> = capture
        .roots
        .iter()
        .filter(|(_, role)| **role != RootRole::Dependency)
        .map(|(k, _)| Node::native(k))
        .collect();
    roots.extend(capture.card_data.card_roots.iter().cloned().map(Node::Card));
    roots.extend(
        capture
            .card_data
            .field_roots
            .iter()
            .cloned()
            .map(Node::Field),
    );
    let mut edges = Vec::new();
    roots.extend(
        capture
            .card_data
            .records
            .layout_roots
            .iter()
            .cloned()
            .map(Node::Layout),
    );
    roots.extend(
        capture
            .card_data
            .records
            .collection_roots
            .iter()
            .cloned()
            .map(Node::Collection),
    );
    let mut work = 0;
    let mut push = |edge: Edge| {
        work += 1;
        if work <= WORK
            && local(&edge.from.identity(), &capture.world.id)
            && local(&edge.identity, &capture.world.id)
        {
            edges.push(edge);
        }
        work <= WORK
    };
    'build: {
        for e in &capture.edges {
            if matches!(e.kind, EdgeKind::Membership | EdgeKind::Ownership) {
                continue;
            }
            let (to, identity) = match &e.to {
                DomainRef::Exact(k) => {
                    let n = Node::native(k);
                    let i = n.identity();
                    (Some(n), i)
                }
                DomainRef::Identity(i) => (None, native_identity(i)),
            };
            // Native source pins already exist in card evidence. Do not duplicate them.
            if matches!(e.site, crate::inspection::EdgeSite::Source)
                && let (Node::Card(from), Some(Node::Card(to))) = (Node::native(&e.from), &to)
                && capture
                    .card_data
                    .cards
                    .get(&from)
                    .is_some_and(|r| r.source.as_ref() == Some(to))
            {
                continue;
            }
            if !push(Edge {
                from: Node::native(&e.from),
                to,
                identity,
                site: Site::Native(Box::new(e.clone())),
            }) {
                break 'build;
            }
        }
        for (pin, card) in &capture.card_data.cards {
            let from = Node::Card(pin.clone());
            if let Some(source) = &card.source {
                let to = Node::Card(source.clone());
                if !push(Edge {
                    from: from.clone(),
                    identity: to.identity(),
                    to: Some(to),
                    site: Site::CardSource,
                }) {
                    break 'build;
                }
            }
            for (slot, field) in card.component.fields.iter().flatten().enumerate() {
                let to = Node::Field(field.definition.pin());
                if !push(Edge {
                    from: from.clone(),
                    identity: to.identity(),
                    to: Some(to),
                    site: Site::Assignment { slot: slot + 1 },
                }) {
                    break 'build;
                }
                if let FieldValue::CardReference(target) = &field.value {
                    let to = Node::Card(target.clone());
                    if !push(Edge {
                        from: from.clone(),
                        identity: to.identity(),
                        to: Some(to),
                        site: Site::CardReference {
                            slot: slot + 1,
                            field: field.definition.pin(),
                        },
                    }) {
                        break 'build;
                    }
                }
            }
        }
        for (pin, field) in &capture.card_data.fields {
            if let Some(source) = &field.source {
                let to = Node::Field(source.clone());
                if !push(Edge {
                    from: Node::Field(pin.clone()),
                    identity: to.identity(),
                    to: Some(to),
                    site: Site::FieldSource,
                }) {
                    break 'build;
                }
            }
        }
        for (pin, layout) in &capture.card_data.records.layouts {
            let from = Node::Layout(pin.clone());
            if let Some(source) = &layout.source {
                let to = Node::Layout(source.clone());
                if !push(Edge {
                    from: from.clone(),
                    identity: to.identity(),
                    to: Some(to),
                    site: Site::LayoutSource,
                }) {
                    break 'build;
                }
            }
            for (slot, column) in layout.columns.iter().enumerate() {
                if let crate::relations::ColumnKind::Field(field) = &column.kind {
                    let to = Node::Field(field.pin());
                    if !push(Edge {
                        from: from.clone(),
                        identity: to.identity(),
                        to: Some(to),
                        site: Site::LayoutField {
                            slot: slot + 1,
                            column_id: column.id.clone(),
                        },
                    }) {
                        break 'build;
                    }
                }
            }
        }
        for (pin, definition) in &capture.card_data.records.collections {
            let from = Node::Collection(pin.clone());
            let to = Node::Layout(definition.layout.pin());
            if !push(Edge {
                from: from.clone(),
                identity: to.identity(),
                to: Some(to),
                site: Site::CollectionLayout,
            }) {
                break 'build;
            }
            if let Some(source) = &definition.source {
                let to = Node::Collection(source.clone());
                if !push(Edge {
                    from,
                    identity: to.identity(),
                    to: Some(to),
                    site: Site::CollectionSource,
                }) {
                    break 'build;
                }
            }
        }
        for (pin, record) in &capture.card_data.records.relations {
            let from = Node::Card(pin.clone());
            for (slot, assignment) in record.relations.collections.iter().flatten().enumerate() {
                let to = Node::Collection(assignment.definition.pin());
                if !push(Edge {
                    from: from.clone(),
                    identity: to.identity(),
                    to: Some(to),
                    site: Site::RecordCollection { slot: slot + 1 },
                }) {
                    break 'build;
                }
                if let crate::relations::CollectionValue::Entries(entries) = &assignment.value {
                    for (row_slot, entry) in entries.iter().enumerate() {
                        for (cell_slot, cell) in entry.cells.iter().enumerate() {
                            if let crate::relations::RecordValue::Reference(pin) = &cell.value {
                                let to = record_target(pin);
                                if !push(Edge {
                                    from: from.clone(),
                                    identity: to.identity(),
                                    to: Some(to),
                                    site: Site::RecordReference {
                                        collection: assignment.definition.pin(),
                                        entry_id: entry.id.clone(),
                                        column_id: cell.column_id.clone(),
                                        row_slot: row_slot + 1,
                                        cell_slot: cell_slot + 1,
                                    },
                                }) {
                                    break 'build;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    result.truncated = work > WORK;
    edges.sort(); // keep repeated occurrence sites; never deduplicate selections
    if !query.paths {
        for e in edges.iter().filter(|e| {
            e.identity == query.target
                && query
                    .version
                    .is_none_or(|v| e.to.as_ref().is_some_and(|n| n.version() == v))
        }) {
            result.total += 1;
            if result.direct.len() < RESULTS {
                result.direct.push(e.clone());
            } else {
                result.truncated = true;
            }
        }
    } else {
        let mut reverse: BTreeMap<&Node, Vec<usize>> = BTreeMap::new();
        for (i, e) in edges.iter().enumerate() {
            if let Some(to) = &e.to {
                reverse.entry(to).or_default().push(i);
            }
        }
        let mut next: BTreeMap<Node, Option<usize>> =
            result.captured.iter().cloned().map(|n| (n, None)).collect();
        let mut queue: VecDeque<_> = result.captured.iter().cloned().map(|n| (n, 0)).collect();
        let mut traversal = 0;
        while let Some((node, depth)) = queue.pop_front() {
            let Some(incoming) = reverse.get(&node) else {
                continue;
            };
            if depth >= DEPTH {
                if !incoming.is_empty() {
                    result.truncated = true;
                }
                continue;
            }
            for &i in incoming {
                traversal += 1;
                if traversal > WORK {
                    result.truncated = true;
                    break;
                }
                let from = &edges[i].from;
                if let std::collections::btree_map::Entry::Vacant(entry) = next.entry(from.clone())
                {
                    entry.insert(Some(i));
                    queue.push_back((from.clone(), depth + 1));
                }
            }
            if traversal > WORK {
                break;
            }
        }
        for root in roots {
            if !matches!(next.get(&root), Some(Some(_))) {
                continue;
            }
            result.total += 1;
            if result.paths.len() == RESULTS {
                result.truncated = true;
                continue;
            }
            let mut path = Vec::new();
            let mut node = &root;
            while let Some(Some(i)) = next.get(node) {
                let e = &edges[*i];
                path.push(e.clone());
                node = e.to.as_ref().expect("only exact edges enter BFS");
            }
            result.paths.push(Path { root, edges: path });
        }
    }
    result.complete &= !result.truncated;
    result
}

pub fn record_target(pin: &crate::relations::DefinitionPin) -> Node {
    match pin {
        crate::relations::DefinitionPin::Card(p) => Node::Card(p.clone()),
        crate::relations::DefinitionPin::Field(p) => Node::Field(p.clone()),
        crate::relations::DefinitionPin::Stat {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        authoring::ContentFields,
        card_fields::{CardField, FieldKind},
        cards::CardComponent,
        inspection::{CaptureGap, CaptureGapKind, EdgeSite, PublicWorldContent},
    };
    fn id(n: usize) -> String {
        format!("{n:032x}")
    }
    fn fields() -> ContentFields {
        ContentFields {
            slug: "sample".into(),
            display_name: "Sample".into(),
            description: String::new(),
        }
    }
    fn capture() -> CapturedWorld {
        CapturedWorld {
            contract_version: 1,
            world: PublicWorldContent {
                id: id(1),
                fields: fields(),
                revision: 1,
                archived_at: None,
            },
            roots: BTreeMap::new(),
            nodes: BTreeMap::new(),
            edges: vec![],
            heads: BTreeMap::new(),
            absences: vec![],
            gaps: vec![],
            card_data: CapturedCards::default(),
        }
    }
    fn card(n: usize, version: i64) -> CardRecord {
        CardRecord {
            identity: CardIdentity {
                namespace: CardNamespace::Move,
                world_id: Some(id(1)),
                id: id(n),
            },
            revision: version,
            fields: fields(),
            archived_at: None,
            source: None,
            component: CardComponent::default(),
            legacy_unmodeled: false,
        }
    }
    fn field(n: usize) -> FieldDefinition {
        FieldDefinition {
            id: id(n),
            world_id: Some(id(1)),
            source: None,
            fields: fields(),
            revision: 1,
            archived_at: None,
            kind: FieldKind::CardReference,
        }
    }
    fn insert(c: &mut CapturedWorld, r: CardRecord, root: bool) -> CardPin {
        let p = r.pin();
        if root {
            c.card_data.card_roots.insert(p.clone());
        }
        c.card_data.cards.insert(p.clone(), r);
        p
    }
    fn reference(r: &mut CardRecord, target: &CardPin, n: usize) {
        r.component.fields.get_or_insert_default().push(CardField {
            definition: field(n),
            value: FieldValue::CardReference(target.clone()),
        });
    }
    fn query(p: &CardPin, paths: bool) -> Query {
        Query {
            target: Identity::Card(p.identity.clone()),
            version: Some(p.version),
            paths,
        }
    }
    fn native(p: &CardPin) -> DomainNodeKey {
        DomainNodeKey {
            kind: DomainKind::Move,
            world_id: p.identity.world_id.clone(),
            id: p.identity.id.clone(),
            revision: p.version,
        }
    }
    #[test]
    fn repeated_reference_sites_and_versions_stay_distinct() {
        let mut c = capture();
        let target = insert(&mut c, card(2, 1), true);
        let newer = insert(&mut c, card(2, 2), true);
        let mut r = card(3, 1);
        reference(&mut r, &target, 10);
        reference(&mut r, &target, 11);
        reference(&mut r, &newer, 12);
        insert(&mut c, r, true);
        let found = where_used(&c, &query(&target, false));
        assert_eq!(found.total, 2);
        assert_ne!(found.direct[0].site, found.direct[1].site);
        assert_eq!(where_used(&c, &query(&newer, false)).total, 1);
        let mut q = query(&target, false);
        q.version = None;
        let all = where_used(&c, &q);
        assert_eq!(all.total, 3);
        assert_eq!(all.captured.len(), 2);
    }
    #[test]
    fn native_consumers_and_custom_references_share_mixed_shortest_paths_with_cycles() {
        let mut c = capture();
        let target = insert(&mut c, card(2, 1), true);
        let mut r = card(3, 1);
        reference(&mut r, &target, 10);
        let consumer = insert(&mut c, r, true);
        reference(c.card_data.cards.get_mut(&target).unwrap(), &consumer, 11);
        let preset = DomainNodeKey {
            kind: DomainKind::Preset,
            world_id: Some(id(1)),
            id: id(4),
            revision: 1,
        };
        c.roots.insert(preset.clone(), RootRole::Active);
        c.edges.push(InspectionEdge {
            from: preset.clone(),
            to: DomainRef::Exact(native(&consumer)),
            kind: EdgeKind::Selection,
            site: EdgeSite::PresetSelection {
                slot: 1,
                record_id: id(20),
                learnset: DomainNodeKey {
                    kind: DomainKind::Learnset,
                    ..preset.clone()
                },
            },
        });
        let q = query(&target, true);
        let found = where_used(&c, &q);
        assert!(found.complete);
        assert_eq!(found.paths.len(), 2);
        let mixed = found
            .paths
            .iter()
            .find(|p| p.root == Node::Typed(preset.clone()))
            .unwrap();
        assert_eq!(mixed.edges.len(), 2);
        assert!(matches!(mixed.edges[1].site, Site::CardReference { .. }));
        c.edges.reverse();
        assert_eq!(where_used(&c, &q), found);
    }
    #[test]
    fn root_membership_and_ownership_are_not_usage_and_partial_never_complete() {
        let mut c = capture();
        let p = insert(&mut c, card(2, 1), true);
        for kind in [EdgeKind::Membership, EdgeKind::Ownership] {
            c.edges.push(InspectionEdge {
                from: c.world_key(),
                to: DomainRef::Exact(native(&p)),
                kind,
                site: EdgeSite::Owner,
            });
        }
        for paths in [false, true] {
            let f = where_used(&c, &query(&p, paths));
            assert_eq!(f.total, 0);
            assert!(f.complete);
        }
        c.gaps.push(CaptureGap {
            kind: CaptureGapKind::Budget,
            subject: None,
            message: "Partial".into(),
        });
        assert!(!where_used(&c, &query(&p, false)).complete);
    }
    #[test]
    fn assignments_and_explicit_sources_use_exact_field_and_card_revisions() {
        let mut c = capture();
        let mut shared = card(2, 1);
        shared.identity.world_id = None;
        let source = insert(&mut c, shared, false);
        let mut local_card = card(3, 1);
        local_card.source = Some(source.clone());
        reference(&mut local_card, &source, 10);
        let f = local_card.component.fields.as_ref().unwrap()[0]
            .definition
            .clone();
        c.card_data.fields.insert(f.pin(), f.clone());
        insert(&mut c, local_card, true);
        assert_eq!(where_used(&c, &query(&source, false)).total, 2);
        let q = Query {
            target: Identity::Field {
                world_id: f.world_id.clone(),
                id: f.id.clone(),
            },
            version: Some(1),
            paths: false,
        };
        assert_eq!(where_used(&c, &q).total, 1);
        let mut shared_field = field(11);
        shared_field.world_id = None;
        let pin = shared_field.pin();
        c.card_data.fields.insert(pin.clone(), shared_field);
        c.card_data.fields.get_mut(&f.pin()).unwrap().source = Some(pin.clone());
        let q = Query {
            target: Identity::Field {
                world_id: None,
                id: pin.field_id,
            },
            version: Some(1),
            paths: true,
        };
        assert_eq!(where_used(&c, &q).paths[0].edges.len(), 2);
    }
    #[test]
    fn repeated_native_acquisitions_are_preserved_and_foreign_scopes_excluded() {
        let mut c = capture();
        let p = insert(&mut c, card(2, 1), false);
        let from = DomainNodeKey {
            kind: DomainKind::Learnset,
            world_id: Some(id(1)),
            id: id(3),
            revision: 1,
        };
        for slot in 1..=2 {
            c.edges.push(InspectionEdge {
                from: from.clone(),
                to: DomainRef::Exact(native(&p)),
                kind: EdgeKind::ExactPin,
                site: EdgeSite::Acquisition {
                    slot,
                    record_id: id(slot),
                },
            });
        }
        let mut foreign = from;
        foreign.world_id = Some(id(99));
        c.edges.push(InspectionEdge {
            from: foreign,
            to: DomainRef::Exact(native(&p)),
            kind: EdgeKind::ExactPin,
            site: EdgeSite::Source,
        });
        assert_eq!(where_used(&c, &query(&p, false)).total, 2);
    }
    #[test]
    fn result_and_depth_bounds_report_truncation() {
        let mut c = capture();
        let p = insert(&mut c, card(2, 1), false);
        for n in 3..303 {
            let mut r = card(n, 1);
            reference(&mut r, &p, 1000);
            insert(&mut c, r, true);
        }
        for paths in [false, true] {
            let f = where_used(&c, &query(&p, paths));
            assert_eq!(f.total, 300);
            assert!(f.truncated);
            assert!(!f.complete);
            assert_eq!(if paths { f.paths.len() } else { f.direct.len() }, 256);
        }
        let mut c = capture();
        let p = insert(&mut c, card(2, 1), false);
        let mut last = p.clone();
        for n in 3..38 {
            let mut r = card(n, 1);
            reference(&mut r, &last, 1000);
            last = insert(&mut c, r, n == 37);
        }
        let f = where_used(&c, &query(&p, true));
        assert!(f.paths.is_empty());
        assert!(f.truncated);
        assert!(!f.complete);
    }
}
