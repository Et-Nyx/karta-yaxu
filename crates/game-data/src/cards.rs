//! Public card DTOs. Existing definitions are cards at their native identity/revision.
use crate::{
    authoring::{ContentFields, FieldErrors},
    card_fields::{CardField, FieldPin, FieldValue, invalid},
    types::valid_id,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardNamespace {
    Form,
    Variant,
    Type,
    Move,
    Standalone,
}
impl CardNamespace {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Form => "form",
            Self::Variant => "variant",
            Self::Type => "type",
            Self::Move => "move",
            Self::Standalone => "standalone",
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CardIdentity {
    pub namespace: CardNamespace,
    pub world_id: Option<String>,
    pub id: String,
}
impl CardIdentity {
    pub fn validate(&self) -> Result<(), FieldErrors> {
        if !valid_id(&self.id)
            || self.world_id.as_ref().is_some_and(|w| !valid_id(w))
            || (self.namespace == CardNamespace::Form && self.world_id.is_some())
            || (self.namespace == CardNamespace::Variant && self.world_id.is_none())
        {
            return Err(invalid("Choose a valid qualified card identity."));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CardPin {
    pub identity: CardIdentity,
    pub version: i64,
}
impl CardPin {
    pub fn validate(&self) -> Result<(), FieldErrors> {
        self.identity.validate()?;
        if self.version <= 0 {
            return Err(invalid("Choose a positive exact card revision."));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CardComponent {
    pub fields: Option<Vec<CardField>>,
    pub rules_text: Option<String>,
    pub labels: Option<Vec<String>>,
}
impl CardComponent {
    pub fn validate(&self, identity: &CardIdentity) -> Result<(), FieldErrors> {
        identity.validate()?;
        if self.rules_text.as_ref().is_some_and(|s| {
            s.chars().count() > 2000 || s.chars().any(|c| c.is_control() && c != '\n')
        }) {
            return Err(invalid(
                "Rules text is bounded inert plain text (2000 characters, LF paragraphs).",
            ));
        }
        let mut labels = HashSet::new();
        for l in self.labels.iter().flatten() {
            if l.is_empty()
                || l.trim() != l
                || l.chars().count() > 120
                || l.chars().any(char::is_control)
                || !labels.insert(l)
            {
                return Err(invalid(
                    "Use distinct normalized labels, at most 120 characters each.",
                ));
            }
        }
        let mut fields = HashSet::new();
        for f in self.fields.iter().flatten() {
            f.validate()?;
            if f.definition.world_id != identity.world_id || !fields.insert(&f.definition.id) {
                return Err(invalid(
                    "Select distinct field identities in the card's scope.",
                ));
            }
            if let FieldValue::CardReference(p) = &f.value
                && p.identity.world_id.is_some()
                && p.identity.world_id != identity.world_id
            {
                return Err(invalid("References must be shared or in this same world."));
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CardRecord {
    pub identity: CardIdentity,
    pub revision: i64,
    pub fields: ContentFields,
    pub archived_at: Option<i64>,
    pub source: Option<CardPin>,
    pub component: CardComponent,
    /// True only for an explicitly recorded pre-migration native revision.
    pub legacy_unmodeled: bool,
}
impl CardRecord {
    pub fn pin(&self) -> CardPin {
        CardPin {
            identity: self.identity.clone(),
            version: self.revision,
        }
    }
    pub fn validate(&self) -> Result<(), FieldErrors> {
        self.pin().validate()?;
        if self.fields.normalized().ok().as_ref() != Some(&self.fields) {
            return Err(invalid("Use normalized card metadata."));
        }
        if let Some(p) = &self.source {
            p.validate()?;
            let expected = if self.identity.namespace == CardNamespace::Variant {
                CardNamespace::Form
            } else {
                self.identity.namespace
            };
            if self.identity.world_id.is_none()
                || p.identity.world_id.is_some()
                || p.identity.namespace != expected
            {
                return Err(invalid(
                    "Only local cards may pin shared sources in the corresponding native namespace.",
                ));
            }
        }
        if self.legacy_unmodeled && self.component != CardComponent::default() {
            return Err(invalid("Legacy components must remain unmodeled."));
        }
        self.component.validate(&self.identity)
    }
}
pub fn field_dependency_pins(component: &CardComponent) -> Vec<FieldPin> {
    component
        .fields
        .iter()
        .flatten()
        .map(|f| f.definition.pin())
        .collect()
}
pub fn component_ref_pins(component: &CardComponent) -> Vec<CardPin> {
    component
        .fields
        .iter()
        .flatten()
        .filter_map(|f| match &f.value {
            FieldValue::CardReference(p) => Some(p.clone()),
            _ => None,
        })
        .collect()
}
