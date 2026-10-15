//! Independent stat identities and exact, signed base values. No battle formulas.
use crate::{
    authoring::{ContentFields, FieldErrors},
    types::valid_id,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use std::{cmp::Ordering, collections::HashSet, fmt, str::FromStr};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Decimal {
    coefficient: i64,
    scale: u8,
}
impl Decimal {
    pub fn new(mut coefficient: i64, mut scale: u8) -> Result<Self, String> {
        if scale > 6 {
            return Err("Use at most six fractional digits.".into());
        }
        while scale > 0 && coefficient % 10 == 0 {
            coefficient /= 10;
            scale -= 1;
        }
        Ok(Self { coefficient, scale })
    }
    pub fn coefficient(self) -> i64 {
        self.coefficient
    }
    pub fn scale(self) -> u8 {
        self.scale
    }
}
impl FromStr for Decimal {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        let bad = || {
            "Enter a signed decimal with at most six fractional digits within the 64-bit coefficient range.".to_owned()
        };
        let (negative, digits) = match s.as_bytes().first() {
            Some(b'-') => (true, &s[1..]),
            Some(b'+') => (false, &s[1..]),
            _ => (false, s),
        };
        let (whole, fraction) = match digits.split_once('.') {
            Some((w, f)) => (w, Some(f)),
            None => (digits, None),
        };
        if whole.is_empty()
            || !whole.bytes().all(|b| b.is_ascii_digit())
            || fraction.is_some_and(|f| {
                f.is_empty() || f.len() > 6 || !f.bytes().all(|b| b.is_ascii_digit())
            })
        {
            return Err(bad());
        }
        // Range-check the normalized coefficient: appending fractional zeroes
        // must not reject an otherwise representable boundary value.
        let fraction = fraction.map(|f| f.trim_end_matches('0'));
        // Accumulate negatively so i64::MIN is accepted without an overflowing abs.
        let mut coefficient = 0_i64;
        for b in whole.bytes().chain(fraction.unwrap_or("").bytes()) {
            coefficient = coefficient
                .checked_mul(10)
                .and_then(|c| c.checked_sub(i64::from(b - b'0')))
                .ok_or_else(bad)?;
        }
        if !negative {
            coefficient = coefficient.checked_neg().ok_or_else(bad)?;
        }
        Self::new(coefficient, fraction.map_or(0, |f| f.len() as u8))
    }
}
impl fmt::Display for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let n = i128::from(self.coefficient);
        if self.scale == 0 {
            return write!(f, "{n}");
        }
        let divisor = 10_i128.pow(u32::from(self.scale));
        let magnitude = n.abs();
        write!(
            f,
            "{}{}.{:0width$}",
            if n < 0 { "-" } else { "" },
            magnitude / divisor,
            magnitude % divisor,
            width = usize::from(self.scale)
        )
    }
}
impl Ord for Decimal {
    fn cmp(&self, other: &Self) -> Ordering {
        // Each product is bounded by i64 * 10^6, well inside i128.
        let scale = self.scale.max(other.scale);
        let left = i128::from(self.coefficient)
            .checked_mul(10_i128.pow(u32::from(scale - self.scale)))
            .expect("bounded decimal comparison");
        let right = i128::from(other.coefficient)
            .checked_mul(10_i128.pow(u32::from(scale - other.scale)))
            .expect("bounded decimal comparison");
        left.cmp(&right)
    }
}
impl PartialOrd for Decimal {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Serialize for Decimal {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}
impl<'de> Deserialize<'de> for Decimal {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        let value: Self = s.parse().map_err(de::Error::custom)?;
        if value.to_string() != s {
            return Err(de::Error::custom(
                "Decimal snapshots require canonical strings.",
            ));
        }
        Ok(value)
    }
}

