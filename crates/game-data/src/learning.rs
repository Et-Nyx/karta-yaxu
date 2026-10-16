//! Independent acquisition records and named presets. Text is descriptive, never executable.
use crate::{
    authoring::{ContentFields, FieldErrors},
    moves::{MovePin, MoveRecord, NumericValue},
    rulesets::{RepeatPolicy, RulesetPin, RulesetRecord, ValidationInput, ValidationReport},
    types::valid_id,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const VALIDATOR_VERSION: i64 = crate::rulesets::CURRENT_VALIDATOR_VERSION;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LearnsetPin {
    pub learnset_id: String,
    pub version: i64,
}
impl LearnsetPin {
    pub fn validate(&self) -> Result<(), FieldErrors> {
        check_pin(&self.learnset_id, self.version, "learnset")
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PresetPin {
    pub preset_id: String,
    pub version: i64,
}
impl PresetPin {
    pub fn validate(&self) -> Result<(), FieldErrors> {
        check_pin(&self.preset_id, self.version, "preset")
    }
}
fn check_pin(id: &str, version: i64, field: &str) -> Result<(), FieldErrors> {
    if !valid_id(id) || version < 1 {
        Err(FieldErrors::from([(
            field.into(),
            "Choose an exact positive revision of a canonical identity.".into(),
        )]))
    } else {
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MoveMapping {
    pub shared_move_id: String,
    pub local_move: MovePin,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LearningRecord {
    pub id: String,
    pub move_record: MoveRecord,
    pub method: Option<String>,
    pub context: Option<String>,
    pub condition: Option<String>,
    pub threshold: NumericValue,
}
impl LearningRecord {
    pub fn validate(&self) -> Result<(), FieldErrors> {
        check_pin(&self.id, 1, "record")?;
        self.move_record.validate()?;
        for (field, value) in [
            ("method", &self.method),
            ("context", &self.context),
            ("condition", &self.condition),
        ] {
            if let Some(text) = value
                && (text.is_empty()
                    || text.trim() != text
                    || text.chars().count() > 2000
                    || text.chars().any(|c| c.is_control() && c != '\n'))
            {
                return Err(FieldErrors::from([(
                    field.into(),
                    "Use 1–2000 characters of plain descriptive text, or Unknown.".into(),
                )]));
            }
        }
        // Decimal itself guarantees a canonical signed coefficient and scale.
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LearnsetRecord {
    pub id: String,
    pub world_id: Option<String>,
    pub form_id: Option<String>,
    pub variant_id: Option<String>,
    pub source: Option<LearnsetPin>,
    pub revision: i64,
    pub records: Option<Vec<LearningRecord>>,
    pub mappings: Vec<MoveMapping>,
}
impl LearnsetRecord {
    pub fn pin(&self) -> LearnsetPin {
        LearnsetPin {
            learnset_id: self.id.clone(),
            version: self.revision,
        }
    }
    pub fn validate(&self) -> Result<(), FieldErrors> {
        check_pin(&self.id, self.revision, "learnset")?;
        let owner_ok = match (&self.world_id, &self.form_id, &self.variant_id) {
            (None, Some(form), None) => {
                valid_id(form) && self.source.is_none() && self.mappings.is_empty()
            }
            (Some(world), None, Some(variant)) => valid_id(world) && valid_id(variant),
            _ => false,
        };
        if !owner_ok {
            return Err(FieldErrors::from([(
                "owner".into(),
                "A learnset belongs to one shared form or one same-world variant.".into(),
            )]));
        }
        if let Some(pin) = &self.source {
            pin.validate()?;
        }
        if self.source.is_none() && !self.mappings.is_empty() {
            return Err(FieldErrors::from([(
                "mappings".into(),
                "Mappings require an explicit shared learnset source.".into(),
            )]));
        }
        let mut ids = BTreeSet::new();
        for record in self.records.iter().flatten() {
            record.validate()?;
            if record.move_record.world_id != self.world_id || !ids.insert(&record.id) {
                return Err(FieldErrors::from([(
                    "records".into(),
                    "Use unique acquisition IDs and exact same-scope moves.".into(),
                )]));
            }
        }
        let (mut shared, mut local) = (BTreeSet::new(), BTreeSet::new());
        for mapping in &self.mappings {
            mapping.local_move.validate()?;
            if !valid_id(&mapping.shared_move_id)
                || !shared.insert(&mapping.shared_move_id)
                || !local.insert(&mapping.local_move.move_id)
            {
                return Err(FieldErrors::from([(
                    "mappings".into(),
                    "Choose an injective explicit shared-to-local move mapping.".into(),
                )]));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MovesetRecord {
    pub id: String,
    pub world_id: String,
    pub variant_id: String,
    pub fields: ContentFields,
    pub revision: i64,
    pub archived_at: Option<i64>,
    pub learnset: LearnsetPin,
    pub ruleset: RulesetPin,
    pub entries: Vec<String>,
    pub validator_version: i64,
}
impl MovesetRecord {
    pub fn pin(&self) -> PresetPin {
        PresetPin {
            preset_id: self.id.clone(),
            version: self.revision,
        }
    }
    pub fn validate(&self) -> Result<(), FieldErrors> {
        check_pin(&self.id, self.revision, "preset")?;
        self.learnset.validate()?;
        self.ruleset.validate()?;
        if !valid_id(&self.world_id)
            || !valid_id(&self.variant_id)
            || self.fields.normalized().as_ref() != Ok(&self.fields)
            || self.validator_version < 1
            || self.entries.iter().any(|id| !valid_id(id))
        {
            return Err(FieldErrors::from([(
                "preset".into(),
                "Use valid owner, metadata, exact pins and acquisition IDs.".into(),
            )]));
        }
        Ok(())
    }
}

/// Saved-policy authoring diagnostics, never combat readiness. Callers validate scope and pins first.
pub fn validate_preset(
    preset: &MovesetRecord,
    learnset: &LearnsetRecord,
    ruleset: &RulesetRecord,
) -> ValidationReport {
    let mut report = ValidationReport {
        validator_version: preset.validator_version,
        inputs: vec![
            ValidationInput {
                kind: "preset".into(),
                id: preset.id.clone(),
                version: preset.revision,
            },
            ValidationInput {
                kind: "learnset".into(),
                id: learnset.id.clone(),
                version: learnset.revision,
            },
            ValidationInput {
                kind: "ruleset".into(),
                id: ruleset.id.clone(),
                version: ruleset.revision,
            },
        ],
        contradictions: vec![],
        incomplete: vec![],
        not_evaluated: vec![],
    };
    if !matches!(preset.validator_version, 1 | 2) {
        report.unsupported();
        return report;
    }
    if preset.validator_version == 2 {
        let configuration =
            crate::rulesets::validate_ruleset_with_policy(ruleset, preset.validator_version);
        report.inputs.extend(configuration.inputs);
        report.contradictions.extend(configuration.contradictions);
        report.incomplete.extend(configuration.incomplete);
        report.not_evaluated.extend(configuration.not_evaluated);
    }
    if learnset.records.is_none() {
        report
            .incomplete
            .push("The pinned learnset is unmodeled.".into());
    }
    if let Some(pin) = &learnset.source {
        report.inputs.push(ValidationInput {
            kind: "learnset".into(),
            id: pin.learnset_id.clone(),
            version: pin.version,
        });
    }
    if let Some(pin) = &ruleset.stat_roster {
        report.inputs.push(ValidationInput {
            kind: "stat_roster".into(),
            id: pin.roster_id.clone(),
            version: pin.version,
        });
    }
    if let Some(pin) = &ruleset.chart {
        report.inputs.push(ValidationInput {
            kind: "chart".into(),
            id: pin.chart_id.clone(),
            version: pin.version,
        });
    }
    for expectation in ruleset.expectations.iter().flatten() {
        report.inputs.push(ValidationInput {
            kind: "move_attribute".into(),
            id: expectation.definition.id.clone(),
            version: expectation.definition.revision,
        });
        if let Some(pin) = &expectation.definition.source {
            report.inputs.push(ValidationInput {
                kind: "move_attribute".into(),
                id: pin.attribute_id.clone(),
                version: pin.version,
            });
        }
    }
    if ruleset.expectations.is_none() {
        report
            .incomplete
            .push("Ruleset attribute expectations are unmodeled.".into());
    }
    // The learnset snapshot is itself an input, including unselected acquisitions.
    for record in learnset.records.iter().flatten() {
        report.inputs.extend(
            crate::rulesets::validate_move_with_policy(
                &record.move_record,
                ruleset,
                preset.validator_version,
            )
            .inputs,
        );
    }
    if let Some(cap) = ruleset.max_moves {
        if i128::try_from(preset.entries.len()).unwrap_or(i128::MAX) > i128::from(cap) {
            report.contradictions.push(format!(
                "The preset has {} entries; the configured maximum is {cap}.",
                preset.entries.len()
            ));
        }
    } else if preset.validator_version == 1 {
        report
            .incomplete
            .push("The ruleset has no moveset maximum.".into());
    }
    if ruleset.repeat_policy == RepeatPolicy::Unspecified {
        let messages = if preset.validator_version == 1 {
            &mut report.incomplete
        } else {
            &mut report.not_evaluated
        };
        messages.push("The ruleset repeat policy is unspecified.".into());
    }
    let mut moves = BTreeSet::new();
    let mut checked = BTreeSet::new();
    for id in &preset.entries {
        let Some(record) = learnset.records.iter().flatten().find(|r| &r.id == id) else {
            report.contradictions.push(format!(
                "Entry {id} is absent from the exact pinned learnset."
            ));
            continue;
        };
        if !moves.insert(&record.move_record.id) && ruleset.repeat_policy == RepeatPolicy::Forbid {
            report.contradictions.push(format!(
                "Move {} is selected more than once; repeats are forbidden.",
                record.move_record.id
            ));
        }
        if checked.insert((&record.move_record.id, record.move_record.revision)) {
            let move_report = crate::rulesets::validate_move_with_policy(
                &record.move_record,
                ruleset,
                preset.validator_version,
            );
            report.inputs.extend(move_report.inputs);
            report.contradictions.extend(move_report.contradictions);
            report.incomplete.extend(move_report.incomplete);
            report.not_evaluated.extend(move_report.not_evaluated);
        }
        if record.method.is_none()
            || record.context.is_none()
            || record.condition.is_none()
            || matches!(record.threshold, NumericValue::Unknown)
        {
            report.incomplete.push(format!(
                "Acquisition {} has unknown descriptive inputs.",
                record.id
            ));
        }
    }
    report.not_evaluated.push("Acquisition methods, contexts, conditions and thresholds are descriptive; acquisition eligibility is not evaluated.".into());
    report
        .not_evaluated
        .push("Effects, formulas and combat readiness are not evaluated.".into());
    report
        .inputs
        .sort_by(|a, b| (&a.kind, &a.id, a.version).cmp(&(&b.kind, &b.id, b.version)));
    report
        .inputs
        .dedup_by(|a, b| a.kind == b.kind && a.id == b.id && a.version == b.version);
    for messages in [
        &mut report.contradictions,
        &mut report.incomplete,
        &mut report.not_evaluated,
    ] {
        messages.sort();
        messages.dedup();
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        moves::MoveAttributeDefinition,
        rulesets::{validate_move_with_policy, validate_ruleset_with_policy},
        stats::NumericConstraints,
    };
    use serde_json::{Value, json};

    struct Fixtures {
        moves: Vec<MoveRecord>,
        rules: Vec<RulesetRecord>,
        learnsets: Vec<LearnsetRecord>,
        presets: Vec<MovesetRecord>,
    }

    // Resolve the fixture's exact pins into domain snapshots without persistence or a server.
    fn fixtures() -> Fixtures {
        let root: Value =
            serde_json::from_str(include_str!("../../../fixtures/moves.json")).unwrap();
        let attributes: Vec<MoveAttributeDefinition> =
            serde_json::from_value(root["attributes"].clone()).unwrap();
        let resolve_attributes = |rows: &mut Value, key: &str| {
            for row in rows.as_array_mut().unwrap() {
                for value in row[key].as_array_mut().unwrap() {
                    let pin = value.as_object_mut().unwrap().remove("attribute").unwrap();
                    let definition = attributes
                        .iter()
                        .find(|a| a.id == pin["attribute_id"] && a.revision == pin["version"])
                        .unwrap();
                    value["definition"] = serde_json::to_value(definition).unwrap();
                }
            }
        };
        let mut moves = root["moves"].clone();
        resolve_attributes(&mut moves, "attributes");
        let moves: Vec<MoveRecord> = serde_json::from_value(moves).unwrap();
        let mut rules = root["rulesets"].clone();
        resolve_attributes(&mut rules, "expectations");
        let rules: Vec<RulesetRecord> = serde_json::from_value(rules).unwrap();
        let mut learnsets = root["learnsets"].clone();
        for set in learnsets.as_array_mut().unwrap() {
            for record in set["records"].as_array_mut().unwrap() {
                let pin = record.as_object_mut().unwrap().remove("move_pin").unwrap();
                let m = moves
                    .iter()
                    .find(|m| m.id == pin["move_id"] && m.revision == pin["version"])
                    .unwrap();
                record["move_record"] = serde_json::to_value(m).unwrap();
            }
        }
        let learnsets: Vec<LearnsetRecord> = serde_json::from_value(learnsets).unwrap();
        let presets: Vec<MovesetRecord> = serde_json::from_value(root["presets"].clone()).unwrap();
        for m in &moves {
            m.validate().unwrap();
        }
        for r in &rules {
            r.validate().unwrap();
        }
        for set in &learnsets {
            set.validate().unwrap();
        }
        for preset in &presets {
            preset.validate().unwrap();
        }
        Fixtures {
            moves,
            rules,
            learnsets,
            presets,
        }
    }

    fn input(kind: &str, id: &str) -> ValidationInput {
        ValidationInput {
            kind: kind.into(),
            id: id.into(),
            version: 1,
        }
    }

    // Golden closure includes both selected and unselected Move revisions and their sources.
    fn golden_preset_inputs(classic: bool, preset_id: &str) -> Vec<ValidationInput> {
        let (chart, learnset, local_move_one, local_move_two, attr_one, attr_two, ruleset, roster) =
            if classic {
                (
                    "fe0bb8c103c8492591f28c4c4bc79721",
                    "44a00000000040008000000000000002",
                    "42b00000000040008000000000000001",
                    "42b00000000040008000000000000002",
                    "41b00000000040008000000000000001",
                    "41b00000000040008000000000000002",
                    "43a00000000040008000000000000001",
                    "31d00000000040008000000000000001",
                )
            } else {
                (
                    "894a9df1d3334550b0ed24e5840b7b4b",
                    "44a00000000040008000000000000003",
                    "42c00000000040008000000000000001",
                    "42c00000000040008000000000000002",
                    "41c00000000040008000000000000001",
                    "41c00000000040008000000000000002",
                    "43a00000000040008000000000000002",
                    "31d00000000040008000000000000002",
                )
            };
        vec![
            input("chart", chart),
            input("learnset", "44a00000000040008000000000000001"),
            input("learnset", learnset),
            input("move", "42a00000000040008000000000000001"),
            input("move", "42a00000000040008000000000000002"),
            input("move", local_move_one),
            input("move", local_move_two),
            input("move_attribute", "41a00000000040008000000000000001"),
            input("move_attribute", "41a00000000040008000000000000002"),
            input("move_attribute", attr_one),
            input("move_attribute", attr_two),
            input("preset", preset_id),
            input("ruleset", ruleset),
            input("stat_roster", roster),
        ]
    }

    fn assert_golden(report: &ValidationReport, golden: Value) {
        let expected: ValidationReport = serde_json::from_value(golden).unwrap();
        assert_eq!(report, &expected);
        assert_eq!(
            serde_json::to_string(report).unwrap(),
            serde_json::to_string(&expected).unwrap(),
            "serialized reports must preserve exact wording and array ordering"
        );
    }

    #[test]
    fn policy_one_all_saved_fixture_reports_match_exact_golden_messages_and_inputs() {
        let f = fixtures();
        let messages = [
            (
                vec![
                    "Attribute 41b00000000040008000000000000001 revision 1: NotApplicable is not permitted.",
                    "Move 42b00000000040008000000000000001 is selected more than once; repeats are forbidden.",
                    "The preset has 3 entries; the configured maximum is 2.",
                ],
                vec![
                    "Acquisition 45a00000000040008000000000000001 has unknown descriptive inputs.",
                    "Acquisition 45a00000000040008000000000000002 has unknown descriptive inputs.",
                    "Attribute 41b00000000040008000000000000002 revision 1: required value is Unknown.",
                ],
            ),
            (
                vec![],
                vec![
                    "Acquisition 45a00000000040008000000000000001 has unknown descriptive inputs.",
                    "Attribute 41b00000000040008000000000000002 revision 1: required value is Unknown.",
                ],
            ),
            (
                vec![
                    "Attribute 41c00000000040008000000000000001 revision 1: NotApplicable is not permitted.",
                ],
                vec!["The ruleset has no moveset maximum."],
            ),
            (
                vec![],
                vec![
                    "Acquisition 45a00000000040008000000000000001 has unknown descriptive inputs.",
                    "Acquisition 45a00000000040008000000000000002 has unknown descriptive inputs.",
                    "Attribute 41c00000000040008000000000000002 revision 1: required value is Unknown.",
                    "The ruleset has no moveset maximum.",
                ],
            ),
        ];
        for (index, preset) in f.presets.iter().enumerate() {
            assert_eq!(preset.validator_version, 1);
            let world = usize::from(index >= 2);
            let report = validate_preset(preset, &f.learnsets[world + 1], &f.rules[world]);
            assert_golden(
                &report,
                json!({
                    "validator_version": 1,
                    "inputs": golden_preset_inputs(world == 0, &preset.id),
                    "contradictions": messages[index].0,
                    "incomplete": messages[index].1,
                    "not_evaluated": [
                        "Acquisition methods, contexts, conditions and thresholds are descriptive; acquisition eligibility is not evaluated.",
                        "Effects, formulas and combat readiness are not evaluated.",
                        "Formulas, conditions, interactions, environment effects and combat eligibility are not evaluated; this report is not combat readiness."
                    ]
                }),
            );
        }
    }

    #[test]
    fn policy_one_fixture_move_reports_preserve_original_insertion_order_and_wording() {
        let f = fixtures();
        for world in 0..2 {
            let rules = &f.rules[world];
            let expectations = rules.expectations.as_ref().unwrap();
            for which_move in 0..2 {
                let m = &f.moves[2 + 2 * world + which_move];
                let mut inputs = vec![
                    input("move", &m.id),
                    input("ruleset", &rules.id),
                    input("move", &m.source.as_ref().unwrap().move_id),
                    input(
                        "stat_roster",
                        &rules.stat_roster.as_ref().unwrap().roster_id,
                    ),
                    input("chart", &rules.chart.as_ref().unwrap().chart_id),
                ];
                for e in expectations {
                    inputs.push(input("move_attribute", &e.definition.id));
                    inputs.push(input(
                        "move_attribute",
                        &e.definition.source.as_ref().unwrap().attribute_id,
                    ));
                }
                let (contradictions, incomplete) = if which_move == 0 {
                    (
                        vec![],
                        vec![format!(
                            "Attribute {} revision 1: required value is Unknown.",
                            expectations[1].definition.id
                        )],
                    )
                } else {
                    (
                        vec![format!(
                            "Attribute {} revision 1: NotApplicable is not permitted.",
                            expectations[0].definition.id
                        )],
                        vec![],
                    )
                };
                assert_golden(
                    &validate_move_with_policy(m, rules, 1),
                    json!({
                        "validator_version": 1,
                        "inputs": inputs,
                        "contradictions": contradictions,
                        "incomplete": incomplete,
                        "not_evaluated": ["Formulas, conditions, interactions, environment effects and combat eligibility are not evaluated; this report is not combat readiness."]
                    }),
                );
            }
        }
    }

    #[test]
    fn optional_policies_are_incomplete_only_under_policy_one() {
        let mut f = fixtures();
        let preset = &mut f.presets[0];
        preset.entries.clear();
        let rules = &mut f.rules[0];
        rules.expectations = Some(vec![]);
        rules.max_moves = None;
        rules.repeat_policy = RepeatPolicy::Unspecified;
        let set = &mut f.learnsets[1];
        set.records = Some(vec![]);
        for version in [1, 2] {
            preset.validator_version = version;
            let report = validate_preset(preset, set, rules);
            assert_eq!(report.validator_version, version);
            assert!(report.contradictions.is_empty());
            if version == 1 {
                assert_eq!(
                    report.incomplete,
                    [
                        "The ruleset has no moveset maximum.",
                        "The ruleset repeat policy is unspecified."
                    ]
                );
                assert!(
                    !report
                        .not_evaluated
                        .iter()
                        .any(|s| s.contains("repeat policy"))
                );
            } else {
                assert!(report.incomplete.is_empty());
                assert!(
                    report
                        .not_evaluated
                        .contains(&"The ruleset repeat policy is unspecified.".into())
                );
            }
            assert!(
                report
                    .not_evaluated
                    .contains(&"Effects, formulas and combat readiness are not evaluated.".into())
            );
        }
        assert_eq!(VALIDATOR_VERSION, 2);
    }

    #[test]
    fn empty_presets_report_impossible_required_configuration_only_in_policy_two() {
        let mut f = fixtures();
        let preset = &mut f.presets[0];
        preset.entries.clear();
        let rules = &mut f.rules[0];
        let expectation = &mut rules.expectations.as_mut().unwrap()[0];
        expectation.definition.constraints = NumericConstraints {
            whole_only: false,
            minimum: Some("0".parse().unwrap()),
            maximum: Some("10".parse().unwrap()),
        };
        expectation.constraints = NumericConstraints {
            whole_only: false,
            minimum: Some("20".parse().unwrap()),
            maximum: Some("30".parse().unwrap()),
        };
        rules.validate().unwrap();
        // No Move at all is necessary for the configuration contradiction.
        let set = &mut f.learnsets[1];
        set.records = Some(vec![]);
        assert!(
            validate_preset(preset, set, rules)
                .contradictions
                .is_empty()
        );
        preset.validator_version = 2;
        let report = validate_preset(preset, set, rules);
        assert_eq!(
            report.contradictions,
            validate_ruleset_with_policy(rules, 2).contradictions
        );
        assert_eq!(report.contradictions.len(), 1);
        assert!(report.inputs.iter().all(|i| i.kind != "move"));
        assert!(
            report
                .not_evaluated
                .iter()
                .any(|s| s.contains("combat readiness"))
        );
    }

    #[test]
    fn saved_policy_controls_nested_moves_and_unselected_input_closure_with_stable_dedup() {
        let mut f = fixtures();
        // Probe is selected twice; Veil is unselected but must remain in the input closure.
        let preset = &mut f.presets[1];
        preset.entries.push(preset.entries[0].clone());
        let rules = &mut f.rules[0];
        rules.repeat_policy = RepeatPolicy::Allow;
        rules.expectations.as_mut().unwrap()[0].constraints.minimum = Some("101".parse().unwrap());
        for version in [2, 1, 2, 1] {
            preset.validator_version = version;
            let report = validate_preset(preset, &f.learnsets[1], rules);
            assert_eq!(report.validator_version, version);
            assert_eq!(report.inputs, golden_preset_inputs(true, &preset.id));
            assert_eq!(
                report.contradictions.len(),
                if version == 1 { 1 } else { 2 }
            );
            assert!(
                report
                    .contradictions
                    .iter()
                    .any(|s| s.contains("known value violates additional ruleset restrictions"))
            );
            assert!(
                !report
                    .contradictions
                    .iter()
                    .any(|s| s.contains("NotApplicable"))
            );
            assert_eq!(report.incomplete.len(), 2);
            assert_eq!(report, validate_preset(preset, &f.learnsets[1], rules));
            if version == 2 {
                let first = report.clone();
                f.learnsets[1].records.as_mut().unwrap().reverse();
                assert_eq!(first, validate_preset(preset, &f.learnsets[1], rules));
            }
        }
    }

    #[test]
    fn future_saved_policy_is_explicitly_unsupported_and_skips_all_nested_validation() {
        let mut f = fixtures();
        let preset = &mut f.presets[0];
        preset.validator_version = 99;
        let report = validate_preset(preset, &f.learnsets[1], &f.rules[0]);
        assert_eq!(report.validator_version, 99);
        assert_eq!(
            report.inputs,
            vec![
                input("preset", &preset.id),
                input("learnset", &f.learnsets[1].id),
                input("ruleset", &f.rules[0].id),
            ]
        );
        assert!(report.contradictions.is_empty());
        assert!(report.incomplete.is_empty());
        assert_eq!(report.not_evaluated.len(), 1);
        assert!(report.not_evaluated[0].contains("99 is unsupported"));
    }
}
