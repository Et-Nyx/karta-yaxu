//! Independent learnsets and preset authoring. All writes lock, authenticate and
//! check the expected revision before interpreting submitted dependencies.
use super::authoring::{self as author, AddressKey, Authoring, Error, HistoryEntry};
use crate::{
    accounts::Accounts,
    auth::Actor,
    persistence::{authoring as content, learning as store, moves as core},
};
use game_data::authoring::{slug_candidate, slug_from_name, validate_reason};
use sqlx::{Sqlite, SqliteConnection, Transaction};

pub use game_data::{
    authoring::ContentFields,
    learning::*,
    moves::{MovePin, NumericValue},
    rulesets::{RulesetPin, ValidationReport},
};

/// Preview-only identity, never persisted. Acquisition IDs copied from a source
/// remain stable in their separate local learnset context.
pub const PROVISIONAL_ID: &str = "00000000000000000000000000000000";

#[derive(Clone, Debug)]
pub struct LearningRecordInput {
    pub id: String,
    pub move_pin: MovePin,
    pub method: Option<String>,
    pub context: Option<String>,
    pub condition: Option<String>,
    pub threshold: NumericValue,
}
#[derive(Clone, Debug)]
pub struct LearnsetEdit {
    pub expected_revision: i64,
    pub records: Option<Vec<LearningRecordInput>>,
    pub reason: Option<String>,
}
#[derive(Clone, Debug)]
pub struct LearnsetAdoptEdit {
    pub expected_revision: i64,
    pub source: LearnsetPin,
    pub apply_source: bool,
    pub mappings: Vec<MoveMapping>,
    pub reason: Option<String>,
}
#[derive(Clone, Debug)]
pub struct LearnsetAdoptionPreview {
    pub entry: Option<LearnsetRecord>,
    pub old_source: Option<LearnsetRecord>,
    pub new_source: LearnsetRecord,
    pub proposed: LearnsetRecord,
}
#[derive(Clone, Debug)]
pub struct MovesetEdit {
    pub expected_revision: i64,
    pub fields: ContentFields,
    pub learnset: LearnsetPin,
    pub ruleset: RulesetPin,
    pub entries: Vec<String>,
    pub reason: Option<String>,
}
#[derive(Clone, Debug)]
pub struct PresetPreview {
    pub entry: Option<MovesetRecord>,
    pub proposed: MovesetRecord,
    pub report: ValidationReport,
}
#[derive(Clone, Debug)]
pub struct Learning {
    accounts: Accounts,
    authoring: Authoring,
}
impl Learning {
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
    pub async fn learnset(
        &self,
        world: Option<&str>,
        owner: &str,
    ) -> Result<Option<LearnsetRecord>, Error> {
        store::learnset(&mut *self.read().await?, world, owner).await
    }
    pub async fn learnset_version(
        &self,
        world: Option<&str>,
        id: &str,
        version: i64,
    ) -> Result<LearnsetRecord, Error> {
        store::learnset_version(&mut *self.read().await?, world, id, version).await
    }
    pub async fn shared_learnsets(&self) -> Result<Vec<LearnsetRecord>, Error> {
        store::shared_learnsets(&mut *self.read().await?).await
    }
    pub async fn presets(&self, world: &str, variant: &str) -> Result<Vec<MovesetRecord>, Error> {
        store::presets(&mut *self.read().await?, world, variant).await
    }
    pub async fn preset(
        &self,
        world: &str,
        variant: &str,
        id: &str,
    ) -> Result<MovesetRecord, Error> {
        store::preset(&mut *self.read().await?, world, variant, id).await
    }
    pub async fn preset_version(
        &self,
        world: &str,
        variant: &str,
        id: &str,
        version: i64,
    ) -> Result<MovesetRecord, Error> {
        store::preset_version(&mut *self.read().await?, world, variant, id, version).await
    }
    pub async fn resolve_preset(
        &self,
        world: &str,
        variant: &str,
        key: AddressKey,
    ) -> Result<MovesetRecord, Error> {
        store::resolve_preset(&mut *self.read().await?, world, variant, key).await
    }
    pub async fn learnset_history(
        &self,
        world: Option<&str>,
        owner: &str,
        before: Option<i64>,
    ) -> Result<Vec<HistoryEntry>, Error> {
        store::learnset_history(&mut *self.read().await?, world, owner, before).await
    }
    pub async fn preset_history(
        &self,
        world: &str,
        variant: &str,
        id: &str,
        before: Option<i64>,
    ) -> Result<Vec<HistoryEntry>, Error> {
        store::preset_history(&mut *self.read().await?, world, variant, id, before).await
    }
    async fn actor(
        &self,
        c: &mut SqliteConnection,
        token: &str,
        world: Option<&str>,
        owner: &str,
    ) -> Result<Actor, Error> {
        let actor = self.authoring.actor(c, token).await?;
        if let Some(w) = world {
            let row = content::world(c, w).await?;
            self.authoring.world_actor(c, &actor, &row, false).await?;
            if content::variant(c, w, owner).await?.archived_at.is_some() {
                return Err(author::invalid(
                    "owner",
                    "Unarchive the variant before editing its learning data.",
                ));
            }
        } else {
            author::require_admin(&actor)?;
            if content::form(c, owner).await?.archived_at.is_some() {
                return Err(author::invalid(
                    "owner",
                    "Unarchive the form before editing its learning data.",
                ));
            }
        }
        Ok(actor)
    }
    pub async fn save_learnset(
        &self,
        token: &str,
        world: Option<&str>,
        owner: &str,
        edit: LearnsetEdit,
    ) -> Result<LearnsetRecord, Error> {
        let mut tx = self.authoring.write().await?;
        let actor = self.actor(&mut tx, token, world, owner).await?;
        let old = store::learnset(&mut tx, world, owner).await?;
        author::expected(
            old.as_ref().map_or(0, |r| r.revision),
            edit.expected_revision,
        )?;
        let reason = validate_reason(edit.reason.as_deref()).map_err(Error::Invalid)?;
        let mut row = old.clone().unwrap_or_else(|| empty_learnset(world, owner));
        let mut records = vec![];
        for input in edit.records.iter().flatten() {
            let saved = old
                .as_ref()
                .into_iter()
                .flat_map(|s| s.records.iter().flatten())
                .find(|r| r.id == input.id);
            let id = if input.id.is_empty() {
                author::new_id()?
            } else {
                if saved.is_none() {
                    return Err(author::invalid(
                        "records",
                        "Keep an existing acquisition ID, or leave it empty to create a new record.",
                    ));
                }
                input.id.clone()
            };
            // Retention belongs to this acquisition ID, not another acquisition
            // of the same Move. Additions and retargets remain new selections.
            let move_record = if saved.is_some_and(|r| r.move_record.pin() == input.move_pin) {
                core::move_version(
                    &mut tx,
                    world,
                    &input.move_pin.move_id,
                    input.move_pin.version,
                )
                .await
            } else {
                core::select_move(&mut tx, world, &input.move_pin).await
            }
            .map_err(|e| author::missing_reference(e, "records"))?;
            records.push(LearningRecord {
                id,
                move_record,
                method: normalize_text(&input.method),
                context: normalize_text(&input.context),
                condition: normalize_text(&input.condition),
                threshold: input.threshold.clone(),
            });
        }
        row.records = edit.records.as_ref().map(|_| records);
        row.validate().map_err(Error::Invalid)?;
        if old.as_ref() != Some(&row) {
            if let Some(old) = &old {
                row.revision = author::next(old.revision)?;
            } else {
                row.id = author::new_id()?;
            }
            store::put_learnset(
                &mut tx,
                &row,
                Some(&actor.user_id),
                if old.is_some() { "edit" } else { "create" },
                reason.as_deref(),
            )
            .await?;
        }
        tx.commit().await.map_err(content::storage_error)?;
        Ok(row)
    }
    async fn adoption(
        c: &mut SqliteConnection,
        world: &str,
        variant: &str,
        edit: &LearnsetAdoptEdit,
    ) -> Result<LearnsetAdoptionPreview, Error> {
        let entry = store::learnset(c, Some(world), variant).await?;
        author::expected(
            entry.as_ref().map_or(0, |r| r.revision),
            edit.expected_revision,
        )?;
        validate_reason(edit.reason.as_deref()).map_err(Error::Invalid)?;
        edit.source.validate().map_err(Error::Invalid)?;
        let old_source = if let Some(pin) = entry.as_ref().and_then(|r| r.source.as_ref()) {
            if pin.learnset_id != edit.source.learnset_id || edit.source.version < pin.version {
                return Err(author::invalid(
                    "source",
                    "Select the current or a newer revision of the same shared learnset.",
                ));
            }
            Some(store::learnset_version(c, None, &pin.learnset_id, pin.version).await?)
        } else {
            None
        };
        let new_source =
            store::learnset_version(c, None, &edit.source.learnset_id, edit.source.version)
                .await
                .map_err(|e| author::missing_reference(e, "source"))?;
        let mut proposed = entry
            .clone()
            .unwrap_or_else(|| empty_learnset(Some(world), variant));
        proposed.source = Some(edit.source.clone());
        proposed.mappings = edit.mappings.clone();
        proposed
            .mappings
            .sort_by(|a, b| a.shared_move_id.cmp(&b.shared_move_id));
        // Validate injectivity and provenance even for keep-local. Never choose
        // between duplicate local source candidates by name or iteration order.
        proposed.validate().map_err(Error::Invalid)?;
        for mapping in &proposed.mappings {
            if !new_source
                .records
                .iter()
                .flatten()
                .any(|r| r.move_record.id == mapping.shared_move_id)
            {
                return Err(author::invalid(
                    "mappings",
                    "Map only moves in the selected shared learnset.",
                ));
            }
            let local = core::select_move(c, Some(world), &mapping.local_move)
                .await
                .map_err(|e| author::missing_reference(e, "mappings"))?;
            if local.source.as_ref().map(|p| &p.move_id) != Some(&mapping.shared_move_id) {
                return Err(author::invalid(
                    "mappings",
                    "Each local move must explicitly pin the mapped shared identity.",
                ));
            }
        }
        if edit.apply_source {
            let mut records = vec![];
            for source in new_source.records.iter().flatten() {
                let mapping = proposed
                    .mappings
                    .iter()
                    .find(|m| m.shared_move_id == source.move_record.id)
                    .ok_or_else(|| {
                        author::invalid(
                            "mappings",
                            "Copying requires an explicit mapping for every source move.",
                        )
                    })?;
                let mut record = source.clone();
                record.move_record = core::select_move(c, Some(world), &mapping.local_move).await?;
                records.push(record);
            }
            proposed.records = new_source.records.as_ref().map(|_| records);
        }
        proposed.validate().map_err(Error::Invalid)?;
        if let Some(old) = &entry
            && &proposed != old
        {
            proposed.revision = author::next(old.revision)?;
        }
        Ok(LearnsetAdoptionPreview {
            entry,
            old_source,
            new_source,
            proposed,
        })
    }
    pub async fn preview_adoption(
        &self,
        world: &str,
        variant: &str,
        edit: &LearnsetAdoptEdit,
    ) -> Result<LearnsetAdoptionPreview, Error> {
        Self::adoption(&mut *self.read().await?, world, variant, edit).await
    }
    pub async fn adopt_learnset(
        &self,
        token: &str,
        world: &str,
        variant: &str,
        edit: LearnsetAdoptEdit,
    ) -> Result<LearnsetRecord, Error> {
        let mut tx = self.authoring.write().await?;
        let actor = self.actor(&mut tx, token, Some(world), variant).await?;
        let preview = Self::adoption(&mut tx, world, variant, &edit).await?;
        let mut row = preview.proposed;
        let reason = validate_reason(edit.reason.as_deref()).map_err(Error::Invalid)?;
        if preview.entry.as_ref() != Some(&row) {
            if preview.entry.is_none() {
                row.id = author::new_id()?;
            }
            store::put_learnset(
                &mut tx,
                &row,
                Some(&actor.user_id),
                "adopt",
                reason.as_deref(),
            )
            .await?;
        }
        tx.commit().await.map_err(content::storage_error)?;
        Ok(row)
    }
    async fn preset_proposal(
        c: &mut SqliteConnection,
        world: &str,
        variant: &str,
        id: Option<&str>,
        edit: &MovesetEdit,
        revalidate: bool,
    ) -> Result<PresetPreview, Error> {
        store::owner(c, Some(world), variant).await?;
        let entry = if let Some(id) = id {
            Some(store::preset(c, world, variant, id).await?)
        } else {
            None
        };
        author::expected(
            entry.as_ref().map_or(0, |r| r.revision),
            edit.expected_revision,
        )?;
        let (mut fields, _) = author::fields_and_reason(&edit.fields, edit.reason.as_deref())?;
        if let Some(old) = &entry
            && fields.display_name == old.fields.display_name
        {
            fields.slug = old.fields.slug.clone();
        } else {
            let base = slug_from_name(&fields.display_name);
            let mut allocated = false;
            for number in 1..=10_000 {
                let slug = slug_candidate(&base, number);
                if !store::slug_exists(c, world, variant, &slug, id).await? {
                    fields.slug = slug;
                    allocated = true;
                    break;
                }
            }
            if !allocated {
                return Err(author::invalid(
                    "display_name",
                    "Too many presets share this address; choose a different name.",
                ));
            }
        }
        let mut proposed = MovesetRecord {
            id: entry
                .as_ref()
                .map_or_else(|| PROVISIONAL_ID.into(), |r| r.id.clone()),
            world_id: world.into(),
            variant_id: variant.into(),
            fields,
            revision: entry.as_ref().map_or(1, |r| r.revision),
            archived_at: entry.as_ref().and_then(|r| r.archived_at),
            learnset: edit.learnset.clone(),
            ruleset: edit.ruleset.clone(),
            entries: edit.entries.clone(),
            // A metadata/context edit is not implicit permission to reinterpret
            // the saved report. Policy advancement is a separate reviewed choice.
            validator_version: if revalidate {
                VALIDATOR_VERSION
            } else {
                entry
                    .as_ref()
                    .map_or(VALIDATOR_VERSION, |r| r.validator_version)
            },
        };
        if let Some(old) = &entry
            && &proposed != old
        {
            proposed.revision = author::next(old.revision)?;
        }
        let (learnset, ruleset) = store::preset_dependencies(c, &proposed)
            .await
            .map_err(|e| author::missing_reference(e, "dependencies"))?;
        check_preset_selections(c, entry.as_ref(), &proposed, &learnset).await?;
        let report = validate_preset(&proposed, &learnset, &ruleset);
        Ok(PresetPreview {
            entry,
            proposed,
            report,
        })
    }
    pub async fn preview_preset(
        &self,
        world: &str,
        variant: &str,
        id: Option<&str>,
        edit: &MovesetEdit,
    ) -> Result<PresetPreview, Error> {
        Self::preset_proposal(&mut *self.read().await?, world, variant, id, edit, false).await
    }
    /// Preview an explicitly requested current-policy report as a new revision.
    pub async fn preview_preset_revalidation(
        &self,
        world: &str,
        variant: &str,
        id: &str,
        edit: &MovesetEdit,
    ) -> Result<PresetPreview, Error> {
        Self::preset_proposal(
            &mut *self.read().await?,
            world,
            variant,
            Some(id),
            edit,
            true,
        )
        .await
    }
    pub async fn create_preset(
        &self,
        token: &str,
        world: &str,
        variant: &str,
        edit: MovesetEdit,
    ) -> Result<MovesetRecord, Error> {
        self.write_preset(token, world, variant, None, edit, false)
            .await
    }
    pub async fn save_preset(
        &self,
        token: &str,
        world: &str,
        variant: &str,
        id: &str,
        edit: MovesetEdit,
    ) -> Result<MovesetRecord, Error> {
        self.write_preset(token, world, variant, Some(id), edit, false)
            .await
    }
    /// Revalidation retains exact dependencies and records the selected current
    /// policy together with any reviewed edits in one attributed revision.
    pub async fn revalidate_preset(
        &self,
        token: &str,
        world: &str,
        variant: &str,
        id: &str,
        edit: MovesetEdit,
    ) -> Result<MovesetRecord, Error> {
        self.write_preset(token, world, variant, Some(id), edit, true)
            .await
    }
    async fn write_preset(
        &self,
        token: &str,
        world: &str,
        variant: &str,
        id: Option<&str>,
        edit: MovesetEdit,
        revalidate: bool,
    ) -> Result<MovesetRecord, Error> {
        let mut tx = self.authoring.write().await?;
        let actor = self.actor(&mut tx, token, Some(world), variant).await?;
        let preview = Self::preset_proposal(&mut tx, world, variant, id, &edit, revalidate).await?;
        let mut row = preview.proposed;
        let reason = validate_reason(edit.reason.as_deref()).map_err(Error::Invalid)?;
        if preview.entry.as_ref() != Some(&row) {
            if preview.entry.is_none() {
                row.id = author::new_id()?;
            }
            store::put_preset(&mut tx, &row, Some(&actor.user_id), reason.as_deref()).await?;
        }
        tx.commit().await.map_err(content::storage_error)?;
        Ok(row)
    }
    pub async fn validation(
        &self,
        world: &str,
        variant: &str,
        id: &str,
        version: Option<i64>,
    ) -> Result<ValidationReport, Error> {
        let mut tx = self.read().await?;
        let preset = if let Some(v) = version {
            store::preset_version(&mut tx, world, variant, id, v).await?
        } else {
            store::preset(&mut tx, world, variant, id).await?
        };
        let (learnset, ruleset) = store::preset_dependencies(&mut tx, &preset).await?;
        Ok(validate_preset(&preset, &learnset, &ruleset))
    }

