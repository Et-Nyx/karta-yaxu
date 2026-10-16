//! Extensible numeric move data. Attribute names have no executable meaning.
pub use crate::stats::{Decimal, NumericConstraints};
use crate::{
    authoring::{ContentFields, FieldErrors},
    types::valid_id,
};
use serde::{Deserialize, Deserializer, Serialize, de};
use std::collections::HashSet;

pub(crate) fn invalid(field: &str, message: &str) -> FieldErrors {
    FieldErrors::from([(field.into(), message.into())])
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttributePin {
    pub attribute_id: String,
    pub version: i64,
}
impl AttributePin {
    pub fn validate(&self) -> Result<(), FieldErrors> {
        if !valid_id(&self.attribute_id) || self.version <= 0 {
            return Err(invalid(
                "attribute",
                "Choose an exact positive attribute revision.",
            ));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MovePin {
    pub move_id: String,
    pub version: i64,
}
impl MovePin {
    pub fn validate(&self) -> Result<(), FieldErrors> {
        if !valid_id(&self.move_id) || self.version <= 0 {
            return Err(invalid("move", "Choose an exact positive move revision."));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MoveAttributeDefinition {
    pub id: String,
    pub world_id: Option<String>,
    pub source: Option<AttributePin>,
    pub fields: ContentFields,
    pub unit: String,
    pub revision: i64,
    pub archived_at: Option<i64>,
    pub constraints: NumericConstraints,
}
impl MoveAttributeDefinition {
    pub fn pin(&self) -> AttributePin {
        AttributePin {
            attribute_id: self.id.clone(),
            version: self.revision,
        }
    }
    pub fn validate(&self) -> Result<(), FieldErrors> {
        self.pin().validate()?;
        validate_scope(&self.world_id, self.source.is_some())?;
        if let Some(p) = &self.source {
            p.validate()?;
        }
        validate_fields(&self.fields)?;
        if self.unit.trim() != self.unit
            || self.unit.len() > 100
            || self.unit.chars().any(char::is_control)
        {
            return Err(invalid(
                "unit",
                "Use at most 100 bytes of normalized descriptive unit text.",
            ));
        }
        self.constraints.validate()
    }
}
pub(crate) fn validate_fields(fields: &ContentFields) -> Result<(), FieldErrors> {
    if fields.normalized().ok().as_ref() != Some(fields) {
        return Err(invalid("fields", "Metadata must be valid normalized text."));
    }
    Ok(())
}
fn validate_scope(world: &Option<String>, source: bool) -> Result<(), FieldErrors> {
    if world.as_ref().is_some_and(|w| !valid_id(w)) || (world.is_none() && source) {
        return Err(invalid(
            "source",
            "Only local objects may pin a shared source.",
        ));
    }
    Ok(())
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(
    tag = "state",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum NumericValue {
    Known(Decimal),
    Unknown,
    NotApplicable,
}
impl<'de> Deserialize<'de> for NumericValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // Presence is significant: even an explicit null payload is invalid on a
        // payload-free state. The derived adjacent enum decoder accepts that null.
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            state: String,
            #[serde(default, deserialize_with = "present")]
            value: Option<Option<Decimal>>,
        }
        fn present<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Option<Decimal>>, D::Error> {
            Option::<Decimal>::deserialize(d).map(Some)
        }
        let wire = Wire::deserialize(deserializer)?;
        match (wire.state.as_str(), wire.value) {
            ("known", Some(Some(d))) => Ok(Self::Known(d)),
            ("unknown", None) => Ok(Self::Unknown),
            ("not_applicable", None) => Ok(Self::NotApplicable),
            _ => Err(de::Error::custom(
                "Use a canonical state; only known requires a decimal value payload.",
            )),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MoveAttributeValue {
    pub definition: MoveAttributeDefinition,
    pub value: NumericValue,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttributeMapping {
    pub shared_attribute_id: String,
    pub local_attribute: AttributePin,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MoveRecord {
    pub id: String,
    pub world_id: Option<String>,
    pub source: Option<MovePin>,
    pub fields: ContentFields,
    pub revision: i64,
    pub archived_at: Option<i64>,
    pub attributes: Option<Vec<MoveAttributeValue>>,
    pub mappings: Vec<AttributeMapping>,
}
impl MoveRecord {
    pub fn pin(&self) -> MovePin {
        MovePin {
            move_id: self.id.clone(),
            version: self.revision,
        }
    }
    pub fn validate(&self) -> Result<(), FieldErrors> {
        self.pin().validate()?;
        validate_scope(&self.world_id, self.source.is_some())?;
        if let Some(p) = &self.source {
            p.validate()?;
        }
        validate_fields(&self.fields)?;
        let mut ids = HashSet::new();
        for a in self.attributes.iter().flatten() {
            a.definition.validate()?;
            if a.definition.world_id != self.world_id || !ids.insert(&a.definition.id) {
                return Err(invalid(
                    "attributes",
                    "Choose distinct same-scope attribute identities.",
                ));
            }
            if let NumericValue::Known(v) = a.value
                && !a.definition.constraints.accepts(v)
            {
                return Err(invalid(
                    "value",
                    "The value violates its exact attribute definition.",
                ));
            }
        }
        let (mut shared, mut local) = (HashSet::new(), HashSet::new());
        for m in &self.mappings {
            m.local_attribute.validate()?;
            if self.source.is_none()
                || self.world_id.is_none()
                || !valid_id(&m.shared_attribute_id)
                || !shared.insert(&m.shared_attribute_id)
                || !local.insert(&m.local_attribute.attribute_id)
            {
                return Err(invalid(
                    "mappings",
                    "Use explicit injective shared-to-local attribute mappings on sourced local moves.",
                ));
            }
        }
        Ok(())
    }
}
