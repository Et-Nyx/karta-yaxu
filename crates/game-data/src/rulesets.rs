//! Versioned declarative configuration and deterministic diagnostics, never combat readiness.
use crate::{
    authoring::{ContentFields, FieldErrors},
    charts::ChartPin,
    moves::{MoveAttributeDefinition, MoveRecord, NumericValue, invalid, validate_fields},
    stats::{Decimal, NumericConstraints, RosterPin},
    types::valid_id,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const CURRENT_VALIDATOR_VERSION: i64 = 2;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RulesetPin {
    pub ruleset_id: String,
    pub version: i64,
}
impl RulesetPin {
    pub fn validate(&self) -> Result<(), FieldErrors> {
        if !valid_id(&self.ruleset_id) || self.version <= 0 {
            return Err(invalid(
                "ruleset",
                "Choose an exact positive ruleset revision.",
            ));
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepeatPolicy {
    #[default]
    Unspecified,
    Allow,
    Forbid,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttributeExpectation {
    pub definition: MoveAttributeDefinition,
    pub required: bool,
    pub allow_not_applicable: bool,
    pub constraints: NumericConstraints,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RulesetRecord {
    pub id: String,
    pub world_id: String,
    pub revision: i64,
    pub fields: ContentFields,
    pub expectations: Option<Vec<AttributeExpectation>>,
    pub stat_roster: Option<RosterPin>,
    pub chart: Option<ChartPin>,
    pub max_moves: Option<i64>,
    pub repeat_policy: RepeatPolicy,
    pub notes: String,
}
impl RulesetRecord {
    pub fn pin(&self) -> RulesetPin {
        RulesetPin {
            ruleset_id: self.id.clone(),
            version: self.revision,
        }
    }
    pub fn validate(&self) -> Result<(), FieldErrors> {
        self.pin().validate()?;
        if !valid_id(&self.world_id) {
            return Err(invalid("world", "Choose a valid world."));
        }
        validate_fields(&self.fields)?;
        if let Some(p) = &self.stat_roster {
            p.validate()?;
        }
        if let Some(p) = &self.chart
            && (!valid_id(&p.chart_id) || p.version <= 0)
        {
            return Err(invalid("chart", "Choose an exact positive chart revision."));
        }
        if self.max_moves.is_some_and(|n| n <= 0) {
            return Err(invalid(
                "max_moves",
                "The optional maximum must be positive.",
            ));
        }
        if self.notes.trim() != self.notes
            || self.notes.len() > 2000
            || self.notes.chars().any(|c| c.is_control() && c != '\n')
        {
            return Err(invalid(
                "notes",
                "Use at most 2000 bytes of normalized inert notes.",
            ));
        }
        let mut ids = HashSet::new();
        for e in self.expectations.iter().flatten() {
            e.definition.validate()?;
            e.constraints.validate()?;
            if e.definition.world_id.as_deref() != Some(&self.world_id)
                || !ids.insert(&e.definition.id)
            {
                return Err(invalid(
                    "expectations",
                    "Choose distinct same-world attribute expectations.",
                ));
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidationInput {
    pub kind: String,
    pub id: String,
    pub version: i64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidationReport {
    pub validator_version: i64,
    pub inputs: Vec<ValidationInput>,
    pub contradictions: Vec<String>,
    pub incomplete: Vec<String>,
    pub not_evaluated: Vec<String>,
}
impl Default for ValidationReport {
    fn default() -> Self {
        Self::new()
    }
}
impl ValidationReport {
    pub fn new() -> Self {
        Self::for_policy(CURRENT_VALIDATOR_VERSION)
    }
    pub fn for_policy(validator_version: i64) -> Self {
        Self {
            validator_version,
            inputs: vec![],
            contradictions: vec![],
            incomplete: vec![],
            not_evaluated: vec![],
        }
    }
    pub fn input(&mut self, kind: &str, id: &str, version: i64) {
        let input = ValidationInput {
            kind: kind.into(),
            id: id.into(),
            version,
        };
        if !self.inputs.contains(&input) {
            self.inputs.push(input);
        }
    }
}
pub fn validate_move(m: &MoveRecord, rules: &RulesetRecord) -> ValidationReport {
    validate_move_with_policy(m, rules, CURRENT_VALIDATOR_VERSION)
}

/// Evaluate an explicitly selected authoring policy, without upgrading historical reports.
pub fn validate_move_with_policy(
    m: &MoveRecord,
    rules: &RulesetRecord,
    version: i64,
) -> ValidationReport {
    match version {
        1 => validate_move_policy_one(m, rules),
        2 => {
            let mut report = validate_move_policy_one(m, rules);
            report.validator_version = version;
            configuration_contradictions(&mut report, rules);
            report.sort_and_dedup();
            report
        }
        _ => {
            let mut report = ValidationReport::for_policy(version);
            report.input("move", &m.id, m.revision);
            report.input("ruleset", &rules.id, rules.revision);
            report.unsupported();
            report
        }
    }
}

// Preserve policy 1's input order, wording and checks independently of the current policy.
fn validate_move_policy_one(m: &MoveRecord, rules: &RulesetRecord) -> ValidationReport {
    let mut r = ValidationReport::for_policy(1);
    r.input("move", &m.id, m.revision);
    r.input("ruleset", &rules.id, rules.revision);
    if let Some(p) = &m.source {
        r.input("move", &p.move_id, p.version);
    }
    if let Some(p) = &rules.stat_roster {
        r.input("stat_roster", &p.roster_id, p.version);
    }
    if let Some(p) = &rules.chart {
        r.input("chart", &p.chart_id, p.version);
    }
    for a in m.attributes.iter().flatten() {
        attribute_input(&mut r, &a.definition);
    }
    for mapping in &m.mappings {
        r.input(
            "move_attribute",
            &mapping.local_attribute.attribute_id,
            mapping.local_attribute.version,
        );
    }
    for e in rules.expectations.iter().flatten() {
        attribute_input(&mut r, &e.definition);
    }
    if let Err(e) = m.validate() {
        for (k, v) in e {
            r.contradictions.push(format!("Move {k}: {v}"));
        }
    }
    if let Err(e) = rules.validate() {
        for (k, v) in e {
            r.contradictions.push(format!("Ruleset {k}: {v}"));
        }
    }
    if m.world_id.as_deref() != Some(&rules.world_id) {
        r.contradictions
            .push("Move and ruleset have different scope contexts.".into());
    }
    if m.attributes.is_none() {
        r.incomplete.push("Move attributes are unmodeled.".into());
    }
    if rules.expectations.is_none() {
        r.incomplete
            .push("Ruleset attribute expectations are unmodeled.".into());
    }
    for e in rules.expectations.iter().flatten() {
        let label = format!(
            "Attribute {} revision {}",
            e.definition.id, e.definition.revision
        );
        let a = m
            .attributes
            .iter()
            .flatten()
            .find(|a| a.definition.id == e.definition.id);
        match a {
            None if e.required => r
                .incomplete
                .push(format!("{label}: required value is absent.")),
            None => {}
            Some(a) if a.definition != e.definition => r.incomplete.push(format!(
                "{label}: exact definition-version context mismatch (move revision {}).",
                a.definition.revision
            )),
            Some(a) => match a.value {
                NumericValue::Unknown if e.required => r
                    .incomplete
                    .push(format!("{label}: required value is Unknown.")),
                NumericValue::NotApplicable if e.required || !e.allow_not_applicable => r
                    .contradictions
                    .push(format!("{label}: NotApplicable is not permitted.")),
                NumericValue::Known(v) if !e.constraints.accepts(v) => r.contradictions.push(
                    format!("{label}: known value violates additional ruleset restrictions."),
                ),
                _ => {}
            },
        }
    }
    r.not_evaluated.push("Formulas, conditions, interactions, environment effects and combat eligibility are not evaluated; this report is not combat readiness.".into());
    r
}
fn attribute_input(r: &mut ValidationReport, a: &MoveAttributeDefinition) {
    r.input("move_attribute", &a.id, a.revision);
    if let Some(p) = &a.source {
        r.input("move_attribute", &p.attribute_id, p.version);
    }
}

/// Configuration-only diagnostics, including impossible required numeric domains in policy 2.
/// Contradictory but structurally well-formed rulesets remain valid authoring drafts.
pub fn validate_ruleset_with_policy(rules: &RulesetRecord, version: i64) -> ValidationReport {
    let mut report = ValidationReport::for_policy(version);
    report.input("ruleset", &rules.id, rules.revision);
    if !matches!(version, 1 | 2) {
        report.unsupported();
        return report;
    }
    if let Some(pin) = &rules.stat_roster {
        report.input("stat_roster", &pin.roster_id, pin.version);
    }
    if let Some(pin) = &rules.chart {
        report.input("chart", &pin.chart_id, pin.version);
    }
    for expectation in rules.expectations.iter().flatten() {
        attribute_input(&mut report, &expectation.definition);
    }
    if let Err(errors) = rules.validate() {
        for (field, message) in errors {
            report
                .contradictions
                .push(format!("Ruleset {field}: {message}"));
        }
    }
    if rules.expectations.is_none() {
        report
            .incomplete
            .push("Ruleset attribute expectations are unmodeled.".into());
    }
    if version == 2 {
        configuration_contradictions(&mut report, rules);
    }
    report
        .not_evaluated
        .push("Effects, formulas and combat readiness are not evaluated.".into());
    report.sort_and_dedup();
    report
}

fn configuration_contradictions(report: &mut ValidationReport, rules: &RulesetRecord) {
    for expectation in rules.expectations.iter().flatten() {
        if expectation.required
            && impossible_numeric_intersection(
                &expectation.definition.constraints,
                &expectation.constraints,
            )
        {
            report.contradictions.push(format!(
                "Attribute {} revision {}: required Known value has no possible value satisfying intrinsic and additional numeric restrictions.",
                expectation.definition.id, expectation.definition.revision
            ));
        }
    }
}

fn impossible_numeric_intersection(
    intrinsic: &NumericConstraints,
    additional: &NumericConstraints,
) -> bool {
    let lower = intrinsic
        .minimum
        .into_iter()
        .chain(additional.minimum)
        .max();
    let upper = intrinsic
        .maximum
        .into_iter()
        .chain(additional.maximum)
        .min();
    if lower.zip(upper).is_some_and(|(lower, upper)| lower > upper) {
        return true;
    }
    if intrinsic.whole_only || additional.whole_only {
        // Euclidean division rounds toward negative infinity even for negative fractions.
        // Decimal coefficients fit i64 and divisors are 1..=10^6, so all arithmetic,
        // including the ceiling increment at signed boundaries, is bounded within i128.
        let ceil = |value: Decimal| {
            let coefficient = i128::from(value.coefficient());
            let divisor = 10_i128.pow(u32::from(value.scale()));
            coefficient.div_euclid(divisor) + i128::from(coefficient.rem_euclid(divisor) != 0)
        };
        let floor = |value: Decimal| {
            i128::from(value.coefficient()).div_euclid(10_i128.pow(u32::from(value.scale())))
        };
        return lower.map_or(i128::from(i64::MIN), ceil)
            > upper.map_or(i128::from(i64::MAX), floor);
    }
    false
}

impl ValidationReport {
    pub(crate) fn unsupported(&mut self) {
        self.not_evaluated.push(format!(
            "Validator policy {} is unsupported; this report has not been reinterpreted using a supported policy.",
            self.validator_version
        ));
    }

    pub(crate) fn sort_and_dedup(&mut self) {
        self.inputs
            .sort_by(|a, b| (&a.kind, &a.id, a.version).cmp(&(&b.kind, &b.id, b.version)));
        self.inputs.dedup();
        for messages in [
            &mut self.contradictions,
            &mut self.incomplete,
            &mut self.not_evaluated,
        ] {
            messages.sort();
            messages.dedup();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::moves::MoveAttributeValue;

    fn constraints(
        minimum: Option<&str>,
        maximum: Option<&str>,
        whole: bool,
    ) -> NumericConstraints {
        NumericConstraints {
            minimum: minimum.map(|v| v.parse().unwrap()),
            maximum: maximum.map(|v| v.parse().unwrap()),
            whole_only: whole,
        }
    }

    fn sample(
        intrinsic: NumericConstraints,
        additional: NumericConstraints,
    ) -> (MoveRecord, RulesetRecord) {
        let fields = ContentFields {
            slug: "sample".into(),
            display_name: "Sample".into(),
            description: "Inert test content".into(),
        };
        let definition = MoveAttributeDefinition {
            id: "01".repeat(16),
            world_id: Some("02".repeat(16)),
            source: None,
            fields: fields.clone(),
            unit: String::new(),
            revision: 1,
            archived_at: None,
            constraints: intrinsic,
        };
        (
            MoveRecord {
                id: "03".repeat(16),
                world_id: definition.world_id.clone(),
                source: None,
                fields: fields.clone(),
                revision: 1,
                archived_at: None,
                attributes: Some(vec![MoveAttributeValue {
                    definition: definition.clone(),
                    value: NumericValue::Unknown,
                }]),
                mappings: vec![],
            },
            RulesetRecord {
                id: "04".repeat(16),
                world_id: definition.world_id.clone().unwrap(),
                revision: 1,
                fields,
                expectations: Some(vec![AttributeExpectation {
                    definition,
                    required: true,
                    allow_not_applicable: false,
                    constraints: additional,
                }]),
                stat_roster: None,
                chart: None,
                max_moves: None,
                repeat_policy: RepeatPolicy::Unspecified,
                notes: String::new(),
            },
        )
    }

    #[test]
    fn required_disjoint_domains_are_diagnostics_independent_of_move_values() {
        let (mut m, rules) = sample(
            constraints(Some("0"), Some("10"), false),
            constraints(Some("20"), Some("30"), false),
        );
        assert!(
            rules.validate().is_ok(),
            "contradictory drafts stay readable"
        );
        let expected = vec![format!(
            "Attribute {} revision 1: required Known value has no possible value satisfying intrinsic and additional numeric restrictions.",
            "01".repeat(16)
        )];
        assert_eq!(
            validate_ruleset_with_policy(&rules, 2).contradictions,
            expected
        );
        assert!(
            validate_ruleset_with_policy(&rules, 1)
                .contradictions
                .is_empty()
        );
        for attributes in [m.attributes.clone(), Some(vec![]), None] {
            m.attributes = attributes;
            assert!(m.validate().is_ok());
            let current = validate_move(&m, &rules);
            assert_eq!(current.validator_version, 2);
            assert_eq!(current.contradictions, expected);
            assert!(!current.incomplete.is_empty());
            assert!(
                validate_move_with_policy(&m, &rules, 1)
                    .contradictions
                    .is_empty()
            );
            assert!(
                current
                    .not_evaluated
                    .iter()
                    .any(|s| s.contains("not combat readiness"))
            );
        }
    }

    #[test]
    fn whole_number_intersections_round_signed_bounds_exactly_without_overflow() {
        for (lower, upper, impossible) in [
            (Some("0.1"), Some("0.9"), true),
            (Some("-0.9"), Some("-0.1"), true),
            (Some("-0.9"), Some("0.1"), false),
            (Some("0.1"), Some("1"), false),
            (Some("-1"), Some("-0.1"), false),
            (Some("1"), Some("1"), false),
            (Some("-1"), Some("-1"), false),
            (Some("0"), Some("0"), false),
            (None, None, false),
            (Some("-9223372036854775808"), None, false),
            (None, Some("9223372036854775807"), false),
            (
                Some("-9223372036854775808"),
                Some("-9223372036854775808"),
                false,
            ),
            (
                Some("9223372036854775807"),
                Some("9223372036854775807"),
                false,
            ),
            (
                Some("-9223372036854.775808"),
                Some("-9223372036854.775808"),
                true,
            ),
            (
                Some("9223372036854.775807"),
                Some("9223372036854.775807"),
                true,
            ),
            (Some("-9223372036854.775808"), Some("-9223372036854"), false),
            (Some("-9223372036855"), Some("-9223372036854.775808"), false),
            (Some("9223372036854.775807"), Some("9223372036855"), false),
            (Some("9223372036854"), Some("9223372036854.775807"), false),
            (
                Some("-9223372036854.775808"),
                Some("9223372036854.775807"),
                false,
            ),
        ] {
            // Exercise OR of the two whole-only constraints and bounds supplied by
            // different owners, rather than only testing a single constraint object.
            for intrinsic_whole in [false, true] {
                let (m, rules) = sample(
                    constraints(lower, None, intrinsic_whole),
                    constraints(None, upper, !intrinsic_whole),
                );
                assert!(rules.validate().is_ok());
                assert_eq!(
                    !validate_move(&m, &rules).contradictions.is_empty(),
                    impossible,
                    "{lower:?}..={upper:?}, intrinsic whole {intrinsic_whole}"
                );
            }
        }
        let (m, rules) = sample(
            constraints(Some("0.1"), None, false),
            constraints(None, Some("0.9"), false),
        );
        assert!(validate_move(&m, &rules).contradictions.is_empty());
    }

    #[test]
    fn numeric_intersection_uses_tightest_bounds_and_optional_values_need_no_number() {
        let (mut m, mut rules) = sample(
            constraints(Some("0"), Some("10"), true),
            constraints(Some("0.1"), Some("0.9"), false),
        );
        assert_eq!(validate_move(&m, &rules).contradictions.len(), 1);
        // Reverse the tighter interval: both directions must intersect, not replace.
        let e = &mut rules.expectations.as_mut().unwrap()[0];
        std::mem::swap(&mut e.definition.constraints, &mut e.constraints);
        assert_eq!(
            validate_ruleset_with_policy(&rules, 2).contradictions.len(),
            1
        );
        let e = &mut rules.expectations.as_mut().unwrap()[0];
        e.required = false;
        e.allow_not_applicable = true;
        m.attributes = Some(vec![]);
        assert!(validate_move(&m, &rules).contradictions.is_empty());
        m.attributes = Some(vec![MoveAttributeValue {
            definition: rules.expectations.as_ref().unwrap()[0].definition.clone(),
            value: NumericValue::NotApplicable,
        }]);
        let report = validate_move(&m, &rules);
        assert!(report.contradictions.is_empty());
        assert!(report.incomplete.is_empty());
    }

    #[test]
    fn current_policy_preserves_intrinsic_known_checks_and_additional_value_checks() {
        let (mut m, rules) = sample(
            constraints(Some("0"), Some("10"), false),
            constraints(Some("5"), Some("8"), false),
        );
        for (value, expected) in [
            (
                "11",
                "Move value: The value violates its exact attribute definition.",
            ),
            ("4", "known value violates additional ruleset restrictions."),
        ] {
            m.attributes.as_mut().unwrap()[0].value = NumericValue::Known(value.parse().unwrap());
            for version in [1, 2] {
                assert!(
                    validate_move_with_policy(&m, &rules, version)
                        .contradictions
                        .iter()
                        .any(|s| s.contains(expected))
                );
            }
        }
    }

    #[test]
    fn report_defaults_are_current_but_future_policies_never_run_current_checks() {
        assert_eq!(CURRENT_VALIDATOR_VERSION, 2);
        assert_eq!(ValidationReport::new().validator_version, 2);
        assert_eq!(ValidationReport::default().validator_version, 2);
        let (m, rules) = sample(
            constraints(Some("0"), Some("10"), false),
            constraints(Some("20"), Some("30"), false),
        );
        for report in [
            validate_move_with_policy(&m, &rules, 99),
            validate_ruleset_with_policy(&rules, 99),
        ] {
            assert_eq!(report.validator_version, 99);
            assert!(report.contradictions.is_empty());
            assert!(report.incomplete.is_empty());
            assert_eq!(report.not_evaluated.len(), 1);
            assert!(report.not_evaluated[0].contains("99 is unsupported"));
        }
    }
}
