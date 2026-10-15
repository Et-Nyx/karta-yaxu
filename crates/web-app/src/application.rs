use game_data::{Catalog, Creature};

pub mod authoring;
mod chart_fixtures;
pub mod charts;
pub mod fixtures;
mod stat_fixtures;
pub mod stat_values;
pub mod stats;
pub mod types;

pub fn search<'a>(catalog: &'a Catalog, query: &str) -> Vec<&'a Creature> {
    let query = query.trim().to_lowercase();
    catalog
        .creatures()
        .iter()
        .filter(|creature| creature.display_name().to_lowercase().contains(&query))
        .collect()
}

pub fn find<'a>(catalog: &'a Catalog, id: &str) -> Option<&'a Creature> {
    catalog
        .creatures()
        .iter()
        .find(|creature| creature.id() == id)
}
