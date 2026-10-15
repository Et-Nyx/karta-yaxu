//! Independent chart authoring; authorization and expected revisions precede edits.
use super::authoring::{self as author, AddressKey, Authoring, Error, HistoryEntry, WorldRecord};
use crate::{
    accounts::Accounts,
    auth::Actor,
    persistence::{authoring as content, charts as store, types},
};
use game_data::authoring::{slug_candidate, slug_from_name, validate_reason};
use sqlx::{Sqlite, SqliteConnection, Transaction};

pub use game_data::{
    authoring::ContentFields,
    charts::{
        ChartCell, ChartMapping, ChartPin, ChartRecord, Effectiveness, Factor, MAX_CHART_TYPES,
    },
    types::{TypeAssignment, TypePin},
};

#[derive(Clone, Debug)]
pub struct NewChart {
    pub fields: ContentFields,
    pub roster: Vec<TypePin>,
    pub source: Option<ChartPin>,
    pub mapping: Vec<ChartMapping>,
    pub apply_source: bool,
    pub reason: Option<String>,
}
#[derive(Clone, Debug)]
pub struct DefinitionEdit {
    pub expected_revision: i64,
    pub fields: ContentFields,
    pub roster: Vec<TypePin>,
    pub mapping: Vec<ChartMapping>,
    pub reason: Option<String>,
}
#[derive(Clone, Debug)]
pub struct RowEdit {
    pub expected_revision: i64,
    pub attacker_type_id: String,
    pub cells: Vec<ChartCell>,
    pub reason: Option<String>,
}
#[derive(Clone, Debug)]
pub struct NeutralEdit {
    pub expected_revision: i64,
    pub reason: Option<String>,
}
#[derive(Clone, Debug)]
pub struct AdoptEdit {
    pub expected_revision: i64,
    pub target_version: i64,
    pub mapping: Vec<ChartMapping>,
    pub apply_source: bool,
    pub reason: Option<String>,
}
#[derive(Clone, Debug)]
pub struct ChartAdoptionPreview {
    pub entry: ChartRecord,
    pub old_source: ChartRecord,
    pub new_source: ChartRecord,
    pub proposed: ChartRecord,
}
#[derive(Clone, Debug)]
pub struct Charts {
    accounts: Accounts,
    authoring: Authoring,
}
impl Charts {
    pub fn new(accounts: Accounts) -> Self {
        Self {
            authoring: Authoring::new(accounts.clone()),
            accounts,
        }
    }
    async fn read(&self) -> Result<Transaction<'static, Sqlite>, Error> {
        self.accounts
            .pool()
            .begin()
            .await
            .map_err(content::storage_error)
    }
    pub async fn charts(&self) -> Result<Vec<ChartRecord>, Error> {
        let mut tx = self.read().await?;
        let mut rows = Vec::new();
        for id in store::ids(&mut tx, None).await? {
            rows.push(store::detail(&mut tx, None, &id).await?);
        }
        Ok(rows)
    }
    pub async fn world_chart(&self, world_id: &str) -> Result<Option<ChartRecord>, Error> {
        let mut tx = self.read().await?;
        content::world(&mut tx, world_id).await?;
        let ids = store::ids(&mut tx, Some(world_id)).await?;
        if ids.len() > 1 {
            return Err(Error::Unavailable);
        }
        match ids.first() {
            Some(id) => Ok(Some(store::detail(&mut tx, Some(world_id), id).await?)),
            None => Ok(None),
        }
    }
    pub async fn detail(&self, world_id: Option<&str>, id: &str) -> Result<ChartRecord, Error> {
        store::detail(&mut *self.read().await?, world_id, id).await
    }
    pub async fn version(
        &self,
        world_id: Option<&str>,
        id: &str,
        version: i64,
    ) -> Result<ChartRecord, Error> {
        store::version(&mut *self.read().await?, world_id, id, version).await
    }
    pub async fn resolve_shared(&self, key: AddressKey) -> Result<ChartRecord, Error> {
        store::resolve(&mut *self.read().await?, None, key).await
    }
    pub async fn resolve_world(
        &self,
        world_key: AddressKey,
        chart_key: AddressKey,
    ) -> Result<(WorldRecord, ChartRecord), Error> {
        let mut tx = self.read().await?;
        let world = content::resolve_world(&mut tx, world_key).await?;
        let chart = store::resolve(&mut tx, Some(&world.id), chart_key).await?;
        Ok((world, chart))
    }
    async fn actor(
        &self,
        c: &mut SqliteConnection,
        token: &str,
        world: Option<&str>,
    ) -> Result<Actor, Error> {
        let actor = self.authoring.actor(c, token).await?;
        if let Some(world) = world {
            let world = content::world(c, world).await?;
            self.authoring.world_actor(c, &actor, &world, false).await?;
        } else {
            author::require_admin(&actor)?;
        }
        Ok(actor)
    }
    async fn proposed_create(
        c: &mut SqliteConnection,
        world: Option<&str>,
        new: &NewChart,
    ) -> Result<ChartRecord, Error> {
        if let Some(world) = world {
            content::world(c, world).await?;
            if !store::ids(c, Some(world)).await?.is_empty() {
                return Err(author::invalid(
                    "chart",
                    "This world already has a chart. Edit its existing chart.",
                ));
            }
        }
        if world.is_none() && (new.source.is_some() || !new.mapping.is_empty() || new.apply_source)
        {
            return Err(author::invalid(
                "source",
                "Shared charts cannot have a source, mappings, or apply-source policy.",
            ));
        }
        if new.apply_source && new.source.is_none() {
            return Err(author::invalid(
                "source",
                "Select an exact shared chart before applying source cells.",
            ));
        }
        let (mut fields, _) = author::fields_and_reason(&new.fields, new.reason.as_deref())?;
        allocate_slug(c, world, &mut fields, None).await?;
        let mut row = ChartRecord {
            id: author::new_id()?,
            world_id: world.map(str::to_owned),
            fields,
            revision: 1,
            source: new.source.clone(),
            roster: select_roster(c, world, &new.roster).await?,
            cells: Vec::new(),
            mapping: new.mapping.clone(),
            archived_at: None,
        };
        canonicalize(&mut row)?;
        store::dependencies(c, &row)
            .await
            .map_err(|e| author::missing_reference(e, "source"))?;
        if new.apply_source {
            let pin = row.source.as_ref().ok_or(Error::Unavailable)?;
            let source = store::version(c, None, &pin.chart_id, pin.version).await?;
            apply_source(&mut row, &source);
        }
        row.validate().map_err(Error::Invalid)?;
        Ok(row)
    }
    pub async fn preview_create(
        &self,
        world_id: Option<&str>,
        new: &NewChart,
    ) -> Result<ChartRecord, Error> {
        Self::proposed_create(&mut *self.read().await?, world_id, new).await
    }
    pub async fn create(
        &self,
        token: &str,
        world_id: Option<&str>,
        new: NewChart,
    ) -> Result<ChartRecord, Error> {
        let mut tx = self.authoring.write().await?;
        let actor = self.actor(&mut tx, token, world_id).await?;
        let row = Self::proposed_create(&mut tx, world_id, &new).await?;
        let reason = validate_reason(new.reason.as_deref()).map_err(Error::Invalid)?;
        store::put(
            &mut tx,
            &row,
            Some(&actor.user_id),
            "create",
            reason.as_deref(),
        )
        .await?;
        tx.commit().await.map_err(content::storage_error)?;
        Ok(row)
    }
    async fn finish(
        c: &mut SqliteConnection,
        old: &ChartRecord,
        mut proposed: ChartRecord,
        actor: &Actor,
        action: &str,
        reason: Option<&str>,
    ) -> Result<ChartRecord, Error> {
        let reason = validate_reason(reason).map_err(Error::Invalid)?;
        canonicalize(&mut proposed)?;
        store::dependencies(c, &proposed)
            .await
            .map_err(|e| author::missing_reference(e, "roster"))?;
        if &proposed != old {
            proposed.revision = author::next(old.revision)?;
            store::put(
                c,
                &proposed,
                Some(&actor.user_id),
                action,
                reason.as_deref(),
            )
            .await?;
        }
        Ok(proposed)
    }
    pub async fn save_definition(
        &self,
        token: &str,
        world_id: Option<&str>,
        id: &str,
        edit: DefinitionEdit,
    ) -> Result<ChartRecord, Error> {
        let mut tx = self.authoring.write().await?;
        let actor = self.actor(&mut tx, token, world_id).await?;
        let old = store::detail(&mut tx, world_id, id).await?;
        author::expected(old.revision, edit.expected_revision)?;
        let (mut fields, _) = author::fields_and_reason(&edit.fields, edit.reason.as_deref())?;
        allocate_slug(&mut tx, world_id, &mut fields, Some(&old)).await?;
        let mut proposed = old.clone();
        proposed.fields = fields;
        proposed.roster = select_roster(&mut tx, world_id, &edit.roster).await?;
        let retained: std::collections::HashSet<_> = proposed
            .roster
            .iter()
            .map(|a| a.pin.type_id.as_str())
            .collect();
        proposed.cells.retain(|c| {
            retained.contains(c.attacker_type_id.as_str())
                && retained.contains(c.defender_type_id.as_str())
        });
        proposed.mapping = edit
            .mapping
            .into_iter()
            .filter(|m| {
                retained.contains(m.local_type_id.as_str())
                    || !old.roster.iter().any(|a| a.pin.type_id == m.local_type_id)
            })
            .collect();
        let row = Self::finish(
            &mut tx,
            &old,
            proposed,
            &actor,
            "edit",
            edit.reason.as_deref(),
        )
        .await?;
        tx.commit().await.map_err(content::storage_error)?;
        Ok(row)
    }
    pub async fn save_row(
        &self,
        token: &str,
        world_id: Option<&str>,
        id: &str,
        edit: RowEdit,
    ) -> Result<ChartRecord, Error> {
        let mut tx = self.authoring.write().await?;
        let actor = self.actor(&mut tx, token, world_id).await?;
        let old = store::detail(&mut tx, world_id, id).await?;
        author::expected(old.revision, edit.expected_revision)?;
        if edit.cells.len() > MAX_CHART_TYPES
            || !old
                .roster
                .iter()
                .any(|a| a.pin.type_id == edit.attacker_type_id)
            || edit
                .cells
                .iter()
                .any(|c| c.attacker_type_id != edit.attacker_type_id)
        {
            return Err(author::invalid(
                "cells",
                "Save only the selected attacking Type's row, with at most 32 cells.",
            ));
        }
        let mut proposed = old.clone();
        proposed
            .cells
            .retain(|c| c.attacker_type_id != edit.attacker_type_id);
        proposed.cells.extend(edit.cells);
        let row = Self::finish(
            &mut tx,
            &old,
            proposed,
            &actor,
            "edit",
            edit.reason.as_deref(),
        )
        .await?;
        tx.commit().await.map_err(content::storage_error)?;
        Ok(row)
    }
    pub async fn fill_neutral(
        &self,
        token: &str,
        world_id: Option<&str>,
        id: &str,
        edit: NeutralEdit,
    ) -> Result<ChartRecord, Error> {
        let mut tx = self.authoring.write().await?;
        let actor = self.actor(&mut tx, token, world_id).await?;
        let old = store::detail(&mut tx, world_id, id).await?;
        author::expected(old.revision, edit.expected_revision)?;
        let mut proposed = old.clone();
        for a in &old.roster {
            for d in &old.roster {
                if old.lookup(&a.pin.type_id, &d.pin.type_id) == Effectiveness::Unknown {
                    proposed.cells.push(ChartCell {
                        attacker_type_id: a.pin.type_id.clone(),
                        defender_type_id: d.pin.type_id.clone(),
                        factor: Factor::new(1, 1).map_err(|_| Error::Unavailable)?,
                    });
                }
            }
        }
        let row = Self::finish(
            &mut tx,
            &old,
            proposed,
            &actor,
            "edit",
            edit.reason.as_deref(),
        )
        .await?;
        tx.commit().await.map_err(content::storage_error)?;
        Ok(row)
    }
    async fn preview_in(
        c: &mut SqliteConnection,
        world: &str,
        id: &str,
        edit: &AdoptEdit,
    ) -> Result<ChartAdoptionPreview, Error> {
        let entry = store::detail(c, Some(world), id).await?;
        author::expected(entry.revision, edit.expected_revision)?;
        validate_reason(edit.reason.as_deref()).map_err(Error::Invalid)?;
        let pin = entry.source.as_ref().ok_or_else(|| {
            author::invalid("source", "This chart has no shared source to adopt.")
        })?;
        if edit.target_version <= pin.version {
            return Err(author::invalid(
                "target_version",
                "Select a newer revision of the same shared chart.",
            ));
        }
        let old_source = store::version(c, None, &pin.chart_id, pin.version).await?;
        let new_source = store::version(c, None, &pin.chart_id, edit.target_version)
            .await
            .map_err(|e| author::missing_reference(e, "target_version"))?;
        let mut proposed = entry.clone();
        proposed.source = Some(ChartPin {
            chart_id: pin.chart_id.clone(),
            version: edit.target_version,
        });
        proposed.mapping = edit.mapping.clone();
        canonicalize(&mut proposed)?;
        store::dependencies(c, &proposed).await?;
        if edit.apply_source {
            apply_source(&mut proposed, &new_source);
        }
        proposed.revision = author::next(entry.revision)?;
        proposed.validate().map_err(Error::Invalid)?;
        Ok(ChartAdoptionPreview {
            entry,
            old_source,
            new_source,
            proposed,
        })
    }
    pub async fn adoption_preview(
        &self,
        world_id: &str,
        id: &str,
        edit: &AdoptEdit,
    ) -> Result<ChartAdoptionPreview, Error> {
        Self::preview_in(&mut *self.read().await?, world_id, id, edit).await
    }
    pub async fn adopt(
        &self,
        token: &str,
        world_id: &str,
        id: &str,
        edit: AdoptEdit,
    ) -> Result<ChartRecord, Error> {
        let mut tx = self.authoring.write().await?;
        let actor = self.actor(&mut tx, token, Some(world_id)).await?;
        let preview = Self::preview_in(&mut tx, world_id, id, &edit).await?;
        let reason = validate_reason(edit.reason.as_deref()).map_err(Error::Invalid)?;
        store::put(
            &mut tx,
            &preview.proposed,
            Some(&actor.user_id),
            "adopt",
            reason.as_deref(),
        )
        .await?;
        tx.commit().await.map_err(content::storage_error)?;
        Ok(preview.proposed)
    }
    pub async fn history(
        &self,
        world_id: Option<&str>,
        id: &str,
        before: Option<i64>,
    ) -> Result<Vec<HistoryEntry>, Error> {
        store::history(&mut *self.read().await?, world_id, id, before, None).await
    }
    pub async fn history_entry(
        &self,
        world_id: Option<&str>,
        id: &str,
        revision: i64,
    ) -> Result<HistoryEntry, Error> {
        store::history(&mut *self.read().await?, world_id, id, None, Some(revision))
            .await?
            .into_iter()
            .next()
            .ok_or(Error::NotFound)
    }
    /// Trusted system loader, on the complete fixture transaction's connection.
    pub(super) async fn seed_in(
        &self,
        c: &mut SqliteConnection,
        record: &ChartRecord,
    ) -> Result<bool, Error> {
        let error = || {
            author::invalid(
                "fixtures",
                "The chart creation baseline is invalid or collides with existing content. No fixtures were changed.",
            )
        };
        record.validate().map_err(|_| error())?;
        if record.revision != 1 || record.archived_at.is_some() {
            return Err(error());
        }
        store::dependencies(c, record).await.map_err(|_| error())?;
        if types::identity_exists(c, &record.id).await? {
            return Err(error());
        }
        if store::identity_exists(c, &record.id).await? {
            if store::version(c, record.world_id.as_deref(), &record.id, 1)
                .await
                .map_err(|_| error())?
                != *record
                || !store::fixture_actor(c, &record.id).await?
            {
                return Err(error());
            }
            // A rerun validates the current reader as well, without overwriting edits.
            store::detail(c, record.world_id.as_deref(), &record.id)
                .await
                .map_err(|_| error())?;
            return Ok(false);
        }
        if store::slug_exists(c, record.world_id.as_deref(), &record.fields.slug, None).await?
            || store::baseline_slug_exists(c, record.world_id.as_deref(), &record.fields.slug)
                .await?
            || (record.world_id.is_some()
                && !store::ids(c, record.world_id.as_deref()).await?.is_empty())
        {
            return Err(error());
        }
        store::put(c, record, None, "create", None).await?;
        Ok(true)
    }
}
async fn allocate_slug(
    c: &mut SqliteConnection,
    world: Option<&str>,
    fields: &mut ContentFields,
    old: Option<&ChartRecord>,
) -> Result<(), Error> {
    if let Some(old) = old
        && old.fields.display_name == fields.display_name
    {
        fields.slug = old.fields.slug.clone();
        return Ok(());
    }
    let base = slug_from_name(&fields.display_name);
    for number in 1..=10_000 {
        let slug = slug_candidate(&base, number);
        if !store::slug_exists(c, world, &slug, old.map(|r| r.id.as_str())).await? {
            fields.slug = slug;
            return Ok(());
        }
    }
    Err(author::invalid(
        "display_name",
        "Too many charts share this address. Choose a different name.",
    ))
}
async fn select_roster(
    c: &mut SqliteConnection,
    world: Option<&str>,
    pins: &[TypePin],
) -> Result<Vec<TypeAssignment>, Error> {
    store::roster(c, world, pins)
        .await
        .map_err(|e| author::missing_reference(e, "roster"))
}
fn canonicalize(row: &mut ChartRecord) -> Result<(), Error> {
    if row.cells.len() > MAX_CHART_TYPES * MAX_CHART_TYPES || row.mapping.len() > MAX_CHART_TYPES {
        return Err(author::invalid(
            "chart",
            "The chart exceeds its bounded pair or mapping limit.",
        ));
    }
    for cell in &mut row.cells {
        cell.factor = Factor::new(cell.factor.numerator, cell.factor.denominator)
            .map_err(|s| author::invalid("cells", &s))?;
    }
    row.cells.sort_by(|a, b| {
        (&a.attacker_type_id, &a.defender_type_id).cmp(&(&b.attacker_type_id, &b.defender_type_id))
    });
    row.mapping
        .sort_by(|a, b| a.source_type_id.cmp(&b.source_type_id));
    row.validate().map_err(Error::Invalid)
}
fn apply_source(local: &mut ChartRecord, source: &ChartRecord) {
    let mapped: std::collections::HashSet<_> = local
        .mapping
        .iter()
        .map(|m| m.local_type_id.as_str())
        .collect();
    local.cells.retain(|c| {
        !mapped.contains(c.attacker_type_id.as_str())
            || !mapped.contains(c.defender_type_id.as_str())
    });
    for a in &local.mapping {
        for d in &local.mapping {
            if let Effectiveness::Known(factor) =
                source.lookup(&a.source_type_id, &d.source_type_id)
            {
                local.cells.push(ChartCell {
                    attacker_type_id: a.local_type_id.clone(),
                    defender_type_id: d.local_type_id.clone(),
                    factor,
                });
            }
        }
    }
    local.cells.sort_by(|a, b| {
        (&a.attacker_type_id, &a.defender_type_id).cmp(&(&b.attacker_type_id, &b.defender_type_id))
    });
}
