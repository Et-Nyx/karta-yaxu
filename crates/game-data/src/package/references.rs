use super::*;
use crate::inspection::{
    DomainRef, EdgeKind, EdgeSite, InspectionEdge, MappingEndpoint, MappingKind,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct References {
    pub exact: Vec<DomainNodeKey>,
    pub identities: Vec<DomainIdentity>,
}

pub(super) fn key(kind: DomainKind, world: Option<&str>, id: &str, revision: i64) -> DomainNodeKey {
    DomainNodeKey {
        kind,
        world_id: world.map(str::to_owned),
        id: id.into(),
        revision,
    }
}

pub fn payload_key(payload: &InspectionPayload) -> DomainNodeKey {
    use DomainKind as K;
    use InspectionPayload as P;
    match payload {
        P::World(r) => key(K::World, None, &r.id, r.revision),
        P::Species(r) => key(K::Species, None, &r.id, r.revision),
        P::Form(r) => key(K::Form, None, &r.id, r.revision),
        P::Variant(r) => key(K::Variant, Some(&r.world_id), &r.id, r.revision),
        P::Type(r) => key(K::Type, None, &r.id, r.revision),
        P::WorldType(r) => key(K::Type, Some(&r.world_id), &r.id, r.revision),
        P::Stat(r) => key(K::Stat, r.world_id.as_deref(), &r.id, r.revision),
        P::StatRoster(r) => key(K::StatRoster, Some(&r.world_id), &r.id, r.revision),
        P::MoveAttribute(r) => key(K::MoveAttribute, r.world_id.as_deref(), &r.id, r.revision),
        P::Move(r) => key(K::Move, r.world_id.as_deref(), &r.id, r.revision),
        P::Chart(r) => key(K::Chart, r.world_id.as_deref(), &r.id, r.revision),
        P::Ruleset(r) => key(K::Ruleset, Some(&r.world_id), &r.id, r.revision),
        P::Learnset(r) => key(K::Learnset, r.world_id.as_deref(), &r.id, r.revision),
        P::Preset(r) => key(K::Preset, Some(&r.world_id), &r.id, r.revision),
    }
}

/// Direct required references, retaining occurrence order and duplicates. Source
/// mapping endpoints are context-relative identities within the exact source
/// aggregate, not global identity bindings. Their records are reached through
/// that aggregate's closure and checked by the cross-record validator.
pub fn references(payload: &InspectionPayload) -> Result<References, String> {
    validation::payload_occurrences(payload)?;
    validation::intrinsic(payload)?;
    let mut result = References {
        exact: vec![],
        identities: vec![],
    };
    for edge in edges(payload) {
        match edge.to {
            DomainRef::Exact(key) => result.exact.push(key),
            DomainRef::Identity(identity) => result.identities.push(identity),
        }
    }
    Ok(result)
}

pub(super) fn edges(payload: &InspectionPayload) -> Vec<InspectionEdge> {
    use DomainKind as K;
    use InspectionPayload as P;
    let from = payload_key(payload);
    let w = from.world_id.as_deref();
    let mut result = vec![];
    let mut add = |kind, world: Option<&str>, id: &str, revision, site, relation| {
        result.push(InspectionEdge {
            from: from.clone(),
            to: DomainRef::Exact(key(kind, world, id, revision)),
            kind: relation,
            site,
        });
    };
    let (typing, stats) = match payload {
        P::Form(r) => {
            add(
                K::Species,
                None,
                &r.species_id,
                r.species_version,
                EdgeSite::Species,
                EdgeKind::ExactPin,
            );
            (Some(&r.typing), Some(&r.stats))
        }
        P::Variant(r) => {
            if let Some(p) = &r.source {
                add(
                    K::Form,
                    None,
                    &p.form_id,
                    p.version,
                    EdgeSite::Source,
                    EdgeKind::Provenance,
                );
            }
            (Some(&r.typing), Some(&r.stats))
        }
        _ => (None, None),
    };
    if let Some(typing) = typing {
        for (i, a) in typing.assignments.iter().flatten().enumerate() {
            add(
                K::Type,
                w,
                &a.pin.type_id,
                a.pin.version,
                EdgeSite::Typing { slot: i + 1 },
                EdgeKind::ExactPin,
            );
        }
    }
    if let Some(stats) = stats {
        if let Some(p) = &stats.roster {
            add(
                K::StatRoster,
                w,
                &p.roster_id,
                p.version,
                EdgeSite::StatRoster,
                EdgeKind::ExactPin,
            );
        }
        for (i, a) in stats.assignments.iter().flatten().enumerate() {
            add(
                K::Stat,
                w,
                &a.definition.id,
                a.definition.revision,
                EdgeSite::StatValue { slot: i + 1 },
                EdgeKind::ExactPin,
            );
        }
    }
    match payload {
        P::WorldType(r) => {
            if let Some(p) = &r.source {
                add(
                    K::Type,
                    None,
                    &p.type_id,
                    p.version,
                    EdgeSite::Source,
                    EdgeKind::Provenance,
                );
            }
        }
        P::Stat(r) => {
            if let Some(p) = &r.source {
                add(
                    K::Stat,
                    None,
                    &p.stat_id,
                    p.version,
                    EdgeSite::Source,
                    EdgeKind::Provenance,
                );
            }
        }
        P::MoveAttribute(r) => {
            if let Some(p) = &r.source {
                add(
                    K::MoveAttribute,
                    None,
                    &p.attribute_id,
                    p.version,
                    EdgeSite::Source,
                    EdgeKind::Provenance,
                );
            }
        }
        P::StatRoster(r) => {
            for (i, a) in r.entries.iter().enumerate() {
                add(
                    K::Stat,
                    w,
                    &a.id,
                    a.revision,
                    EdgeSite::RosterEntry { slot: i + 1 },
                    EdgeKind::ExactPin,
                );
            }
        }
        P::Move(r) => {
            if let Some(p) = &r.source {
                add(
                    K::Move,
                    None,
                    &p.move_id,
                    p.version,
                    EdgeSite::Source,
                    EdgeKind::Provenance,
                );
            }
            for (i, a) in r.attributes.iter().flatten().enumerate() {
                add(
                    K::MoveAttribute,
                    w,
                    &a.definition.id,
                    a.definition.revision,
                    EdgeSite::Attribute { slot: i + 1 },
                    EdgeKind::ExactPin,
                );
            }
            for (i, m) in r.mappings.iter().enumerate() {
                add(
                    K::MoveAttribute,
                    w,
                    &m.local_attribute.attribute_id,
                    m.local_attribute.version,
                    EdgeSite::Mapping {
                        kind: MappingKind::MoveAttribute,
                        slot: i + 1,
                        endpoint: MappingEndpoint::Local,
                    },
                    EdgeKind::ExactPin,
                );
            }
        }
        P::Chart(r) => {
            if let Some(p) = &r.source {
                add(
                    K::Chart,
                    None,
                    &p.chart_id,
                    p.version,
                    EdgeSite::Source,
                    EdgeKind::Provenance,
                );
            }
            for (i, a) in r.roster.iter().enumerate() {
                add(
                    K::Type,
                    w,
                    &a.pin.type_id,
                    a.pin.version,
                    EdgeSite::ChartRoster { slot: i + 1 },
                    EdgeKind::ExactPin,
                );
            }
        }
        P::Ruleset(r) => {
            if let Some(p) = &r.stat_roster {
                add(
                    K::StatRoster,
                    w,
                    &p.roster_id,
                    p.version,
                    EdgeSite::StatRoster,
                    EdgeKind::ExactPin,
                );
            }
            if let Some(p) = &r.chart {
                add(
                    K::Chart,
                    w,
                    &p.chart_id,
                    p.version,
                    EdgeSite::Source,
                    EdgeKind::ExactPin,
                );
            }
            for (i, a) in r.expectations.iter().flatten().enumerate() {
                add(
                    K::MoveAttribute,
                    w,
                    &a.definition.id,
                    a.definition.revision,
                    EdgeSite::Expectation { slot: i + 1 },
                    EdgeKind::ExactPin,
                );
            }
        }
        P::Learnset(r) => {
            if let Some(p) = &r.source {
                add(
                    K::Learnset,
                    None,
                    &p.learnset_id,
                    p.version,
                    EdgeSite::Source,
                    EdgeKind::Provenance,
                );
            }
            for (i, a) in r.records.iter().flatten().enumerate() {
                add(
                    K::Move,
                    w,
                    &a.move_record.id,
                    a.move_record.revision,
                    EdgeSite::Acquisition {
                        slot: i + 1,
                        record_id: a.id.clone(),
                    },
                    EdgeKind::ExactPin,
                );
            }
            for (i, m) in r.mappings.iter().enumerate() {
                add(
                    K::Move,
                    w,
                    &m.local_move.move_id,
                    m.local_move.version,
                    EdgeSite::Mapping {
                        kind: MappingKind::LearnsetMove,
                        slot: i + 1,
                        endpoint: MappingEndpoint::Local,
                    },
                    EdgeKind::ExactPin,
                );
            }
        }
        P::Preset(r) => {
            add(
                K::Learnset,
                w,
                &r.learnset.learnset_id,
                r.learnset.version,
                EdgeSite::PresetLearnset,
                EdgeKind::ExactPin,
            );
            add(
                K::Ruleset,
                w,
                &r.ruleset.ruleset_id,
                r.ruleset.version,
                EdgeSite::PresetRuleset,
                EdgeKind::ExactPin,
            );
        }
        _ => {}
    }
    let identity = match payload {
        P::Species(r) => Some((
            K::Form,
            None,
            r.default_form_id.as_str(),
            EdgeSite::DefaultForm,
        )),
        P::Learnset(r) => r
            .form_id
            .as_deref()
            .map(|id| (K::Form, None, id, EdgeSite::Owner))
            .or_else(|| {
                r.variant_id
                    .as_deref()
                    .map(|id| (K::Variant, w, id, EdgeSite::Owner))
            }),
        P::Preset(r) => Some((K::Variant, w, r.variant_id.as_str(), EdgeSite::Owner)),
        _ => None,
    };
    if let Some((kind, world, id, site)) = identity {
        result.push(InspectionEdge {
            from: from.clone(),
            to: DomainRef::Identity(key(kind, world, id, 1).identity()),
            kind: EdgeKind::Ownership,
            site,
        });
    }
    result
}
