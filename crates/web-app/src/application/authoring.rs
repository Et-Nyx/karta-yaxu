//! World-aware authoring policy and transaction orchestration.
use crate::{
    accounts::Accounts,
    auth::{Actor, Role},
    persistence::{self, authoring as store},
};
use game_data::{
    Catalog,
    authoring::{ContentFields, FieldErrors, validate_reason},
};
use serde::{Deserialize, Serialize};
use sqlx::{Sqlite, SqliteConnection, Transaction};
use std::fmt;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpeciesRecord {
    pub id: String,
    pub fields: ContentFields,
    pub revision: i64,
    pub default_form_id: String,
    pub archived_at: Option<i64>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FormRecord {
    pub id: String,
    pub species_id: String,
    pub species_version: i64,
    pub fields: ContentFields,
    pub revision: i64,
    pub is_default: bool,
    pub archived_at: Option<i64>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorldRecord {
    pub id: String,
    pub owner_user_id: String,
    pub fields: ContentFields,
    pub revision: i64,
    pub access_revision: i64,
    pub archived_at: Option<i64>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourcePin {
    pub form_id: String,
    pub version: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct VariantRecord {
    pub id: String,
    pub world_id: String,
    pub source: Option<SourcePin>,
    pub fields: ContentFields,
    pub revision: i64,
    pub archived_at: Option<i64>,
}
#[derive(Clone, Debug)]
pub struct Edit {
    pub expected_revision: i64,
    pub fields: ContentFields,
    pub reason: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    Unauthorized,
    Forbidden,
    NotFound,
    Invalid(FieldErrors),
    Conflict,
    Unavailable,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Unauthorized => "Sign in again to continue.",
            Self::Forbidden => "Your account does not have permission for this action.",
            Self::NotFound => "The requested record was not found.",
            Self::Invalid(_) => "Please correct the indicated fields.",
            Self::Conflict => {
                "This record has changed. Review the current version before saving again."
            }
            Self::Unavailable => "Storage is unavailable. Please retry later.",
        })
    }
}
impl std::error::Error for Error {}
#[derive(Clone, Debug, Default)]
pub struct Capabilities {
    pub authenticated: bool,
    pub can_create_world: bool,
    pub can_edit_library: bool,
    pub can_edit_world: bool,
    pub can_manage_access: bool,
}
#[derive(Clone, Copy, Debug)]
pub enum HistoryKind {
    Species,
    Form,
    World,
    Variant,
}
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct HistoryEntry {
    pub revision: i64,
    pub previous_revision: Option<i64>,
    pub actor_name: String,
    pub action: String,
    pub occurred_at: String,
    pub reason: Option<String>,
    pub before_json: Option<String>,
    pub after_json: String,
}
#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct Collaborator {
    pub user_id: String,
    pub display_name: String,
}
#[derive(Clone, Debug)]
pub struct AccessState {
    pub world: WorldRecord,
    pub collaborators: Vec<Collaborator>,
    pub history: Vec<HistoryEntry>,
}
#[derive(Clone, Debug)]
pub struct AdoptionPreview {
    pub entry: VariantRecord,
    pub old_source: FormRecord,
    pub new_source: FormRecord,
    pub old_species: SpeciesRecord,
    pub new_species: SpeciesRecord,
}
#[derive(Clone, Debug, Default)]
pub struct SeedReport {
    pub species_created: usize,
    pub forms_created: usize,
    pub species_skipped: usize,
}

#[derive(Clone, Debug)]
pub struct Authoring {
    accounts: Accounts,
}

