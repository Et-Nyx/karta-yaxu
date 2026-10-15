//! Ordered exact definition pins. The limit bounds authoring work, not game rules.
use crate::authoring::FieldErrors;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const MAX_TYPE_ASSIGNMENTS: usize = 32;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TypePin {
    pub type_id: String,
    pub version: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TypeRecord {
    pub id: String,
    pub fields: crate::authoring::ContentFields,
    pub revision: i64,
    pub archived_at: Option<i64>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorldTypeRecord {
    pub id: String,
    pub world_id: String,
    pub source: Option<TypePin>,
    pub fields: crate::authoring::ContentFields,
    pub revision: i64,
    pub archived_at: Option<i64>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TypeAssignment {
    pub pin: TypePin,
    pub fields: crate::authoring::ContentFields,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TypingState {
    pub assignments: Option<Vec<TypeAssignment>>,
}
impl TypingState {
    pub fn unmodeled() -> Self {
        Self { assignments: None }
    }
    pub fn pins(&self) -> Option<Vec<TypePin>> {
        self.assignments
            .as_ref()
            .map(|v| v.iter().map(|a| a.pin.clone()).collect())
    }
}

pub fn valid_id(id: &str) -> bool {
    id.len() == 32
        && id
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

pub fn validate_pins(pins: Option<&[TypePin]>) -> Result<(), FieldErrors> {
    let Some(pins) = pins else {
        return Ok(());
    };
    let mut ids = HashSet::new();
    if pins.len() > MAX_TYPE_ASSIGNMENTS {
        return Err(FieldErrors::from([(
            "typing".into(),
            "Choose at most 32 Types.".into(),
        )]));
    }
    for pin in pins {
        if !valid_id(&pin.type_id) || pin.version <= 0 || !ids.insert(&pin.type_id) {
            return Err(FieldErrors::from([(
                "typing".into(),
                "Choose exact positive versions without repeating a Type.".into(),
            )]));
        }
    }
    Ok(())
}
