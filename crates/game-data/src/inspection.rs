//! Public, materialized inspection input. No account, audit, storage or URL data.
use crate::{
    authoring::ContentFields,
    charts::ChartRecord,
    learning::{LearnsetRecord, MovesetRecord},
    moves::{MoveAttributeDefinition, MoveRecord},
    rulesets::RulesetRecord,
    stats::{BaseStats, StatDefinition, StatRoster},
    types::{TypeRecord, TypingState, WorldTypeRecord},
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, str::FromStr};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum DomainKind {
    World,
    Species,
    Form,
    Variant,
    Type,
    Stat,
    StatRoster,
    MoveAttribute,
    Move,
    Chart,
    Ruleset,
    Learnset,
    Preset,
}
impl DomainKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::World => "world",
            Self::Species => "species",
            Self::Form => "form",
            Self::Variant => "variant",
            Self::Type => "type",
            Self::Stat => "stat",
            Self::StatRoster => "stat-roster",
            Self::MoveAttribute => "move-attribute",
            Self::Move => "move",
            Self::Chart => "chart",
            Self::Ruleset => "ruleset",
            Self::Learnset => "learnset",
            Self::Preset => "preset",
        }
    }
}
impl FromStr for DomainKind {
    type Err = &'static str;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "world" => Ok(Self::World),
            "species" => Ok(Self::Species),
            "form" => Ok(Self::Form),
            "variant" => Ok(Self::Variant),
            "type" => Ok(Self::Type),
            "stat" => Ok(Self::Stat),
            "stat-roster" => Ok(Self::StatRoster),
            "move-attribute" => Ok(Self::MoveAttribute),
            "move" => Ok(Self::Move),
            "chart" => Ok(Self::Chart),
            "ruleset" => Ok(Self::Ruleset),
            "learnset" => Ok(Self::Learnset),
            "preset" => Ok(Self::Preset),
            _ => Err("Unknown inspection object kind."),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DomainIdentity {
    pub kind: DomainKind,
    pub world_id: Option<String>,
    pub id: String,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DomainNodeKey {
    pub kind: DomainKind,
    pub world_id: Option<String>,
    pub id: String,
    pub revision: i64,
}
impl DomainNodeKey {
    pub fn identity(&self) -> DomainIdentity {
        DomainIdentity {
            kind: self.kind,
            world_id: self.world_id.clone(),
            id: self.id.clone(),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum RootRole {
    Active,
    Archived,
    Dependency,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum EdgeKind {
    Ownership,
    Membership,
    ExactPin,
    Provenance,
    Selection,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum DomainRef {
    Exact(DomainNodeKey),
    Identity(DomainIdentity),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum MappingKind {
    ChartType,
    MoveAttribute,
    LearnsetMove,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum MappingEndpoint {
    Source,
    Local,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum EdgeSite {
    WorldMember {
        kind: DomainKind,
    },
    Owner,
    Source,
    Species,
    DefaultForm,
    Typing {
        slot: usize,
    },
    StatValue {
        slot: usize,
    },
    StatRoster,
    RosterEntry {
        slot: usize,
    },
    ChartRoster {
        slot: usize,
    },
    Mapping {
        kind: MappingKind,
        slot: usize,
        endpoint: MappingEndpoint,
    },
    Attribute {
        slot: usize,
    },
    Expectation {
        slot: usize,
    },
    Acquisition {
        slot: usize,
        record_id: String,
    },
    PresetLearnset,
    PresetRuleset,
    PresetSelection {
        slot: usize,
        record_id: String,
        learnset: DomainNodeKey,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InspectionEdge {
    pub from: DomainNodeKey,
    pub to: DomainRef,
    pub kind: EdgeKind,
    pub site: EdgeSite,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicWorldContent {
    pub id: String,
    pub fields: ContentFields,
    pub revision: i64,
    pub archived_at: Option<i64>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicSpeciesContent {
    pub id: String,
    pub fields: ContentFields,
    pub revision: i64,
    pub default_form_id: String,
    pub archived_at: Option<i64>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicFormContent {
    pub id: String,
    pub species_id: String,
    pub species_version: i64,
    pub fields: ContentFields,
    pub revision: i64,
    pub is_default: bool,
    pub archived_at: Option<i64>,
    pub typing: TypingState,
    pub stats: BaseStats,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InspectionFormPin {
    pub form_id: String,
    pub version: i64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicVariantContent {
    pub id: String,
    pub world_id: String,
    pub source: Option<InspectionFormPin>,
    pub fields: ContentFields,
    pub revision: i64,
    pub archived_at: Option<i64>,
    pub typing: TypingState,
    pub stats: BaseStats,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum InspectionPayload {
    World(PublicWorldContent),
    Species(PublicSpeciesContent),
    Form(PublicFormContent),
    Variant(PublicVariantContent),
    Type(TypeRecord),
    WorldType(WorldTypeRecord),
    Stat(StatDefinition),
    StatRoster(StatRoster),
    MoveAttribute(MoveAttributeDefinition),
    Move(MoveRecord),
    Chart(ChartRecord),
    Ruleset(RulesetRecord),
    Learnset(LearnsetRecord),
    Preset(MovesetRecord),
}
impl InspectionPayload {
    pub fn fields(&self) -> Option<&ContentFields> {
        match self {
            Self::World(r) => Some(&r.fields),
            Self::Species(r) => Some(&r.fields),
            Self::Form(r) => Some(&r.fields),
            Self::Variant(r) => Some(&r.fields),
            Self::Type(r) => Some(&r.fields),
            Self::WorldType(r) => Some(&r.fields),
            Self::Stat(r) => Some(&r.fields),
            Self::MoveAttribute(r) => Some(&r.fields),
            Self::Move(r) => Some(&r.fields),
            Self::Chart(r) => Some(&r.fields),
            Self::Ruleset(r) => Some(&r.fields),
            Self::Preset(r) => Some(&r.fields),
            Self::StatRoster(_) | Self::Learnset(_) => None,
        }
    }
    pub fn archived_at(&self) -> Option<i64> {
        match self {
            Self::World(r) => r.archived_at,
            Self::Species(r) => r.archived_at,
            Self::Form(r) => r.archived_at,
            Self::Variant(r) => r.archived_at,
            Self::Type(r) => r.archived_at,
            Self::WorldType(r) => r.archived_at,
            Self::Stat(r) => r.archived_at,
            Self::MoveAttribute(r) => r.archived_at,
            Self::Move(r) => r.archived_at,
            Self::Chart(r) => r.archived_at,
            Self::Preset(r) => r.archived_at,
            Self::StatRoster(_) | Self::Ruleset(_) | Self::Learnset(_) => None,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HeadObservation {
    pub revision: i64,
    pub archived_at: Option<i64>,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InventoryAbsence {
    pub kind: DomainKind,
    pub owner_id: Option<String>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum CaptureGapKind {
    Integrity,
    Budget,
    Inventory,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureGap {
    pub kind: CaptureGapKind,
    pub subject: Option<DomainNodeKey>,
    pub message: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapturedWorld {
    pub contract_version: u32,
    pub world: PublicWorldContent,
    pub roots: BTreeMap<DomainNodeKey, RootRole>,
    pub nodes: BTreeMap<DomainNodeKey, InspectionPayload>,
    pub edges: Vec<InspectionEdge>,
    pub heads: BTreeMap<DomainIdentity, HeadObservation>,
    pub absences: Vec<InventoryAbsence>,
    pub gaps: Vec<CaptureGap>,
}
impl CapturedWorld {
    pub fn is_complete(&self) -> bool {
        self.gaps.is_empty()
    }
    pub fn world_key(&self) -> DomainNodeKey {
        DomainNodeKey {
            kind: DomainKind::World,
            world_id: None,
            id: self.world.id.clone(),
            revision: self.world.revision,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InspectionLimits {
    pub max_roots: usize,
    pub max_nodes: usize,
    pub max_edges: usize,
    pub max_snapshot_bytes: usize,
    pub max_total_snapshot_bytes: usize,
    pub deadline_ms: u64,
}
impl Default for InspectionLimits {
    fn default() -> Self {
        Self {
            max_roots: 2048,
            max_nodes: 8192,
            max_edges: 65536,
            max_snapshot_bytes: 1024 * 1024,
            max_total_snapshot_bytes: 16 * 1024 * 1024,
            deadline_ms: 10_000,
        }
    }
}