fn invalid(field: &str, message: &str) -> FieldErrors {
    FieldErrors::from([(field.into(), message.into())])
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NumericConstraints {
    pub whole_only: bool,
    pub minimum: Option<Decimal>,
    pub maximum: Option<Decimal>,
}
impl NumericConstraints {
    pub fn validate(&self) -> Result<(), FieldErrors> {
        if self.minimum.zip(self.maximum).is_some_and(|(a, b)| a > b) {
            return Err(invalid(
                "maximum",
                "Maximum must be greater than or equal to minimum.",
            ));
        }
        Ok(())
    }
    pub fn accepts(&self, value: Decimal) -> bool {
        self.validate().is_ok()
            && (!self.whole_only || value.scale() == 0)
            && self.minimum.is_none_or(|n| value >= n)
            && self.maximum.is_none_or(|n| value <= n)
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatPin {
    pub stat_id: String,
    pub version: i64,
}
impl StatPin {
    pub fn validate(&self) -> Result<(), FieldErrors> {
        if !valid_id(&self.stat_id) || self.version <= 0 {
            return Err(invalid("stat", "Choose an exact positive stat revision."));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatDefinition {
    pub id: String,
    pub world_id: Option<String>,
    pub source: Option<StatPin>,
    pub fields: ContentFields,
    pub revision: i64,
    pub archived_at: Option<i64>,
    pub constraints: NumericConstraints,
}
impl StatDefinition {
    pub fn pin(&self) -> StatPin {
        StatPin {
            stat_id: self.id.clone(),
            version: self.revision,
        }
    }
    pub fn validate(&self) -> Result<(), FieldErrors> {
        self.pin().validate()?;
        if self.world_id.as_ref().is_some_and(|w| !valid_id(w))
            || (self.world_id.is_none() && self.source.is_some())
        {
            return Err(invalid(
                "source",
                "Only same-world local stats may pin a shared source.",
            ));
        }
        if let Some(pin) = &self.source {
            pin.validate()?;
        }
        if self.fields.normalized().ok().as_ref() != Some(&self.fields) {
            return Err(invalid(
                "fields",
                "Stat metadata must be valid normalized text.",
            ));
        }
        self.constraints.validate()
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RosterPin {
    pub roster_id: String,
    pub version: i64,
}
impl RosterPin {
    pub fn validate(&self) -> Result<(), FieldErrors> {
        if !valid_id(&self.roster_id) || self.version <= 0 {
            return Err(invalid(
                "roster",
                "Choose an exact positive roster revision.",
            ));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatRoster {
    pub id: String,
    pub world_id: String,
    pub revision: i64,
    pub entries: Vec<StatDefinition>,
}
impl StatRoster {
    pub fn pin(&self) -> RosterPin {
        RosterPin {
            roster_id: self.id.clone(),
            version: self.revision,
        }
    }
    pub fn validate(&self) -> Result<(), FieldErrors> {
        self.pin().validate()?;
        if !valid_id(&self.world_id) {
            return Err(invalid("world", "Choose a valid world."));
        }
        let mut ids = HashSet::new();
        for entry in &self.entries {
            entry.validate()?;
            if entry.world_id.as_deref() != Some(self.world_id.as_str()) || !ids.insert(&entry.id) {
                return Err(invalid(
                    "entries",
                    "Choose distinct same-world stat identities.",
                ));
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatValue {
    pub definition: StatDefinition,
    pub value: Option<Decimal>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BaseStats {
    pub roster: Option<RosterPin>,
    pub assignments: Option<Vec<StatValue>>,
}
impl Default for BaseStats {
    fn default() -> Self {
        Self::unmodeled()
    }
}
impl BaseStats {
    pub fn unmodeled() -> Self {
        Self {
            roster: None,
            assignments: None,
        }
    }
    pub fn validate(&self) -> Result<(), FieldErrors> {
        if let Some(pin) = &self.roster {
            pin.validate()?;
            if self.assignments.is_none() {
                return Err(invalid(
                    "stats",
                    "A roster context requires defined assignments, including missing values.",
                ));
            }
        }
        let mut ids = HashSet::new();
        for assignment in self.assignments.iter().flatten() {
            assignment.definition.validate()?;
            if !ids.insert(&assignment.definition.id) {
                return Err(invalid("stats", "Assign each stat identity at most once."));
            }
            if assignment
                .value
                .is_some_and(|v| !assignment.definition.constraints.accepts(v))
            {
                return Err(invalid(
                    "value",
                    "The value does not satisfy its exact pinned stat constraints.",
                ));
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatMapping {
    pub shared_stat_id: String,
    pub local_stat_id: String,
    pub clear_missing: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn decimals_are_exact_signed_normalized_and_bounded() {
        for (input, canonical) in [
            ("+001.2300", "1.23"),
            ("-0.000000", "0"),
            ("-0.125000", "-0.125"),
            ("-9223372036854.775808", "-9223372036854.775808"),
            ("-9223372036854775808", "-9223372036854775808"),
            ("9223372036854775807", "9223372036854775807"),
            ("9223372036854775807.000000", "9223372036854775807"),
            ("-9223372036854775808.0", "-9223372036854775808"),
        ] {
            let d: Decimal = input.parse().unwrap();
            assert_eq!(d.to_string(), canonical);
            assert_eq!(
                serde_json::to_string(&d).unwrap(),
                format!("\"{canonical}\"")
            );
            assert_eq!(
                serde_json::from_str::<Decimal>(&format!("\"{canonical}\"")).unwrap(),
                d
            );
        }
        for s in [
            "",
            "1.",
            ".1",
            " 1",
            "1 ",
            "1e2",
            "NaN",
            "inf",
            "1.0000001",
            "9223372036854775808",
            "-9223372036854775809",
            "9223372036854775807.1",
        ] {
            assert!(s.parse::<Decimal>().is_err(), "{s}");
        }
        for s in ["1", "1.0", "\"1.0\"", "\"+1\"", "\"-0\""] {
            assert!(serde_json::from_str::<Decimal>(s).is_err(), "{s}");
        }
        assert!(Decimal::new(0, 7).is_err());
        let lo: Decimal = "-9223372036854775808".parse().unwrap();
        let hi: Decimal = "9223372036854.775807".parse().unwrap();
        assert!(lo < hi);
        assert!("-0.000001".parse::<Decimal>().unwrap() < Decimal::new(0, 0).unwrap());
        assert_eq!(Decimal::new(1200, 3).unwrap(), "1.2".parse().unwrap());
        assert!(
            "9223372036854.775807".parse::<Decimal>().unwrap()
                < "9223372036854775807".parse().unwrap()
        );
        assert!("-9223372036854.775808".parse::<Decimal>().unwrap() > lo);
    }
    #[test]
    fn constraints_distinguish_missing_zero_and_exact_boundaries() {
        let c = NumericConstraints {
            whole_only: false,
            minimum: Some("-1.25".parse().unwrap()),
            maximum: Some("0".parse().unwrap()),
        };
        assert!(c.validate().is_ok());
        for s in ["-1.25", "-0.000001", "0"] {
            assert!(c.accepts(s.parse().unwrap()));
        }
        for s in ["-1.250001", "0.000001"] {
            assert!(!c.accepts(s.parse().unwrap()));
        }
        let whole = NumericConstraints {
            whole_only: true,
            ..c.clone()
        };
        assert!(whole.validate().is_ok());
        assert!(whole.accepts("-1".parse().unwrap()));
        assert!(!whole.accepts("-0.5".parse().unwrap()));
        assert!(
            NumericConstraints {
                minimum: c.maximum,
                maximum: c.minimum,
                ..c
            }
            .validate()
            .is_err()
        );
        assert_ne!(
            BaseStats::unmodeled(),
            BaseStats {
                roster: None,
                assignments: Some(vec![])
            }
        );
    }
}
