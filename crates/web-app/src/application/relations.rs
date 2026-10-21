//! Reusable record authoring, exact reads, and reviewed explicit reconciliation.
use super::authoring::{self as author, Authoring, Error};
use crate::{
    accounts::Accounts,
    auth::Actor,
    persistence::{authoring as content, cards, relations as store},
};
pub use game_data::relations::*;
use game_data::{
    authoring::{ContentFields, validate_reason},
    cards::{CardIdentity, CardPin},
};
use sqlx::{Sqlite, SqliteConnection, Transaction};
use std::collections::HashSet;
mod search;

#[derive(Clone, Debug)]
pub struct NewLayout {
    pub fields: ContentFields,
    pub source: Option<LayoutPin>,
    pub columns: Vec<RecordColumn>,
    pub reason: Option<String>,
}
#[derive(Clone, Debug)]
pub struct LayoutEdit {
    pub expected_revision: i64,
    pub fields: ContentFields,
    pub columns: Vec<RecordColumn>,
    pub archived_at: Option<i64>,
    pub reason: Option<String>,
}
#[derive(Clone, Debug)]
pub struct NewCollection {
    pub fields: ContentFields,
    pub source: Option<CollectionPin>,
    pub layout: LayoutPin,
    pub reason: Option<String>,
}
#[derive(Clone, Debug)]
pub struct CollectionEdit {
    pub expected_revision: i64,
    pub fields: ContentFields,
    pub layout: LayoutPin,
    pub archived_at: Option<i64>,
    pub reason: Option<String>,
}
#[derive(Clone, Debug)]
pub struct RelationEdit {
    pub expected_revision: i64,
    pub relations: CardRelations,
    pub reason: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColumnMapping {
    pub source_column_id: String,
    pub local_column_id: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CollectionMapping {
    pub shared_collection_id: String,
    pub local_collection: CollectionPin,
    pub columns: Vec<ColumnMapping>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CellEvidence {
    pub collection_id: String,
    pub entry_id: String,
    pub cell: RecordCell,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RelationOperation {
    Reconcile {
        collection_id: String,
        target: CollectionPin,
        columns: Vec<ColumnMapping>,
    },
    CopySource {
        mappings: Vec<CollectionMapping>,
    },
}
/// Entire reviewed input/proposal/evidence is compared again in the writer transaction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelationPreview {
    pub entry: CardRelationRecord,
    pub source: Option<CardRelationRecord>,
    pub operation: RelationOperation,
    pub proposed: CardRelationRecord,
    pub removed: Vec<RecordCollection>,
    pub unmapped: Vec<RecordCollection>,
    pub removed_cells: Vec<CellEvidence>,
    pub unmapped_cells: Vec<CellEvidence>,
}
#[derive(Clone, Debug)]
pub struct Relations {
    accounts: Accounts,
    authoring: Authoring,
}
impl Relations {
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
    async fn actor(
        &self,
        c: &mut SqliteConnection,
        token: &str,
        w: Option<&str>,
    ) -> Result<Actor, Error> {
        let a = self.authoring.actor(c, token).await?;
        if let Some(w) = w {
            let world = content::world(c, w).await?;
            self.authoring.world_actor(c, &a, &world, false).await?;
        } else {
            author::require_admin(&a)?;
        }
        Ok(a)
    }
    pub async fn layouts(
        &self,
        world: Option<&str>,
        query: &str,
        page: usize,
    ) -> Result<Vec<LayoutRecord>, Error> {
        let mut tx = self.read().await?;
        if let Some(w) = world {
            content::world(&mut tx, w).await?;
        }
        let mut out = vec![];
        for id in store::ids(&mut tx, store::Kind::Layout, world, query, page).await? {
            out.push(store::layout(&mut tx, world, &id).await?);
        }
        Ok(out)
    }
    pub async fn collections(
        &self,
        world: Option<&str>,
        query: &str,
        page: usize,
    ) -> Result<Vec<CollectionDefinition>, Error> {
        let mut tx = self.read().await?;
        if let Some(w) = world {
            content::world(&mut tx, w).await?;
        }
        let mut out = vec![];
        for id in store::ids(&mut tx, store::Kind::Collection, world, query, page).await? {
            out.push(store::collection(&mut tx, world, &id).await?);
        }
        Ok(out)
    }
    pub async fn layout(&self, world: Option<&str>, id: &str) -> Result<LayoutRecord, Error> {
        store::layout(&mut *self.read().await?, world, id).await
    }
    pub async fn layout_version(&self, pin: &LayoutPin) -> Result<LayoutRecord, Error> {
        store::layout_version(&mut *self.read().await?, pin).await
    }
    pub async fn collection(
        &self,
        world: Option<&str>,
        id: &str,
    ) -> Result<CollectionDefinition, Error> {
        store::collection(&mut *self.read().await?, world, id).await
    }
    pub async fn collection_version(
        &self,
        pin: &CollectionPin,
    ) -> Result<CollectionDefinition, Error> {
        store::collection_version(&mut *self.read().await?, pin).await
    }
    pub async fn card(&self, pin: &CardPin) -> Result<CardRelationRecord, Error> {
        store::card_version(&mut *self.read().await?, pin).await
    }
    pub async fn layout_history(
        &self,
        world: Option<&str>,
        id: &str,
        before: Option<i64>,
    ) -> Result<Vec<super::cards::CardHistoryEntry>, Error> {
        store::history(
            &mut *self.read().await?,
            store::Kind::Layout,
            world,
            id,
            before,
        )
        .await
    }
    pub async fn collection_history(
        &self,
        world: Option<&str>,
        id: &str,
        before: Option<i64>,
    ) -> Result<Vec<super::cards::CardHistoryEntry>, Error> {
        store::history(
            &mut *self.read().await?,
            store::Kind::Collection,
            world,
            id,
            before,
        )
        .await
    }
    pub async fn card_history(
        &self,
        identity: &CardIdentity,
        before: Option<i64>,
    ) -> Result<Vec<super::cards::CardHistoryEntry>, Error> {
        let mut tx = self.read().await?;
        let out = cards::history(&mut tx, identity, before).await?;
        for e in &out {
            store::card_version(
                &mut tx,
                &CardPin {
                    identity: identity.clone(),
                    version: e.revision,
                },
            )
            .await?;
        }
        Ok(out)
    }
    pub async fn create_layout(
        &self,
        token: &str,
        world: Option<&str>,
        new: NewLayout,
    ) -> Result<LayoutRecord, Error> {
        let mut tx = self.authoring.write().await?;
        let a = self.actor(&mut tx, token, world).await?;
        let (mut fields, reason) = author::fields_and_reason(&new.fields, new.reason.as_deref())?;
        store::allocate_slug(&mut tx, store::Kind::Layout, world, &mut fields, None).await?;
        let row = LayoutRecord {
            id: author::new_id()?,
            world_id: world.map(str::to_owned),
            source: new.source,
            fields,
            revision: 1,
            archived_at: None,
            columns: new.columns,
        };
        row.validate().map_err(Error::Invalid)?;
        if let Some(s) = &row.source {
            store::selectable_layout(&mut tx, s).await?;
        }
        store::validate_columns(&mut tx, &row, None).await?;
        store::put_layout(&mut tx, &row, &a.user_id, "create", reason.as_deref()).await?;
        tx.commit().await.map_err(content::storage_error)?;
        Ok(row)
    }
    async fn edit_layout(
        c: &mut SqliteConnection,
        old: &LayoutRecord,
        edit: &LayoutEdit,
    ) -> Result<LayoutRecord, Error> {
        author::expected(old.revision, edit.expected_revision)?;
        let (mut fields, _) = author::fields_and_reason(&edit.fields, edit.reason.as_deref())?;
        if fields.display_name == old.fields.display_name {
            fields.slug = old.fields.slug.clone();
        } else {
            store::allocate_slug(
                c,
                store::Kind::Layout,
                old.world_id.as_deref(),
                &mut fields,
                Some(&old.id),
            )
            .await?;
        }
        let mut row = old.clone();
        row.fields = fields;
        row.columns = edit.columns.clone();
        row.archived_at = edit.archived_at;
        store::validate_columns(c, &row, Some(old)).await?;
        Ok(row)
    }
    pub async fn save_layout(
        &self,
        token: &str,
        world: Option<&str>,
        id: &str,
        edit: LayoutEdit,
    ) -> Result<LayoutRecord, Error> {
        self.write_layout(token, world, id, edit, None).await
    }
    /// Source adoption keeps the submitted local metadata/columns; never retargets consumers.
    pub async fn adopt_layout(
        &self,
        token: &str,
        world: &str,
        id: &str,
        target_version: i64,
        edit: LayoutEdit,
    ) -> Result<LayoutRecord, Error> {
        self.write_layout(token, Some(world), id, edit, Some(target_version))
            .await
    }
    async fn write_layout(
        &self,
        token: &str,
        world: Option<&str>,
        id: &str,
        edit: LayoutEdit,
        adopt: Option<i64>,
    ) -> Result<LayoutRecord, Error> {
        let mut tx = self.authoring.write().await?;
        let a = self.actor(&mut tx, token, world).await?;
        let old = store::layout(&mut tx, world, id).await?;
        author::expected(old.revision, edit.expected_revision)?;
        let mut row = Self::edit_layout(&mut tx, &old, &edit).await?;
        if let Some(v) = adopt {
            let p = old
                .source
                .as_ref()
                .ok_or_else(|| author::invalid("source", "This layout has no shared source."))?;
            if v < p.version {
                return Err(author::invalid(
                    "source",
                    "Choose the pinned or a newer shared revision.",
                ));
            }
            if v != p.version {
                row.source = Some(
                    store::selectable_layout(
                        &mut tx,
                        &LayoutPin {
                            version: v,
                            ..p.clone()
                        },
                    )
                    .await?
                    .pin(),
                );
            }
        }
        let reason = validate_reason(edit.reason.as_deref()).map_err(Error::Invalid)?;
        if row != old {
            row.revision = author::next(old.revision)?;
            store::put_layout(
                &mut tx,
                &row,
                &a.user_id,
                if adopt.is_some() { "adopt" } else { "edit" },
                reason.as_deref(),
            )
            .await?;
        }
        tx.commit().await.map_err(content::storage_error)?;
        Ok(row)
    }
    pub async fn create_collection(
        &self,
        token: &str,
        world: Option<&str>,
        new: NewCollection,
    ) -> Result<CollectionDefinition, Error> {
        let mut tx = self.authoring.write().await?;
        let a = self.actor(&mut tx, token, world).await?;
        let (mut fields, reason) = author::fields_and_reason(&new.fields, new.reason.as_deref())?;
        store::allocate_slug(&mut tx, store::Kind::Collection, world, &mut fields, None).await?;
        if new.layout.world_id.as_deref() != world {
            return Err(author::invalid(
                "layout",
                "Select a layout in this exact scope.",
            ));
        }
        let layout = store::selectable_layout(&mut tx, &new.layout).await?;
        let row = CollectionDefinition {
            id: author::new_id()?,
            world_id: world.map(str::to_owned),
            source: new.source,
            fields,
            revision: 1,
            archived_at: None,
            layout,
        };
        row.validate().map_err(Error::Invalid)?;
        if let Some(s) = &row.source {
            store::selectable_collection(&mut tx, s).await?;
        }
        store::put_collection(&mut tx, &row, &a.user_id, "create", reason.as_deref()).await?;
        tx.commit().await.map_err(content::storage_error)?;
        Ok(row)
    }
    pub async fn save_collection(
        &self,
        token: &str,
        world: Option<&str>,
        id: &str,
        edit: CollectionEdit,
    ) -> Result<CollectionDefinition, Error> {
        self.write_collection(token, world, id, edit, None).await
    }
    pub async fn adopt_collection(
        &self,
        token: &str,
        world: &str,
        id: &str,
        target_version: i64,
        edit: CollectionEdit,
    ) -> Result<CollectionDefinition, Error> {
        self.write_collection(token, Some(world), id, edit, Some(target_version))
            .await
    }
    async fn write_collection(
        &self,
        token: &str,
        world: Option<&str>,
        id: &str,
        edit: CollectionEdit,
        adopt: Option<i64>,
    ) -> Result<CollectionDefinition, Error> {
        let mut tx = self.authoring.write().await?;
        let a = self.actor(&mut tx, token, world).await?;
        let old = store::collection(&mut tx, world, id).await?;
        author::expected(old.revision, edit.expected_revision)?;
        let (mut fields, reason) = author::fields_and_reason(&edit.fields, edit.reason.as_deref())?;
        if fields.display_name == old.fields.display_name {
            fields.slug = old.fields.slug.clone();
        } else {
            store::allocate_slug(
                &mut tx,
                store::Kind::Collection,
                world,
                &mut fields,
                Some(id),
            )
            .await?;
        }
        if edit.layout.world_id.as_deref() != world {
            return Err(author::invalid(
                "layout",
                "Select a layout in this exact scope.",
            ));
        }
        let layout = if edit.layout == old.layout.pin() {
            old.layout.clone()
        } else {
            store::selectable_layout(&mut tx, &edit.layout).await?
        };
        let mut row = old.clone();
        row.fields = fields;
        row.archived_at = edit.archived_at;
        row.layout = layout;
        if let Some(v) = adopt {
            let p = old.source.as_ref().ok_or_else(|| {
                author::invalid("source", "This collection has no shared source.")
            })?;
            if v < p.version {
                return Err(author::invalid(
                    "source",
                    "Choose the pinned or a newer shared revision.",
                ));
            }
            if v != p.version {
                row.source = Some(
                    store::selectable_collection(
                        &mut tx,
                        &CollectionPin {
                            version: v,
                            ..p.clone()
                        },
                    )
                    .await?
                    .pin(),
                );
            }
        }
        row.validate().map_err(Error::Invalid)?;
        if row != old {
            row.revision = author::next(old.revision)?;
            store::put_collection(
                &mut tx,
                &row,
                &a.user_id,
                if adopt.is_some() { "adopt" } else { "edit" },
                reason.as_deref(),
            )
            .await?;
        }
        tx.commit().await.map_err(content::storage_error)?;
        Ok(row)
    }
    async fn save_in(
        c: &mut SqliteConnection,
        identity: &CardIdentity,
        edit: RelationEdit,
        actor: &str,
    ) -> Result<CardRelationRecord, Error> {
        let mut scalar = cards::card(c, identity).await?;
        author::expected(scalar.revision, edit.expected_revision)?;
        let old = store::card_version(c, &scalar.pin()).await?;
        let reason = validate_reason(edit.reason.as_deref()).map_err(Error::Invalid)?;
        store::validate_selection(c, identity, &edit.relations, &old.relations).await?;
        if old.relations == edit.relations {
            return Ok(old);
        }
        scalar.revision = author::next(scalar.revision)?;
        scalar.legacy_unmodeled = false;
        store::stage(c, &scalar.pin(), &edit.relations).await?;
        cards::put_card(c, &scalar, actor, "edit", reason.as_deref()).await?;
        store::card_version(c, &scalar.pin()).await
    }
    pub async fn save_card(
        &self,
        token: &str,
        identity: &CardIdentity,
        edit: RelationEdit,
    ) -> Result<CardRelationRecord, Error> {
        let mut tx = self.authoring.write().await?;
        let a = self
            .actor(&mut tx, token, identity.world_id.as_deref())
            .await?;
        let out = Self::save_in(&mut tx, identity, edit, &a.user_id).await?;
        tx.commit().await.map_err(content::storage_error)?;
        Ok(out)
    }
    async fn preview_in(
        c: &mut SqliteConnection,
        identity: &CardIdentity,
        expected: i64,
        operation: RelationOperation,
    ) -> Result<RelationPreview, Error> {
        let scalar = cards::card(c, identity).await?;
        author::expected(scalar.revision, expected)?;
        let entry = store::card_version(c, &scalar.pin()).await?;
        let mut p = RelationPreview {
            proposed: entry.clone(),
            entry,
            source: None,
            operation,
            removed: vec![],
            unmapped: vec![],
            removed_cells: vec![],
            unmapped_cells: vec![],
        };
        match &p.operation {
            RelationOperation::Reconcile {
                collection_id,
                target,
                columns,
            } => {
                let old = p
                    .entry
                    .relations
                    .collections
                    .iter()
                    .flatten()
                    .find(|x| &x.definition.id == collection_id)
                    .ok_or_else(|| {
                        author::invalid("collection", "Collection is absent from this card.")
                    })?;
                if target.world_id != identity.world_id || target.collection_id != *collection_id {
                    return Err(author::invalid(
                        "collection",
                        "Reconciliation retargets the same collection identity in this scope.",
                    ));
                }
                let definition = if target == &old.definition.pin() {
                    old.definition.clone()
                } else {
                    store::selectable_collection(c, target).await?
                };
                let (value, unmapped) = map_value(old, &definition, columns)?;
                p.removed_cells = unmapped.clone();
                p.unmapped_cells = unmapped;
                if let Some(cols) = &mut p.proposed.relations.collections {
                    let target = cols
                        .iter_mut()
                        .find(|x| &x.definition.id == collection_id)
                        .ok_or(Error::Unavailable)?;
                    *target = RecordCollection { definition, value };
                }
            }
            RelationOperation::CopySource { mappings } => {
                if identity.world_id.is_none() {
                    return Err(author::invalid(
                        "source",
                        "Only local cards copy shared source collections.",
                    ));
                }
                let source_pin = scalar.source.as_ref().ok_or_else(|| {
                    author::invalid("source", "This card has no exact shared source.")
                })?;
                let source = store::card_version(c, source_pin).await?;
                let mut shared = HashSet::new();
                let mut local = HashSet::new();
                let mut copied = vec![];
                for m in mappings {
                    if !shared.insert(&m.shared_collection_id)
                        || !local.insert(&m.local_collection.collection_id)
                        || m.local_collection.world_id != identity.world_id
                    {
                        return Err(author::invalid(
                            "mappings",
                            "Use explicit injective collection mappings in this world.",
                        ));
                    }
                    let from = source
                        .relations
                        .collections
                        .iter()
                        .flatten()
                        .find(|s| s.definition.id == m.shared_collection_id)
                        .ok_or_else(|| {
                            author::invalid(
                                "mappings",
                                "Mapped collection is absent from the exact source.",
                            )
                        })?;
                    let definition = store::collection_version(c, &m.local_collection).await?;
                    if definition.source.as_ref().is_none_or(|s| {
                        s.world_id.is_some() || s.collection_id != from.definition.id
                    }) || definition.layout.source.as_ref().is_none_or(|s| {
                        s.world_id.is_some() || s.layout_id != from.definition.layout.id
                    }) {
                        return Err(author::invalid(
                            "mappings",
                            "Destination collection and layout must explicitly source the corresponding shared identities.",
                        ));
                    }
                    let (value, unmapped) = map_value(from, &definition, &m.columns)?;
                    p.unmapped_cells.extend(unmapped);
                    copied.push(RecordCollection { definition, value });
                }
                copied.sort_by_key(|v| {
                    source
                        .relations
                        .collections
                        .iter()
                        .flatten()
                        .position(|s| {
                            v.definition
                                .source
                                .as_ref()
                                .is_some_and(|p| p.collection_id == s.definition.id)
                        })
                        .unwrap_or(usize::MAX)
                });
                p.unmapped = source
                    .relations
                    .collections
                    .iter()
                    .flatten()
                    .filter(|v| !shared.contains(&v.definition.id))
                    .cloned()
                    .collect();
                p.proposed.relations.collections =
                    source.relations.collections.as_ref().map(|_| copied);
                p.source = Some(source);
            }
        }
        store::validate_selection(c, identity, &p.proposed.relations, &p.entry.relations).await?;
        p.removed = p
            .entry
            .relations
            .collections
            .iter()
            .flatten()
            .filter(|v| {
                !p.proposed
                    .relations
                    .collections
                    .iter()
                    .flatten()
                    .any(|n| n == *v)
            })
            .cloned()
            .collect();
        // Destination evidence is separate from unmapped source occurrences.
        if matches!(p.operation, RelationOperation::CopySource { .. }) {
            for col in &p.removed {
                if let CollectionValue::Entries(es) = &col.value {
                    for e in es {
                        for cell in &e.cells {
                            p.removed_cells.push(CellEvidence {
                                collection_id: col.definition.id.clone(),
                                entry_id: e.id.clone(),
                                cell: cell.clone(),
                            });
                        }
                    }
                }
            }
        }
        if p.proposed.relations != p.entry.relations {
            p.proposed.card.version = author::next(expected)?;
            p.proposed.legacy_unmodeled = false;
        }
        Ok(p)
    }
    pub async fn preview_reconcile_card(
        &self,
        identity: &CardIdentity,
        expected_revision: i64,
        collection_id: &str,
        target: &CollectionPin,
        columns: &[ColumnMapping],
    ) -> Result<RelationPreview, Error> {
        Self::preview_in(
            &mut *self.read().await?,
            identity,
            expected_revision,
            RelationOperation::Reconcile {
                collection_id: collection_id.into(),
                target: target.clone(),
                columns: columns.to_vec(),
            },
        )
        .await
    }
    pub async fn preview_source_collections(
        &self,
        identity: &CardIdentity,
        expected_revision: i64,
        mappings: &[CollectionMapping],
    ) -> Result<RelationPreview, Error> {
        Self::preview_in(
            &mut *self.read().await?,
            identity,
            expected_revision,
            RelationOperation::CopySource {
                mappings: mappings.to_vec(),
            },
        )
        .await
    }
    pub async fn confirm_card(
        &self,
        token: &str,
        reviewed: &RelationPreview,
        reason: Option<&str>,
    ) -> Result<CardRelationRecord, Error> {
        let mut tx = self.authoring.write().await?;
        let i = &reviewed.entry.card.identity;
        let a = self.actor(&mut tx, token, i.world_id.as_deref()).await?;
        let fresh = Self::preview_in(
            &mut tx,
            i,
            reviewed.entry.card.version,
            reviewed.operation.clone(),
        )
        .await?;
        if &fresh != reviewed {
            return Err(Error::Conflict);
        }
        let out = Self::save_in(
            &mut tx,
            i,
            RelationEdit {
                expected_revision: reviewed.entry.card.version,
                relations: fresh.proposed.relations,
                reason: reason.map(str::to_owned),
            },
            &a.user_id,
        )
        .await?;
        tx.commit().await.map_err(content::storage_error)?;
        Ok(out)
    }
}
fn map_value(
    from: &RecordCollection,
    to: &CollectionDefinition,
    mappings: &[ColumnMapping],
) -> Result<(CollectionValue, Vec<CellEvidence>), Error> {
    let mut shared = HashSet::new();
    let mut local = HashSet::new();
    for m in mappings {
        if !shared.insert(&m.source_column_id)
            || !local.insert(&m.local_column_id)
            || !from
                .definition
                .layout
                .columns
                .iter()
                .any(|c| c.id == m.source_column_id)
            || !to.layout.columns.iter().any(|c| c.id == m.local_column_id)
        {
            return Err(author::invalid(
                "mappings",
                "Use declared distinct source columns and injective destination columns.",
            ));
        }
    }
    let mut unmapped = vec![];
    let value = if let CollectionValue::Entries(es) = &from.value {
        let mut entries = vec![];
        for e in es {
            let mut cells = vec![];
            for cell in &e.cells {
                if let Some(m) = mappings
                    .iter()
                    .find(|m| m.source_column_id == cell.column_id)
                {
                    cells.push(RecordCell {
                        column_id: m.local_column_id.clone(),
                        value: cell.value.clone(),
                    });
                } else {
                    unmapped.push(CellEvidence {
                        collection_id: from.definition.id.clone(),
                        entry_id: e.id.clone(),
                        cell: cell.clone(),
                    });
                }
            }
            entries.push(RecordEntry {
                id: e.id.clone(),
                cells,
            });
        }
        CollectionValue::Entries(entries)
    } else {
        from.value.clone()
    };
    RecordCollection {
        definition: to.clone(),
        value: value.clone(),
    }
    .validate()
    .map_err(Error::Invalid)?;
    Ok((value, unmapped))
}
