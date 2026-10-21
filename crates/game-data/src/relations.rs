//! Flat reusable records. Relationships are inert, exact, and occurrence preserving.
use crate::{
    authoring::{ContentFields, FieldErrors},
    card_fields::{CardField, FieldDefinition, FieldPin, FieldValue, invalid},
    cards::{CardIdentity, CardPin},
    stats::Decimal,
    types::valid_id,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

fn pin_valid(w: &Option<String>, id: &str, version: i64) -> Result<(), FieldErrors> {
    if !valid_id(id) || version <= 0 || w.as_ref().is_some_and(|w| !valid_id(w)) {
        return Err(invalid(
            "Choose an exact qualified record definition revision.",
        ));
    }
    Ok(())
}
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayoutPin {
    pub world_id: Option<String>,
    pub layout_id: String,
    pub version: i64,
}
impl LayoutPin {
    pub fn validate(&self) -> Result<(), FieldErrors> {
        pin_valid(&self.world_id, &self.layout_id, self.version)
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CollectionPin {
    pub world_id: Option<String>,
    pub collection_id: String,
    pub version: i64,
}
impl CollectionPin {
    pub fn validate(&self) -> Result<(), FieldErrors> {
        pin_valid(&self.world_id, &self.collection_id, self.version)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ReferenceKind {
    Card,
    Stat,
    Field,
}
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "pin",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum DefinitionPin {
    Card(CardPin),
    Stat {
        world_id: Option<String>,
        stat_id: String,
        version: i64,
    },
    Field(FieldPin),
}
impl DefinitionPin {
    pub fn kind(&self) -> ReferenceKind {
        match self {
            Self::Card(_) => ReferenceKind::Card,
            Self::Stat { .. } => ReferenceKind::Stat,
            Self::Field(_) => ReferenceKind::Field,
        }
    }
    pub fn world_id(&self) -> Option<&str> {
        match self {
            Self::Card(p) => p.identity.world_id.as_deref(),
            Self::Stat { world_id, .. } => world_id.as_deref(),
            Self::Field(p) => p.world_id.as_deref(),
        }
    }
    pub fn validate(&self) -> Result<(), FieldErrors> {
        match self {
            Self::Card(p) => p.validate(),
            Self::Stat {
                world_id,
                stat_id,
                version,
            } => pin_valid(world_id, stat_id, *version),
            Self::Field(p) => p.validate(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "definition",
    rename_all = "snake_case",
    deny_unknown_fields
)]
// Most authored columns embed an exact scalar definition. Keeping that evidence
// inline avoids a heap allocation per column in the bounded persisted snapshot.
#[allow(clippy::large_enum_variant)]
pub enum ColumnKind {
    Field(FieldDefinition),
    Reference { allowed: Vec<ReferenceKind> },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordColumn {
    pub id: String,
    pub label: String,
    pub kind: ColumnKind,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayoutRecord {
    pub id: String,
    pub world_id: Option<String>,
    pub source: Option<LayoutPin>,
    pub fields: ContentFields,
    pub revision: i64,
    pub archived_at: Option<i64>,
    pub columns: Vec<RecordColumn>,
}
impl LayoutRecord {
    pub fn pin(&self) -> LayoutPin {
        LayoutPin {
            world_id: self.world_id.clone(),
            layout_id: self.id.clone(),
            version: self.revision,
        }
    }
    pub fn validate(&self) -> Result<(), FieldErrors> {
        self.pin().validate()?;
        metadata(&self.fields)?;
        if let Some(s) = &self.source {
            s.validate()?;
            source_scope(&self.world_id, &s.world_id)?;
        }
        let mut ids = HashSet::new();
        for c in &self.columns {
            if !valid_id(&c.id)
                || !ids.insert(&c.id)
                || c.label.is_empty()
                || c.label.trim() != c.label
                || c.label.chars().count() > 120
                || c.label.chars().any(char::is_control)
            {
                return Err(invalid(
                    "Columns need distinct stable IDs and normalized labels (120 characters).",
                ));
            }
            match &c.kind {
                ColumnKind::Field(d) => {
                    d.validate()?;
                    if d.world_id != self.world_id {
                        return Err(invalid(
                            "Layout scalar fields must have exactly the layout scope.",
                        ));
                    }
                }
                ColumnKind::Reference { allowed } => {
                    let set: HashSet<_> = allowed.iter().collect();
                    if allowed.is_empty() || set.len() != allowed.len() {
                        return Err(invalid(
                            "Declare a nonempty distinct reference-kind allowlist.",
                        ));
                    }
                }
            }
        }
        Ok(())
    }
}
fn metadata(f: &ContentFields) -> Result<(), FieldErrors> {
    if f.normalized().ok().as_ref() != Some(f) {
        return Err(invalid("Use normalized record metadata."));
    }
    Ok(())
}
fn source_scope(w: &Option<String>, s: &Option<String>) -> Result<(), FieldErrors> {
    if w.is_none() || s.is_some() {
        return Err(invalid("Only local definitions may pin shared sources."));
    }
    Ok(())
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CollectionDefinition {
    pub id: String,
    pub world_id: Option<String>,
    pub source: Option<CollectionPin>,
    pub fields: ContentFields,
    pub revision: i64,
    pub archived_at: Option<i64>,
    pub layout: LayoutRecord,
}
impl CollectionDefinition {
    pub fn pin(&self) -> CollectionPin {
        CollectionPin {
            world_id: self.world_id.clone(),
            collection_id: self.id.clone(),
            version: self.revision,
        }
    }
    pub fn validate(&self) -> Result<(), FieldErrors> {
        self.pin().validate()?;
        metadata(&self.fields)?;
        self.layout.validate()?;
        if self.world_id != self.layout.world_id {
            return Err(invalid(
                "Collections pin a layout in exactly their own scope.",
            ));
        }
        if let Some(s) = &self.source {
            s.validate()?;
            source_scope(&self.world_id, &s.world_id)?;
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "state",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum RecordValue {
    Missing,
    Unknown,
    NotApplicable,
    Number(Decimal),
    Text(String),
    Boolean(bool),
    Choice(String),
    Reference(DefinitionPin),
}
impl RecordValue {
    pub fn validate(&self, column: &RecordColumn, world: Option<&str>) -> Result<(), FieldErrors> {
        if let Self::Reference(p) = self {
            p.validate()?;
            if p.world_id().is_some() && p.world_id() != world {
                return Err(invalid("References must be shared or in this same world."));
            }
        }
        if matches!(self, Self::Missing | Self::Unknown | Self::NotApplicable) {
            return Ok(());
        }
        match &column.kind {
            ColumnKind::Reference { allowed } => {
                if let Self::Reference(p) = self
                    && allowed.contains(&p.kind())
                {
                    Ok(())
                } else {
                    Err(invalid("Reference kind is not allowed by this column."))
                }
            }
            ColumnKind::Field(d) => {
                let value = match self {
                    Self::Number(v) => FieldValue::Number(*v),
                    Self::Text(v) => FieldValue::Text(v.clone()),
                    Self::Boolean(v) => FieldValue::Boolean(*v),
                    Self::Choice(v) => FieldValue::Choice(v.clone()),
                    Self::Reference(DefinitionPin::Card(p)) => FieldValue::CardReference(p.clone()),
                    _ => return Err(invalid("Value does not match the scalar column.")),
                };
                CardField {
                    definition: d.clone(),
                    value,
                }
                .validate()
            }
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordCell {
    pub column_id: String,
    pub value: RecordValue,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordEntry {
    pub id: String,
    pub cells: Vec<RecordCell>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "state",
    content = "entries",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum CollectionValue {
    Missing,
    Unknown,
    NotApplicable,
    Entries(Vec<RecordEntry>),
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordCollection {
    pub definition: CollectionDefinition,
    pub value: CollectionValue,
}
impl RecordCollection {
    pub fn validate(&self) -> Result<(), FieldErrors> {
        self.definition.validate()?;
        if let CollectionValue::Entries(entries) = &self.value {
            let mut ids = HashSet::new();
            for e in entries {
                if !valid_id(&e.id) || !ids.insert(&e.id) {
                    return Err(invalid(
                        "Entries need distinct stable IDs within each card collection.",
                    ));
                }
                let mut columns = HashSet::new();
                for cell in &e.cells {
                    let col = self
                        .definition
                        .layout
                        .columns
                        .iter()
                        .find(|c| c.id == cell.column_id)
                        .ok_or_else(|| {
                            invalid("Cell column is not declared by this exact layout.")
                        })?;
                    if !columns.insert(&cell.column_id) {
                        return Err(invalid("An entry may have at most one cell per column."));
                    }
                    cell.value
                        .validate(col, self.definition.world_id.as_deref())?;
                }
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CardRelations {
    pub collections: Option<Vec<RecordCollection>>,
}
impl CardRelations {
    pub fn validate(&self, identity: &CardIdentity) -> Result<(), FieldErrors> {
        identity.validate()?;
        let mut ids = HashSet::new();
        for c in self.collections.iter().flatten() {
            c.validate()?;
            if c.definition.world_id != identity.world_id || !ids.insert(&c.definition.id) {
                return Err(invalid(
                    "Select distinct collections in exactly the card scope.",
                ));
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CardRelationRecord {
    pub card: CardPin,
    pub relations: CardRelations,
    pub legacy_unmodeled: bool,
}
impl CardRelationRecord {
    pub fn pin(&self) -> CardPin {
        self.card.clone()
    }
    pub fn validate(&self) -> Result<(), FieldErrors> {
        self.card.validate()?;
        self.relations.validate(&self.card.identity)?;
        if self.legacy_unmodeled && self.relations != CardRelations::default() {
            return Err(invalid("Legacy relations must remain unmodeled."));
        }
        Ok(())
    }
}
