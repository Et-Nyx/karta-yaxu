//! Bounded, directed pairwise effectiveness. Metadata versions do not change identity.
use crate::{
    authoring::{ContentFields, FieldErrors},
    types::{TypeAssignment, valid_id},
};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fmt};

pub const MAX_CHART_TYPES: usize = 32;
pub const MAX_FACTOR_INPUT: u32 = 1_000_000;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Factor {
    pub numerator: u32,
    pub denominator: u32,
}
impl Factor {
    pub fn new(n: u32, d: u32) -> Result<Self, String> {
        if n > MAX_FACTOR_INPUT || d == 0 || d > MAX_FACTOR_INPUT {
            return Err(
                "Use a numerator from 0 to 1,000,000 and denominator from 1 to 1,000,000.".into(),
            );
        }
        let (mut a, mut b) = (n, d);
        while b != 0 {
            (a, b) = (b, a % b);
        }
        Ok(Self {
            numerator: n / a,
            denominator: d / a,
        })
    }
}
impl fmt::Display for Factor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.denominator == 1 {
            write!(f, "{}×", self.numerator)
        } else {
            write!(f, "{}/{}×", self.numerator, self.denominator)
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Effectiveness {
    Known(Factor),
    Unknown,
    OutsideRoster,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ChartPin {
    pub chart_id: String,
    pub version: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ChartCell {
    pub attacker_type_id: String,
    pub defender_type_id: String,
    pub factor: Factor,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ChartMapping {
    pub source_type_id: String,
    pub local_type_id: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ChartRecord {
    pub id: String,
    pub world_id: Option<String>,
    pub fields: ContentFields,
    pub revision: i64,
    pub source: Option<ChartPin>,
    pub roster: Vec<TypeAssignment>,
    pub cells: Vec<ChartCell>,
    pub mapping: Vec<ChartMapping>,
    pub archived_at: Option<i64>,
}
impl ChartRecord {
    pub fn lookup(&self, attacker_id: &str, defender_id: &str) -> Effectiveness {
        if ![attacker_id, defender_id]
            .iter()
            .all(|id| self.roster.iter().any(|a| a.pin.type_id == *id))
        {
            return Effectiveness::OutsideRoster;
        }
        self.cells
            .iter()
            .find(|c| c.attacker_type_id == attacker_id && c.defender_type_id == defender_id)
            .map_or(Effectiveness::Unknown, |c| {
                Effectiveness::Known(c.factor.clone())
            })
    }
    pub fn validate(&self) -> Result<(), FieldErrors> {
        let bad =
            |field: &str, message: &str| Err(FieldErrors::from([(field.into(), message.into())]));
        if !valid_id(&self.id)
            || self.revision <= 0
            || self.world_id.as_ref().is_some_and(|id| !valid_id(id))
        {
            return bad(
                "chart",
                "Choose a valid chart identity and positive revision.",
            );
        }
        if self.fields.normalized().ok().as_ref() != Some(&self.fields) {
            return bad("fields", "Chart metadata must be valid normalized text.");
        }
        if self
            .source
            .as_ref()
            .is_some_and(|p| !valid_id(&p.chart_id) || p.version <= 0)
            || (self.world_id.is_none() && self.source.is_some())
            || (self.source.is_none() && !self.mapping.is_empty())
        {
            return bad(
                "source",
                "Only local charts can pin a shared source and map its Types.",
            );
        }
        let mut ids = HashSet::new();
        if self.roster.len() > MAX_CHART_TYPES
            || self.roster.iter().any(|a| {
                !valid_id(&a.pin.type_id)
                    || a.pin.version <= 0
                    || !ids.insert(a.pin.type_id.as_str())
                    || a.fields.normalized().ok().as_ref() != Some(&a.fields)
            })
        {
            return bad(
                "roster",
                "Choose at most 32 distinct exact Type versions with valid metadata.",
            );
        }
        if self.cells.len() > MAX_CHART_TYPES * MAX_CHART_TYPES
            || self.cells.iter().any(|c| {
                !ids.contains(c.attacker_type_id.as_str())
                    || !ids.contains(c.defender_type_id.as_str())
                    || Factor::new(c.factor.numerator, c.factor.denominator)
                        .ok()
                        .as_ref()
                        != Some(&c.factor)
            })
            || self.cells.windows(2).any(|w| {
                (&w[0].attacker_type_id, &w[0].defender_type_id)
                    >= (&w[1].attacker_type_id, &w[1].defender_type_id)
            })
        {
            return bad(
                "cells",
                "Use unique sorted roster pairs and reduced bounded factors.",
            );
        }
        let mut locals = HashSet::new();
        if self.mapping.len() > MAX_CHART_TYPES
            || self.mapping.iter().any(|m| {
                !valid_id(&m.source_type_id)
                    || !ids.contains(m.local_type_id.as_str())
                    || !locals.insert(&m.local_type_id)
            })
            || self
                .mapping
                .windows(2)
                .any(|w| w[0].source_type_id >= w[1].source_type_id)
        {
            return bad(
                "mapping",
                "Map each source and local Type at most once, in source identity order.",
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_bounded_factors_and_strict_numeric_json() {
        assert_eq!(Factor::new(500_000, 1_000_000).unwrap().to_string(), "1/2×");
        assert_eq!(Factor::new(0, 99).unwrap(), Factor::new(0, 1).unwrap());
        assert_eq!(Factor::new(10, 10).unwrap().to_string(), "1×");
        for (n, d) in [(0, 0), (1_000_001, 1), (1, 1_000_001)] {
            assert!(Factor::new(n, d).is_err());
        }
        for json in [
            r#"{"numerator":-1,"denominator":1}"#,
            r#"{"numerator":1.0,"denominator":1}"#,
            r#"{"numerator":"1","denominator":1}"#,
            r#"{"numerator":1,"denominator":1,"extra":0}"#,
        ] {
            assert!(serde_json::from_str::<Factor>(json).is_err());
        }
    }
}
