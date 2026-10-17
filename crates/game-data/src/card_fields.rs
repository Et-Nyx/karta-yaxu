//! Reusable, inert typed card fields. These definitions have no gameplay meaning.
use crate::{
    authoring::{ContentFields, FieldErrors},
    cards::CardPin,
    stats::{Decimal, NumericConstraints},
    types::valid_id,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub(crate) fn invalid(message: &str) -> FieldErrors {
    FieldErrors::from([("fields".into(), message.into())])
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldPin {
    pub world_id: Option<String>,
    pub field_id: String,
    pub version: i64,
}
impl FieldPin {
    pub fn validate(&self) -> Result<(), FieldErrors> {
        if !valid_id(&self.field_id)
            || self.version <= 0
            || self.world_id.as_ref().is_some_and(|w| !valid_id(w))
        {
            return Err(invalid("Choose an exact qualified field revision."));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChoiceOption {
    pub id: String,
    pub label: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "definition",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum FieldKind {
    Number(NumericConstraints),
    Text { max_length: u32 },
    Boolean,
    Choice { options: Vec<ChoiceOption> },
    CardReference,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldDefinition {
    pub id: String,
    pub world_id: Option<String>,
    pub source: Option<FieldPin>,
    pub fields: ContentFields,
    pub revision: i64,
    pub archived_at: Option<i64>,
    pub kind: FieldKind,
}
impl FieldDefinition {
    pub fn pin(&self) -> FieldPin {
        FieldPin {
            world_id: self.world_id.clone(),
            field_id: self.id.clone(),
            version: self.revision,
        }
    }
    pub fn validate(&self) -> Result<(), FieldErrors> {
        self.pin().validate()?;
        if self.fields.normalized().ok().as_ref() != Some(&self.fields) {
            return Err(invalid("Use normalized field metadata."));
        }
        if let Some(s) = &self.source {
            s.validate()?;
            if self.world_id.is_none() || s.world_id.is_some() {
                return Err(invalid("Only local fields may pin a shared source."));
            }
        }
        match &self.kind {
            FieldKind::Number(c) => c.validate()?,
            FieldKind::Text { max_length } if *max_length > 2000 => {
                return Err(invalid("Text bounds may not exceed 2000 characters."));
            }
            FieldKind::Choice { options } => {
                let mut ids = HashSet::new();
                for o in options {
                    if !valid_id(&o.id)
                        || !ids.insert(&o.id)
                        || o.label.is_empty()
                        || o.label.trim() != o.label
                        || o.label.chars().count() > 120
                        || o.label.chars().any(char::is_control)
                    {
                        return Err(invalid(
                            "Choice options need distinct stable IDs and normalized labels.",
                        ));
                    }
                }
            }
            _ => {}
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
pub enum FieldValue {
    Missing,
    Unknown,
    NotApplicable,
    Number(Decimal),
    Text(String),
    Boolean(bool),
    Choice(String),
    CardReference(CardPin),
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CardField {
    pub definition: FieldDefinition,
    pub value: FieldValue,
}
impl CardField {
    pub fn validate(&self) -> Result<(), FieldErrors> {
        self.definition.validate()?;
        let ok = match (&self.definition.kind, &self.value) {
            (_, FieldValue::Missing | FieldValue::Unknown | FieldValue::NotApplicable) => true,
            (FieldKind::Number(c), FieldValue::Number(n)) => c.accepts(*n),
            (FieldKind::Text { max_length }, FieldValue::Text(t)) => {
                t.chars().count() <= *max_length as usize
                    && !t.chars().any(|c| c.is_control() && c != '\n')
            }
            (FieldKind::Boolean, FieldValue::Boolean(_)) => true,
            (FieldKind::Choice { options }, FieldValue::Choice(id)) => {
                options.iter().any(|o| &o.id == id)
            }
            (FieldKind::CardReference, FieldValue::CardReference(p)) => p.validate().is_ok(),
            _ => false,
        };
        if ok {
            Ok(())
        } else {
            Err(invalid("Value does not match the exact field definition."))
        }
    }
}
