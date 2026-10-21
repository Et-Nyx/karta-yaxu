//! Bounded public named selectors for record authoring.
use super::*;
use crate::persistence::record_search;
impl Relations {
    pub async fn search_stats(
        &self,
        world: Option<&str>,
        query: &str,
        page: usize,
    ) -> Result<Vec<game_data::stats::StatDefinition>, Error> {
        check(query, page)?;
        let mut tx = self.read().await?;
        if let Some(w) = world {
            content::world(&mut tx, w).await?;
        }
        record_search::stats(&mut tx, world, query, page).await
    }
    pub async fn resolve_layout(
        &self,
        world: Option<&str>,
        slug: &str,
    ) -> Result<LayoutRecord, Error> {
        let mut tx = self.read().await?;
        let id = record_search::resolve(&mut tx, world, slug, true).await?;
        store::layout(&mut tx, world, &id).await
    }
    pub async fn resolve_collection(
        &self,
        world: Option<&str>,
        slug: &str,
    ) -> Result<CollectionDefinition, Error> {
        let mut tx = self.read().await?;
        let id = record_search::resolve(&mut tx, world, slug, false).await?;
        store::collection(&mut tx, world, &id).await
    }
}
fn check(query: &str, page: usize) -> Result<(), Error> {
    if query.chars().count() > 100 || !(1..=10000).contains(&page) {
        return Err(author::invalid(
            "query",
            "Use at most 100 search characters and a page from 1 to 10000.",
        ));
    }
    Ok(())
}
