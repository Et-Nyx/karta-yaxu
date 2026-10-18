//! Same-identity card authoring. All permissions are checked under BEGIN IMMEDIATE.
use super::authoring::{self as author, AddressKey, Authoring, Error};
use crate::{
    accounts::Accounts,
    auth::Actor,
    persistence::{authoring as content, cards as store},
};
use game_data::authoring::{ContentFields, validate_reason};
pub use game_data::{card_fields::*, cards::*};
use sqlx::{Sqlite, SqliteConnection, Transaction};
use std::collections::HashSet;

mod search;
pub use search::*;

/// Public attribution only; identities, private audit data and snapshot bytes stay out.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, sqlx::FromRow)]
pub struct CardHistoryEntry {
    pub revision: i64,
    pub actor_name: String,
    pub action: String,
    pub occurred_at: String,
    pub reason: Option<String>,
}

#[derive(Clone, Debug)]
pub struct NewCard {
    pub fields: ContentFields,
    pub source: Option<CardPin>,
    pub component: CardComponent,
    pub reason: Option<String>,
}
#[derive(Clone, Debug)]
pub struct CardEdit {
    pub expected_revision: i64,
    pub component: CardComponent,
    pub reason: Option<String>,
}
#[derive(Clone, Debug)]
pub struct CardMetadataEdit {
    pub expected_revision: i64,
    pub fields: ContentFields,
    pub archived_at: Option<i64>,
    pub reason: Option<String>,
}
#[derive(Clone, Debug)]
pub struct NewField {
    pub fields: ContentFields,
    pub kind: FieldKind,
    pub source: Option<FieldPin>,
    pub reason: Option<String>,
}
#[derive(Clone, Debug)]
pub struct FieldEdit {
    pub expected_revision: i64,
    pub fields: ContentFields,
    pub kind: FieldKind,
    pub archived_at: Option<i64>,
    pub reason: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldMapping {
    pub shared_field_id: String,
    pub local_field: FieldPin,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldCopyPreview {
    pub entry: CardRecord,
    pub source: CardRecord,
    pub mappings: Vec<FieldMapping>,
    pub proposed: CardRecord,
    pub removed: Vec<CardField>,
    pub unmapped: Vec<CardField>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldAdoptionPreview {
    pub entry: FieldDefinition,
    pub old_source: FieldDefinition,
    pub new_source: FieldDefinition,
    pub proposed: FieldDefinition,
}
#[derive(Clone, Debug)]
pub struct Cards {
    accounts: Accounts,
    authoring: Authoring,
}
impl Cards {
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
    pub async fn list(&self, world: Option<&str>, page: usize) -> Result<Vec<CardRecord>, Error> {
        let mut tx = self.read().await?;
        if let Some(w) = world {
            content::world(&mut tx, w).await?;
        }
        let mut out = vec![];
        for i in store::identities(&mut tx, world, page).await? {
            out.push(store::card(&mut tx, &i).await?);
        }
        Ok(out)
    }
    pub async fn detail(&self, identity: &CardIdentity) -> Result<CardRecord, Error> {
        store::card(&mut *self.read().await?, identity).await
    }
    /// Descending exact revisions, at most 25; `before` is an exclusive cursor.
    pub async fn history(
        &self,
        identity: &CardIdentity,
        before: Option<i64>,
    ) -> Result<Vec<CardHistoryEntry>, Error> {
        store::history(&mut *self.read().await?, identity, before).await
    }
    pub async fn field_history(
        &self,
        world: Option<&str>,
        id: &str,
        before: Option<i64>,
    ) -> Result<Vec<CardHistoryEntry>, Error> {
        store::field_history(&mut *self.read().await?, world, id, before).await
    }
    /// Form IDs are accepted; Form slugs require a species-qualified native route.
    pub async fn resolve_card(
        &self,
        world: Option<&str>,
        namespace: CardNamespace,
        key: AddressKey,
    ) -> Result<CardRecord, Error> {
        store::resolve_card(&mut *self.read().await?, world, namespace, key).await
    }
    pub async fn resolve_field(
        &self,
        world: Option<&str>,
        key: AddressKey,
    ) -> Result<FieldDefinition, Error> {
        store::resolve_field(&mut *self.read().await?, world, key).await
    }
    pub async fn standalone(
        &self,
        world: Option<&str>,
        page: usize,
    ) -> Result<Vec<CardRecord>, Error> {
        let mut tx = self.read().await?;
        if let Some(w) = world {
            content::world(&mut tx, w).await?;
        }
        let mut out = vec![];
        for id in store::standalone_ids(&mut tx, world, page).await? {
            out.push(
                store::card(
                    &mut tx,
                    &CardIdentity {
                        namespace: CardNamespace::Standalone,
                        world_id: world.map(str::to_owned),
                        id,
                    },
                )
                .await?,
            );
        }
        Ok(out)
    }
    pub async fn version(&self, pin: &CardPin) -> Result<CardRecord, Error> {
        store::card_version(&mut *self.read().await?, pin).await
    }
    pub async fn fields(
        &self,
        world: Option<&str>,
        page: usize,
    ) -> Result<Vec<FieldDefinition>, Error> {
        let mut tx = self.read().await?;
        if let Some(w) = world {
            content::world(&mut tx, w).await?;
        }
        let mut out = vec![];
        for id in store::field_ids(&mut tx, world, page).await? {
            out.push(store::field(&mut tx, world, &id).await?);
        }
        Ok(out)
    }
    pub async fn field(&self, world: Option<&str>, id: &str) -> Result<FieldDefinition, Error> {
        store::field(&mut *self.read().await?, world, id).await
    }
    pub async fn field_version(&self, pin: &FieldPin) -> Result<FieldDefinition, Error> {
        store::field_version(
            &mut *self.read().await?,
            pin.world_id.as_deref(),
            &pin.field_id,
            pin.version,
        )
        .await
    }
    pub async fn create(
        &self,
        token: &str,
        world: Option<&str>,
        new: NewCard,
    ) -> Result<CardRecord, Error> {
        let mut tx = self.authoring.write().await?;
        let a = self.actor(&mut tx, token, world).await?;
        let (mut fields, reason) = author::fields_and_reason(&new.fields, new.reason.as_deref())?;
        store::allocate_slug(&mut tx, world, &mut fields, None, false).await?;
        if let Some(p) = &new.source {
            if world.is_none()
                || p.identity.world_id.is_some()
                || p.identity.namespace != CardNamespace::Standalone
            {
                return Err(author::invalid(
                    "source",
                    "Local standalone cards may pin a shared standalone source.",
                ));
            }
            selectable_card(&mut tx, p).await?;
        }
        let row = CardRecord {
            identity: CardIdentity {
                namespace: CardNamespace::Standalone,
                world_id: world.map(str::to_owned),
                id: author::new_id()?,
            },
            revision: 1,
            fields,
            archived_at: None,
            source: new.source,
            component: new.component,
            legacy_unmodeled: false,
        };
        store::validate_selection(
            &mut tx,
            &row.identity,
            &row.component,
            &CardComponent::default(),
        )
        .await?;
        store::put_card(&mut tx, &row, &a.user_id, "create", reason.as_deref()).await?;
        tx.commit().await.map_err(content::storage_error)?;
        Ok(row)
    }
    pub async fn save_card(
        &self,
        token: &str,
        identity: &CardIdentity,
        edit: CardEdit,
    ) -> Result<CardRecord, Error> {
        let mut tx = self.authoring.write().await?;
        let a = self
            .actor(&mut tx, token, identity.world_id.as_deref())
            .await?;
        let old = store::card(&mut tx, identity).await?;
        author::expected(old.revision, edit.expected_revision)?;
        let reason = validate_reason(edit.reason.as_deref()).map_err(Error::Invalid)?;
        store::validate_selection(&mut tx, identity, &edit.component, &old.component).await?;
        let mut row = old.clone();
        row.component = edit.component;
        if row.component != old.component {
            row.revision = author::next(old.revision)?;
            row.legacy_unmodeled = false;
            store::put_card(&mut tx, &row, &a.user_id, "edit", reason.as_deref()).await?;
        }
        tx.commit().await.map_err(content::storage_error)?;
        Ok(row)
    }
    /// Standalone metadata only; specialized metadata continues through its native editor.
    pub async fn save_metadata(
        &self,
        token: &str,
        identity: &CardIdentity,
        edit: CardMetadataEdit,
    ) -> Result<CardRecord, Error> {
        let mut tx = self.authoring.write().await?;
        let a = self
            .actor(&mut tx, token, identity.world_id.as_deref())
            .await?;
        let old = store::card(&mut tx, identity).await?;
        author::expected(old.revision, edit.expected_revision)?;
        if identity.namespace != CardNamespace::Standalone {
            return Err(author::invalid(
                "card",
                "Use the native metadata editor for this definition.",
            ));
        }
        let (mut fields, reason) = author::fields_and_reason(&edit.fields, edit.reason.as_deref())?;
        if fields.display_name == old.fields.display_name {
            fields.slug = old.fields.slug.clone();
        } else {
            store::allocate_slug(
                &mut tx,
                identity.world_id.as_deref(),
                &mut fields,
                Some(&identity.id),
                false,
            )
            .await?;
        }
        let mut row = old.clone();
        row.fields = fields;
        row.archived_at = edit.archived_at;
        if row != old {
            row.revision = author::next(old.revision)?;
            store::put_card(&mut tx, &row, &a.user_id, "edit", reason.as_deref()).await?;
        }
        tx.commit().await.map_err(content::storage_error)?;
        Ok(row)
    }
    pub async fn create_field(
        &self,
        token: &str,
        world: Option<&str>,
        new: NewField,
    ) -> Result<FieldDefinition, Error> {
        let mut tx = self.authoring.write().await?;
        let a = self.actor(&mut tx, token, world).await?;
        let (mut fields, reason) = author::fields_and_reason(&new.fields, new.reason.as_deref())?;
        store::allocate_slug(&mut tx, world, &mut fields, None, true).await?;
        if let Some(p) = &new.source {
            if world.is_none() || p.world_id.is_some() {
                return Err(author::invalid(
                    "source",
                    "Local fields may pin shared sources only.",
                ));
            }
            selectable_field(&mut tx, p).await?;
        }
        let row = FieldDefinition {
            id: author::new_id()?,
            world_id: world.map(str::to_owned),
            source: new.source,
            fields,
            revision: 1,
            archived_at: None,
            kind: new.kind,
        };
        store::put_field(&mut tx, &row, &a.user_id, "create", reason.as_deref()).await?;
        tx.commit().await.map_err(content::storage_error)?;
        Ok(row)
    }
    async fn edit_field(
        c: &mut SqliteConnection,
        old: &FieldDefinition,
        edit: &FieldEdit,
    ) -> Result<FieldDefinition, Error> {
        author::expected(old.revision, edit.expected_revision)?;
        let (mut fields, _) = author::fields_and_reason(&edit.fields, edit.reason.as_deref())?;
        if fields.display_name == old.fields.display_name {
            fields.slug = old.fields.slug.clone();
        } else {
            store::allocate_slug(c, old.world_id.as_deref(), &mut fields, Some(&old.id), true)
                .await?;
        }
        let mut row = old.clone();
        row.fields = fields;
        row.kind = edit.kind.clone();
        row.archived_at = edit.archived_at;
        row.validate().map_err(Error::Invalid)?;
        Ok(row)
    }
    pub async fn save_field(
        &self,
        token: &str,
        world: Option<&str>,
        id: &str,
        edit: FieldEdit,
    ) -> Result<FieldDefinition, Error> {
        let mut tx = self.authoring.write().await?;
        let a = self.actor(&mut tx, token, world).await?;
        let old = store::field(&mut tx, world, id).await?;
        let mut row = Self::edit_field(&mut tx, &old, &edit).await?;
        let reason = validate_reason(edit.reason.as_deref()).map_err(Error::Invalid)?;
        if row != old {
            row.revision = author::next(old.revision)?;
            store::put_field(&mut tx, &row, &a.user_id, "edit", reason.as_deref()).await?;
        }
        tx.commit().await.map_err(content::storage_error)?;
        Ok(row)
    }
    async fn field_adoption_in(
        c: &mut SqliteConnection,
        world: &str,
        id: &str,
        target_version: i64,
        edit: &FieldEdit,
    ) -> Result<FieldAdoptionPreview, Error> {
        let entry = store::field(c, Some(world), id).await?;
        author::expected(entry.revision, edit.expected_revision)?;
        let pin = entry
            .source
            .as_ref()
            .ok_or_else(|| author::invalid("source", "This field has no shared source."))?;
        if target_version < pin.version {
            return Err(author::invalid(
                "source",
                "Choose the pinned or a newer source revision.",
            ));
        }
        let old_source = store::field_version(c, None, &pin.field_id, pin.version).await?;
        let new_source = if target_version == pin.version {
            old_source.clone()
        } else {
            selectable_field(
                c,
                &FieldPin {
                    version: target_version,
                    ..pin.clone()
                },
            )
            .await?
        };
        // The reviewed local metadata/kind are authoritative: adoption defaults keep-local.
        let mut proposed = Self::edit_field(c, &entry, edit).await?;
        proposed.source = Some(new_source.pin());
        if proposed != entry {
            proposed.revision = author::next(entry.revision)?;
        }
        Ok(FieldAdoptionPreview {
            entry,
            old_source,
            new_source,
            proposed,
        })
    }
    pub async fn preview_field_adoption(
        &self,
        world: &str,
        id: &str,
        target_version: i64,
        edit: &FieldEdit,
    ) -> Result<FieldAdoptionPreview, Error> {
        Self::field_adoption_in(&mut *self.read().await?, world, id, target_version, edit).await
    }
    pub async fn adopt_field(
        &self,
        token: &str,
        world: &str,
        id: &str,
        target_version: i64,
        edit: FieldEdit,
    ) -> Result<FieldDefinition, Error> {
        let mut tx = self.authoring.write().await?;
        let a = self.actor(&mut tx, token, Some(world)).await?;
        let p = Self::field_adoption_in(&mut tx, world, id, target_version, &edit).await?;
        let reason = validate_reason(edit.reason.as_deref()).map_err(Error::Invalid)?;
        if p.entry != p.proposed {
            store::put_field(&mut tx, &p.proposed, &a.user_id, "adopt", reason.as_deref()).await?;
        }
        tx.commit().await.map_err(content::storage_error)?;
        Ok(p.proposed)
    }
    async fn copy_in(
        c: &mut SqliteConnection,
        i: &CardIdentity,
        expected: i64,
        mappings: &[FieldMapping],
    ) -> Result<FieldCopyPreview, Error> {
        let entry = store::card(c, i).await?;
        author::expected(entry.revision, expected)?;
        if i.world_id.is_none() {
            return Err(author::invalid(
                "source",
                "Only local cards can copy shared source fields.",
            ));
        }
        let pin = entry
            .source
            .as_ref()
            .ok_or_else(|| author::invalid("source", "This card has no pinned shared source."))?;
        let source = store::card_version(c, pin).await?;
        let mut shared = HashSet::new();
        let mut local = HashSet::new();
        let mut copied = vec![];
        for m in mappings {
            if !shared.insert(&m.shared_field_id)
                || !local.insert(&m.local_field.field_id)
                || m.local_field.world_id != i.world_id
            {
                return Err(author::invalid(
                    "mappings",
                    "Use injective explicit mappings to fields in this world.",
                ));
            }
            let from = source
                .component
                .fields
                .iter()
                .flatten()
                .find(|f| f.definition.id == m.shared_field_id)
                .ok_or_else(|| {
                    author::invalid("mappings", "Mapped field is absent from the pinned source.")
                })?;
            let definition = store::field_version(
                c,
                m.local_field.world_id.as_deref(),
                &m.local_field.field_id,
                m.local_field.version,
            )
            .await?;
            if definition
                .source
                .as_ref()
                .is_none_or(|p| p.field_id != from.definition.id || p.world_id.is_some())
            {
                return Err(author::invalid(
                    "mappings",
                    "Local definitions must explicitly source the mapped shared field identity.",
                ));
            }
            copied.push(CardField {
                definition,
                value: from.value.clone(),
            });
        }
        // Preserve the source assignment order, independently of mapping input order.
        copied.sort_by_key(|f| {
            source
                .component
                .fields
                .iter()
                .flatten()
                .position(|s| {
                    f.definition
                        .source
                        .as_ref()
                        .is_some_and(|p| p.field_id == s.definition.id)
                })
                .unwrap_or(usize::MAX)
        });
        let mut proposed = entry.clone();
        proposed.component.fields = source.component.fields.as_ref().map(|_| copied);
        store::validate_selection(c, i, &proposed.component, &entry.component).await?;
        let removed = entry
            .component
            .fields
            .iter()
            .flatten()
            .filter(|f| !proposed.component.fields.iter().flatten().any(|p| p == *f))
            .cloned()
            .collect();
        let unmapped = source
            .component
            .fields
            .iter()
            .flatten()
            .filter(|f| !shared.contains(&f.definition.id))
            .cloned()
            .collect();
        if proposed.component != entry.component {
            proposed.revision = author::next(entry.revision)?;
            proposed.legacy_unmodeled = false;
        }
        Ok(FieldCopyPreview {
            entry,
            source,
            mappings: mappings.to_vec(),
            proposed,
            removed,
            unmapped,
        })
    }
    pub async fn preview_source_fields(
        &self,
        identity: &CardIdentity,
        expected_revision: i64,
        mappings: &[FieldMapping],
    ) -> Result<FieldCopyPreview, Error> {
        Self::copy_in(
            &mut *self.read().await?,
            identity,
            expected_revision,
            mappings,
        )
        .await
    }
    /// Confirm the complete reviewed proposal, including removed/unmapped fields.
    pub async fn copy_source_fields(
        &self,
        token: &str,
        reviewed: &FieldCopyPreview,
        reason: Option<&str>,
    ) -> Result<CardRecord, Error> {
        let mut tx = self.authoring.write().await?;
        let a = self
            .actor(&mut tx, token, reviewed.entry.identity.world_id.as_deref())
            .await?;
        let fresh = Self::copy_in(
            &mut tx,
            &reviewed.entry.identity,
            reviewed.entry.revision,
            &reviewed.mappings,
        )
        .await?;
        if &fresh != reviewed {
            return Err(Error::Conflict);
        }
        let reason = validate_reason(reason).map_err(Error::Invalid)?;
        if fresh.proposed != fresh.entry {
            store::put_card(
                &mut tx,
                &fresh.proposed,
                &a.user_id,
                "edit",
                reason.as_deref(),
            )
            .await?;
        }
        tx.commit().await.map_err(content::storage_error)?;
        Ok(fresh.proposed)
    }
    /// Explicit source retargeting for standalone cards; the local component is kept.
    pub async fn adopt_card(
        &self,
        token: &str,
        identity: &CardIdentity,
        expected_revision: i64,
        target_version: i64,
        reason: Option<&str>,
    ) -> Result<CardRecord, Error> {
        let mut tx = self.authoring.write().await?;
        let a = self
            .actor(&mut tx, token, identity.world_id.as_deref())
            .await?;
        let old = store::card(&mut tx, identity).await?;
        author::expected(old.revision, expected_revision)?;
        if identity.namespace != CardNamespace::Standalone {
            return Err(author::invalid(
                "source",
                "Use the native source adoption operation.",
            ));
        }
        let pin = old
            .source
            .as_ref()
            .ok_or_else(|| author::invalid("source", "This card has no shared source."))?;
        if target_version < pin.version {
            return Err(author::invalid(
                "source",
                "Choose the pinned or a newer source revision.",
            ));
        }
        let mut row = old.clone();
        if target_version != pin.version {
            let source = selectable_card(
                &mut tx,
                &CardPin {
                    version: target_version,
                    ..pin.clone()
                },
            )
            .await?;
            row.source = Some(source.pin());
            row.revision = author::next(old.revision)?;
        }
        let reason = validate_reason(reason).map_err(Error::Invalid)?;
        if row != old {
            store::put_card(&mut tx, &row, &a.user_id, "adopt", reason.as_deref()).await?;
        }
        tx.commit().await.map_err(content::storage_error)?;
        Ok(row)
    }
}
async fn selectable_field(
    c: &mut SqliteConnection,
    p: &FieldPin,
) -> Result<FieldDefinition, Error> {
    p.validate().map_err(Error::Invalid)?;
    let row = store::field_version(c, p.world_id.as_deref(), &p.field_id, p.version)
        .await
        .map_err(|e| author::missing_reference(e, "source"))?;
    if row.archived_at.is_some()
        || store::field(c, p.world_id.as_deref(), &p.field_id)
            .await?
            .archived_at
            .is_some()
    {
        return Err(author::invalid(
            "source",
            "Choose an active field definition.",
        ));
    }
    Ok(row)
}
async fn selectable_card(c: &mut SqliteConnection, p: &CardPin) -> Result<CardRecord, Error> {
    p.validate().map_err(Error::Invalid)?;
    let row = store::card_version(c, p)
        .await
        .map_err(|e| author::missing_reference(e, "source"))?;
    if row.archived_at.is_some() || store::card(c, &p.identity).await?.archived_at.is_some() {
        return Err(author::invalid(
            "source",
            "Choose an active card definition.",
        ));
    }
    Ok(row)
}
