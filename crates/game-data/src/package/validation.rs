use super::{references::key, *};
use crate::{
    inspection::DomainRef,
    learning::LearnsetRecord,
    moves::{MoveAttributeDefinition, MoveRecord},
    stats::{BaseStats, StatDefinition},
    types::{TypingState, valid_id},
};
use std::collections::BTreeSet;

fn require(ok: bool, message: &str) -> Result<(), String> {
    if ok { Ok(()) } else { Err(message.into()) }
}
fn fields(errors: crate::authoring::FieldErrors) -> String {
    errors
        .into_iter()
        .map(|(k, v)| format!("{k}: {v}"))
        .collect::<Vec<_>>()
        .join("; ")
}
fn valid_key(k: &DomainNodeKey) -> bool {
    valid_id(&k.id) && k.revision > 0 && k.world_id.as_deref().is_none_or(valid_id)
}

// Count all authored collection occurrences, including repeated embedded
// snapshots, before validation builds indexes, edges or metadata copies.
pub(crate) fn payload_occurrences(payload: &InspectionPayload) -> Result<usize, String> {
    use InspectionPayload as P;
    fn add(total: &mut usize, n: usize) -> Result<(), String> {
        *total = total
            .checked_add(n)
            .ok_or("Package occurrence count overflow.")?;
        require(
            *total <= MAX_OCCURRENCES,
            "Package exceeds the 65536 occurrence limit.",
        )
    }
    fn move_size(r: &MoveRecord, total: &mut usize) -> Result<(), String> {
        add(total, 1 + usize::from(r.source.is_some()))?;
        add(total, r.attributes.as_ref().map_or(0, Vec::len))?;
        add(total, r.mappings.len().saturating_mul(2))
    }
    let mut total = 1;
    match payload {
        P::Form(r) => {
            add(
                &mut total,
                r.typing.assignments.as_ref().map_or(0, Vec::len),
            )?;
            add(&mut total, r.stats.assignments.as_ref().map_or(0, Vec::len))?;
        }
        P::Variant(r) => {
            add(
                &mut total,
                r.typing.assignments.as_ref().map_or(0, Vec::len),
            )?;
            add(&mut total, r.stats.assignments.as_ref().map_or(0, Vec::len))?;
        }
        P::StatRoster(r) => add(&mut total, r.entries.len())?,
        P::Move(r) => move_size(r, &mut total)?,
        P::Ruleset(r) => add(&mut total, r.expectations.as_ref().map_or(0, Vec::len))?,
        P::Chart(r) => {
            add(&mut total, r.roster.len())?;
            add(&mut total, r.cells.len())?;
            add(&mut total, r.mapping.len().saturating_mul(2))?;
        }
        P::Learnset(r) => {
            add(&mut total, r.records.as_ref().map_or(0, Vec::len))?;
            add(&mut total, r.mappings.len().saturating_mul(2))?;
            for acquisition in r.records.iter().flatten() {
                move_size(&acquisition.move_record, &mut total)?;
            }
        }
        P::Preset(r) => add(&mut total, r.entries.len())?,
        _ => {}
    }
    // Each payload has a small, fixed number of source/context/owner pins.
    add(&mut total, 4)?;
    Ok(total)
}

pub(super) fn preflight(content: &PackageContent) -> Result<(), String> {
    require(
        content.roots.len() <= MAX_ROOTS,
        "Package exceeds the 2048 root limit.",
    )?;
    require(
        content.records.len() <= MAX_RECORDS,
        "Package exceeds the 8192 record limit.",
    )?;
    require(
        content.bindings.len() <= MAX_RECORDS,
        "Package exceeds the identity binding limit.",
    )?;
    let mut total = content.roots.len() + content.bindings.len();
    for record in &content.records {
        total = total
            .checked_add(payload_occurrences(&record.payload)?)
            .ok_or("Package occurrence count overflow.")?;
        require(
            total <= MAX_OCCURRENCES,
            "Package exceeds the 65536 occurrence limit.",
        )?;
    }
    // Bounded serialization never allocates an unbounded output even when a
    // programmatic caller supplies oversized strings instead of parsed bytes.
    bounded_json(content)?;
    Ok(())
}

