//! Exact Type definitions, ordered assignments and explicit fixture use cases.
use super::authoring::{
    self as author, AddressKey, Authoring, Edit, Error, FormRecord, HistoryKind, SpeciesRecord,
    VariantRecord, WorldRecord,
};
use crate::{
    accounts::Accounts,
    persistence::{authoring as content, types as store},
};
use game_data::{
    authoring::{ContentFields, validate_reason},
    types::{TypePin, validate_pins},
};
use serde::Deserialize;
use sqlx::{Sqlite, Transaction};
use std::collections::HashSet;

pub use game_data::types::{TypeAssignment, TypeRecord, TypingState, WorldTypeRecord};
#[derive(Clone, Debug)]
pub struct TypeAdoptionPreview {
    pub entry: WorldTypeRecord,
    pub old_source: TypeRecord,
    pub new_source: TypeRecord,
}
#[derive(Clone, Debug, Default)]
pub struct FixtureReport {
    pub shared_stats_created: usize,
    pub world_stats_created: usize,
    pub stat_rosters_created: usize,
    pub shared_charts_created: usize,
    pub world_charts_created: usize,
    pub types_created: usize,
    pub species_created: usize,
    pub forms_created: usize,
    pub worlds_created: usize,
    pub world_types_created: usize,
    pub variants_created: usize,
    pub skipped: usize,
}
#[derive(Clone, Debug)]
pub struct Types {
    accounts: Accounts,
    authoring: Authoring,
}
impl Types {
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
    pub async fn types(&self) -> Result<Vec<TypeRecord>, Error> {
        store::types(&mut *self.read().await?).await
    }
    pub async fn type_detail(&self, id: &str) -> Result<TypeRecord, Error> {
        store::type_detail(&mut *self.read().await?, id).await
    }
    pub async fn type_version(&self, id: &str, version: i64) -> Result<TypeRecord, Error> {
        store::type_version(&mut *self.read().await?, id, version).await
    }
    pub async fn world_types(&self, world_id: &str) -> Result<Vec<WorldTypeRecord>, Error> {
        let mut tx = self.read().await?;
        content::world(&mut tx, world_id).await?;
        store::world_types(&mut tx, world_id).await
    }
    pub async fn world_type_detail(
        &self,
        world_id: &str,
        id: &str,
    ) -> Result<WorldTypeRecord, Error> {
        store::world_type_detail(&mut *self.read().await?, world_id, id).await
    }
    pub async fn world_type_version(
        &self,
        world_id: &str,
        id: &str,
        version: i64,
    ) -> Result<WorldTypeRecord, Error> {
        store::world_type_version(&mut *self.read().await?, world_id, id, version).await
    }
    pub async fn resolve_type(&self, key: AddressKey) -> Result<TypeRecord, Error> {
        store::resolve_type(&mut *self.read().await?, key).await
    }
    pub async fn resolve_world_type(
        &self,
        world_key: AddressKey,
        type_key: AddressKey,
    ) -> Result<(WorldRecord, WorldTypeRecord), Error> {
        let mut tx = self.read().await?;
        let world = content::resolve_world(&mut tx, world_key).await?;
        let row = store::resolve_world_type(&mut tx, &world.id, type_key).await?;
        Ok((world, row))
    }
    pub async fn create_type(
        &self,
        token: &str,
        fields: ContentFields,
        reason: Option<&str>,
    ) -> Result<TypeRecord, Error> {
        let mut tx = self.authoring.write().await?;
        let actor = self.authoring.actor(&mut tx, token).await?;
        author::require_admin(&actor)?;
        let (mut fields, reason) = author::fields_and_reason(&fields, reason)?;
        author::allocate_slug(&mut tx, content::SlugScope::Type, &mut fields, None).await?;
        let row = TypeRecord {
            id: author::new_id()?,
            fields,
            revision: 1,
            archived_at: None,
        };
        store::put_type(
            &mut tx,
            &row,
            content::Revision::new(
                Some(&actor.user_id),
                "create",
                reason.as_deref(),
                None,
                author::snapshot(&row)?,
            ),
        )
        .await?;
        tx.commit().await.map_err(content::storage_error)?;
        Ok(row)
    }
    pub async fn save_type(&self, token: &str, id: &str, edit: Edit) -> Result<TypeRecord, Error> {
        let mut tx = self.authoring.write().await?;
        let actor = self.authoring.actor(&mut tx, token).await?;
        author::require_admin(&actor)?;
        let mut row = store::type_detail(&mut tx, id).await?;
        author::expected(row.revision, edit.expected_revision)?;
        let (mut fields, reason) = author::fields_and_reason(&edit.fields, edit.reason.as_deref())?;
        author::allocate_slug(
            &mut tx,
            content::SlugScope::Type,
            &mut fields,
            Some((id, &row.fields)),
        )
        .await?;
        if row.fields != fields {
            let before = store::type_after(&mut tx, id, row.revision).await?;
            row.fields = fields;
            row.revision = author::next(row.revision)?;
            store::put_type(
                &mut tx,
                &row,
                content::Revision::new(
                    Some(&actor.user_id),
                    "edit",
                    reason.as_deref(),
                    Some(before),
                    author::snapshot(&row)?,
                ),
            )
            .await?;
        }
        tx.commit().await.map_err(content::storage_error)?;
        Ok(row)
    }
    #[allow(clippy::too_many_arguments)]
    pub async fn create_world_type(
        &self,
        token: &str,
        world_id: &str,
        expected_world_revision: i64,
        source: Option<TypePin>,
        fields: ContentFields,
        reason: Option<&str>,
    ) -> Result<WorldTypeRecord, Error> {
        let mut tx = self.authoring.write().await?;
        let actor = self.authoring.actor(&mut tx, token).await?;
        let mut world = content::world(&mut tx, world_id).await?;
        self.authoring
            .world_actor(&mut tx, &actor, &world, false)
            .await?;
        author::expected(world.revision, expected_world_revision)?;
        let (mut fields, reason) = author::fields_and_reason(&fields, reason)?;
        if let Some(pin) = &source {
            store::select_shared_pin(&mut tx, pin).await?;
        }
        author::allocate_slug(
            &mut tx,
            content::SlugScope::WorldType(world_id),
            &mut fields,
            None,
        )
        .await?;
        let before = author::world_snapshot(&mut tx, &world).await?;
        let row = WorldTypeRecord {
            id: author::new_id()?,
            world_id: world_id.into(),
            source,
            fields,
            revision: 1,
            archived_at: None,
        };
        store::put_world_type(
            &mut tx,
            &row,
            content::Revision::new(
                Some(&actor.user_id),
                "create",
                reason.as_deref(),
                None,
                author::snapshot(&row)?,
            ),
        )
        .await?;
        world.revision = author::next(world.revision)?;
        let after = author::world_snapshot(&mut tx, &world).await?;
        content::put_world(
            &mut tx,
            &world,
            content::Revision::new(
                Some(&actor.user_id),
                "edit",
                reason.as_deref(),
                Some(before),
                after,
            ),
        )
        .await?;
        tx.commit().await.map_err(content::storage_error)?;
        Ok(row)
    }
    pub async fn save_world_type(
        &self,
        token: &str,
        world_id: &str,
        id: &str,
        edit: Edit,
    ) -> Result<WorldTypeRecord, Error> {
        self.edit_world_type(token, world_id, id, None, edit).await
    }
    pub async fn adopt_world_type(
        &self,
        token: &str,
        world_id: &str,
        id: &str,
        target_version: i64,
        edit: Edit,
    ) -> Result<WorldTypeRecord, Error> {
        self.edit_world_type(token, world_id, id, Some(target_version), edit)
            .await
    }
    async fn edit_world_type(
        &self,
        token: &str,
        world_id: &str,
        id: &str,
        target: Option<i64>,
        edit: Edit,
    ) -> Result<WorldTypeRecord, Error> {
        let mut tx = self.authoring.write().await?;
        let actor = self.authoring.actor(&mut tx, token).await?;
        let world = content::world(&mut tx, world_id).await?;
        self.authoring
            .world_actor(&mut tx, &actor, &world, false)
            .await?;
        let mut row = store::world_type_detail(&mut tx, world_id, id).await?;
        author::expected(row.revision, edit.expected_revision)?;
        let (mut fields, reason) = author::fields_and_reason(&edit.fields, edit.reason.as_deref())?;
        let before = store::world_type_after(&mut tx, world_id, id, row.revision).await?;
        if let Some(version) = target {
            let pin = row.source.as_mut().ok_or_else(|| {
                author::invalid("source", "World-exclusive Types have no source to adopt.")
            })?;
            newer(version, pin.version)?;
            store::select_shared_pin(
                &mut tx,
                &TypePin {
                    type_id: pin.type_id.clone(),
                    version,
                },
            )
            .await?;
            pin.version = version;
        }
        author::allocate_slug(
            &mut tx,
            content::SlugScope::WorldType(world_id),
            &mut fields,
            Some((id, &row.fields)),
        )
        .await?;
        if fields != row.fields || target.is_some() {
            row.fields = fields;
            row.revision = author::next(row.revision)?;
            store::put_world_type(
                &mut tx,
                &row,
                content::Revision::new(
                    Some(&actor.user_id),
                    if target.is_some() { "adopt" } else { "edit" },
                    reason.as_deref(),
                    Some(before),
                    author::snapshot(&row)?,
                ),
            )
            .await?;
        }
        tx.commit().await.map_err(content::storage_error)?;
        Ok(row)
    }
    pub async fn adoption_preview(
        &self,
        world_id: &str,
        id: &str,
        target_version: i64,
    ) -> Result<TypeAdoptionPreview, Error> {
        let mut tx = self.read().await?;
        let entry = store::world_type_detail(&mut tx, world_id, id).await?;
        let pin = entry.source.as_ref().ok_or_else(|| {
            author::invalid("source", "World-exclusive Types have no source to adopt.")
        })?;
        newer(target_version, pin.version)?;
        let old_source = store::type_version(&mut tx, &pin.type_id, pin.version).await?;
        let new_source = store::type_version(&mut tx, &pin.type_id, target_version).await?;
        Ok(TypeAdoptionPreview {
            entry,
            old_source,
            new_source,
        })
    }
    pub async fn form_typing(&self, id: &str, revision: Option<i64>) -> Result<TypingState, Error> {
        let mut tx = self.read().await?;
        let revision = match revision {
            Some(v) => v,
            None => content::form(&mut tx, id).await?.revision,
        };
        store::form_typing(&mut tx, id, revision).await
    }
    pub async fn variant_typing(
        &self,
        world_id: &str,
        id: &str,
        revision: Option<i64>,
    ) -> Result<TypingState, Error> {
        let mut tx = self.read().await?;
        let revision = match revision {
            Some(v) => v,
            None => content::variant(&mut tx, world_id, id).await?.revision,
        };
        store::variant_typing(&mut tx, world_id, id, revision).await
    }
    pub async fn save_form_typing(
        &self,
        token: &str,
        id: &str,
        expected_revision: i64,
        pins: Option<Vec<TypePin>>,
        reason: Option<&str>,
    ) -> Result<FormRecord, Error> {
        let mut tx = self.authoring.write().await?;
        let actor = self.authoring.actor(&mut tx, token).await?;
        author::require_admin(&actor)?;
        let mut row = content::form(&mut tx, id).await?;
        author::expected(row.revision, expected_revision)?;
        let reason = validate_reason(reason).map_err(Error::Invalid)?;
        validate_pins(pins.as_deref()).map_err(Error::Invalid)?;
        let old = store::form_typing(&mut tx, id, row.revision).await?;
        if old.pins() != pins {
            let typing = store::select_typing(&mut tx, None, pins).await?;
            row.revision = author::next(row.revision)?;
            content::put_form_typed(
                &mut tx,
                &row,
                content::Revision::new(
                    Some(&actor.user_id),
                    "edit",
                    reason.as_deref(),
                    None,
                    author::snapshot(&row)?,
                ),
                Some(&typing),
            )
            .await?;
        }
        tx.commit().await.map_err(content::storage_error)?;
        Ok(row)
    }
    #[allow(clippy::too_many_arguments)]
    pub async fn save_variant_typing(
        &self,
        token: &str,
        world_id: &str,
        id: &str,
        expected_revision: i64,
        pins: Option<Vec<TypePin>>,
        reason: Option<&str>,
    ) -> Result<VariantRecord, Error> {
        let mut tx = self.authoring.write().await?;
        let actor = self.authoring.actor(&mut tx, token).await?;
        let world = content::world(&mut tx, world_id).await?;
        self.authoring
            .world_actor(&mut tx, &actor, &world, false)
            .await?;
        let mut row = content::variant(&mut tx, world_id, id).await?;
        author::expected(row.revision, expected_revision)?;
        let reason = validate_reason(reason).map_err(Error::Invalid)?;
        validate_pins(pins.as_deref()).map_err(Error::Invalid)?;
        let old = store::variant_typing(&mut tx, world_id, id, row.revision).await?;
        if old.pins() != pins {
            let typing = store::select_typing(&mut tx, Some(world_id), pins).await?;
            row.revision = author::next(row.revision)?;
            content::put_variant_typed(
                &mut tx,
                &row,
                content::Revision::new(
                    Some(&actor.user_id),
                    "edit",
                    reason.as_deref(),
                    None,
                    author::snapshot(&row)?,
                ),
                Some(&typing),
            )
            .await?;
        }
        tx.commit().await.map_err(content::storage_error)?;
        Ok(row)
    }
}
fn newer(target: i64, old: i64) -> Result<(), Error> {
    if target <= old {
        Err(author::invalid(
            "target_version",
            "Select a newer version of the same source Type.",
        ))
    } else {
        Ok(())
    }
}