fn invalid(field: &str, message: &str) -> Error {
    Error::Invalid(FieldErrors::from([(field.into(), message.into())]))
}
fn missing_reference(error: Error, field: &str) -> Error {
    if error == Error::NotFound {
        invalid(field, "Choose an existing exact definition version.")
    } else {
        error
    }
}
fn fields_and_reason(
    fields: &ContentFields,
    reason: Option<&str>,
) -> Result<(ContentFields, Option<String>), Error> {
    let fields_result = fields.normalized();
    let reason_result = validate_reason(reason);
    let mut errors = FieldErrors::new();
    if let Err(e) = &fields_result {
        errors.extend(e.clone());
    }
    if let Err(e) = &reason_result {
        errors.extend(e.clone());
    }
    if !errors.is_empty() {
        return Err(Error::Invalid(errors));
    }
    Ok((
        fields_result.map_err(Error::Invalid)?,
        reason_result.map_err(Error::Invalid)?,
    ))
}
fn next(revision: i64) -> Result<i64, Error> {
    revision.checked_add(1).ok_or(Error::Unavailable)
}
fn expected(actual: i64, expected: i64) -> Result<(), Error> {
    if actual == expected {
        Ok(())
    } else {
        Err(Error::Conflict)
    }
}
fn eligible(actor: &Actor) -> bool {
    matches!(actor.role, Role::Editor | Role::Administrator)
}
fn require_eligible(actor: &Actor) -> Result<(), Error> {
    if eligible(actor) {
        Ok(())
    } else {
        Err(Error::Forbidden)
    }
}
fn require_admin(actor: &Actor) -> Result<(), Error> {
    if actor.role == Role::Administrator {
        Ok(())
    } else {
        Err(Error::Forbidden)
    }
}
pub(crate) fn new_id() -> Result<String, Error> {
    let mut bytes = [0; 16];
    getrandom::fill(&mut bytes).map_err(|_| Error::Unavailable)?;
    Ok(hex::encode(bytes))
}
fn snapshot(value: &impl Serialize) -> Result<String, Error> {
    serde_json::to_string(&serde_json::json!({"snapshot_version": 1, "content": value}))
        .map_err(|_| Error::Unavailable)
}
// Public world snapshots deliberately exclude ownership, access revision and grants.
async fn world_snapshot(c: &mut SqliteConnection, world: &WorldRecord) -> Result<String, Error> {
    let ids = store::variant_ids(c, &world.id).await?;
    snapshot(
        &serde_json::json!({"id":world.id,"fields":world.fields,"revision":world.revision,"archived_at":world.archived_at,"variant_ids":ids}),
    )
}
async fn access_snapshot(c: &mut SqliteConnection, world: &WorldRecord) -> Result<String, Error> {
    let ids = store::collaborator_ids(c, &world.id).await?;
    snapshot(
        &serde_json::json!({"world_id":world.id,"owner_user_id":world.owner_user_id,"access_revision":world.access_revision,"collaborator_ids":ids}),
    )
}