    /// Operator-only complete-fixture adapter. The caller owns the transaction,
    /// including rollback of all other fixtures after any late failure.
    pub async fn seed_learnset_in(
        &self,
        c: &mut SqliteConnection,
        row: &LearnsetRecord,
    ) -> Result<bool, Error> {
        row.validate().map_err(Error::Invalid)?;
        if row.revision != 1 || row.id == PROVISIONAL_ID {
            return Err(author::invalid(
                "learnset",
                "Fixture baselines must be persisted creation revisions.",
            ));
        }
        if store::learnset_fixture_baseline(c, row).await? {
            return Ok(false);
        }
        store::put_learnset(c, row, None, "create", None).await?;
        Ok(true)
    }

    pub async fn seed_preset_in(
        &self,
        c: &mut SqliteConnection,
        row: &MovesetRecord,
    ) -> Result<bool, Error> {
        row.validate().map_err(Error::Invalid)?;
        if row.revision != 1 || row.id == PROVISIONAL_ID || !matches!(row.validator_version, 1 | 2)
        {
            return Err(author::invalid(
                "preset",
                "Fixture baselines must be persisted creation revisions under a supported validator policy.",
            ));
        }
        if store::preset_fixture_baseline(c, row).await? {
            return Ok(false);
        }
        store::put_preset(c, row, None, None).await?;
        Ok(true)
    }
}
async fn check_preset_selections(
    c: &mut SqliteConnection,
    old: Option<&MovesetRecord>,
    proposed: &MovesetRecord,
    learnset: &LearnsetRecord,
) -> Result<(), Error> {
    // Acquisition IDs are scoped to their learnset. Resolve the old exact context
    // rather than interpreting old selections through the proposed revision.
    let old_learnset = if let Some(old) = old
        && old.learnset != proposed.learnset
    {
        Some(
            store::learnset_version(
                c,
                Some(&old.world_id),
                &old.learnset.learnset_id,
                old.learnset.version,
            )
            .await?,
        )
    } else {
        None
    };
    let mut retained = std::collections::BTreeMap::new();
    if let Some(old) = old {
        let saved = old_learnset.as_ref().unwrap_or(learnset);
        for id in &old.entries {
            let record = saved
                .records
                .iter()
                .flatten()
                .find(|r| &r.id == id)
                .ok_or(Error::Unavailable)?;
            let key = (
                saved.id.clone(),
                id.clone(),
                record.move_record.id.clone(),
                record.move_record.revision,
            );
            *retained.entry(key).or_insert(0_usize) += 1;
        }
    }
    for id in &proposed.entries {
        // preset_dependencies already checked membership and exact references.
        let record = learnset
            .records
            .iter()
            .flatten()
            .find(|r| &r.id == id)
            .ok_or(Error::Unavailable)?;
        let key = (
            learnset.id.clone(),
            id.clone(),
            record.move_record.id.clone(),
            record.move_record.revision,
        );
        if let Some(count) = retained.get_mut(&key)
            && *count > 0
        {
            *count -= 1;
            continue;
        }
        core::select_move(c, Some(&proposed.world_id), &record.move_record.pin())
            .await
            .map_err(|e| match e {
                Error::Invalid(mut errors) => {
                    errors.insert(
                        "entries".into(),
                        "New selections require an unarchived Move revision; remove the added acquisition or choose an unarchived Move.".into(),
                    );
                    Error::Invalid(errors)
                }
                e => e,
            })?;
    }
    Ok(())
}
fn empty_learnset(world: Option<&str>, owner: &str) -> LearnsetRecord {
    LearnsetRecord {
        id: PROVISIONAL_ID.into(),
        world_id: world.map(str::to_owned),
        form_id: world.is_none().then(|| owner.into()),
        variant_id: world.map(|_| owner.into()),
        source: None,
        revision: 1,
        records: None,
        mappings: vec![],
    }
}
fn normalize_text(value: &Option<String>) -> Option<String> {
    value
        .as_ref()
        .map(|s| {
            s.replace("\r\n", "\n")
                .replace('\r', "\n")
                .trim()
                .to_owned()
        })
        .filter(|s| !s.is_empty())
}