fn typing_intrinsic(typing: &TypingState) -> Result<(), String> {
    crate::types::validate_pins(typing.pins().as_deref()).map_err(fields)?;
    for assignment in typing.assignments.iter().flatten() {
        require(
            assignment.fields.normalized().as_ref() == Ok(&assignment.fields),
            "Typing metadata must be normalized.",
        )?;
    }
    Ok(())
}

pub(super) fn intrinsic(payload: &InspectionPayload) -> Result<(), String> {
    use InspectionPayload as P;
    let k = payload_key(payload);
    require(
        valid_key(&k),
        "Invalid canonical record identity, scope or revision.",
    )?;
    if let Some(f) = payload.fields() {
        require(
            f.normalized().as_ref() == Ok(f),
            "Record metadata must be normalized.",
        )?;
    }
    match payload {
        P::World(_) | P::Type(_) => Ok(()),
        P::Species(r) => require(
            valid_id(&r.default_form_id),
            "Invalid default Form identity.",
        ),
        P::Form(r) => {
            require(
                valid_id(&r.species_id) && r.species_version > 0,
                "Invalid exact Species pin.",
            )?;
            typing_intrinsic(&r.typing)?;
            r.stats.validate().map_err(fields)
        }
        P::Variant(r) => {
            require(
                r.source
                    .as_ref()
                    .is_none_or(|p| valid_id(&p.form_id) && p.version > 0),
                "Invalid exact source Form pin.",
            )?;
            typing_intrinsic(&r.typing)?;
            r.stats.validate().map_err(fields)
        }
        P::WorldType(r) => require(
            r.source
                .as_ref()
                .is_none_or(|p| valid_id(&p.type_id) && p.version > 0),
            "Invalid exact source Type pin.",
        ),
        P::Stat(r) => r.validate().map_err(fields),
        P::StatRoster(r) => r.validate().map_err(fields),
        P::MoveAttribute(r) => r.validate().map_err(fields),
        P::Move(r) => r.validate().map_err(fields),
        P::Chart(r) => r.validate().map_err(fields),
        P::Ruleset(r) => r.validate().map_err(fields),
        P::Learnset(r) => r.validate().map_err(fields),
        P::Preset(r) => {
            r.validate().map_err(fields)?;
            require(
                matches!(r.validator_version, 1 | 2),
                "Unsupported saved preset validator policy.",
            )
        }
    }
}