impl Authoring {
    pub fn new(accounts: Accounts) -> Self {
        Self { accounts }
    }
    async fn write(&self) -> Result<Transaction<'static, Sqlite>, Error> {
        persistence::begin_write(self.accounts.pool())
            .await
            .map_err(store::storage_error)
    }
    async fn actor(&self, c: &mut SqliteConnection, token: &str) -> Result<Actor, Error> {
        self.accounts
            .authenticate_in_transaction(c, token)
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Unauthorized)
    }
    async fn world_actor(
        &self,
        c: &mut SqliteConnection,
        actor: &Actor,
        world: &WorldRecord,
        owner_only: bool,
    ) -> Result<(), Error> {
        require_eligible(actor)?;
        if world.owner_user_id == actor.user_id {
            return Ok(());
        }
        if !owner_only && store::is_collaborator(c, &world.id, &actor.user_id).await? {
            return Ok(());
        }
        Err(Error::Forbidden)
    }
    pub async fn species(&self) -> Result<Vec<SpeciesRecord>, Error> {
        store::species_list(
            &mut *self
                .accounts
                .pool()
                .acquire()
                .await
                .map_err(store::storage_error)?,
        )
        .await
    }
    pub async fn species_detail(&self, id: &str) -> Result<SpeciesRecord, Error> {
        store::species(
            &mut *self
                .accounts
                .pool()
                .acquire()
                .await
                .map_err(store::storage_error)?,
            id,
        )
        .await
    }
    pub async fn species_version(&self, id: &str, version: i64) -> Result<SpeciesRecord, Error> {
        store::species_version(
            &mut *self
                .accounts
                .pool()
                .acquire()
                .await
                .map_err(store::storage_error)?,
            id,
            version,
        )
        .await
    }
    pub async fn forms(&self, species_id: &str) -> Result<Vec<FormRecord>, Error> {
        let mut c = self
            .accounts
            .pool()
            .acquire()
            .await
            .map_err(store::storage_error)?;
        store::species(&mut c, species_id).await?;
        store::forms(&mut c, species_id).await
    }
    pub async fn form_detail(&self, id: &str) -> Result<FormRecord, Error> {
        store::form(
            &mut *self
                .accounts
                .pool()
                .acquire()
                .await
                .map_err(store::storage_error)?,
            id,
        )
        .await
    }
    pub async fn form_version(&self, id: &str, version: i64) -> Result<FormRecord, Error> {
        store::form_version(
            &mut *self
                .accounts
                .pool()
                .acquire()
                .await
                .map_err(store::storage_error)?,
            id,
            version,
        )
        .await
    }
    pub async fn worlds(&self) -> Result<Vec<WorldRecord>, Error> {
        store::worlds(
            &mut *self
                .accounts
                .pool()
                .acquire()
                .await
                .map_err(store::storage_error)?,
        )
        .await
    }
    pub async fn world_detail(&self, id: &str) -> Result<WorldRecord, Error> {
        store::world(
            &mut *self
                .accounts
                .pool()
                .acquire()
                .await
                .map_err(store::storage_error)?,
            id,
        )
        .await
    }
    pub async fn variants(&self, world_id: &str) -> Result<Vec<VariantRecord>, Error> {
        let mut c = self
            .accounts
            .pool()
            .acquire()
            .await
            .map_err(store::storage_error)?;
        store::world(&mut c, world_id).await?;
        store::variants(&mut c, world_id).await
    }
    pub async fn variant_detail(&self, world_id: &str, id: &str) -> Result<VariantRecord, Error> {
        store::variant(
            &mut *self
                .accounts
                .pool()
                .acquire()
                .await
                .map_err(store::storage_error)?,
            world_id,
            id,
        )
        .await
    }
    pub async fn availability(&self, species_id: &str) -> Result<Vec<WorldRecord>, Error> {
        let mut c = self
            .accounts
            .pool()
            .acquire()
            .await
            .map_err(store::storage_error)?;
        store::species(&mut c, species_id).await?;
        store::availability(&mut c, species_id).await
    }
    pub async fn capabilities(
        &self,
        token: Option<&str>,
        world_id: Option<&str>,
    ) -> Result<Capabilities, Error> {
        let mut tx = self
            .accounts
            .pool()
            .begin()
            .await
            .map_err(store::storage_error)?;
        let actor = if let Some(token) = token {
            self.accounts
                .authenticate_in_transaction(&mut tx, token)
                .await
                .map_err(|_| Error::Unavailable)?
        } else {
            None
        };
        let mut result = Capabilities::default();
        if let Some(actor) = actor {
            result.authenticated = true;
            result.can_create_world = eligible(&actor);
            result.can_edit_library = actor.role == Role::Administrator;
            if let Some(id) = world_id {
                let world = store::world(&mut tx, id).await?;
                result.can_manage_access = eligible(&actor) && world.owner_user_id == actor.user_id;
                result.can_edit_world = eligible(&actor)
                    && (world.owner_user_id == actor.user_id
                        || store::is_collaborator(&mut tx, id, &actor.user_id).await?);
            }
        }
        tx.commit().await.map_err(store::storage_error)?;
        Ok(result)
    }
    pub async fn create_species(
        &self,
        token: &str,
        fields: ContentFields,
        reason: Option<&str>,
    ) -> Result<SpeciesRecord, Error> {
        let mut tx = self.write().await?;
        let actor = self.actor(&mut tx, token).await?;
        require_admin(&actor)?;
        let (fields, reason) = fields_and_reason(&fields, reason)?;
        let result = self
            .insert_species(
                &mut tx,
                new_id()?,
                fields,
                Some(&actor.user_id),
                reason.as_deref(),
            )
            .await?;
        tx.commit().await.map_err(store::storage_error)?;
        Ok(result)
    }
    async fn insert_species(
        &self,
        c: &mut SqliteConnection,
        id: String,
        fields: ContentFields,
        actor: Option<&str>,
        reason: Option<&str>,
    ) -> Result<SpeciesRecord, Error> {
        let species = SpeciesRecord {
            id,
            fields: fields.clone(),
            revision: 1,
            default_form_id: new_id()?,
            archived_at: None,
        };
        let form = FormRecord {
            id: species.default_form_id.clone(),
            species_id: species.id.clone(),
            species_version: 1,
            fields,
            revision: 1,
            is_default: true,
            archived_at: None,
        };
        store::put_species(
            c,
            &species,
            store::Revision::new(actor, "create", reason, None, snapshot(&species)?),
        )
        .await?;
        store::put_form(
            c,
            &form,
            store::Revision::new(actor, "create", reason, None, snapshot(&form)?),
        )
        .await?;
        Ok(species)
    }
    pub async fn save_species(
        &self,
        token: &str,
        id: &str,
        edit: Edit,
    ) -> Result<SpeciesRecord, Error> {
        let mut tx = self.write().await?;
        let actor = self.actor(&mut tx, token).await?;
        require_admin(&actor)?;
        let mut row = store::species(&mut tx, id).await?;
        expected(row.revision, edit.expected_revision)?;
        let (fields, reason) = fields_and_reason(&edit.fields, edit.reason.as_deref())?;
        if fields != row.fields {
            let before = snapshot(&row)?;
            row.fields = fields;
            row.revision = next(row.revision)?;
            store::put_species(
                &mut tx,
                &row,
                store::Revision::new(
                    Some(&actor.user_id),
                    "edit",
                    reason.as_deref(),
                    Some(before),
                    snapshot(&row)?,
                ),
            )
            .await?;
        }
        tx.commit().await.map_err(store::storage_error)?;
        Ok(row)
    }
    pub async fn create_form(
        &self,
        token: &str,
        species_id: &str,
        species_version: i64,
        fields: ContentFields,
        reason: Option<&str>,
    ) -> Result<FormRecord, Error> {
        let mut tx = self.write().await?;
        let actor = self.actor(&mut tx, token).await?;
        require_admin(&actor)?;
        let (fields, reason) = fields_and_reason(&fields, reason)?;
        store::species(&mut tx, species_id).await?;
        store::species_version(&mut tx, species_id, species_version)
            .await
            .map_err(|error| missing_reference(error, "species_version"))?;
        let row = FormRecord {
            id: new_id()?,
            species_id: species_id.into(),
            species_version,
            fields,
            revision: 1,
            is_default: false,
            archived_at: None,
        };
        store::put_form(
            &mut tx,
            &row,
            store::Revision::new(
                Some(&actor.user_id),
                "create",
                reason.as_deref(),
                None,
                snapshot(&row)?,
            ),
        )
        .await?;
        tx.commit().await.map_err(store::storage_error)?;
        Ok(row)
    }
    pub async fn save_form(
        &self,
        token: &str,
        id: &str,
        species_version: i64,
        edit: Edit,
    ) -> Result<FormRecord, Error> {
        let mut tx = self.write().await?;
        let actor = self.actor(&mut tx, token).await?;
        require_admin(&actor)?;
        let mut row = store::form(&mut tx, id).await?;
        expected(row.revision, edit.expected_revision)?;
        let (fields, reason) = fields_and_reason(&edit.fields, edit.reason.as_deref())?;
        store::species_version(&mut tx, &row.species_id, species_version)
            .await
            .map_err(|error| missing_reference(error, "species_version"))?;
        if fields != row.fields || species_version != row.species_version {
            let before = snapshot(&row)?;
            row.fields = fields;
            row.species_version = species_version;
            row.revision = next(row.revision)?;
            store::put_form(
                &mut tx,
                &row,
                store::Revision::new(
                    Some(&actor.user_id),
                    "edit",
                    reason.as_deref(),
                    Some(before),
                    snapshot(&row)?,
                ),
            )
            .await?;
        }
        tx.commit().await.map_err(store::storage_error)?;
        Ok(row)
    }
    pub async fn create_world(
        &self,
        token: &str,
        fields: ContentFields,
        reason: Option<&str>,
    ) -> Result<WorldRecord, Error> {
        let mut tx = self.write().await?;
        let actor = self.actor(&mut tx, token).await?;
        require_eligible(&actor)?;
        let (fields, reason) = fields_and_reason(&fields, reason)?;
        let row = WorldRecord {
            id: new_id()?,
            owner_user_id: actor.user_id.clone(),
            fields,
            revision: 1,
            access_revision: 1,
            archived_at: None,
        };
        let after = world_snapshot(&mut tx, &row).await?;
        store::put_world(
            &mut tx,
            &row,
            store::Revision::new(
                Some(&actor.user_id),
                "create",
                reason.as_deref(),
                None,
                after,
            ),
        )
        .await?;
        let after = access_snapshot(&mut tx, &row).await?;
        store::put_access_revision(
            &mut tx,
            &row,
            store::Revision::new(Some(&actor.user_id), "own", reason.as_deref(), None, after),
        )
        .await?;
        tx.commit().await.map_err(store::storage_error)?;
        Ok(row)
    }
    pub async fn save_world(
        &self,
        token: &str,
        id: &str,
        edit: Edit,
    ) -> Result<WorldRecord, Error> {
        let mut tx = self.write().await?;
        let actor = self.actor(&mut tx, token).await?;
        let mut row = store::world(&mut tx, id).await?;
        self.world_actor(&mut tx, &actor, &row, false).await?;
        expected(row.revision, edit.expected_revision)?;
        let (fields, reason) = fields_and_reason(&edit.fields, edit.reason.as_deref())?;
        if row.fields != fields {
            let before = world_snapshot(&mut tx, &row).await?;
            row.fields = fields;
            row.revision = next(row.revision)?;
            let after = world_snapshot(&mut tx, &row).await?;
            store::put_world(
                &mut tx,
                &row,
                store::Revision::new(
                    Some(&actor.user_id),
                    "edit",
                    reason.as_deref(),
                    Some(before),
                    after,
                ),
            )
            .await?;
        }
        tx.commit().await.map_err(store::storage_error)?;
        Ok(row)
    }
    #[allow(clippy::too_many_arguments)]
    pub async fn create_variant(
        &self,
        token: &str,
        world_id: &str,
        expected_world_revision: i64,
        source: Option<SourcePin>,
        fields: ContentFields,
        reason: Option<&str>,
    ) -> Result<VariantRecord, Error> {
        let mut tx = self.write().await?;
        let actor = self.actor(&mut tx, token).await?;
        let mut world = store::world(&mut tx, world_id).await?;
        self.world_actor(&mut tx, &actor, &world, false).await?;
        expected(world.revision, expected_world_revision)?;
        let (fields, reason) = fields_and_reason(&fields, reason)?;
        if let Some(pin) = &source {
            store::form_version(&mut tx, &pin.form_id, pin.version)
                .await
                .map_err(|error| missing_reference(error, "source_pin"))?;
        }
        let before = world_snapshot(&mut tx, &world).await?;
        let row = VariantRecord {
            id: new_id()?,
            world_id: world_id.into(),
            source,
            fields,
            revision: 1,
            archived_at: None,
        };
        store::put_variant(
            &mut tx,
            &row,
            store::Revision::new(
                Some(&actor.user_id),
                "create",
                reason.as_deref(),
                None,
                snapshot(&row)?,
            ),
        )
        .await?;
        world.revision = next(world.revision)?;
        let after = world_snapshot(&mut tx, &world).await?;
        store::put_world(
            &mut tx,
            &world,
            store::Revision::new(
                Some(&actor.user_id),
                "add-variant",
                reason.as_deref(),
                Some(before),
                after,
            ),
        )
        .await?;
        tx.commit().await.map_err(store::storage_error)?;
        Ok(row)
    }
    pub async fn save_variant(
        &self,
        token: &str,
        world_id: &str,
        id: &str,
        edit: Edit,
    ) -> Result<VariantRecord, Error> {
        self.edit_variant(token, world_id, id, None, edit).await
    }
    pub async fn adopt_variant(
        &self,
        token: &str,
        world_id: &str,
        id: &str,
        target_version: i64,
        edit: Edit,
    ) -> Result<VariantRecord, Error> {
        self.edit_variant(token, world_id, id, Some(target_version), edit)
            .await
    }
    async fn edit_variant(
        &self,
        token: &str,
        world_id: &str,
        id: &str,
        target: Option<i64>,
        edit: Edit,
    ) -> Result<VariantRecord, Error> {
        let mut tx = self.write().await?;
        let actor = self.actor(&mut tx, token).await?;
        let world = store::world(&mut tx, world_id).await?;
        self.world_actor(&mut tx, &actor, &world, false).await?;
        let mut row = store::variant(&mut tx, world_id, id).await?;
        expected(row.revision, edit.expected_revision)?;
        let (fields, reason) = fields_and_reason(&edit.fields, edit.reason.as_deref())?;
        let before = snapshot(&row)?;
        if let Some(version) = target {
            let pin = row.source.as_mut().ok_or_else(|| {
                invalid("source", "World-exclusive entries have no source to adopt.")
            })?;
            if version <= pin.version {
                return Err(invalid(
                    "target_version",
                    "Select a newer version of the same source form.",
                ));
            }
            store::form_version(&mut tx, &pin.form_id, version)
                .await
                .map_err(|error| missing_reference(error, "target_version"))?;
            pin.version = version;
        }
        if fields != row.fields || target.is_some() {
            row.fields = fields;
            row.revision = next(row.revision)?;
            store::put_variant(
                &mut tx,
                &row,
                store::Revision::new(
                    Some(&actor.user_id),
                    if target.is_some() { "adopt" } else { "edit" },
                    reason.as_deref(),
                    Some(before),
                    snapshot(&row)?,
                ),
            )
            .await?;
        }
        tx.commit().await.map_err(store::storage_error)?;
        Ok(row)
    }
    pub async fn adoption_preview(
        &self,
        world_id: &str,
        id: &str,
        target_version: i64,
    ) -> Result<AdoptionPreview, Error> {
        let mut tx = self
            .accounts
            .pool()
            .begin()
            .await
            .map_err(store::storage_error)?;
        let entry = store::variant(&mut tx, world_id, id).await?;
        let pin = entry
            .source
            .as_ref()
            .ok_or_else(|| invalid("source", "World-exclusive entries have no source to adopt."))?;
        if target_version <= pin.version {
            return Err(invalid(
                "target_version",
                "Select a newer version of the same source form.",
            ));
        }
        let old_source = store::form_version(&mut tx, &pin.form_id, pin.version).await?;
        let new_source = store::form_version(&mut tx, &pin.form_id, target_version).await?;
        let old_species =
            store::species_version(&mut tx, &old_source.species_id, old_source.species_version)
                .await?;
        let new_species =
            store::species_version(&mut tx, &new_source.species_id, new_source.species_version)
                .await?;
        tx.commit().await.map_err(store::storage_error)?;
        Ok(AdoptionPreview {
            entry,
            old_source,
            new_source,
            old_species,
            new_species,
        })
    }
    pub async fn access(&self, token: &str, world_id: &str) -> Result<AccessState, Error> {
        let mut tx = self
            .accounts
            .pool()
            .begin()
            .await
            .map_err(store::storage_error)?;
        let actor = self.actor(&mut tx, token).await?;
        let world = store::world(&mut tx, world_id).await?;
        self.world_actor(&mut tx, &actor, &world, true).await?;
        let collaborators = store::collaborators(&mut tx, world_id).await?;
        let history = store::access_history(&mut tx, world_id).await?;
        tx.commit().await.map_err(store::storage_error)?;
        Ok(AccessState {
            world,
            collaborators,
            history,
        })
    }
    pub async fn grant_collaborator(
        &self,
        token: &str,
        world_id: &str,
        expected_access_revision: i64,
        user_id: &str,
        reason: Option<&str>,
    ) -> Result<(), Error> {
        self.change_access(
            token,
            world_id,
            expected_access_revision,
            user_id,
            reason,
            true,
        )
        .await
    }
    pub async fn revoke_collaborator(
        &self,
        token: &str,
        world_id: &str,
        expected_access_revision: i64,
        user_id: &str,
        reason: Option<&str>,
    ) -> Result<(), Error> {
        self.change_access(
            token,
            world_id,
            expected_access_revision,
            user_id,
            reason,
            false,
        )
        .await
    }
    async fn change_access(
        &self,
        token: &str,
        world_id: &str,
        revision: i64,
        user_id: &str,
        reason: Option<&str>,
        grant: bool,
    ) -> Result<(), Error> {
        let mut tx = self.write().await?;
        let actor = self.actor(&mut tx, token).await?;
        let mut world = store::world(&mut tx, world_id).await?;
        self.world_actor(&mut tx, &actor, &world, true).await?;
        expected(world.access_revision, revision)?;
        let reason = validate_reason(reason).map_err(Error::Invalid)?;
        if user_id == world.owner_user_id {
            return Err(invalid("user_id", "The owner is not a collaborator."));
        }
        let user = persistence::user_by_id(&mut *tx, user_id)
            .await
            .map_err(store::storage_error)?
            .ok_or(Error::NotFound)?;
        if grant
            && (user.is_disabled != 0
                || !matches!(
                    Role::parse(&user.role),
                    Some(Role::Editor | Role::Administrator)
                ))
        {
            return Err(invalid(
                "user_id",
                "Choose an enabled editor or administrator.",
            ));
        }
        let present = store::is_collaborator(&mut tx, world_id, user_id).await?;
        if present != grant {
            let before = access_snapshot(&mut tx, &world).await?;
            store::set_collaborator(&mut tx, world_id, user_id, grant).await?;
            world.access_revision = next(world.access_revision)?;
            let after = access_snapshot(&mut tx, &world).await?;
            store::put_access_revision(
                &mut tx,
                &world,
                store::Revision::new(
                    Some(&actor.user_id),
                    if grant { "grant" } else { "revoke" },
                    reason.as_deref(),
                    Some(before),
                    after,
                ),
            )
            .await?;
        }
        tx.commit().await.map_err(store::storage_error)
    }
    pub async fn history(
        &self,
        kind: HistoryKind,
        id: &str,
        before: Option<i64>,
    ) -> Result<Vec<HistoryEntry>, Error> {
        store::history(
            &mut *self
                .accounts
                .pool()
                .acquire()
                .await
                .map_err(store::storage_error)?,
            kind,
            id,
            before,
            None,
        )
        .await
    }
    pub async fn history_entry(
        &self,
        kind: HistoryKind,
        id: &str,
        revision: i64,
    ) -> Result<HistoryEntry, Error> {
        store::history(
            &mut *self
                .accounts
                .pool()
                .acquire()
                .await
                .map_err(store::storage_error)?,
            kind,
            id,
            None,
            Some(revision),
        )
        .await?
        .into_iter()
        .next()
        .ok_or(Error::NotFound)
    }
    /// Trusted local operator only. Compare immutable initial baselines, never live edits.
    pub async fn seed_catalog(&self, catalog: &Catalog) -> Result<SeedReport, Error> {
        // Catalog construction validates IDs and uniqueness; validate authoring text too.
        let mut rows = Vec::new();
        for creature in catalog.creatures() {
            let fields = ContentFields {
                slug: creature.slug().into(),
                display_name: creature.display_name().into(),
                description: creature.description().into(),
            };
            let normalized = fields.normalized().map_err(Error::Invalid)?;
            if normalized != fields {
                return Err(invalid(
                    "catalog",
                    "Fixture content must already be normalized.",
                ));
            }
            rows.push((creature.id(), fields));
        }
        let mut tx = self.write().await?;
        let mut report = SeedReport::default();
        for (id, fields) in rows {
            if store::species_exists(&mut tx, id).await? {
                let baseline = store::species_version(&mut tx, id, 1).await?;
                let form = store::form_version(&mut tx, &baseline.default_form_id, 1).await?;
                if baseline.fields != fields
                    || baseline.archived_at.is_some()
                    || form.fields != fields
                    || !form.is_default
                    || form.species_id != id
                    || form.species_version != 1
                    || form.archived_at.is_some()
                    || !store::fixture_baseline(&mut tx, id, &form.id).await?
                {
                    return Err(invalid(
                        "catalog",
                        "The fixture baseline differs from this catalog or collides with authored content.",
                    ));
                }
                report.species_skipped += 1;
            } else {
                if store::initial_slug_exists(&mut tx, &fields.slug).await? {
                    return Err(invalid(
                        "catalog",
                        "A fixture slug has a different initial species identity.",
                    ));
                }
                self.insert_species(&mut tx, id.into(), fields, None, None)
                    .await?;
                report.species_created += 1;
                report.forms_created += 1;
            }
        }
        tx.commit().await.map_err(store::storage_error)?;
        Ok(report)
    }
}
