//! Sanitized same-snapshot record evidence; maps are hashed as sorted vectors.
use crate::{
    cards::CardPin,
    relations::{CardRelationRecord, CollectionDefinition, CollectionPin, LayoutPin, LayoutRecord},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapturedRecords {
    pub layouts: BTreeMap<LayoutPin, LayoutRecord>,
    pub collections: BTreeMap<CollectionPin, CollectionDefinition>,
    pub relations: BTreeMap<CardPin, CardRelationRecord>,
    pub layout_roots: BTreeSet<LayoutPin>,
    pub collection_roots: BTreeSet<CollectionPin>,
    pub layout_heads: Vec<LayoutPin>,
    pub collection_heads: Vec<CollectionPin>,
}

/// Explicit record targets that also have a legacy typed identity. No label/name
/// inference and no conversion of standalone cards or reusable fields into Stats.
pub fn referenced_typed<'a>(
    records: impl IntoIterator<Item = &'a CardRelationRecord>,
) -> BTreeSet<crate::inspection::DomainIdentity> {
    use crate::{
        inspection::{DomainIdentity, DomainKind},
        relations::{CollectionValue, DefinitionPin, RecordValue},
    };
    let mut result = BTreeSet::new();
    for record in records {
        for a in record.relations.collections.iter().flatten() {
            if let CollectionValue::Entries(es) = &a.value {
                for e in es {
                    for cell in &e.cells {
                        if let RecordValue::Reference(pin) = &cell.value {
                            match pin {
                                DefinitionPin::Card(pin) => {
                                    if let Some(key) = crate::card_package::native_key(pin) {
                                        result.insert(key.identity());
                                    }
                                }
                                DefinitionPin::Stat {
                                    world_id, stat_id, ..
                                } => {
                                    result.insert(DomainIdentity {
                                        kind: DomainKind::Stat,
                                        world_id: world_id.clone(),
                                        id: stat_id.clone(),
                                    });
                                }
                                DefinitionPin::Field(_) => {}
                            }
                        }
                    }
                }
            }
        }
    }
    result
}
