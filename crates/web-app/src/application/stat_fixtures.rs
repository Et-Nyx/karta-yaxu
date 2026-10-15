//! Synthetic, additive stat examples. Only complete setup calls this batch.
use super::{
    authoring::{self as author, Error, FormRecord, HistoryKind, SourcePin, VariantRecord},
    stats::Stats,
    types::FixtureReport,
};
use crate::persistence::{authoring as content, stats as store, types};
use game_data::{
    authoring::ContentFields,
    stats::{BaseStats, Decimal, RosterPin, StatDefinition, StatPin, StatRoster, StatValue},
};
use serde::{Deserialize, Serialize};
use sqlx::SqliteConnection;
use std::collections::HashSet;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct StatFixtures {
    fixture_version: i64,
    provenance: String,
    definitions: Vec<StatDefinition>,
    rosters: Vec<FixtureRoster>,
    form: FixtureForm,
    variants: Vec<FixtureVariant>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct FixtureRoster {
    id: String,
    world_id: String,
    revision: i64,
    entries: Vec<StatPin>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct FixtureValue {
    stat: StatPin,
    value: Option<Decimal>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct FixtureForm {
    id: String,
    species_id: String,
    species_version: i64,
    revision: i64,
    fields: ContentFields,
    values: Vec<FixtureValue>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct FixtureVariant {
    id: String,
    world_id: String,
    source_form_id: String,
    source_version: i64,
    revision: i64,
    fields: ContentFields,
    roster: RosterPin,
    values: Vec<FixtureValue>,
}

fn error() -> Error {
    author::invalid(
        "fixtures",
        "The synthetic stat manifest or creation baseline is incomplete, inconsistent, or collides with authored content. No fixtures were changed.",
    )
}

fn identity(ids: &mut HashSet<String>, id: &str) -> Result<(), Error> {
    if game_data::types::valid_id(id) && ids.insert(id.into()) {
        Ok(())
    } else {
        Err(error())
    }
}

fn fields(fields: &ContentFields) -> Result<(), Error> {
    if fields.normalized().ok().as_ref() == Some(fields) {
        Ok(())
    } else {
        Err(error())
    }
}

impl FixtureForm {
    fn record(&self) -> FormRecord {
        FormRecord {
            id: self.id.clone(),
            species_id: self.species_id.clone(),
            species_version: self.species_version,
            fields: self.fields.clone(),
            revision: self.revision,
            is_default: false,
            archived_at: None,
        }
    }
}

impl FixtureVariant {
    fn record(&self) -> VariantRecord {
        VariantRecord {
            id: self.id.clone(),
            world_id: self.world_id.clone(),
            source: Some(SourcePin {
                form_id: self.source_form_id.clone(),
                version: self.source_version,
            }),
            fields: self.fields.clone(),
            revision: self.revision,
            archived_at: None,
        }
    }
}

impl StatFixtures {
    pub(super) fn load() -> Result<Self, Error> {
        let raw: serde_json::Value =
            serde_json::from_str(include_str!("../../../../fixtures/stats.json"))
                .map_err(|_| error())?;
        let value: Self = serde_json::from_value(raw.clone()).map_err(|_| error())?;
        // Require explicit nullable fields and canonical decimal strings throughout.
        if serde_json::to_value(&value).map_err(|_| error())? != raw
            || value.fixture_version != 1
            || value.provenance
                != "synthetic-design-example-v1; invented values, not PokeAPI observations or battle formulas"
            || value.definitions.len() != 10
            || value.rosters.len() != 2
            || value.variants.len() != 2
        {
            return Err(error());
        }
        let mut ids = HashSet::new();
        for definition in &value.definitions {
            identity(&mut ids, &definition.id)?;
            definition.validate().map_err(|_| error())?;
            if definition.revision != 1 || definition.archived_at.is_some() {
                return Err(error());
            }
            if let Some(pin) = &definition.source
                && value.select(pin)?.world_id.is_some()
            {
                return Err(error());
            }
        }
        let mut worlds = HashSet::new();
        for roster in &value.rosters {
            identity(&mut ids, &roster.id)?;
            if roster.revision != 1 || !worlds.insert(&roster.world_id) {
                return Err(error());
            }
            value.roster(roster)?.validate().map_err(|_| error())?;
        }
        let form = &value.form;
        identity(&mut ids, &form.id)?;
        fields(&form.fields)?;
        if form.revision != 1
            || form.species_version != 1
            || !game_data::types::valid_id(&form.species_id)
        {
            return Err(error());
        }
        let stats = value.values(None, &form.values)?;
        if stats
            .assignments
            .iter()
            .flatten()
            .any(|v| v.definition.world_id.is_some())
        {
            return Err(error());
        }
        for variant in &value.variants {
            identity(&mut ids, &variant.id)?;
            fields(&variant.fields)?;
            if variant.revision != 1
                || variant.source_form_id != form.id
                || variant.source_version != 1
            {
                return Err(error());
            }
            value.variant_stats(variant)?;
        }
        Ok(value)
    }

    fn select(&self, pin: &StatPin) -> Result<StatDefinition, Error> {
        self.definitions
            .iter()
            .find(|d| d.id == pin.stat_id && d.revision == pin.version)
            .cloned()
            .ok_or_else(error)
    }

    fn roster(&self, row: &FixtureRoster) -> Result<StatRoster, Error> {
        Ok(StatRoster {
            id: row.id.clone(),
            world_id: row.world_id.clone(),
            revision: row.revision,
            entries: row
                .entries
                .iter()
                .map(|pin| self.select(pin))
                .collect::<Result<_, _>>()?,
        })
    }

    fn values(
        &self,
        roster: Option<RosterPin>,
        values: &[FixtureValue],
    ) -> Result<BaseStats, Error> {
        let assignments = values
            .iter()
            .map(|v| {
                Ok(StatValue {
                    definition: self.select(&v.stat)?,
                    value: v.value,
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let state = BaseStats {
            roster,
            assignments: Some(assignments),
        };
        state.validate().map_err(|_| error())?;
        Ok(state)
    }

    fn variant_stats(&self, row: &FixtureVariant) -> Result<BaseStats, Error> {
        let roster = self
            .rosters
            .iter()
            .find(|r| {
                r.id == row.roster.roster_id
                    && r.revision == row.roster.version
                    && r.world_id == row.world_id
            })
            .ok_or_else(error)?;
        if roster.entries
            != row
                .values
                .iter()
                .map(|v| v.stat.clone())
                .collect::<Vec<_>>()
        {
            return Err(error());
        }
        self.values(Some(row.roster.clone()), &row.values)
    }

    pub(super) async fn seed_in(
        &self,
        c: &mut SqliteConnection,
        service: &Stats,
        report: &mut FixtureReport,
    ) -> Result<(), Error> {
        let mut present = 0;
        for row in &self.definitions {
            present += usize::from(store::identity_exists(c, &row.id).await?);
        }
        for row in &self.rosters {
            if let Some(head) = store::roster(c, &row.world_id).await? {
                if head.id != row.id {
                    return Err(error());
                }
                present += 1;
            }
        }
        present += usize::from(types::identity_exists(c, &self.form.id).await?);
        for row in &self.variants {
            present += usize::from(types::identity_exists(c, &row.id).await?);
        }
        let total = self.definitions.len() + self.rosters.len() + 1 + self.variants.len();
        if present != 0 && present != total {
            return Err(error());
        }
        for row in &self.definitions {
            if service.seed_definition_in(c, row).await? {
                if row.world_id.is_some() {
                    report.world_stats_created += 1;
                } else {
                    report.shared_stats_created += 1;
                }
            } else {
                report.skipped += 1;
            }
        }
        for row in &self.rosters {
            if service.seed_roster_in(c, &self.roster(row)?).await? {
                report.stat_rosters_created += 1;
            } else {
                report.skipped += 1;
            }
        }
        let form = self.form.record();
        let stats = self.values(None, &self.form.values)?;
        content::species_version(c, &form.species_id, form.species_version)
            .await
            .map_err(|_| error())?;
        if present != 0 {
            self.check_creature(
                c,
                HistoryKind::Form,
                &form.id,
                None,
                &author::snapshot(&form)?,
                &stats,
            )
            .await?;
            report.skipped += 1;
        } else {
            if content::slug_exists(
                c,
                content::SlugScope::Form(&form.species_id),
                &form.fields.slug,
                None,
            )
            .await?
            {
                return Err(error());
            }
            content::put_form_stats(
                c,
                &form,
                content::Revision::new(None, "create", None, None, author::snapshot(&form)?),
                &stats,
            )
            .await?;
            report.forms_created += 1;
        }
        for fixture in &self.variants {
            let row = fixture.record();
            let stats = self.variant_stats(fixture)?;
            if present != 0 {
                self.check_creature(
                    c,
                    HistoryKind::Variant,
                    &row.id,
                    Some(&row.world_id),
                    &author::snapshot(&row)?,
                    &stats,
                )
                .await?;
                report.skipped += 1;
                continue;
            }
            if content::slug_exists(
                c,
                content::SlugScope::Variant(&row.world_id),
                &row.fields.slug,
                None,
            )
            .await?
            {
                return Err(error());
            }
            let mut world = content::world(c, &row.world_id).await?;
            let before = author::world_snapshot(c, &world).await?;
            content::put_variant_stats(
                c,
                &row,
                content::Revision::new(None, "create", None, None, author::snapshot(&row)?),
                &stats,
            )
            .await?;
            world.revision = author::next(world.revision)?;
            let after = author::world_snapshot(c, &world).await?;
            content::put_world(
                c,
                &world,
                content::Revision::new(None, "add-variant", None, Some(before), after),
            )
            .await?;
            report.variants_created += 1;
        }
        Ok(())
    }

    async fn check_creature(
        &self,
        c: &mut SqliteConnection,
        kind: HistoryKind,
        id: &str,
        world: Option<&str>,
        projection: &str,
        stats: &BaseStats,
    ) -> Result<(), Error> {
        let initial = types::full_content(c, kind, id, world, 1)
            .await
            .map_err(|_| error())?;
        let actual: serde_json::Value =
            serde_json::from_str(&initial.projection_after).map_err(|_| error())?;
        let expected: serde_json::Value = serde_json::from_str(projection).map_err(|_| error())?;
        let full: serde_json::Value = serde_json::from_str(&initial.after).map_err(|_| error())?;
        if actual != expected
            || full["snapshot_version"] != 3
            || initial.stats != *stats
            || initial.typing.assignments.is_some()
            || !types::fixture_actor(c, kind, id).await?
        {
            return Err(error());
        }
        Ok(())
    }
}