struct Index<'a> {
    records: BTreeMap<DomainNodeKey, &'a InspectionPayload>,
    bindings: BTreeMap<DomainIdentity, DomainNodeKey>,
}
impl<'a> Index<'a> {
    fn get(&self, k: &DomainNodeKey) -> Result<&'a InspectionPayload, String> {
        self.records.get(k).copied().ok_or_else(|| {
            format!(
                "Missing exact {} record {} revision {} in its declared scope.",
                k.kind.as_str(),
                k.id,
                k.revision
            )
        })
    }
    fn exact(
        &self,
        kind: DomainKind,
        world: Option<&str>,
        id: &str,
        revision: i64,
    ) -> Result<&'a InspectionPayload, String> {
        self.get(&key(kind, world, id, revision))
    }
    fn bound(
        &self,
        kind: DomainKind,
        world: Option<&str>,
        id: &str,
    ) -> Result<&'a InspectionPayload, String> {
        let identity = key(kind, world, id, 1).identity();
        let target = self
            .bindings
            .get(&identity)
            .ok_or("Missing identity binding for owner or default Form.")?;
        self.get(target)
    }
    fn stat(&self, r: &StatDefinition) -> Result<(), String> {
        require(
            matches!(self.exact(DomainKind::Stat, r.world_id.as_deref(), &r.id, r.revision)?,
            InspectionPayload::Stat(exact) if exact == r),
            "Embedded Stat snapshot differs from its exact record.",
        )
    }
    fn attribute(&self, r: &MoveAttributeDefinition) -> Result<(), String> {
        require(
            matches!(self.exact(DomainKind::MoveAttribute, r.world_id.as_deref(), &r.id, r.revision)?,
            InspectionPayload::MoveAttribute(exact) if exact == r),
            "Embedded attribute snapshot differs from its exact record.",
        )
    }
    fn move_record(&self, r: &MoveRecord) -> Result<(), String> {
        require(
            matches!(self.exact(DomainKind::Move, r.world_id.as_deref(), &r.id, r.revision)?,
            InspectionPayload::Move(exact) if exact == r),
            "Embedded Move snapshot differs from its exact record.",
        )
    }
    fn typing(&self, world: Option<&str>, typing: &TypingState) -> Result<(), String> {
        for a in typing.assignments.iter().flatten() {
            let record = self.exact(DomainKind::Type, world, &a.pin.type_id, a.pin.version)?;
            require(
                record.fields() == Some(&a.fields),
                "Embedded Type metadata differs from its exact record.",
            )?;
        }
        Ok(())
    }
    fn stats(&self, world: Option<&str>, stats: &BaseStats) -> Result<(), String> {
        for value in stats.assignments.iter().flatten() {
            require(
                value.definition.world_id.as_deref() == world,
                "Stat assignments have the wrong scope.",
            )?;
            self.stat(&value.definition)?;
        }
        match (world, &stats.roster, &stats.assignments) {
            (None, None, _) | (Some(_), None, None) => Ok(()),
            (Some(world), Some(pin), Some(values)) => {
                let InspectionPayload::StatRoster(roster) = self.exact(
                    DomainKind::StatRoster,
                    Some(world),
                    &pin.roster_id,
                    pin.version,
                )?
                else {
                    return Err("Expected an exact Stat roster.".into());
                };
                require(
                    roster.entries.len() == values.len()
                        && roster
                            .entries
                            .iter()
                            .zip(values)
                            .all(|(a, b)| a == &b.definition),
                    "Variant Stat assignments must match its complete exact roster in order.",
                )
            }
            _ => Err(
                "Shared Forms cannot pin a roster; modeled Variants require an exact local roster."
                    .into(),
            ),
        }
    }

    fn cross(&self, payload: &InspectionPayload) -> Result<(), String> {
        use DomainKind as K;
        use InspectionPayload as P;
        match payload {
            P::Species(r) => {
                require(
                    matches!(self.bound(K::Form, None, &r.default_form_id)?,
                    P::Form(form) if form.species_id == r.id && form.is_default),
                    "Default Form must belong to its Species and be default.",
                )?;
            }
            P::Form(r) => {
                let P::Species(species) =
                    self.exact(K::Species, None, &r.species_id, r.species_version)?
                else {
                    return Err("Expected an exact Species.".into());
                };
                require(
                    r.is_default == (species.default_form_id == r.id),
                    "Form default status disagrees with its exact Species.",
                )?;
                self.typing(None, &r.typing)?;
                self.stats(None, &r.stats)?;
            }
            P::Variant(r) => {
                self.typing(Some(&r.world_id), &r.typing)?;
                self.stats(Some(&r.world_id), &r.stats)?;
            }
            P::StatRoster(r) => {
                for entry in &r.entries {
                    self.stat(entry)?;
                }
            }
            P::Move(r) => {
                for a in r.attributes.iter().flatten() {
                    self.attribute(&a.definition)?;
                }
                if let Some(pin) = &r.source {
                    let P::Move(source) = self.exact(K::Move, None, &pin.move_id, pin.version)?
                    else {
                        return Err("Expected an exact shared source Move.".into());
                    };
                    let shared: BTreeSet<_> = source
                        .attributes
                        .iter()
                        .flatten()
                        .map(|a| &a.definition.id)
                        .collect();
                    for m in &r.mappings {
                        require(
                            shared.contains(&m.shared_attribute_id),
                            "Mapped attribute is absent from the exact source Move.",
                        )?;
                        let P::MoveAttribute(local) = self.exact(
                            K::MoveAttribute,
                            r.world_id.as_deref(),
                            &m.local_attribute.attribute_id,
                            m.local_attribute.version,
                        )?
                        else {
                            return Err("Expected a mapped local attribute.".into());
                        };
                        require(
                            local
                                .source
                                .as_ref()
                                .is_some_and(|p| p.attribute_id == m.shared_attribute_id),
                            "Mapped local attribute must explicitly source the shared attribute identity.",
                        )?;
                    }
                }
            }
            P::Ruleset(r) => {
                for a in r.expectations.iter().flatten() {
                    self.attribute(&a.definition)?;
                }
            }
            P::Chart(r) => {
                self.typing(
                    r.world_id.as_deref(),
                    &TypingState {
                        assignments: Some(r.roster.clone()),
                    },
                )?;
                if let Some(pin) = &r.source {
                    let P::Chart(source) =
                        self.exact(K::Chart, None, &pin.chart_id, pin.version)?
                    else {
                        return Err("Expected an exact shared source chart.".into());
                    };
                    for m in &r.mapping {
                        require(
                            source
                                .roster
                                .iter()
                                .any(|a| a.pin.type_id == m.source_type_id),
                            "Mapped Type is absent from the exact source chart roster.",
                        )?;
                        let assignment = r
                            .roster
                            .iter()
                            .find(|a| a.pin.type_id == m.local_type_id)
                            .ok_or("Mapped local Type is absent from the chart roster.")?;
                        let P::WorldType(local) = self.exact(
                            K::Type,
                            r.world_id.as_deref(),
                            &assignment.pin.type_id,
                            assignment.pin.version,
                        )?
                        else {
                            return Err("Expected an exact mapped local Type.".into());
                        };
                        require(
                            local
                                .source
                                .as_ref()
                                .is_some_and(|p| p.type_id == m.source_type_id),
                            "Mapped local Type must explicitly source the shared Type identity.",
                        )?;
                    }
                }
            }
            P::Learnset(r) => self.learnset(r)?,
            P::Preset(r) => {
                self.bound(K::Variant, Some(&r.world_id), &r.variant_id)?;
                let P::Learnset(learnset) = self.exact(
                    K::Learnset,
                    Some(&r.world_id),
                    &r.learnset.learnset_id,
                    r.learnset.version,
                )?
                else {
                    return Err("Expected an exact local Learnset.".into());
                };
                require(
                    learnset.variant_id.as_ref() == Some(&r.variant_id),
                    "Preset Learnset belongs to another Variant.",
                )?;
                let ids: BTreeSet<_> = learnset.records.iter().flatten().map(|a| &a.id).collect();
                require(
                    r.entries.iter().all(|id| ids.contains(id)),
                    "Preset selection is absent from its exact Learnset.",
                )?;
            }
            _ => {}
        }
        Ok(())
    }

    fn learnset(&self, r: &LearnsetRecord) -> Result<(), String> {
        use DomainKind as K;
        use InspectionPayload as P;
        if let Some(form) = &r.form_id {
            self.bound(K::Form, None, form)?;
        }
        if let Some(variant) = &r.variant_id {
            self.bound(K::Variant, r.world_id.as_deref(), variant)?;
        }
        for record in r.records.iter().flatten() {
            self.move_record(&record.move_record)?;
        }
        if let Some(pin) = &r.source {
            let P::Learnset(source) =
                self.exact(K::Learnset, None, &pin.learnset_id, pin.version)?
            else {
                return Err("Expected an exact shared source Learnset.".into());
            };
            let shared: BTreeSet<_> = source
                .records
                .iter()
                .flatten()
                .map(|a| &a.move_record.id)
                .collect();
            for m in &r.mappings {
                require(
                    shared.contains(&m.shared_move_id),
                    "Mapped Move is absent from the exact source Learnset.",
                )?;
                let P::Move(local) = self.exact(
                    K::Move,
                    r.world_id.as_deref(),
                    &m.local_move.move_id,
                    m.local_move.version,
                )?
                else {
                    return Err("Expected an exact mapped local Move.".into());
                };
                require(
                    local
                        .source
                        .as_ref()
                        .is_some_and(|p| p.move_id == m.shared_move_id),
                    "Mapped local Move must explicitly source the shared Move identity.",
                )?;
            }
        }
        // Explicit source Learnsets are independent of the Variant's source Form.
        Ok(())
    }
}

