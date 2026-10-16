//! Move authoring and independent world rulesets. Policy is checked under writer lock.
use super::authoring::{self as author, AddressKey, Authoring, Error, HistoryEntry};
use crate::{
    accounts::Accounts,
    auth::Actor,
    persistence::{
        authoring as content, charts,
        moves::{self as store, Kind},
        stats,
    },
};
use game_data::authoring::{slug_candidate, slug_from_name, validate_reason};
pub use game_data::{authoring::ContentFields, moves::*, rulesets::*};
use sqlx::{Sqlite, SqliteConnection, Transaction};
use std::collections::HashSet;

#[derive(Clone, Debug)]
pub struct NewAttribute {
    pub fields: ContentFields,
    pub unit: String,
    pub constraints: NumericConstraints,
    pub source: Option<AttributePin>,
    pub reason: Option<String>,
}
#[derive(Clone, Debug)]
pub struct AttributeEdit {
    pub expected_revision: i64,
    pub fields: ContentFields,
    pub unit: String,
    pub constraints: NumericConstraints,
    pub reason: Option<String>,
}
#[derive(Clone, Debug)]
pub struct AttributeAdoptionPreview {
    pub entry: MoveAttributeDefinition,
    pub old_source: MoveAttributeDefinition,
    pub new_source: MoveAttributeDefinition,
    pub proposed: MoveAttributeDefinition,
}
#[derive(Clone, Debug)]
pub struct AttributeValueInput {
    pub attribute: AttributePin,
    pub value: NumericValue,
}
#[derive(Clone, Debug)]
pub struct NewMove {
    pub fields: ContentFields,
    pub source: Option<MovePin>,
    pub attributes: Option<Vec<AttributeValueInput>>,
    pub reason: Option<String>,
}
#[derive(Clone, Debug)]
pub struct MoveEdit {
    pub expected_revision: i64,
    pub fields: ContentFields,
    pub attributes: Option<Vec<AttributeValueInput>>,
    pub reason: Option<String>,
}
#[derive(Clone, Debug)]
pub struct MoveAdoptEdit {
    pub expected_revision: i64,
    pub target_version: i64,
    pub fields: ContentFields,
    pub apply_source: bool,
    pub mappings: Vec<AttributeMapping>,
    pub reason: Option<String>,
}
#[derive(Clone, Debug)]
pub struct MoveAdoptionPreview {
    pub entry: MoveRecord,
    pub old_source: MoveRecord,
    pub new_source: MoveRecord,
    pub proposed: MoveRecord,
    /// Source assignments omitted by the explicit mapping, in source order.
    pub unmapped: Vec<MoveAttributeValue>,
    /// Local assignments removed by whole-collection replacement, in local order.
    pub removed: Vec<MoveAttributeValue>,
}
#[derive(Clone, Debug)]
pub struct ExpectationInput {
    pub attribute: AttributePin,
    pub required: bool,
    pub allow_not_applicable: bool,
    pub constraints: NumericConstraints,
}
#[derive(Clone, Debug)]
pub struct RulesetEdit {
    pub expected_revision: i64,
    pub fields: ContentFields,
    pub expectations: Option<Vec<ExpectationInput>>,
    pub stat_roster: Option<game_data::stats::RosterPin>,
    pub chart: Option<game_data::charts::ChartPin>,
    pub max_moves: Option<i64>,
    pub repeat_policy: RepeatPolicy,
    pub notes: String,
    pub reason: Option<String>,
}
#[derive(Clone, Debug)]
pub struct Moves {
    accounts: Accounts,
    authoring: Authoring,
}
impl Moves {
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
        world: Option<&str>,
    ) -> Result<Actor, Error> {
        let a = self.authoring.actor(c, token).await?;
        if let Some(w) = world {
            let w = content::world(c, w).await?;
            self.authoring.world_actor(c, &a, &w, false).await?;
        } else {
            author::require_admin(&a)?;
        }
        Ok(a)
    }
    pub async fn attributes(
        &self,
        world: Option<&str>,
    ) -> Result<Vec<MoveAttributeDefinition>, Error> {
        let mut tx = self.read().await?;
        if let Some(w) = world {
            content::world(&mut tx, w).await?;
        }
        let mut out = vec![];
        for id in store::ids(&mut tx, Kind::Attribute, world).await? {
            out.push(store::attribute(&mut tx, world, &id).await?);
        }
        Ok(out)
    }
    pub async fn attribute(
        &self,
        world: Option<&str>,
        id: &str,
    ) -> Result<MoveAttributeDefinition, Error> {
        store::attribute(&mut *self.read().await?, world, id).await
    }
    pub async fn attribute_version(
        &self,
        world: Option<&str>,
        id: &str,
        version: i64,
    ) -> Result<MoveAttributeDefinition, Error> {
        store::attribute_version(&mut *self.read().await?, world, id, version).await
    }
    pub async fn list(&self, world: Option<&str>) -> Result<Vec<MoveRecord>, Error> {
        let mut tx = self.read().await?;
        if let Some(w) = world {
            content::world(&mut tx, w).await?;
        }
        let mut out = vec![];
        for id in store::ids(&mut tx, Kind::Move, world).await? {
            out.push(store::move_detail(&mut tx, world, &id).await?);
        }
        Ok(out)
    }
    pub async fn detail(&self, world: Option<&str>, id: &str) -> Result<MoveRecord, Error> {
        store::move_detail(&mut *self.read().await?, world, id).await
    }
    pub async fn version(
        &self,
        world: Option<&str>,
        id: &str,
        version: i64,
    ) -> Result<MoveRecord, Error> {
        store::move_version(&mut *self.read().await?, world, id, version).await
    }
    pub async fn ruleset(&self, world: &str) -> Result<Option<RulesetRecord>, Error> {
        store::ruleset(&mut *self.read().await?, world).await
    }
    pub async fn ruleset_version(
        &self,
        world: &str,
        id: &str,
        version: i64,
    ) -> Result<RulesetRecord, Error> {
        store::ruleset_version(&mut *self.read().await?, world, id, version).await
    }
    pub async fn resolve_attribute(
        &self,
        world: Option<&str>,
        key: AddressKey,
    ) -> Result<MoveAttributeDefinition, Error> {
        let mut tx = self.read().await?;
        let id = store::resolve_id(&mut tx, Kind::Attribute, world, key).await?;
        store::attribute(&mut tx, world, &id).await
    }
    pub async fn resolve_move(
        &self,
        world: Option<&str>,
        key: AddressKey,
    ) -> Result<MoveRecord, Error> {
        let mut tx = self.read().await?;
        let id = store::resolve_id(&mut tx, Kind::Move, world, key).await?;
        store::move_detail(&mut tx, world, &id).await
    }
    async fn new_attribute(
        c: &mut SqliteConnection,
        world: Option<&str>,
        new: &NewAttribute,
    ) -> Result<MoveAttributeDefinition, Error> {
        let (mut fields, _) = author::fields_and_reason(&new.fields, new.reason.as_deref())?;
        allocate(c, Kind::Attribute, world, &mut fields, None).await?;
        if let Some(p) = &new.source {
            if world.is_none() {
                return Err(author::invalid(
                    "source",
                    "Shared attributes cannot have a source.",
                ));
            }
            store::select_attribute(c, None, p)
                .await
                .map_err(|e| author::missing_reference(e, "source"))?;
        }
        let row = MoveAttributeDefinition {
            id: author::new_id()?,
            world_id: world.map(str::to_owned),
            source: new.source.clone(),
            fields,
            unit: new.unit.trim().into(),
            revision: 1,
            archived_at: None,
            constraints: new.constraints.clone(),
        };
        row.validate().map_err(Error::Invalid)?;
        Ok(row)
    }
    pub async fn create_attribute(
        &self,
        token: &str,
        world: Option<&str>,
        new: NewAttribute,
    ) -> Result<MoveAttributeDefinition, Error> {
        let mut tx = self.authoring.write().await?;
        let a = self.actor(&mut tx, token, world).await?;
        let row = Self::new_attribute(&mut tx, world, &new).await?;
        let reason = validate_reason(new.reason.as_deref()).map_err(Error::Invalid)?;
        store::put_attribute(&mut tx, &row, Some(&a.user_id), "create", reason.as_deref()).await?;
        tx.commit().await.map_err(content::storage_error)?;
        Ok(row)
    }
    async fn edit_attribute(
        c: &mut SqliteConnection,
        old: &MoveAttributeDefinition,
        edit: &AttributeEdit,
    ) -> Result<MoveAttributeDefinition, Error> {
        author::expected(old.revision, edit.expected_revision)?;
        let (mut fields, _) = author::fields_and_reason(&edit.fields, edit.reason.as_deref())?;
        allocate(
            c,
            Kind::Attribute,
            old.world_id.as_deref(),
            &mut fields,
            Some((&old.id, &old.fields)),
        )
        .await?;
        let mut row = old.clone();
        row.fields = fields;
        row.unit = edit.unit.trim().into();
        row.constraints = edit.constraints.clone();
        row.validate().map_err(Error::Invalid)?;
        Ok(row)
    }
    pub async fn save_attribute(
        &self,
        token: &str,
        world: Option<&str>,
        id: &str,
        edit: AttributeEdit,
    ) -> Result<MoveAttributeDefinition, Error> {
        let mut tx = self.authoring.write().await?;
        let a = self.actor(&mut tx, token, world).await?;
        let old = store::attribute(&mut tx, world, id).await?;
        let mut row = Self::edit_attribute(&mut tx, &old, &edit).await?;
        let reason = validate_reason(edit.reason.as_deref()).map_err(Error::Invalid)?;
        if row != old {
            row.revision = author::next(old.revision)?;
            store::put_attribute(&mut tx, &row, Some(&a.user_id), "edit", reason.as_deref())
                .await?;
        }
        tx.commit().await.map_err(content::storage_error)?;
        Ok(row)
    }
    async fn attribute_preview_in(
        c: &mut SqliteConnection,
        world: &str,
        id: &str,
        target_version: i64,
        edit: &AttributeEdit,
    ) -> Result<AttributeAdoptionPreview, Error> {
        let entry = store::attribute(c, Some(world), id).await?;
        author::expected(entry.revision, edit.expected_revision)?;
        let p = entry
            .source
            .as_ref()
            .ok_or_else(|| author::invalid("source", "This attribute has no shared source."))?;
        if target_version < p.version {
            return Err(author::invalid(
                "target_version",
                "Choose the pinned or a newer revision of the same source.",
            ));
        }
        let old_source = store::attribute_version(c, None, &p.attribute_id, p.version).await?;
        let new_source = store::select_attribute(
            c,
            None,
            &AttributePin {
                attribute_id: p.attribute_id.clone(),
                version: target_version,
            },
        )
        .await
        .map_err(|e| author::missing_reference(e, "target_version"))?;
        let mut proposed = Self::edit_attribute(c, &entry, edit).await?;
        proposed.source = Some(new_source.pin());
        if proposed != entry {
            proposed.revision = author::next(entry.revision)?;
        }
        proposed.validate().map_err(Error::Invalid)?;
        Ok(AttributeAdoptionPreview {
            entry,
            old_source,
            new_source,
            proposed,
        })
    }
    pub async fn preview_attribute_adoption(
        &self,
        world: &str,
        id: &str,
        target_version: i64,
        edit: &AttributeEdit,
    ) -> Result<AttributeAdoptionPreview, Error> {
        Self::attribute_preview_in(&mut *self.read().await?, world, id, target_version, edit).await
    }
    pub async fn adopt_attribute(
        &self,
        token: &str,
        world: &str,
        id: &str,
        target_version: i64,
        edit: AttributeEdit,
    ) -> Result<MoveAttributeDefinition, Error> {
        let mut tx = self.authoring.write().await?;
        let a = self.actor(&mut tx, token, Some(world)).await?;
        let p = Self::attribute_preview_in(&mut tx, world, id, target_version, &edit).await?;
        let reason = validate_reason(edit.reason.as_deref()).map_err(Error::Invalid)?;
        if p.proposed != p.entry {
            store::put_attribute(
                &mut tx,
                &p.proposed,
                Some(&a.user_id),
                "adopt",
                reason.as_deref(),
            )
            .await?;
        }
        tx.commit().await.map_err(content::storage_error)?;
        Ok(p.proposed)
    }
    pub async fn create(
        &self,
        token: &str,
        world: Option<&str>,
        new: NewMove,
    ) -> Result<MoveRecord, Error> {
        let mut tx = self.authoring.write().await?;
        let a = self.actor(&mut tx, token, world).await?;
        let (mut fields, reason) = author::fields_and_reason(&new.fields, new.reason.as_deref())?;
        allocate(&mut tx, Kind::Move, world, &mut fields, None).await?;
        if let Some(p) = &new.source {
            if world.is_none() {
                return Err(author::invalid(
                    "source",
                    "Shared moves cannot have a source.",
                ));
            }
            store::select_move(&mut tx, None, p)
                .await
                .map_err(|e| author::missing_reference(e, "source"))?;
        }
        let attributes = values(&mut tx, world, new.attributes.as_deref(), None).await?;
        let row = MoveRecord {
            id: author::new_id()?,
            world_id: world.map(str::to_owned),
            source: new.source,
            fields,
            revision: 1,
            archived_at: None,
            attributes,
            mappings: vec![],
        };
        row.validate().map_err(Error::Invalid)?;
        store::put_move(&mut tx, &row, Some(&a.user_id), "create", reason.as_deref()).await?;
        tx.commit().await.map_err(content::storage_error)?;
        Ok(row)
    }
    pub async fn save(
        &self,
        token: &str,
        world: Option<&str>,
        id: &str,
        edit: MoveEdit,
    ) -> Result<MoveRecord, Error> {
        let mut tx = self.authoring.write().await?;
        let a = self.actor(&mut tx, token, world).await?;
        let old = store::move_detail(&mut tx, world, id).await?;
        author::expected(old.revision, edit.expected_revision)?;
        let (mut fields, reason) = author::fields_and_reason(&edit.fields, edit.reason.as_deref())?;
        allocate(
            &mut tx,
            Kind::Move,
            world,
            &mut fields,
            Some((&old.id, &old.fields)),
        )
        .await?;
        let mut row = old.clone();
        row.fields = fields;
        row.attributes = values(
            &mut tx,
            world,
            edit.attributes.as_deref(),
            old.attributes.as_deref(),
        )
        .await?;
        row.validate().map_err(Error::Invalid)?;
        if row != old {
            row.revision = author::next(old.revision)?;
            store::put_move(&mut tx, &row, Some(&a.user_id), "edit", reason.as_deref()).await?;
        }
        tx.commit().await.map_err(content::storage_error)?;
        Ok(row)
    }
    async fn adoption_in(
        c: &mut SqliteConnection,
        world: &str,
        id: &str,
        edit: &MoveAdoptEdit,
    ) -> Result<MoveAdoptionPreview, Error> {
        let entry = store::move_detail(c, Some(world), id).await?;
        author::expected(entry.revision, edit.expected_revision)?;
        let p = entry
            .source
            .as_ref()
            .ok_or_else(|| author::invalid("source", "This move has no shared source."))?;
        if edit.target_version < p.version {
            return Err(author::invalid(
                "target_version",
                "Choose the pinned or a newer revision of the same source.",
            ));
        }
        let old_source = store::move_version(c, None, &p.move_id, p.version).await?;
        let new_source = store::select_move(
            c,
            None,
            &MovePin {
                move_id: p.move_id.clone(),
                version: edit.target_version,
            },
        )
        .await
        .map_err(|e| author::missing_reference(e, "target_version"))?;
        let (mut fields, _) = author::fields_and_reason(&edit.fields, edit.reason.as_deref())?;
        allocate(
            c,
            Kind::Move,
            Some(world),
            &mut fields,
            Some((&entry.id, &entry.fields)),
        )
        .await?;
        let (mut shared, mut local) = (HashSet::new(), HashSet::new());
        let mut selected = vec![];
        for m in &edit.mappings {
            if !shared.insert(&m.shared_attribute_id)
                || !local.insert(&m.local_attribute.attribute_id)
            {
                return Err(author::invalid(
                    "mappings",
                    "Map each shared and local identity at most once.",
                ));
            }
            if !new_source
                .attributes
                .iter()
                .flatten()
                .any(|a| a.definition.id == m.shared_attribute_id)
            {
                return Err(author::invalid(
                    "mappings",
                    "A mapped shared attribute is absent from the selected source revision.",
                ));
            }
            let d = store::select_attribute(c, Some(world), &m.local_attribute)
                .await
                .map_err(|e| author::missing_reference(e, "mappings"))?;
            if !d
                .source
                .as_ref()
                .is_some_and(|p| p.attribute_id == m.shared_attribute_id)
            {
                return Err(author::invalid(
                    "mappings",
                    "The local attribute must explicitly pin this shared source identity.",
                ));
            }
            selected.push((m, d));
        }
        let unmapped = new_source
            .attributes
            .iter()
            .flatten()
            .filter(|a| !shared.contains(&a.definition.id))
            .cloned()
            .collect();
        let mut proposed = entry.clone();
        proposed.fields = fields;
        proposed.source = Some(new_source.pin());
        proposed.mappings = edit.mappings.clone();
        if edit.apply_source {
            proposed.attributes = match &new_source.attributes {
                None => None,
                Some(source) => {
                    let mut copied = vec![];
                    for a in source {
                        if let Some((_, d)) = selected
                            .iter()
                            .find(|(m, _)| m.shared_attribute_id == a.definition.id)
                        {
                            copied.push(MoveAttributeValue {
                                definition: d.clone(),
                                value: a.value.clone(),
                            });
                        }
                    }
                    Some(copied)
                }
            };
        }
        let removed = if edit.apply_source {
            entry
                .attributes
                .iter()
                .flatten()
                .filter(|a| {
                    !proposed
                        .attributes
                        .iter()
                        .flatten()
                        .any(|b| a.definition.id == b.definition.id)
                })
                .cloned()
                .collect()
        } else {
            vec![]
        };
        proposed.validate().map_err(Error::Invalid)?;
        if proposed != entry {
            proposed.revision = author::next(entry.revision)?;
        }
        Ok(MoveAdoptionPreview {
            entry,
            old_source,
            new_source,
            proposed,
            unmapped,
            removed,
        })
    }
    pub async fn adoption_preview(
        &self,
        world: &str,
        id: &str,
        edit: &MoveAdoptEdit,
    ) -> Result<MoveAdoptionPreview, Error> {
        Self::adoption_in(&mut *self.read().await?, world, id, edit).await
    }
    pub async fn adopt(
        &self,
        token: &str,
        world: &str,
        id: &str,
        edit: MoveAdoptEdit,
    ) -> Result<MoveRecord, Error> {
        let mut tx = self.authoring.write().await?;
        let a = self.actor(&mut tx, token, Some(world)).await?;
        let p = Self::adoption_in(&mut tx, world, id, &edit).await?;
        let reason = validate_reason(edit.reason.as_deref()).map_err(Error::Invalid)?;
        if p.proposed != p.entry {
            store::put_move(
                &mut tx,
                &p.proposed,
                Some(&a.user_id),
                "adopt",
                reason.as_deref(),
            )
            .await?;
        }
        tx.commit().await.map_err(content::storage_error)?;
        Ok(p.proposed)
    }
    pub async fn save_ruleset(
        &self,
        token: &str,
        world: &str,
        edit: RulesetEdit,
    ) -> Result<RulesetRecord, Error> {
        let mut tx = self.authoring.write().await?;
        let a = self.actor(&mut tx, token, Some(world)).await?;
        let old = store::ruleset(&mut tx, world).await?;
        author::expected(
            old.as_ref().map_or(0, |r| r.revision),
            edit.expected_revision,
        )?;
        let (mut fields, reason) = author::fields_and_reason(&edit.fields, edit.reason.as_deref())?;
        fields.slug = if let Some(r) = &old
            && r.fields.display_name == fields.display_name
        {
            r.fields.slug.clone()
        } else {
            slug_from_name(&fields.display_name)
        };
        let expectations = match edit.expectations {
            None => None,
            Some(inputs) => {
                let mut entries = vec![];
                for e in inputs {
                    let retained = old
                        .as_ref()
                        .into_iter()
                        .flat_map(|r| r.expectations.iter().flatten())
                        .any(|saved| saved.definition.pin() == e.attribute);
                    entries.push(AttributeExpectation {
                        definition: attribute_for_edit(
                            &mut tx,
                            Some(world),
                            &e.attribute,
                            retained,
                        )
                        .await
                        .map_err(|e| author::missing_reference(e, "expectations"))?,
                        required: e.required,
                        allow_not_applicable: e.allow_not_applicable,
                        constraints: e.constraints,
                    });
                }
                Some(entries)
            }
        };
        if let Some(p) = &edit.stat_roster {
            stats::roster_version(&mut tx, world, &p.roster_id, p.version)
                .await
                .map_err(|e| author::missing_reference(e, "stat_roster"))?;
        }
        if let Some(p) = &edit.chart {
            let r = charts::version(&mut tx, Some(world), &p.chart_id, p.version)
                .await
                .map_err(|e| author::missing_reference(e, "chart"))?;
            if old.as_ref().and_then(|r| r.chart.as_ref()) != Some(p) {
                let head = charts::detail(&mut tx, Some(world), &p.chart_id)
                    .await
                    .map_err(|e| author::missing_reference(e, "chart"))?;
                if head.archived_at.is_some() || r.archived_at.is_some() {
                    return Err(author::invalid(
                        "chart",
                        "Choose an unarchived chart revision.",
                    ));
                }
            }
        }
        let mut row = RulesetRecord {
            id: match &old {
                Some(r) => r.id.clone(),
                None => author::new_id()?,
            },
            world_id: world.into(),
            revision: old.as_ref().map_or(1, |r| r.revision),
            fields,
            expectations,
            stat_roster: edit.stat_roster,
            chart: edit.chart,
            max_moves: edit.max_moves,
            repeat_policy: edit.repeat_policy,
            notes: edit.notes.trim().replace("\r\n", "\n"),
        };
        row.validate().map_err(Error::Invalid)?;
        if old.as_ref() != Some(&row) {
            if let Some(old) = old {
                row.revision = author::next(old.revision)?;
            }
            store::put_ruleset(&mut tx, &row, Some(&a.user_id), reason.as_deref()).await?;
        }
        tx.commit().await.map_err(content::storage_error)?;
        Ok(row)
    }
    pub async fn attribute_history(
        &self,
        world: Option<&str>,
        id: &str,
        before: Option<i64>,
    ) -> Result<Vec<HistoryEntry>, Error> {
        store::history(
            &mut *self.read().await?,
            Kind::Attribute,
            world,
            id,
            before,
            None,
        )
        .await
    }
    pub async fn history(
        &self,
        world: Option<&str>,
        id: &str,
        before: Option<i64>,
    ) -> Result<Vec<HistoryEntry>, Error> {
        store::history(
            &mut *self.read().await?,
            Kind::Move,
            world,
            id,
            before,
            None,
        )
        .await
    }
    pub async fn ruleset_history(
        &self,
        world: &str,
        before: Option<i64>,
    ) -> Result<Vec<HistoryEntry>, Error> {
        let mut tx = self.read().await?;
        let Some(r) = store::ruleset(&mut tx, world).await? else {
            return Ok(vec![]);
        };
        store::history(&mut tx, Kind::Ruleset, Some(world), &r.id, before, None).await
    }
    pub async fn attribute_history_entry(
        &self,
        world: Option<&str>,
        id: &str,
        revision: i64,
    ) -> Result<HistoryEntry, Error> {
        store::history(
            &mut *self.read().await?,
            Kind::Attribute,
            world,
            id,
            None,
            Some(revision),
        )
        .await?
        .into_iter()
        .next()
        .ok_or(Error::NotFound)
    }
    pub async fn history_entry(
        &self,
        world: Option<&str>,
        id: &str,
        revision: i64,
    ) -> Result<HistoryEntry, Error> {
        store::history(
            &mut *self.read().await?,
            Kind::Move,
            world,
            id,
            None,
            Some(revision),
        )
        .await?
        .into_iter()
        .next()
        .ok_or(Error::NotFound)
    }
    pub async fn ruleset_history_entry(
        &self,
        world: &str,
        revision: i64,
    ) -> Result<HistoryEntry, Error> {
        let mut tx = self.read().await?;
        let r = store::ruleset(&mut tx, world)
            .await?
            .ok_or(Error::NotFound)?;
        store::history(
            &mut tx,
            Kind::Ruleset,
            Some(world),
            &r.id,
            None,
            Some(revision),
        )
        .await?
        .into_iter()
        .next()
        .ok_or(Error::NotFound)
    }
    /// Complete-fixture creation baseline; caller owns the outer transaction.
    pub async fn seed_attribute_in(
        &self,
        c: &mut SqliteConnection,
        row: &MoveAttributeDefinition,
    ) -> Result<bool, Error> {
        row.validate().map_err(Error::Invalid)?;
        if row.revision != 1 || row.archived_at.is_some() {
            return Err(fixture_error());
        }
        store::attribute_dependencies(c, row).await?;
        if store::fixture_baseline(
            c,
            Kind::Attribute,
            row.world_id.as_deref(),
            &row.id,
            &author::snapshot(row)?,
        )
        .await?
        {
            return Ok(false);
        }
        baseline_slug(
            c,
            Kind::Attribute,
            row.world_id.as_deref(),
            &row.fields.slug,
        )
        .await?;
        store::put_attribute(c, row, None, "create", None).await?;
        Ok(true)
    }
    pub async fn seed_move_in(
        &self,
        c: &mut SqliteConnection,
        row: &MoveRecord,
    ) -> Result<bool, Error> {
        row.validate().map_err(Error::Invalid)?;
        if row.revision != 1 || row.archived_at.is_some() {
            return Err(fixture_error());
        }
        store::move_dependencies(c, row).await?;
        if store::fixture_baseline(
            c,
            Kind::Move,
            row.world_id.as_deref(),
            &row.id,
            &author::snapshot(row)?,
        )
        .await?
        {
            return Ok(false);
        }
        baseline_slug(c, Kind::Move, row.world_id.as_deref(), &row.fields.slug).await?;
        store::put_move(c, row, None, "create", None).await?;
        Ok(true)
    }
    pub async fn seed_ruleset_in(
        &self,
        c: &mut SqliteConnection,
        row: &RulesetRecord,
    ) -> Result<bool, Error> {
        row.validate().map_err(Error::Invalid)?;
        if row.revision != 1 {
            return Err(fixture_error());
        }
        store::ruleset_dependencies(c, row).await?;
        if let Some(head) = store::ruleset(c, &row.world_id).await?
            && head.id != row.id
        {
            return Err(fixture_error());
        }
        if store::fixture_baseline(
            c,
            Kind::Ruleset,
            Some(&row.world_id),
            &row.id,
            &author::snapshot(row)?,
        )
        .await?
        {
            return Ok(false);
        }
        store::put_ruleset(c, row, None, None).await?;
        Ok(true)
    }
}
async fn values(
    c: &mut SqliteConnection,
    world: Option<&str>,
    inputs: Option<&[AttributeValueInput]>,
    saved: Option<&[MoveAttributeValue]>,
) -> Result<Option<Vec<MoveAttributeValue>>, Error> {
    let Some(inputs) = inputs else {
        return Ok(None);
    };
    let mut out = vec![];
    for a in inputs {
        let retained = saved
            .iter()
            .flat_map(|rows| rows.iter())
            .any(|v| v.definition.pin() == a.attribute);
        out.push(MoveAttributeValue {
            definition: attribute_for_edit(c, world, &a.attribute, retained)
                .await
                .map_err(|e| author::missing_reference(e, "attributes"))?,
            value: a.value.clone(),
        });
    }
    Ok(Some(out))
}
async fn attribute_for_edit(
    c: &mut SqliteConnection,
    world: Option<&str>,
    pin: &AttributePin,
    retained: bool,
) -> Result<MoveAttributeDefinition, Error> {
    // Retaining an exact binding does not select a new dependency. Historical
    // readers still verify its head, snapshot, typed rows, audit and dependencies.
    if retained {
        store::attribute_version(c, world, &pin.attribute_id, pin.version).await
    } else {
        store::select_attribute(c, world, pin).await
    }
}
async fn allocate(
    c: &mut SqliteConnection,
    kind: Kind,
    world: Option<&str>,
    fields: &mut ContentFields,
    old: Option<(&str, &ContentFields)>,
) -> Result<(), Error> {
    if let Some((_, f)) = old
        && f.display_name == fields.display_name
    {
        fields.slug = f.slug.clone();
        return Ok(());
    }
    let base = slug_from_name(&fields.display_name);
    for n in 1..=10_000 {
        let slug = slug_candidate(&base, n);
        if !store::slug_exists(c, kind, world, &slug, old.map(|(id, _)| id)).await? {
            fields.slug = slug;
            return Ok(());
        }
    }
    Err(author::invalid(
        "display_name",
        "Too many objects share this address. Choose another name.",
    ))
}
fn fixture_error() -> Error {
    author::invalid(
        "fixtures",
        "The move/ruleset creation baseline is invalid or collides with existing content.",
    )
}
async fn baseline_slug(
    c: &mut SqliteConnection,
    kind: Kind,
    world: Option<&str>,
    slug: &str,
) -> Result<(), Error> {
    if store::slug_exists(c, kind, world, slug, None).await? {
        return Err(fixture_error());
    }
    store::check_baseline_slug(c, kind, world, slug).await
}
