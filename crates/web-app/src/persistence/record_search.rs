//! Filter before paging; exact readers verify every returned public definition.
use super::{authoring::storage_error, stats as store};
use crate::application::authoring::Error;
use game_data::stats::StatDefinition;
use sqlx::SqliteConnection;

pub(crate) async fn stats(
    c: &mut SqliteConnection,
    world: Option<&str>,
    query: &str,
    page: usize,
) -> Result<Vec<StatDefinition>, Error> {
    let ids: Vec<String> = sqlx::query_scalar("SELECT d.id FROM stat_definitions d LEFT JOIN stat_versions v ON v.scope=d.scope AND v.stat_id=d.id AND v.revision=d.revision WHERE d.scope=? AND (v.stat_id IS NULL OR instr(lower(v.display_name),lower(?))>0) ORDER BY v.display_name,d.id LIMIT 25 OFFSET ?")
        .bind(world.unwrap_or("")).bind(query).bind(((page - 1) * 25) as i64).fetch_all(&mut *c).await.map_err(storage_error)?;
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        out.push(store::definition(c, world, &id).await?);
    }
    Ok(out)
}
pub(crate) async fn resolve(
    c: &mut SqliteConnection,
    world: Option<&str>,
    slug: &str,
    layout: bool,
) -> Result<String, Error> {
    sqlx::query_scalar(if layout {
        "SELECT id FROM record_layouts WHERE scope=? AND slug=?"
    } else {
        "SELECT id FROM record_collection_definitions WHERE scope=? AND slug=?"
    })
    .bind(world.unwrap_or(""))
    .bind(slug)
    .fetch_optional(c)
    .await
    .map_err(storage_error)?
    .ok_or(Error::NotFound)
}