pub(super) fn validate(content: &PackageContent) -> Result<(), String> {
    validate_with_external_seeds(content, &[])
}

/// Additional exact seeds are internal to the v2 combined-closure validator.
/// Catalog-root rules, payload validation and binding checks remain unchanged.
pub(crate) fn validate_with_external_seeds(
    content: &PackageContent,
    external_seeds: &[DomainNodeKey],
) -> Result<(), String> {
    preflight(content)?;
    require(
        content.schema_version == SCHEMA_VERSION && content.root_policy == ROOT_POLICY,
        "Unsupported package schema or root policy.",
    )?;
    require(
        content.world.kind == DomainKind::World
            && content.world.world_id.is_none()
            && valid_key(&content.world),
        "Invalid world anchor key.",
    )?;
    require(
        content.roots.windows(2).all(|w| w[0] < w[1])
            && content.records.windows(2).all(|w| w[0].key < w[1].key)
            && content
                .bindings
                .windows(2)
                .all(|w| w[0].identity < w[1].identity),
        "Package roots, records and bindings must be uniquely canonical-sorted.",
    )?;
    let index = Index {
        records: content
            .records
            .iter()
            .map(|r| (r.key.clone(), &r.payload))
            .collect(),
        bindings: content
            .bindings
            .iter()
            .map(|b| (b.identity.clone(), b.target.clone()))
            .collect(),
    };
    let mut identities = BTreeMap::new();
    let mut singletons = BTreeMap::new();
    let mut learnset_owners = BTreeMap::new();
    for record in &content.records {
        intrinsic(&record.payload)?;
        require(
            record.key == payload_key(&record.payload),
            "Record key disagrees with its payload.",
        )?;
        require(
            record
                .key
                .world_id
                .as_ref()
                .is_none_or(|world| world == &content.world.id)
                && (record.key.kind != DomainKind::World || record.key == content.world),
            "Package includes content outside its world scope.",
        )?;
        if record.key.world_id.is_some()
            && matches!(
                record.key.kind,
                DomainKind::Chart | DomainKind::StatRoster | DomainKind::Ruleset
            )
            && let Some(previous) = singletons.insert(record.key.kind, &record.key.id)
        {
            require(
                previous == &record.key.id,
                "Multiple independent local singleton identities of one kind.",
            )?;
        }
        if let InspectionPayload::Learnset(r) = &record.payload {
            let owner = (
                r.world_id.as_deref(),
                r.form_id.as_deref(),
                r.variant_id.as_deref(),
            );
            if let Some(previous) = learnset_owners.insert(owner, &r.id) {
                require(
                    previous == &r.id,
                    "Multiple independent Learnsets for one owner.",
                )?;
            }
        }
        if let Some(previous) = identities.insert(record.key.identity(), &record.payload) {
            match (previous, &record.payload) {
                (InspectionPayload::Form(a), InspectionPayload::Form(b)) => require(
                    a.species_id == b.species_id && a.is_default == b.is_default,
                    "Form ownership changed between revisions.",
                )?,
                (InspectionPayload::Learnset(a), InspectionPayload::Learnset(b)) => require(
                    a.form_id == b.form_id && a.variant_id == b.variant_id,
                    "Learnset ownership changed between revisions.",
                )?,
                (InspectionPayload::Preset(a), InspectionPayload::Preset(b)) => require(
                    a.variant_id == b.variant_id,
                    "Preset ownership changed between revisions.",
                )?,
                _ => {}
            }
        }
    }
    for binding in &content.bindings {
        require(
            binding.identity == binding.target.identity(),
            "Identity binding target has another identity or scope.",
        )?;
        index.get(&binding.target)?;
    }
    validate_roots(content, &index)?;
    // Derive adjacency once. Never traverse producer-supplied membership edges.
    let mut adjacency = BTreeMap::new();
    let mut required_bindings = BTreeSet::new();
    let mut edge_count = 0_usize;
    for record in &content.records {
        let mut targets = vec![];
        for edge in references::edges(&record.payload) {
            edge_count += 1;
            require(
                edge_count <= MAX_OCCURRENCES,
                "Package exceeds the reference occurrence limit.",
            )?;
            let target = match edge.to {
                DomainRef::Exact(k) => k,
                DomainRef::Identity(identity) => {
                    required_bindings.insert(identity.clone());
                    index
                        .bindings
                        .get(&identity)
                        .ok_or("Missing required identity binding.")?
                        .clone()
                }
            };
            index.get(&target)?;
            targets.push(target);
        }
        adjacency.insert(record.key.clone(), targets);
        index.cross(&record.payload)?;
    }
    require(
        required_bindings.len() == index.bindings.len(),
        "Package contains unrelated identity bindings.",
    )?;
    let mut visited = BTreeSet::new();
    let mut pending: BTreeSet<_> = content.roots.iter().cloned().collect();
    pending.extend(external_seeds.iter().cloned());
    while let Some(k) = pending.pop_first() {
        if visited.insert(k.clone()) {
            for target in adjacency
                .get(&k)
                .ok_or("Missing root or dependency record.")?
            {
                if !visited.contains(target) {
                    pending.insert(target.clone());
                }
            }
        }
    }
    require(
        visited.len() == index.records.len(),
        "Package contains records outside its declared root closure.",
    )
}

