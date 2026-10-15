//! Independent stat catalogs and world rosters. Every mutation locks before policy.
use super::authoring::{self as author, AddressKey, Authoring, Error, HistoryEntry, WorldRecord};
use crate::{
    accounts::Accounts,
    auth::Actor,
    persistence::{authoring as content, stats as store},
};
use game_data::authoring::{slug_candidate, slug_from_name, validate_reason};
use sqlx::{Sqlite, SqliteConnection, Transaction};

pub use game_data::{authoring::ContentFields, stats::*};

#[derive(Clone, Debug)]
pub struct NewStat {
    pub fields: ContentFields,
    pub constraints: NumericConstraints,
    pub source: Option<StatPin>,
    pub reason: Option<String>,
}
#[derive(Clone, Debug)]
pub struct DefinitionEdit {
    pub expected_revision: i64,
    pub fields: ContentFields,
    pub constraints: NumericConstraints,
    pub reason: Option<String>,
}
#[derive(Clone, Debug)]
pub struct StatAdoptionPreview {
    pub entry: StatDefinition,
    pub old_source: StatDefinition,
    pub new_source: StatDefinition,
    pub proposed: StatDefinition,
}
#[derive(Clone, Debug)]
pub struct Stats {
    accounts: Accounts,
    authoring: Authoring,
}
impl Stats {
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
    pub async fn definitions(&self, world: Option<&str>) -> Result<Vec<StatDefinition>, Error> {
        let mut tx = self.read().await?;
        if let Some(w) = world {
            content::world(&mut tx, w).await?;
        }
        let mut rows = Vec::new();
        for id in store::ids(&mut tx, world).await? {
            rows.push(store::definition(&mut tx, world, &id).await?);
        }
        Ok(rows)
    }
    pub async fn detail(&self, world: Option<&str>, id: &str) -> Result<StatDefinition, Error> {
        store::definition(&mut *self.read().await?, world, id).await
    }
    pub async fn version(
        &self,
        world: Option<&str>,
        id: &str,
        revision: i64,
    ) -> Result<StatDefinition, Error> {
        store::definition_version(&mut *self.read().await?, world, id, revision).await
    }
    pub async fn resolve_shared(&self, key: AddressKey) -> Result<StatDefinition, Error> {
        store::resolve(&mut *self.read().await?, None, key).await
    }
    pub async fn resolve_world(
        &self,
        world_key: AddressKey,
        stat_key: AddressKey,
    ) -> Result<(WorldRecord, StatDefinition), Error> {
        let mut tx = self.read().await?;
        let world = content::resolve_world(&mut tx, world_key).await?;
        let row = store::resolve(&mut tx, Some(&world.id), stat_key).await?;
        Ok((world, row))
    }
    pub async fn world_roster(&self, world: &str) -> Result<Option<StatRoster>, Error> {
        store::roster(&mut *self.read().await?, world).await
    }
    pub async fn roster_version(
        &self,
        world: &str,
        id: &str,
        revision: i64,
    ) -> Result<StatRoster, Error> {
        store::roster_version(&mut *self.read().await?, world, id, revision).await
    }
    async fn actor(
        &self,
        c: &mut SqliteConnection,
        token: &str,
        world: Option<&str>,
    ) -> Result<Actor, Error> {
        let actor = self.authoring.actor(c, token).await?;
        if let Some(w) = world {
            let world = content::world(c, w).await?;
            self.authoring.world_actor(c, &actor, &world, false).await?;
        } else {
            author::require_admin(&actor)?;
        }
        Ok(actor)
    }
    async fn proposed_create(
        c: &mut SqliteConnection,
        world: Option<&str>,
        new: &NewStat,
    ) -> Result<StatDefinition, Error> {
        if let Some(w) = world {
            content::world(c, w).await?;
        }
        if world.is_none() && new.source.is_some() {
            return Err(author::invalid(
                "source",
                "Shared stats cannot have a source.",
            ));
        }
        let (mut fields, _) = author::fields_and_reason(&new.fields, new.reason.as_deref())?;
        allocate_slug(c, world, &mut fields, None).await?;
        if let Some(pin) = &new.source {
            store::select_definition(c, None, pin)
                .await
                .map_err(|e| author::missing_reference(e, "source"))?;
        }
        let row = StatDefinition {
            id: author::new_id()?,
            world_id: world.map(str::to_owned),
            source: new.source.clone(),
            fields,
            revision: 1,
            archived_at: None,
            constraints: new.constraints.clone(),
        };
        row.validate().map_err(Error::Invalid)?;
        Ok(row)
    }
    pub async fn preview_create(
        &self,
        world: Option<&str>,
        new: &NewStat,
    ) -> Result<StatDefinition, Error> {
        Self::proposed_create(&mut *self.read().await?, world, new).await
    }
    pub async fn create(
        &self,
        token: &str,
        world: Option<&str>,
        new: NewStat,
    ) -> Result<StatDefinition, Error> {
        let mut tx = self.authoring.write().await?;
        let actor = self.actor(&mut tx, token, world).await?;
        let row = Self::proposed_create(&mut tx, world, &new).await?;
        let reason = validate_reason(new.reason.as_deref()).map_err(Error::Invalid)?;
        store::put_definition(
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
    async fn proposed_edit(
        c: &mut SqliteConnection,
        old: &StatDefinition,
        edit: &DefinitionEdit,
    ) -> Result<StatDefinition, Error> {
        author::expected(old.revision, edit.expected_revision)?;
        let (mut fields, _) = author::fields_and_reason(&edit.fields, edit.reason.as_deref())?;
        allocate_slug(c, old.world_id.as_deref(), &mut fields, Some(old)).await?;
        let mut row = old.clone();
        row.fields = fields;
        row.constraints = edit.constraints.clone();
        row.validate().map_err(Error::Invalid)?;
        Ok(row)
    }
    pub async fn save_definition(
        &self,
        token: &str,
        world: Option<&str>,
        id: &str,
        edit: DefinitionEdit,
    ) -> Result<StatDefinition, Error> {
        let mut tx = self.authoring.write().await?;
        let actor = self.actor(&mut tx, token, world).await?;
        let old = store::definition(&mut tx, world, id).await?;
        let mut row = Self::proposed_edit(&mut tx, &old, &edit).await?;
        let reason = validate_reason(edit.reason.as_deref()).map_err(Error::Invalid)?;
        if row != old {
            row.revision = author::next(old.revision)?;
            store::put_definition(
                &mut tx,
                &row,
                Some(&actor.user_id),
                "edit",
                reason.as_deref(),
            )
            .await?;
        }
        tx.commit().await.map_err(content::storage_error)?;
        Ok(row)
    }
    async fn preview_in(
        c: &mut SqliteConnection,
        world: &str,
        id: &str,
        target_version: i64,
        edit: &DefinitionEdit,
    ) -> Result<StatAdoptionPreview, Error> {
        let entry = store::definition(c, Some(world), id).await?;
        author::expected(entry.revision, edit.expected_revision)?;
        let pin = entry
            .source
            .as_ref()
            .ok_or_else(|| author::invalid("source", "This stat has no shared source to adopt."))?;
        if target_version <= pin.version {
            return Err(author::invalid(
                "target_version",
                "Select a newer revision of the same shared stat.",
            ));
        }
        let old_source = store::definition_version(c, None, &pin.stat_id, pin.version).await?;
        let new_source = store::select_definition(
            c,
            None,
            &StatPin {
                stat_id: pin.stat_id.clone(),
                version: target_version,
            },
        )
        .await
        .map_err(|e| author::missing_reference(e, "target_version"))?;
        let mut proposed = Self::proposed_edit(c, &entry, edit).await?;
        proposed.source = Some(new_source.pin());
        proposed.revision = author::next(entry.revision)?;
        proposed.validate().map_err(Error::Invalid)?;
        Ok(StatAdoptionPreview {
            entry,
            old_source,
            new_source,
            proposed,
        })
    }
    pub async fn adoption_preview(
        &self,
        world: &str,
        id: &str,
        target_version: i64,
        edit: &DefinitionEdit,
    ) -> Result<StatAdoptionPreview, Error> {
        Self::preview_in(&mut *self.read().await?, world, id, target_version, edit).await
    }
    pub async fn adopt(
        &self,
        token: &str,
        world: &str,
        id: &str,
        target_version: i64,
        edit: DefinitionEdit,
    ) -> Result<StatDefinition, Error> {
        let mut tx = self.authoring.write().await?;
        let actor = self.actor(&mut tx, token, Some(world)).await?;
        let preview = Self::preview_in(&mut tx, world, id, target_version, &edit).await?;
        let reason = validate_reason(edit.reason.as_deref()).map_err(Error::Invalid)?;
        store::put_definition(
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
    pub async fn save_roster(
        &self,
        token: &str,
        world: &str,
        expected_revision: i64,
        entries: Vec<StatPin>,
        reason: Option<String>,
    ) -> Result<StatRoster, Error> {
        let mut tx = self.authoring.write().await?;
        let actor = self.actor(&mut tx, token, Some(world)).await?;
        let old = store::roster(&mut tx, world).await?;
        // Conflict first, even if the submitted dependencies or reason are invalid.
        author::expected(old.as_ref().map_or(0, |r| r.revision), expected_revision)?;
        let reason = validate_reason(reason.as_deref()).map_err(Error::Invalid)?;
        let mut selected = Vec::with_capacity(entries.len());
        let mut identities = std::collections::HashSet::new();
        for pin in entries {
            if !identities.insert(pin.stat_id.clone()) {
                return Err(author::invalid(
                    "entries",
                    "Choose each stat identity at most once.",
                ));
            }
            selected.push(
                store::select_definition(&mut tx, Some(world), &pin)
                    .await
                    .map_err(|e| author::missing_reference(e, "entries"))?,
            );
        }
        if let Some(old) = &old
            && old.entries == selected
        {
            return Ok(old.clone());
        }
        let row = StatRoster {
            id: match &old {
                Some(r) => r.id.clone(),
                None => author::new_id()?,
            },
            world_id: world.into(),
            revision: match old {
                Some(r) => author::next(r.revision)?,
                None => 1,
            },
            entries: selected,
        };
        row.validate().map_err(Error::Invalid)?;
        store::put_roster(&mut tx, &row, Some(&actor.user_id), reason.as_deref()).await?;
        tx.commit().await.map_err(content::storage_error)?;
        Ok(row)
    }
    pub async fn history(
        &self,
        world: Option<&str>,
        id: &str,
        before: Option<i64>,
    ) -> Result<Vec<HistoryEntry>, Error> {
        store::history(&mut *self.read().await?, world, id, before, None).await
    }
    pub async fn history_entry(
        &self,
        world: Option<&str>,
        id: &str,
        revision: i64,
    ) -> Result<HistoryEntry, Error> {
        store::history(&mut *self.read().await?, world, id, None, Some(revision))
            .await?
            .into_iter()
            .next()
            .ok_or(Error::NotFound)
    }
    pub async fn roster_history(
        &self,
        world: &str,
        before: Option<i64>,
    ) -> Result<Vec<HistoryEntry>, Error> {
        store::roster_history(&mut *self.read().await?, world, before).await
    }
    /// Complete-fixture loader only; caller owns the outer atomic transaction.
    pub(super) async fn seed_definition_in(
        &self,
        c: &mut SqliteConnection,
        row: &StatDefinition,
    ) -> Result<bool, Error> {
        seed_definition_in(c, row).await
    }
    pub(super) async fn seed_roster_in(
        &self,
        c: &mut SqliteConnection,
        row: &StatRoster,
    ) -> Result<bool, Error> {
        seed_roster_in(c, row).await
    }
}
async fn allocate_slug(
    c: &mut SqliteConnection,
    world: Option<&str>,
    fields: &mut ContentFields,
    old: Option<&StatDefinition>,
) -> Result<(), Error> {
    if let Some(old) = old
        && old.fields.display_name == fields.display_name
    {
        fields.slug = old.fields.slug.clone();
        return Ok(());
    }
    let base = slug_from_name(&fields.display_name);
    for n in 1..=10_000 {
        let slug = slug_candidate(&base, n);
        if !store::slug_exists(c, world, &slug, old.map(|r| r.id.as_str())).await? {
            fields.slug = slug;
            return Ok(());
        }
    }
    Err(author::invalid(
        "display_name",
        "Too many stats share this address. Choose a different name.",
    ))
}
fn fixture_error() -> Error {
    author::invalid(
        "fixtures",
        "The stat creation baseline is invalid or collides with existing content. No fixtures were changed.",
    )
}
pub(super) async fn seed_definition_in(
    c: &mut SqliteConnection,
    row: &StatDefinition,
) -> Result<bool, Error> {
    row.validate().map_err(|_| fixture_error())?;
    if row.revision != 1 || row.archived_at.is_some() {
        return Err(fixture_error());
    }
    store::definition_dependencies(c, row)
        .await
        .map_err(|_| fixture_error())?;
    if store::identity_exists(c, &row.id).await? {
        if store::definition_version(c, row.world_id.as_deref(), &row.id, 1)
            .await
            .map_err(|_| fixture_error())?
            != *row
            || !store::definition_fixture_actor(c, &row.id).await?
        {
            return Err(fixture_error());
        }
        store::definition(c, row.world_id.as_deref(), &row.id)
            .await
            .map_err(|_| fixture_error())?;
        return Ok(false);
    }
    if store::slug_exists(c, row.world_id.as_deref(), &row.fields.slug, None).await?
        || store::baseline_slug_exists(c, row.world_id.as_deref(), &row.fields.slug).await?
    {
        return Err(fixture_error());
    }
    store::put_definition(c, row, None, "create", None).await?;
    Ok(true)
}
pub(super) async fn seed_roster_in(
    c: &mut SqliteConnection,
    row: &StatRoster,
) -> Result<bool, Error> {
    row.validate().map_err(|_| fixture_error())?;
    if row.revision != 1 {
        return Err(fixture_error());
    }
    store::roster_dependencies(c, row)
        .await
        .map_err(|_| fixture_error())?;
    if let Some(head) = store::roster(c, &row.world_id).await? {
        if head.id != row.id
            || store::roster_version(c, &row.world_id, &row.id, 1)
                .await
                .map_err(|_| fixture_error())?
                != *row
            || !store::roster_fixture_actor(c, &row.id).await?
        {
            return Err(fixture_error());
        }
        return Ok(false);
    }
    store::put_roster(c, row, None, None).await?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        auth::{Role, Subject},
        persistence,
    };

    #[tokio::test]
    async fn fixture_baselines_are_idempotent_preserve_edits_and_rollback_as_a_unit() {
        let dir = tempfile::tempdir().unwrap();
        let pool = persistence::connect_for_migration(&format!(
            "sqlite://{}/seed-stats.db",
            dir.path().display()
        ))
        .await
        .unwrap();
        persistence::migrate(&pool).await.unwrap();
        let accounts = Accounts::new(pool.clone(), 3600).unwrap();
        accounts
            .invite("test", "owner", Role::Editor, None)
            .await
            .unwrap();
        let actor = accounts
            .sign_in(Subject {
                provider: "test".into(),
                provider_subject: "owner".into(),
                display_name: "Owner".into(),
            })
            .await
            .unwrap();
        let (token, _) = accounts.start_session(&actor.user_id).await.unwrap();
        let authoring = Authoring::new(accounts.clone());
        let fields = ContentFields {
            slug: "baseline".into(),
            display_name: "Baseline".into(),
            description: "Synthetic fixture baseline.".into(),
        };
        let world = authoring
            .create_world(&token, fields.clone(), None)
            .await
            .unwrap();
        let stats = Stats::new(accounts);
        let baseline = StatDefinition {
            id: "12345678901234567890123456789012".into(),
            world_id: Some(world.id.clone()),
            source: None,
            fields,
            revision: 1,
            archived_at: None,
            constraints: NumericConstraints::default(),
        };
        let roster = StatRoster {
            id: "12345678901234567890123456789013".into(),
            world_id: world.id.clone(),
            revision: 1,
            entries: vec![baseline.clone()],
        };
        let mut tx = persistence::begin_write(&pool).await.unwrap();
        assert!(stats.seed_definition_in(&mut tx, &baseline).await.unwrap());
        assert!(stats.seed_roster_in(&mut tx, &roster).await.unwrap());
        tx.rollback().await.unwrap();
        assert!(stats.definitions(Some(&world.id)).await.unwrap().is_empty());
        assert!(stats.world_roster(&world.id).await.unwrap().is_none());
        let mut tx = persistence::begin_write(&pool).await.unwrap();
        assert!(seed_definition_in(&mut tx, &baseline).await.unwrap());
        assert!(seed_roster_in(&mut tx, &roster).await.unwrap());
        tx.commit().await.unwrap();
        let mut fields = baseline.fields.clone();
        fields.display_name = "Designer edit".into();
        let edited = stats
            .save_definition(
                &token,
                Some(&world.id),
                &baseline.id,
                DefinitionEdit {
                    expected_revision: 1,
                    fields,
                    constraints: NumericConstraints {
                        whole_only: true,
                        ..Default::default()
                    },
                    reason: None,
                },
            )
            .await
            .unwrap();
        let empty = stats
            .save_roster(&token, &world.id, 1, vec![], None)
            .await
            .unwrap();
        let audit_count: i64 = sqlx::query_scalar("SELECT count(*) FROM audit_events")
            .fetch_one(&pool)
            .await
            .unwrap();
        let mut tx = persistence::begin_write(&pool).await.unwrap();
        assert!(!stats.seed_definition_in(&mut tx, &baseline).await.unwrap());
        assert!(!stats.seed_roster_in(&mut tx, &roster).await.unwrap());
        let mut collision = baseline.clone();
        collision.constraints.whole_only = true;
        assert!(seed_definition_in(&mut tx, &collision).await.is_err());
        let mut collision = roster.clone();
        collision.entries.clear();
        assert!(seed_roster_in(&mut tx, &collision).await.is_err());
        tx.commit().await.unwrap();
        assert_eq!(
            stats.detail(Some(&world.id), &baseline.id).await.unwrap(),
            edited
        );
        assert_eq!(stats.world_roster(&world.id).await.unwrap(), Some(empty));
        assert_eq!(
            stats
                .history(Some(&world.id), &baseline.id, None)
                .await
                .unwrap()[1]
                .actor_name,
            "bundled-fixture-loader"
        );
        let after: i64 = sqlx::query_scalar("SELECT count(*) FROM audit_events")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(audit_count, after);

        let mut archived = edited.clone();
        archived.revision = 3;
        archived.archived_at = Some(1_800_000_000);
        let mut tx = persistence::begin_write(&pool).await.unwrap();
        store::put_definition(&mut tx, &archived, Some(&actor.user_id), "edit", None)
            .await
            .unwrap();
        tx.commit().await.unwrap();
        let mut tx = persistence::begin_write(&pool).await.unwrap();
        assert!(matches!(
            store::select_definition(&mut tx, Some(&world.id), &baseline.pin()).await,
            Err(Error::Invalid(_))
        ));
        assert_eq!(
            store::definition_version(&mut tx, Some(&world.id), &baseline.id, 1)
                .await
                .unwrap(),
            baseline
        );
        assert_eq!(
            store::roster_version(&mut tx, &world.id, &roster.id, 1)
                .await
                .unwrap(),
            roster
        );
        assert!(!seed_definition_in(&mut tx, &baseline).await.unwrap());
        tx.commit().await.unwrap();
        assert_eq!(
            stats.detail(Some(&world.id), &baseline.id).await.unwrap(),
            archived
        );
    }
}
