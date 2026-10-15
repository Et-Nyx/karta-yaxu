//! Bounded synthetic charts, loaded only by complete fixture setup.
use super::{
    authoring::{Error, invalid},
    charts::{ChartCell, ChartMapping, ChartPin, ChartRecord, Charts, ContentFields, TypePin},
    types::FixtureReport,
};
use crate::persistence::{charts as store, types};
use game_data::types::TypeAssignment;
use serde::Deserialize;
use sqlx::SqliteConnection;
use std::collections::HashSet;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ChartFixtures {
    fixture_version: i64,
    charts: Vec<ChartFixture>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ChartFixture {
    id: String,
    world_id: Option<String>,
    fields: ContentFields,
    source: Option<ChartPin>,
    roster: Vec<TypePin>,
    cells: Vec<ChartCell>,
    mapping: Vec<ChartMapping>,
}

fn error() -> Error {
    invalid(
        "fixtures",
        "The synthetic chart manifest or creation baseline is incomplete or inconsistent. No fixtures were changed.",
    )
}

impl ChartFixtures {
    pub(super) fn load() -> Result<Self, Error> {
        let value: Self = serde_json::from_str(include_str!("../../../../fixtures/charts.json"))
            .map_err(|_| error())?;
        let mut ids = HashSet::new();
        let mut worlds = HashSet::new();
        if value.fixture_version != 1 || value.charts.len() != 3 {
            return Err(error());
        }
        for row in &value.charts {
            if !game_data::types::valid_id(&row.id)
                || !ids.insert(&row.id)
                || !worlds.insert(&row.world_id)
                || row.fields.normalized().ok().as_ref() != Some(&row.fields)
            {
                return Err(error());
            }
        }
        Ok(value)
    }

    pub(super) async fn seed_in(
        &self,
        c: &mut SqliteConnection,
        service: &Charts,
        report: &mut FixtureReport,
    ) -> Result<(), Error> {
        let mut present = 0;
        for row in &self.charts {
            if store::identity_exists(c, &row.id).await? {
                present += 1;
            }
        }
        if present > 0 && present != self.charts.len() {
            return Err(error());
        }
        for row in &self.charts {
            let mut roster = Vec::new();
            for pin in &row.roster {
                let fields = match row.world_id.as_deref() {
                    Some(world) => {
                        types::world_type_version(c, world, &pin.type_id, pin.version)
                            .await?
                            .fields
                    }
                    None => {
                        types::type_version(c, &pin.type_id, pin.version)
                            .await?
                            .fields
                    }
                };
                roster.push(TypeAssignment {
                    pin: pin.clone(),
                    fields,
                });
            }
            let mut record = ChartRecord {
                id: row.id.clone(),
                world_id: row.world_id.clone(),
                fields: row.fields.clone(),
                revision: 1,
                source: row.source.clone(),
                roster,
                cells: row.cells.clone(),
                mapping: row.mapping.clone(),
                archived_at: None,
            };
            record.cells.sort_by(|a, b| {
                (&a.attacker_type_id, &a.defender_type_id)
                    .cmp(&(&b.attacker_type_id, &b.defender_type_id))
            });
            record
                .mapping
                .sort_by(|a, b| a.source_type_id.cmp(&b.source_type_id));
            if service.seed_in(c, &record).await? {
                if row.world_id.is_some() {
                    report.world_charts_created += 1;
                } else {
                    report.shared_charts_created += 1;
                }
            } else {
                report.skipped += 1;
            }
        }
        Ok(())
    }
}