fn validate_roots(content: &PackageContent, index: &Index<'_>) -> Result<(), String> {
    use DomainKind as K;
    use InspectionPayload as P;
    require(
        content.roots.contains(&content.world),
        "The world anchor must be a package root.",
    )?;
    let mut identities = BTreeSet::new();
    let mut singletons = BTreeSet::new();
    let mut learnset_owners = BTreeSet::new();
    let root_set: BTreeSet<_> = content.roots.iter().collect();
    for root in &content.roots {
        let payload = index.get(root)?;
        require(
            payload.archived_at().is_none(),
            "Archived records cannot be active catalog roots.",
        )?;
        require(
            identities.insert(root.identity()),
            "Multiple root revisions of one identity are not allowed.",
        )?;
        require(
            root == &content.world || root.world_id.as_ref() == Some(&content.world.id),
            "Only the world anchor and local catalog records can be roots.",
        )?;
        if matches!(root.kind, K::Chart | K::StatRoster | K::Ruleset) {
            require(
                singletons.insert(root.kind),
                "Multiple current singleton roots of one kind.",
            )?;
        }
        let owner = match payload {
            P::Learnset(r) => {
                let owner = r
                    .variant_id
                    .as_ref()
                    .ok_or("A root Learnset must belong to a local Variant.")?;
                require(
                    learnset_owners.insert(owner),
                    "Multiple root Learnsets for one Variant.",
                )?;
                Some(owner)
            }
            P::Preset(r) => Some(&r.variant_id),
            _ => None,
        };
        if let Some(owner) = owner {
            let identity = key(K::Variant, Some(&content.world.id), owner, 1).identity();
            let target = index
                .bindings
                .get(&identity)
                .ok_or("Missing root child owner binding.")?;
            require(
                root_set.contains(target) && target.kind == K::Variant,
                "Root Learnsets and Presets require their bound active Variant as a root.",
            )?;
        }
    }
    Ok(())
}