const REFERENCE_FIXTURES: &str = include_str!("../../../../fixtures/reference-types.json");
const WORLD_FIXTURES: &str = include_str!("../../../../fixtures/worlds.json");
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ReferenceFixtures {
    fixture_version: i64,
    types: Vec<ReferenceType>,
    species: ReferenceSpecies,
    forms: Vec<ReferenceForm>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReferenceType {
    record: TypeRecord,
    source_file: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReferenceSpecies {
    record: SpeciesRecord,
    source_file: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReferenceForm {
    record: FormRecord,
    typing: Vec<TypePin>,
    source_file: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WorldFixtures {
    fixture_version: i64,
    worlds: Vec<FixtureWorld>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureWorld {
    id: String,
    fields: ContentFields,
    types: Vec<WorldTypeRecord>,
    variants: Vec<FixtureVariant>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureVariant {
    record: VariantRecord,
    typing: Option<Vec<TypePin>>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CachedReference {
    source: store::ReferenceSource,
    projection: serde_json::Value,
}
fn fixture_error() -> Error {
    author::invalid(
        "fixtures",
        "The fixture creation baseline is incomplete, differs from the bundled manifest, or collides with authored content. No fixtures were changed.",
    )
}
fn cache(file: &str) -> Result<CachedReference, Error> {
    let s = match file {
        "type-fire.json" => include_str!("../../../../fixtures/pokeapi/type-fire.json"),
        "type-flying.json" => include_str!("../../../../fixtures/pokeapi/type-flying.json"),
        "type-dragon.json" => include_str!("../../../../fixtures/pokeapi/type-dragon.json"),
        "species-charizard.json" => {
            include_str!("../../../../fixtures/pokeapi/species-charizard.json")
        }
        "form-charizard.json" => include_str!("../../../../fixtures/pokeapi/form-charizard.json"),
        "form-charizard-mega-x.json" => {
            include_str!("../../../../fixtures/pokeapi/form-charizard-mega-x.json")
        }
        _ => return Err(fixture_error()),
    };
    serde_json::from_str(s).map_err(|_| fixture_error())
}
fn fixture_fields(fields: &ContentFields) -> Result<(), Error> {
    if fields.normalized().ok().as_ref() == Some(fields) {
        Ok(())
    } else {
        Err(fixture_error())
    }
}
fn fixture_id(ids: &mut HashSet<String>, id: &str) -> Result<(), Error> {
    if game_data::types::valid_id(id) && ids.insert(id.into()) {
        Ok(())
    } else {
        Err(fixture_error())
    }
}
impl ReferenceFixtures {
    pub(super) fn load() -> Result<Self, Error> {
        let value: Self = serde_json::from_str(REFERENCE_FIXTURES).map_err(|_| fixture_error())?;
        if value.fixture_version != 1 || value.types.is_empty() || value.forms.is_empty() {
            return Err(fixture_error());
        }
        let mut ids = HashSet::new();
        for t in &value.types {
            fixture_id(&mut ids, &t.record.id)?;
            fixture_fields(&t.record.fields)?;
            let cached = cache(&t.source_file)?;
            if t.record.revision != 1
                || t.record.archived_at.is_some()
                || !cached.source.valid("type")
                || cached.projection["id"] != cached.source.external_id
                || cached.projection["english_name"] != t.record.fields.display_name
                || cached.projection["name"] != t.record.fields.slug
            {
                return Err(fixture_error());
            }
        }
        let s = &value.species.record;
        fixture_id(&mut ids, &s.id)?;
        fixture_fields(&s.fields)?;
        let cached = cache(&value.species.source_file)?;
        if s.revision != 1
            || s.archived_at.is_some()
            || !cached.source.valid("pokemon-species")
            || cached.projection["id"] != cached.source.external_id
            || cached.projection["english_name"] != s.fields.display_name
        {
            return Err(fixture_error());
        }
        for f in &value.forms {
            let r = &f.record;
            fixture_id(&mut ids, &r.id)?;
            fixture_fields(&r.fields)?;
            validate_pins(Some(&f.typing)).map_err(|_| fixture_error())?;
            let cached = cache(&f.source_file)?;
            if r.revision != 1
                || r.archived_at.is_some()
                || r.species_id != s.id
                || r.species_version != 1
                || r.is_default != (r.id == s.default_form_id)
                || !cached.source.valid("pokemon-form")
                || cached.projection["id"] != cached.source.external_id
            {
                return Err(fixture_error());
            }
            let slots = cached.projection["types"]
                .as_array()
                .ok_or_else(fixture_error)?;
            if slots.len() != f.typing.len() {
                return Err(fixture_error());
            }
            for (i, (slot, pin)) in slots.iter().zip(&f.typing).enumerate() {
                let t = value
                    .types
                    .iter()
                    .find(|t| t.record.id == pin.type_id)
                    .ok_or_else(fixture_error)?;
                if pin.version != 1
                    || slot["slot"] != (i + 1)
                    || slot["type_url"] != cache(&t.source_file)?.source.source_url
                {
                    return Err(fixture_error());
                }
            }
        }
        if !value.forms.iter().any(|f| f.record.id == s.default_form_id) {
            return Err(fixture_error());
        }
        Ok(value)
    }
}
impl WorldFixtures {
    pub(super) fn load(reference: &ReferenceFixtures) -> Result<Self, Error> {
        let value: Self = serde_json::from_str(WORLD_FIXTURES).map_err(|_| fixture_error())?;
        if value.fixture_version != 1 || value.worlds.is_empty() {
            return Err(fixture_error());
        }
        let mut ids = HashSet::new();
        for w in &value.worlds {
            fixture_id(&mut ids, &w.id)?;
            fixture_fields(&w.fields)?;
            for t in &w.types {
                fixture_id(&mut ids, &t.id)?;
                fixture_fields(&t.fields)?;
                if t.world_id != w.id
                    || t.revision != 1
                    || t.archived_at.is_some()
                    || t.source.as_ref().is_some_and(|p| {
                        p.version != 1 || !reference.types.iter().any(|t| t.record.id == p.type_id)
                    })
                {
                    return Err(fixture_error());
                }
            }
            for v in &w.variants {
                fixture_id(&mut ids, &v.record.id)?;
                fixture_fields(&v.record.fields)?;
                validate_pins(v.typing.as_deref()).map_err(|_| fixture_error())?;
                if v.record.world_id != w.id
                    || v.record.revision != 1
                    || v.record.archived_at.is_some()
                    || v.record.source.as_ref().is_some_and(|p| {
                        p.version != 1 || !reference.forms.iter().any(|f| f.record.id == p.form_id)
                    })
                    || v.typing.as_ref().is_some_and(|pins| {
                        pins.iter()
                            .any(|p| p.version != 1 || !w.types.iter().any(|t| t.id == p.type_id))
                    })
                {
                    return Err(fixture_error());
                }
            }
        }
        Ok(value)
    }

    pub(super) async fn existing_owner(
        &self,
        c: &mut sqlx::SqliteConnection,
    ) -> Result<Option<String>, Error> {
        let mut owner = None;
        let mut present = 0;
        for fixture in &self.worlds {
            if store::identity_exists(c, &fixture.id).await? {
                let world = content::world(c, &fixture.id)
                    .await
                    .map_err(|_| fixture_error())?;
                if owner.as_ref().is_some_and(|id| id != &world.owner_user_id) {
                    return Err(fixture_error());
                }
                owner = Some(world.owner_user_id);
                present += 1;
            }
        }
        if present > 0 && present != self.worlds.len() {
            return Err(fixture_error());
        }
        Ok(owner)
    }
}
impl Types {
    /// Trusted local operator only. No runtime network requests or synthetic accounts.
    pub async fn seed_reference_fixtures(&self) -> Result<FixtureReport, Error> {
        let reference = ReferenceFixtures::load()?;
        let mut tx = self.authoring.write().await?;
        let mut report = FixtureReport::default();
        self.seed_reference_in(&mut tx, &reference, &mut report)
            .await?;
        tx.commit().await.map_err(content::storage_error)?;
        Ok(report)
    }
    /// Prerequisites and complete worlds commit together, after validating the real owner.
    pub async fn seed_world_fixtures(&self, owner_user_id: &str) -> Result<FixtureReport, Error> {
        let reference = ReferenceFixtures::load()?;
        let worlds = WorldFixtures::load(&reference)?;
        let mut tx = self.authoring.write().await?;
        let report = self
            .seed_world_fixtures_in(&mut tx, &reference, &worlds, owner_user_id)
            .await?;
        tx.commit().await.map_err(content::storage_error)?;
        Ok(report)
    }

    pub(super) async fn seed_world_fixtures_in(
        &self,
        c: &mut sqlx::SqliteConnection,
        reference: &ReferenceFixtures,
        worlds: &WorldFixtures,
        owner_user_id: &str,
    ) -> Result<FixtureReport, Error> {
        let owner = crate::persistence::user_by_id(&mut *c, owner_user_id)
            .await
            .map_err(content::storage_error)?
            .ok_or_else(|| {
                author::invalid(
                    "owner_user_id",
                    "Choose an existing enabled editor or administrator.",
                )
            })?;
        if owner.is_disabled != 0
            || !matches!(
                crate::auth::Role::parse(&owner.role),
                Some(crate::auth::Role::Editor | crate::auth::Role::Administrator)
            )
        {
            return Err(author::invalid(
                "owner_user_id",
                "Choose an existing enabled editor or administrator.",
            ));
        }
        let mut report = FixtureReport::default();
        worlds.existing_owner(c).await?;
        self.seed_reference_in(c, reference, &mut report).await?;
        for fixture in &worlds.worlds {
            if store::identity_exists(c, &fixture.id).await? {
                self.check_world_baseline(c, fixture, owner_user_id).await?;
                report.skipped += 1 + fixture.types.len() + fixture.variants.len();
            } else {
                if store::initial_fixture_slug_exists(c, HistoryKind::World, &fixture.fields.slug)
                    .await?
                {
                    return Err(fixture_error());
                }
                // A child without its manifest world is a collision, never a repair opportunity.
                for id in fixture
                    .types
                    .iter()
                    .map(|t| &t.id)
                    .chain(fixture.variants.iter().map(|v| &v.record.id))
                {
                    if store::identity_exists(c, id).await? {
                        return Err(fixture_error());
                    }
                }
                let world = WorldRecord {
                    id: fixture.id.clone(),
                    owner_user_id: owner_user_id.into(),
                    fields: fixture.fields.clone(),
                    revision: 1,
                    access_revision: 1,
                    archived_at: None,
                };
                content::put_world_head(c, &world).await?;
                for row in &fixture.types {
                    store::put_world_type(
                        c,
                        row,
                        content::Revision::new(None, "create", None, None, author::snapshot(row)?),
                    )
                    .await?;
                    report.world_types_created += 1;
                }
                for v in &fixture.variants {
                    let typing = fixture_typing(c, Some(&world.id), v.typing.clone()).await?;
                    content::put_variant_typed(
                        c,
                        &v.record,
                        content::Revision::new(
                            None,
                            "create",
                            None,
                            None,
                            author::snapshot(&v.record)?,
                        ),
                        Some(&typing),
                    )
                    .await?;
                    report.variants_created += 1;
                }
                let after = author::world_snapshot(c, &world).await?;
                content::seal_world_revision(
                    c,
                    &world,
                    content::Revision::new(None, "create", None, None, after),
                )
                .await?;
                let access = author::access_snapshot(c, &world).await?;
                content::put_access_revision(
                    c,
                    &world,
                    content::Revision::new(None, "own", None, None, access),
                )
                .await?;
                report.worlds_created += 1;
            }
        }
        Ok(report)
    }
    async fn seed_reference_in(
        &self,
        c: &mut sqlx::SqliteConnection,
        reference: &ReferenceFixtures,
        report: &mut FixtureReport,
    ) -> Result<(), Error> {
        let mut present = 0;
        let ids: Vec<&str> = reference
            .types
            .iter()
            .map(|t| t.record.id.as_str())
            .chain(std::iter::once(reference.species.record.id.as_str()))
            .chain(reference.forms.iter().map(|f| f.record.id.as_str()))
            .collect();
        for id in &ids {
            if store::identity_exists(c, id).await? {
                present += 1;
            }
        }
        if present > 0 && present != ids.len() {
            return Err(fixture_error());
        }
        if present > 0 {
            for t in &reference.types {
                let row = store::type_version(c, &t.record.id, 1)
                    .await
                    .map_err(|_| fixture_error())?;
                if row != t.record
                    || !store::fixture_actor(c, HistoryKind::Type, &row.id).await?
                    || !store::reference_source_matches(
                        c,
                        HistoryKind::Type,
                        &row.id,
                        &cache(&t.source_file)?.source,
                    )
                    .await?
                {
                    return Err(fixture_error());
                }
            }
            let s = &reference.species.record;
            if content::species_version(c, &s.id, 1)
                .await
                .map_err(|_| fixture_error())?
                != *s
                || !store::fixture_actor(c, HistoryKind::Species, &s.id).await?
                || !store::reference_source_matches(
                    c,
                    HistoryKind::Species,
                    &s.id,
                    &cache(&reference.species.source_file)?.source,
                )
                .await?
            {
                return Err(fixture_error());
            }
            // Validate species JSON, not just its typed metadata baseline.
            let species_history =
                content::history(c, HistoryKind::Species, &s.id, None, Some(1)).await?;
            let stored = species_history.first().ok_or_else(fixture_error)?;
            if serde_json::from_str::<serde_json::Value>(&stored.after_json)
                .map_err(|_| fixture_error())?
                != serde_json::from_str::<serde_json::Value>(&author::snapshot(s)?)
                    .map_err(|_| fixture_error())?
            {
                return Err(fixture_error());
            }
            for f in &reference.forms {
                let row = content::form_version(c, &f.record.id, 1)
                    .await
                    .map_err(|_| fixture_error())?;
                if row != f.record
                    || store::form_typing(c, &row.id, 1).await?.pins() != Some(f.typing.clone())
                    || !store::fixture_actor(c, HistoryKind::Form, &row.id).await?
                    || !store::reference_source_matches(
                        c,
                        HistoryKind::Form,
                        &row.id,
                        &cache(&f.source_file)?.source,
                    )
                    .await?
                {
                    return Err(fixture_error());
                }
            }
            report.skipped += ids.len();
            return Ok(());
        }
        for t in &reference.types {
            if content::slug_exists(c, content::SlugScope::Type, &t.record.fields.slug, None)
                .await?
                || store::initial_fixture_slug_exists(c, HistoryKind::Type, &t.record.fields.slug)
                    .await?
            {
                return Err(fixture_error());
            }
            store::put_type(
                c,
                &t.record,
                content::Revision::new(None, "create", None, None, author::snapshot(&t.record)?),
            )
            .await?;
            store::put_reference_source(
                c,
                HistoryKind::Type,
                &t.record.id,
                &cache(&t.source_file)?.source,
            )
            .await?;
            report.types_created += 1;
        }
        let s = &reference.species.record;
        if content::slug_exists(c, content::SlugScope::Species, &s.fields.slug, None).await?
            || content::initial_slug_exists(c, &s.fields.slug).await?
        {
            return Err(fixture_error());
        }
        content::put_species(
            c,
            s,
            content::Revision::new(None, "create", None, None, author::snapshot(s)?),
        )
        .await?;
        store::put_reference_source(
            c,
            HistoryKind::Species,
            &s.id,
            &cache(&reference.species.source_file)?.source,
        )
        .await?;
        report.species_created += 1;
        for f in &reference.forms {
            let typing = fixture_typing(c, None, Some(f.typing.clone())).await?;
            content::put_form_typed(
                c,
                &f.record,
                content::Revision::new(None, "create", None, None, author::snapshot(&f.record)?),
                Some(&typing),
            )
            .await?;
            store::put_reference_source(
                c,
                HistoryKind::Form,
                &f.record.id,
                &cache(&f.source_file)?.source,
            )
            .await?;
            report.forms_created += 1;
        }
        Ok(())
    }
    async fn check_world_baseline(
        &self,
        c: &mut sqlx::SqliteConnection,
        fixture: &FixtureWorld,
        owner: &str,
    ) -> Result<(), Error> {
        let world = content::world(c, &fixture.id)
            .await
            .map_err(|_| fixture_error())?;
        if world.owner_user_id != owner
            || !store::fixture_actor(c, HistoryKind::World, &fixture.id).await?
            || !store::fixture_access(c, &fixture.id, owner).await?
        {
            return Err(fixture_error());
        }
        let mut variants: Vec<&str> = fixture
            .variants
            .iter()
            .map(|v| v.record.id.as_str())
            .collect();
        variants.sort();
        let mut types: Vec<&str> = fixture.types.iter().map(|t| t.id.as_str()).collect();
        types.sort();
        let initial = store::full_content(c, HistoryKind::World, &fixture.id, None, 1).await?;
        let value: serde_json::Value =
            serde_json::from_str(&initial.after).map_err(|_| fixture_error())?;
        let expected = serde_json::json!({"snapshot_version":2,"content":{"id":fixture.id,"fields":fixture.fields,"revision":1,"archived_at":null,"variant_ids":variants,"type_ids":types}});
        if value != expected {
            return Err(fixture_error());
        }
        for t in &fixture.types {
            if store::world_type_version(c, &fixture.id, &t.id, 1)
                .await
                .map_err(|_| fixture_error())?
                != *t
                || !store::fixture_actor(c, HistoryKind::WorldType, &t.id).await?
            {
                return Err(fixture_error());
            }
        }
        for v in &fixture.variants {
            let initial =
                store::full_content(c, HistoryKind::Variant, &v.record.id, Some(&fixture.id), 1)
                    .await
                    .map_err(|_| fixture_error())?;
            let projection: serde_json::Value =
                serde_json::from_str(&initial.projection_after).map_err(|_| fixture_error())?;
            if projection["content"]
                != serde_json::to_value(&v.record).map_err(|_| fixture_error())?
                || initial.typing.pins() != v.typing
                || !store::fixture_actor(c, HistoryKind::Variant, &v.record.id).await?
            {
                return Err(fixture_error());
            }
        }
        Ok(())
    }
}
async fn fixture_typing(
    c: &mut sqlx::SqliteConnection,
    world: Option<&str>,
    pins: Option<Vec<TypePin>>,
) -> Result<TypingState, Error> {
    let Some(pins) = pins else {
        return Ok(TypingState::unmodeled());
    };
    let mut assignments = Vec::new();
    for pin in pins {
        let fields = if let Some(world) = world {
            store::world_type_version(c, world, &pin.type_id, pin.version)
                .await?
                .fields
        } else {
            store::type_version(c, &pin.type_id, pin.version)
                .await?
                .fields
        };
        assignments.push(TypeAssignment { pin, fields });
    }
    Ok(TypingState {
        assignments: Some(assignments),
    })
}
