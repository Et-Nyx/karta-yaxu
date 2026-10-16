//! Synthetic ADR0012 examples. Only complete setup calls this indivisible batch.
use super::{
    authoring::{self as author, Error},
    learning::Learning,
    moves::Moves,
    types::FixtureReport,
};
use game_data::{
    authoring::ContentFields,
    charts::ChartPin,
    learning::{
        LearningRecord, LearnsetPin, LearnsetRecord, MoveMapping, MovesetRecord, validate_preset,
    },
    moves::{
        AttributeMapping, AttributePin, MoveAttributeDefinition, MoveAttributeValue, MovePin,
        MoveRecord, NumericValue,
    },
    rulesets::{AttributeExpectation, RepeatPolicy, RulesetRecord},
    stats::{NumericConstraints, RosterPin},
};
use serde::{Deserialize, Serialize};
use sqlx::SqliteConnection;
use std::collections::HashSet;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct MoveFixtures {
    fixture_version: i64,
    provenance: String,
    attributes: Vec<MoveAttributeDefinition>,
    moves: Vec<FixtureMove>,
    rulesets: Vec<FixtureRuleset>,
    learnsets: Vec<FixtureLearnset>,
    presets: Vec<MovesetRecord>,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct FixtureMove {
    id: String,
    world_id: Option<String>,
    source: Option<MovePin>,
    fields: ContentFields,
    revision: i64,
    archived_at: Option<i64>,
    attributes: Option<Vec<FixtureValue>>,
    mappings: Vec<AttributeMapping>,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct FixtureValue {
    attribute: AttributePin,
    value: NumericValue,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct FixtureExpectation {
    attribute: AttributePin,
    required: bool,
    allow_not_applicable: bool,
    constraints: NumericConstraints,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct FixtureRuleset {
    id: String,
    world_id: String,
    revision: i64,
    fields: ContentFields,
    expectations: Option<Vec<FixtureExpectation>>,
    stat_roster: Option<RosterPin>,
    chart: Option<ChartPin>,
    max_moves: Option<i64>,
    repeat_policy: RepeatPolicy,
    notes: String,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct FixtureAcquisition {
    id: String,
    move_pin: MovePin,
    method: Option<String>,
    context: Option<String>,
    condition: Option<String>,
    threshold: NumericValue,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct FixtureLearnset {
    id: String,
    world_id: Option<String>,
    form_id: Option<String>,
    variant_id: Option<String>,
    source: Option<LearnsetPin>,
    revision: i64,
    records: Option<Vec<FixtureAcquisition>>,
    mappings: Vec<MoveMapping>,
}
struct Records {
    moves: Vec<MoveRecord>,
    rulesets: Vec<RulesetRecord>,
    learnsets: Vec<LearnsetRecord>,
}
fn error() -> Error {
    author::invalid(
        "fixtures",
        "The synthetic move/learning manifest or creation baseline is incomplete, inconsistent, or collides with authored content. No fixtures were changed.",
    )
}
fn identity(ids: &mut HashSet<String>, id: &str) -> Result<(), Error> {
    if !game_data::types::valid_id(id) || !ids.insert(id.into()) {
        return Err(error());
    }
    Ok(())
}
impl MoveFixtures {
    pub(super) fn load() -> Result<Self, Error> {
        Self::parse(include_str!("../../../../fixtures/moves.json"))
    }
    fn parse(text: &str) -> Result<Self, Error> {
        let raw: serde_json::Value = serde_json::from_str(text).map_err(|_| error())?;
        let value: Self = serde_json::from_value(raw.clone()).map_err(|_| error())?;
        // Explicit nulls, tagged numeric states, and exact canonical decimal strings
        // are part of the immutable manifest contract, including compact children.
        if serde_json::to_value(&value).map_err(|_| error())? != raw
            || value.fixture_version != 1
            || value.provenance
                != "synthetic-moves-learning-v1; invented authoring examples, not PokeAPI observations, acquisition mechanics or combat readiness"
            || (
                value.attributes.len(),
                value.moves.len(),
                value.rulesets.len(),
                value.learnsets.len(),
                value.presets.len(),
            ) != (6, 6, 2, 3, 4)
        {
            return Err(error());
        }
        let records = value.records()?;
        let mut ids = HashSet::new();
        for row in &value.attributes {
            identity(&mut ids, &row.id)?;
            row.validate().map_err(|_| error())?;
            if row.revision != 1 || row.archived_at.is_some() {
                return Err(error());
            }
            if let Some(pin) = &row.source
                && value.attribute(pin)?.world_id.is_some()
            {
                return Err(error());
            }
        }
        for row in &records.moves {
            identity(&mut ids, &row.id)?;
            row.validate().map_err(|_| error())?;
            if row.revision != 1 || row.archived_at.is_some() {
                return Err(error());
            }
            if let Some(pin) = &row.source
                && records.move_row(pin)?.world_id.is_some()
            {
                return Err(error());
            }
            for mapping in &row.mappings {
                let local = value.attribute(&mapping.local_attribute)?;
                if local.world_id != row.world_id
                    || local.source.as_ref().map(|p| &p.attribute_id)
                        != Some(&mapping.shared_attribute_id)
                {
                    return Err(error());
                }
            }
        }
        let mut worlds = HashSet::new();
        for row in &records.rulesets {
            identity(&mut ids, &row.id)?;
            row.validate().map_err(|_| error())?;
            if row.revision != 1
                || !worlds.insert(&row.world_id)
                || row.stat_roster.is_none()
                || row.chart.is_none()
            {
                return Err(error());
            }
        }
        let mut owners = HashSet::new();
        for row in &records.learnsets {
            identity(&mut ids, &row.id)?;
            row.validate().map_err(|_| error())?;
            if row.revision != 1
                || row.records.as_ref().map(Vec::len) != Some(3)
                || !owners.insert((&row.world_id, &row.form_id, &row.variant_id))
            {
                return Err(error());
            }
            if let Some(pin) = &row.source {
                let source = records.learnset(pin)?;
                if source.world_id.is_some() {
                    return Err(error());
                }
                // The local examples are explicit copies; preserve every scoped
                // acquisition ID and every Unknown/NA/zero/text value separately.
                for (local, shared) in row
                    .records
                    .iter()
                    .flatten()
                    .zip(source.records.iter().flatten())
                {
                    let mapping = row
                        .mappings
                        .iter()
                        .find(|m| m.shared_move_id == shared.move_record.id)
                        .ok_or_else(error)?;
                    let mapped = records.move_row(&mapping.local_move)?;
                    let mut expected = shared.clone();
                    expected.move_record = mapped.clone();
                    if local != &expected
                        || mapped.source.as_ref().map(|p| &p.move_id)
                            != Some(&shared.move_record.id)
                    {
                        return Err(error());
                    }
                }
            }
        }
        for row in &value.presets {
            identity(&mut ids, &row.id)?;
            row.validate().map_err(|_| error())?;
            let set = records.learnset(&row.learnset)?;
            let rules = records
                .rulesets
                .iter()
                .find(|r| r.pin() == row.ruleset)
                .ok_or_else(error)?;
            if row.revision != 1
                || row.archived_at.is_some()
                || row.validator_version != 1
                || set.variant_id.as_deref() != Some(row.variant_id.as_str())
                || set.world_id.as_deref() != Some(row.world_id.as_str())
                || rules.world_id != row.world_id
                || row
                    .entries
                    .iter()
                    .any(|id| !set.records.iter().flatten().any(|r| &r.id == id))
            {
                return Err(error());
            }
            let report = validate_preset(row, set, rules);
            if report.validator_version != 1 || report.not_evaluated.is_empty() {
                return Err(error());
            }
        }
        Ok(value)
    }
    fn attribute(&self, pin: &AttributePin) -> Result<&MoveAttributeDefinition, Error> {
        self.attributes
            .iter()
            .find(|r| r.pin() == *pin)
            .ok_or_else(error)
    }
    fn records(&self) -> Result<Records, Error> {
        let moves = self
            .moves
            .iter()
            .map(|row| {
                Ok(MoveRecord {
                    id: row.id.clone(),
                    world_id: row.world_id.clone(),
                    source: row.source.clone(),
                    fields: row.fields.clone(),
                    revision: row.revision,
                    archived_at: row.archived_at,
                    attributes: row
                        .attributes
                        .as_ref()
                        .map(|values| {
                            values
                                .iter()
                                .map(|v| {
                                    Ok(MoveAttributeValue {
                                        definition: self.attribute(&v.attribute)?.clone(),
                                        value: v.value.clone(),
                                    })
                                })
                                .collect::<Result<_, Error>>()
                        })
                        .transpose()?,
                    mappings: row.mappings.clone(),
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let rulesets = self
            .rulesets
            .iter()
            .map(|row| {
                Ok(RulesetRecord {
                    id: row.id.clone(),
                    world_id: row.world_id.clone(),
                    revision: row.revision,
                    fields: row.fields.clone(),
                    expectations: row
                        .expectations
                        .as_ref()
                        .map(|values| {
                            values
                                .iter()
                                .map(|v| {
                                    Ok(AttributeExpectation {
                                        definition: self.attribute(&v.attribute)?.clone(),
                                        required: v.required,
                                        allow_not_applicable: v.allow_not_applicable,
                                        constraints: v.constraints.clone(),
                                    })
                                })
                                .collect::<Result<_, Error>>()
                        })
                        .transpose()?,
                    stat_roster: row.stat_roster.clone(),
                    chart: row.chart.clone(),
                    max_moves: row.max_moves,
                    repeat_policy: row.repeat_policy,
                    notes: row.notes.clone(),
                })
            })
            .collect::<Result<_, Error>>()?;
        let learnsets = self
            .learnsets
            .iter()
            .map(|row| {
                Ok(LearnsetRecord {
                    id: row.id.clone(),
                    world_id: row.world_id.clone(),
                    form_id: row.form_id.clone(),
                    variant_id: row.variant_id.clone(),
                    source: row.source.clone(),
                    revision: row.revision,
                    records: row
                        .records
                        .as_ref()
                        .map(|values| {
                            values
                                .iter()
                                .map(|v| {
                                    Ok(LearningRecord {
                                        id: v.id.clone(),
                                        move_record: moves
                                            .iter()
                                            .find(|m| m.pin() == v.move_pin)
                                            .ok_or_else(error)?
                                            .clone(),
                                        method: v.method.clone(),
                                        context: v.context.clone(),
                                        condition: v.condition.clone(),
                                        threshold: v.threshold.clone(),
                                    })
                                })
                                .collect::<Result<_, Error>>()
                        })
                        .transpose()?,
                    mappings: row.mappings.clone(),
                })
            })
            .collect::<Result<_, Error>>()?;
        Ok(Records {
            moves,
            rulesets,
            learnsets,
        })
    }
    pub(super) async fn seed_in(
        &self,
        c: &mut SqliteConnection,
        moves: &Moves,
        learning: &Learning,
        report: &mut FixtureReport,
    ) -> Result<(), Error> {
        let rows = self.records()?;
        let mut present = 0;
        for (query, ids) in [
            (
                "SELECT EXISTS(SELECT 1 FROM move_attribute_definitions WHERE id=?)",
                self.attributes
                    .iter()
                    .map(|r| r.id.as_str())
                    .collect::<Vec<_>>(),
            ),
            (
                "SELECT EXISTS(SELECT 1 FROM moves WHERE id=?)",
                rows.moves.iter().map(|r| r.id.as_str()).collect(),
            ),
            (
                "SELECT EXISTS(SELECT 1 FROM world_rulesets WHERE id=?)",
                rows.rulesets.iter().map(|r| r.id.as_str()).collect(),
            ),
            (
                "SELECT EXISTS(SELECT 1 FROM learnsets WHERE id=?)",
                rows.learnsets.iter().map(|r| r.id.as_str()).collect(),
            ),
            (
                "SELECT EXISTS(SELECT 1 FROM moveset_presets WHERE id=?)",
                self.presets.iter().map(|r| r.id.as_str()).collect(),
            ),
        ] {
            for id in ids {
                let exists: bool = sqlx::query_scalar(query)
                    .bind(id)
                    .fetch_one(&mut *c)
                    .await
                    .map_err(crate::persistence::authoring::storage_error)?;
                present += usize::from(exists);
            }
        }
        let total = self.attributes.len()
            + rows.moves.len()
            + rows.rulesets.len()
            + rows.learnsets.len()
            + self.presets.len();
        if present != 0 && present != total {
            return Err(error());
        }
        // A custom singleton is a conflict even when no fixture identity exists.
        // All actual writes remain in the services' typed persistence writers.
        for row in &rows.rulesets {
            let id: Option<String> =
                sqlx::query_scalar("SELECT id FROM world_rulesets WHERE world_id=?")
                    .bind(&row.world_id)
                    .fetch_optional(&mut *c)
                    .await
                    .map_err(crate::persistence::authoring::storage_error)?;
            if id.as_deref().is_some_and(|id| id != row.id) {
                return Err(error());
            }
        }
        for row in &rows.learnsets {
            let id: Option<String> = sqlx::query_scalar(
                "SELECT id FROM learnsets WHERE scope=? AND coalesce(form_id,variant_id)=?",
            )
            .bind(row.world_id.as_deref().unwrap_or(""))
            .bind(
                row.form_id
                    .as_deref()
                    .or(row.variant_id.as_deref())
                    .ok_or_else(error)?,
            )
            .fetch_optional(&mut *c)
            .await
            .map_err(crate::persistence::authoring::storage_error)?;
            if id.as_deref().is_some_and(|id| id != row.id) {
                return Err(error());
            }
        }
        for row in &self.attributes {
            if moves.seed_attribute_in(c, row).await? {
                report.move_attributes_created += 1;
            } else {
                report.skipped += 1;
            }
        }
        for row in &rows.moves {
            if moves.seed_move_in(c, row).await? {
                if row.world_id.is_none() {
                    report.shared_moves_created += 1;
                } else {
                    report.world_moves_created += 1;
                }
            } else {
                report.skipped += 1;
            }
        }
        for row in &rows.rulesets {
            if moves.seed_ruleset_in(c, row).await? {
                report.rulesets_created += 1;
            } else {
                report.skipped += 1;
            }
        }
        for row in &rows.learnsets {
            if learning.seed_learnset_in(c, row).await? {
                report.learnsets_created += 1;
            } else {
                report.skipped += 1;
            }
        }
        for row in &self.presets {
            if learning.seed_preset_in(c, row).await? {
                report.movesets_created += 1;
            } else {
                report.skipped += 1;
            }
        }
        Ok(())
    }
}
impl Records {
    fn move_row(&self, pin: &MovePin) -> Result<&MoveRecord, Error> {
        self.moves
            .iter()
            .find(|r| r.pin() == *pin)
            .ok_or_else(error)
    }
    fn learnset(&self, pin: &LearnsetPin) -> Result<&LearnsetRecord, Error> {
        self.learnsets
            .iter()
            .find(|r| r.pin() == *pin)
            .ok_or_else(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn strict_manifest_rejects_drift_and_preserves_descriptive_numeric_states() {
        let text = include_str!("../../../../fixtures/moves.json");
        let fixtures = MoveFixtures::parse(text).unwrap();
        assert!(
            MoveFixtures::parse(&text.replacen(
                "\"fixture_version\": 1",
                "\"fixture_version\": 2",
                1
            ))
            .is_err()
        );
        assert!(
            MoveFixtures::parse(&text.replacen("\"value\":\"0\"", "\"value\":\"0.0\"", 1)).is_err()
        );
        let mut value: serde_json::Value = serde_json::from_str(text).unwrap();
        value["learnsets"][0]
            .as_object_mut()
            .unwrap()
            .remove("source");
        assert!(MoveFixtures::parse(&value.to_string()).is_err());
        let rows = fixtures.records().unwrap();
        let records = rows.learnsets[0].records.as_ref().unwrap();
        assert_eq!(records[0].move_record, records[1].move_record);
        assert_ne!(records[0].id, records[1].id);
        assert_eq!(records[0].threshold, NumericValue::Unknown);
        assert_eq!(
            records[1].threshold,
            NumericValue::Known("0".parse().unwrap())
        );
        assert_eq!(records[2].threshold, NumericValue::NotApplicable);
    }
}
