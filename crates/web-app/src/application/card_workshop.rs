//! Original ADR0017 examples, initialized only by the explicit trusted operator command.
//! This typed v1 manifest is an immutable creation contract, independent of live heads.
use super::authoring::{self as author, Error, HistoryKind, VariantRecord, WorldRecord};
use crate::{
    accounts::Accounts,
    auth::Role,
    persistence::{self, authoring as content, cards, moves, types},
};
use game_data::{
    authoring::ContentFields,
    card_fields::{CardField, ChoiceOption, FieldDefinition, FieldKind, FieldValue},
    cards::{CardComponent, CardIdentity, CardNamespace, CardRecord},
    moves::MoveRecord,
    stats::{BaseStats, Decimal, NumericConstraints},
    types::{TypingState, WorldTypeRecord},
};
use sqlx::{AssertSqlSafe, SqliteConnection};
use std::collections::HashSet;

pub const WORLD_ID: &str = "c1700000000040008000000000000001";
pub const MOSSRUNNER_ID: &str = "c1710000000040008000000000000001";
pub const EMBER_ID: &str = "c1710000000040008000000000000002";
pub const STEADY_ASCENT_ID: &str = "c1710000000040008000000000000003";
pub const TRAIL_KIT_ID: &str = "c1710000000040008000000000000004";
pub const FOCUSED_EFFORT_ID: &str = "c1710000000040008000000000000005";
pub const FIELD_WRAP_ID: &str = "c1710000000040008000000000000006";
pub const SHARED_TRAIL_KIT_ID: &str = "c1720000000040008000000000000001";
pub const SHARED_ROLL_BONUS_ID: &str = "c1730000000040008000000000000001";
pub const ROLL_BONUS_ID: &str = "c1740000000040008000000000000001";
pub const TARGET_NUMBER_ID: &str = "c1740000000040008000000000000002";
pub const USAGE_NOTES_ID: &str = "c1740000000040008000000000000003";
pub const OPTIONAL_MARKER_ID: &str = "c1740000000040008000000000000004";
pub const USE_CONTEXT_ID: &str = "c1740000000040008000000000000005";
pub const ASSOCIATED_CARD_ID: &str = "c1740000000040008000000000000006";
pub const CONTEXT_TRAIL_ID: &str = "c1750000000040008000000000000001";
pub const CONTEXT_ATTACHMENT_ID: &str = "c1750000000040008000000000000002";
pub const CONTEXT_RESOURCE_ID: &str = "c1750000000040008000000000000003";
pub const REASON: &str = "Original Card Workshop example v1 (ADR0017)";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkshopReport {
    pub world_id: String,
    pub created: usize,
    pub skipped: usize,
    pub cards: usize,
    pub fields: usize,
}

pub struct CardWorkshop {
    accounts: Accounts,
}
impl CardWorkshop {
    pub fn new(accounts: Accounts) -> Self {
        Self { accounts }
    }

    /// Local operator authority, not proof that the selected owner initiated this import.
    pub async fn seed(&self, owner: &str) -> Result<WorkshopReport, Error> {
        if !game_data::types::valid_id(owner) {
            return Err(owner_error());
        }
        persistence::require_schema(self.accounts.pool())
            .await
            .map_err(|_| Error::Unavailable)?;
        let manifest = Manifest::v1(owner)?;
        let mut tx = persistence::begin_write(self.accounts.pool())
            .await
            .map_err(content::storage_error)?;
        let user = persistence::user_by_id(&mut *tx, owner)
            .await
            .map_err(content::storage_error)?
            .ok_or_else(owner_error)?;
        if user.is_disabled != 0
            || !matches!(
                Role::parse(&user.role),
                Some(Role::Editor | Role::Administrator)
            )
        {
            return Err(owner_error());
        }
        let roots = manifest.creations()?;
        let mut present = 0;
        for root in &roots {
            present += usize::from(root.present(&mut tx).await?);
        }
        if present != 0 && present != 15 {
            return Err(baseline_error());
        }
        if present == 0 {
            for root in &roots {
                root.check_slug(&mut tx).await?;
            }
            manifest.create(&mut tx).await?;
        }
        // Also verify fresh writes before commit: a missing/altered final seal cannot
        // publish a partial batch, even when a trigger silently suppresses a write.
        manifest.verify(&mut tx).await?;
        tx.commit().await.map_err(content::storage_error)?;
        Ok(WorkshopReport {
            world_id: WORLD_ID.into(),
            created: if present == 0 { 15 } else { 0 },
            skipped: present,
            cards: 7,
            fields: 7,
        })
    }
}
fn owner_error() -> Error {
    author::invalid(
        "owner_user_id",
        "Choose an existing enabled editor or administrator using the immutable user id from list-users.",
    )
}
fn baseline_error() -> Error {
    author::invalid(
        "card_workshop",
        "The Card Workshop v1 creation baseline is incomplete, differs from the original examples, has a different owner, or collides with existing content. No Workshop content was changed.",
    )
}
fn require(ok: bool) -> Result<(), Error> {
    if ok { Ok(()) } else { Err(baseline_error()) }
}
fn fields(name: &str, description: &str) -> ContentFields {
    ContentFields {
        slug: game_data::authoring::slug_from_name(name),
        display_name: name.into(),
        description: description.into(),
    }
}
fn revision(after: String, action: &str) -> content::Revision<'_> {
    content::Revision::new(None, action, Some(REASON), None, after)
}

struct Manifest {
    world: WorldRecord,
    fields: Vec<FieldDefinition>, // shared first, then six local definitions
    cards: Vec<CardRecord>, // dependency order, including native cards at their own identities
    variant: VariantRecord,
    ember: WorldTypeRecord,
    ascent: MoveRecord,
}
impl Manifest {
    fn v1(owner: &str) -> Result<Self, Error> {
        let world = WorldRecord {
            id: WORLD_ID.into(),
            owner_user_id: owner.into(),
            fields: fields(
                "Card Workshop",
                "Original design examples for learning card authoring. Mossrunner explores a winding moss trail. A Trail Kit contributes an illustrative +3 and Focused Effort an illustrative +5: 10 + 3 + 5 = 18 against target 15. These are inert authored values and prose; no roll, arithmetic, resource spending or success resolution is executed. Labels describe this world's conventions, not global gameplay classes or final game lore.",
            ),
            revision: 1,
            access_revision: 1,
            archived_at: None,
        };
        let integer = FieldKind::Number(NumericConstraints {
            whole_only: true,
            minimum: None,
            maximum: None,
        });
        let mut definitions = Vec::new();
        for (id, shared, name, kind, description) in [
            (
                SHARED_ROLL_BONUS_ID,
                true,
                "Roll Bonus",
                integer.clone(),
                "An exact signed integer used as an illustrative bonus; no calculation is performed.",
            ),
            (
                ROLL_BONUS_ID,
                false,
                "Roll Bonus",
                integer.clone(),
                "Local bonus definition explicitly sourced from the shared Roll Bonus revision 1.",
            ),
            (
                TARGET_NUMBER_ID,
                false,
                "Target Number",
                integer,
                "An illustrative integer target, without success evaluation.",
            ),
            (
                USAGE_NOTES_ID,
                false,
                "Usage Notes",
                FieldKind::Text { max_length: 2000 },
                "Plain-text design notes; empty text is a known value.",
            ),
            (
                OPTIONAL_MARKER_ID,
                false,
                "Optional Marker",
                FieldKind::Boolean,
                "An optional design marker. Explicit false differs from Missing, Unknown and Not Applicable.",
            ),
            (
                USE_CONTEXT_ID,
                false,
                "Use Context",
                FieldKind::Choice {
                    options: vec![
                        ChoiceOption {
                            id: CONTEXT_TRAIL_ID.into(),
                            label: "Trail".into(),
                        },
                        ChoiceOption {
                            id: CONTEXT_ATTACHMENT_ID.into(),
                            label: "Attachment example".into(),
                        },
                        ChoiceOption {
                            id: CONTEXT_RESOURCE_ID.into(),
                            label: "Resource example".into(),
                        },
                    ],
                },
                "Named example contexts with stable option identities, not executable conditions.",
            ),
            (
                ASSOCIATED_CARD_ID,
                false,
                "Associated Card",
                FieldKind::CardReference,
                "An exact qualified relationship, not an equipment slot or executable effect.",
            ),
        ] {
            let row = FieldDefinition {
                id: id.into(),
                world_id: (!shared).then(|| WORLD_ID.into()),
                source: if id == ROLL_BONUS_ID {
                    Some(
                        definitions
                            .first()
                            .ok_or_else(baseline_error)
                            .map(|d: &FieldDefinition| d.pin())?,
                    )
                } else {
                    None
                },
                fields: fields(name, description),
                revision: 1,
                archived_at: None,
                kind,
            };
            row.validate().map_err(Error::Invalid)?;
            definitions.push(row);
        }
        let assignment = |id: &str, value| -> Result<CardField, Error> {
            Ok(CardField {
                definition: definitions
                    .iter()
                    .find(|d| d.id == id)
                    .ok_or_else(baseline_error)?
                    .clone(),
                value,
            })
        };
        let number = |n| FieldValue::Number(Decimal::new(n, 0).expect("v1 integers are exact"));
        let identity = |namespace, shared, id: &str| CardIdentity {
            namespace,
            world_id: if shared { None } else { Some(WORLD_ID.into()) },
            id: id.into(),
        };
        let card = |namespace,
                    shared,
                    id: &str,
                    name: &str,
                    description: &str,
                    labels: &[&str],
                    rules: &str,
                    values: Vec<CardField>| CardRecord {
            identity: identity(namespace, shared, id),
            revision: 1,
            fields: fields(name, description),
            archived_at: None,
            source: None,
            component: CardComponent {
                fields: Some(values),
                rules_text: Some(rules.into()),
                labels: Some(labels.iter().map(|s| (*s).into()).collect()),
            },
            legacy_unmodeled: false,
        };
        let shared_tool = card(
            CardNamespace::Standalone,
            true,
            SHARED_TRAIL_KIT_ID,
            "Trail Kit example",
            "Original shared source for the local Trail Kit. Its Roll Bonus field is explicitly mapped by identity when copying source fields.",
            &["Tool", "Example"],
            "",
            vec![assignment(SHARED_ROLL_BONUS_ID, number(3))?],
        );
        let ember = card(
            CardNamespace::Type,
            false,
            EMBER_ID,
            "Ember",
            "A warm guiding spark used as a Type card in this original example world.",
            &["Type", "Attachment", "Resource"],
            "In Card Workshop, Ember may be described as an attachment or resource. These are example world roles, not global classes; no attachment or spending is executed.",
            vec![
                assignment(ROLL_BONUS_ID, FieldValue::NotApplicable)?,
                assignment(USAGE_NOTES_ID, FieldValue::Text(String::new()))?,
                assignment(OPTIONAL_MARKER_ID, FieldValue::Unknown)?,
                assignment(
                    USE_CONTEXT_ID,
                    FieldValue::Choice(CONTEXT_RESOURCE_ID.into()),
                )?,
            ],
        );
        let mossrunner = card(
            CardNamespace::Variant,
            false,
            MOSSRUNNER_ID,
            "Mossrunner",
            "An original sure-footed creature following the moss trail; a native world Variant with no required shared Form.",
            &["Creature"],
            "Mossrunner considers a climb with illustrative target 15. This card records design intent only.",
            vec![
                assignment(TARGET_NUMBER_ID, number(15))?,
                assignment(ROLL_BONUS_ID, FieldValue::Missing)?,
                assignment(ASSOCIATED_CARD_ID, FieldValue::CardReference(ember.pin()))?,
            ],
        );
        let ascent = card(
            CardNamespace::Move,
            false,
            STEADY_ASCENT_ID,
            "Steady Ascent",
            "An original native Move describing a patient climb. Native numeric move attributes remain unmodeled, separately from custom numeric card fields.",
            &["Move"],
            "Illustrative plan: roll 10, Trail Kit +3, Focused Effort +5; 10 + 3 + 5 = 18 against target 15. No evaluation occurs.",
            vec![
                assignment(TARGET_NUMBER_ID, number(15))?,
                assignment(
                    ASSOCIATED_CARD_ID,
                    FieldValue::CardReference(mossrunner.pin()),
                )?,
                assignment(USE_CONTEXT_ID, FieldValue::Choice(CONTEXT_TRAIL_ID.into()))?,
            ],
        );
        let mut tool = card(
            CardNamespace::Standalone,
            false,
            TRAIL_KIT_ID,
            "Trail Kit",
            "An original local Tool with an explicitly declared shared source; its local fields remain independently authored.",
            &["Tool"],
            "Illustrative +3 for the written climbing plan. Source adoption keeps local values; copying requires an explicit Roll Bonus mapping.",
            vec![
                assignment(ROLL_BONUS_ID, number(3))?,
                assignment(OPTIONAL_MARKER_ID, FieldValue::Boolean(false))?,
                assignment(
                    USAGE_NOTES_ID,
                    FieldValue::Text(
                        "A compact cord and trail guide, recorded as design prose only.".into(),
                    ),
                )?,
                assignment(ASSOCIATED_CARD_ID, FieldValue::CardReference(ascent.pin()))?,
                assignment(USE_CONTEXT_ID, FieldValue::Choice(CONTEXT_TRAIL_ID.into()))?,
            ],
        );
        tool.source = Some(shared_tool.pin());
        let effort = card(
            CardNamespace::Standalone,
            false,
            FOCUSED_EFFORT_ID,
            "Focused Effort",
            "An original ability example expressed as a standalone card and a world-local label.",
            &["Ability"],
            "Illustrative +5 in the written plan. No bonus is applied by the application.",
            vec![
                assignment(ROLL_BONUS_ID, number(5))?,
                assignment(OPTIONAL_MARKER_ID, FieldValue::NotApplicable)?,
                assignment(ASSOCIATED_CARD_ID, FieldValue::CardReference(ascent.pin()))?,
            ],
        );
        let wrap = card(
            CardNamespace::Standalone,
            false,
            FIELD_WRAP_ID,
            "Field Wrap",
            "An original Item example for discussing supplies without implementing item consumption or healing.",
            &["Item"],
            "Use is a design note only; no resource is consumed and no state changes.",
            vec![
                assignment(ROLL_BONUS_ID, FieldValue::Unknown)?,
                assignment(TARGET_NUMBER_ID, FieldValue::NotApplicable)?,
                assignment(USAGE_NOTES_ID, FieldValue::Missing)?,
                assignment(OPTIONAL_MARKER_ID, FieldValue::Boolean(false))?,
                assignment(ASSOCIATED_CARD_ID, FieldValue::CardReference(tool.pin()))?,
                assignment(
                    USE_CONTEXT_ID,
                    FieldValue::Choice(CONTEXT_ATTACHMENT_ID.into()),
                )?,
            ],
        );
        let variant = VariantRecord {
            id: MOSSRUNNER_ID.into(),
            world_id: WORLD_ID.into(),
            source: None,
            fields: mossrunner.fields.clone(),
            revision: 1,
            archived_at: None,
        };
        let native_type = WorldTypeRecord {
            id: EMBER_ID.into(),
            world_id: WORLD_ID.into(),
            source: None,
            fields: ember.fields.clone(),
            revision: 1,
            archived_at: None,
        };
        let native_move = MoveRecord {
            id: STEADY_ASCENT_ID.into(),
            world_id: Some(WORLD_ID.into()),
            source: None,
            fields: ascent.fields.clone(),
            revision: 1,
            archived_at: None,
            attributes: None,
            mappings: vec![],
        };
        let cards = vec![shared_tool, ember, mossrunner, ascent, tool, effort, wrap];
        let mut ids = HashSet::from([WORLD_ID.to_owned()]);
        for d in &definitions {
            require(ids.insert(d.id.clone()))?;
        }
        for c in &cards {
            c.validate().map_err(Error::Invalid)?;
            require(ids.insert(c.identity.id.clone()))?;
        }
        require(ids.len() == 15)?;
        native_move.validate().map_err(Error::Invalid)?;
        Ok(Self {
            world,
            fields: definitions,
            cards,
            variant,
            ember: native_type,
            ascent: native_move,
        })
    }

    async fn create(&self, c: &mut SqliteConnection) -> Result<(), Error> {
        content::put_world_head(c, &self.world).await?;
        for d in &self.fields {
            cards::put_field_as(c, d, None, "create", Some(REASON)).await?;
        }
        for row in &self.cards {
            match row.identity.namespace {
                CardNamespace::Standalone => {
                    cards::put_card_as(c, row, None, "create", Some(REASON)).await?
                }
                CardNamespace::Type => {
                    cards::stage(c, &row.pin(), &row.component).await?;
                    types::put_world_type(
                        c,
                        &self.ember,
                        revision(author::snapshot(&self.ember)?, "create"),
                    )
                    .await?;
                }
                CardNamespace::Variant => {
                    cards::stage(c, &row.pin(), &row.component).await?;
                    content::put_variant(
                        c,
                        &self.variant,
                        revision(author::snapshot(&self.variant)?, "create"),
                    )
                    .await?;
                }
                CardNamespace::Move => {
                    cards::stage(c, &row.pin(), &row.component).await?;
                    moves::put_move(c, &self.ascent, None, "create", Some(REASON)).await?;
                }
                CardNamespace::Form => return Err(baseline_error()),
            }
        }
        let after = author::world_snapshot(c, &self.world).await?;
        content::seal_world_revision(c, &self.world, revision(after, "create")).await?;
        let after = author::access_snapshot(c, &self.world).await?;
        content::put_access_revision(c, &self.world, revision(after, "own")).await
    }

    fn creations(&self) -> Result<Vec<Creation>, Error> {
        let mut out = vec![Creation::new(
            Kind::World,
            WORLD_ID,
            "",
            &self.world.fields.slug,
            author::snapshot(
                &serde_json::json!({"id":WORLD_ID,"fields":self.world.fields,"revision":1,"archived_at":null,"variant_ids":[MOSSRUNNER_ID]}),
            )?,
        )];
        for d in &self.fields {
            out.push(Creation::new(
                Kind::Field,
                &d.id,
                d.world_id.as_deref().unwrap_or(""),
                &d.fields.slug,
                author::snapshot(d)?,
            ));
        }
        for row in &self.cards {
            let (kind, after) = match row.identity.namespace {
                CardNamespace::Standalone => (Kind::Card, author::snapshot(row)?),
                CardNamespace::Type => (Kind::Type, author::snapshot(&self.ember)?),
                CardNamespace::Variant => (Kind::Variant, author::snapshot(&self.variant)?),
                CardNamespace::Move => (Kind::Move, author::snapshot(&self.ascent)?),
                CardNamespace::Form => return Err(baseline_error()),
            };
            out.push(Creation::new(
                kind,
                &row.identity.id,
                row.identity.world_id.as_deref().unwrap_or(""),
                &row.fields.slug,
                after,
            ));
        }
        require(out.len() == 15)?;
        Ok(out)
    }

    async fn verify(&self, c: &mut SqliteConnection) -> Result<(), Error> {
        // Ownership is immutable under this seed contract; later grants are independent.
        require(content::world(c, WORLD_ID).await?.owner_user_id == self.world.owner_user_id)?;
        let mut audits = HashSet::new();
        for root in self.creations()? {
            require(audits.insert(root.verify(c).await?))?;
        }
        let access = Creation::new(
            Kind::Access,
            WORLD_ID,
            "",
            "",
            author::snapshot(&serde_json::json!({
                "world_id":WORLD_ID,"owner_user_id":self.world.owner_user_id,"access_revision":1,"collaborator_ids":[]
            }))?,
        );
        require(audits.insert(access.verify(c).await?))?;
        require(audits.len() == 16)?;
        for d in &self.fields {
            require(cards::field_version(c, d.world_id.as_deref(), &d.id, 1).await? == *d)?;
        }
        for row in &self.cards {
            require(cards::card_version(c, &row.pin()).await? == *row)?;
            let snapshot: String = sqlx::query_scalar("SELECT snapshot_json FROM card_components WHERE namespace=? AND scope=? AND card_id=? AND revision=1")
                .bind(row.identity.namespace.as_str()).bind(row.identity.world_id.as_deref().unwrap_or("")).bind(&row.identity.id)
                .fetch_one(&mut *c).await.map_err(content::storage_error)?;
            require(snapshot == author::snapshot(row)?)?;
        }
        require(moves::move_version(c, Some(WORLD_ID), STEADY_ASCENT_ID, 1).await? == self.ascent)?;
        require(types::world_type_version(c, WORLD_ID, EMBER_ID, 1).await? == self.ember)?;
        let full =
            types::full_content(c, HistoryKind::Variant, MOSSRUNNER_ID, Some(WORLD_ID), 1).await?;
        require(
            full.before.is_none()
                && full.typing == TypingState::unmodeled()
                && full.stats == BaseStats::unmodeled(),
        )?;
        let mut expected = serde_json::to_value(&self.variant).map_err(|_| Error::Unavailable)?;
        expected["typing"] = serde_json::json!(TypingState::unmodeled());
        let v2 = serde_json::json!({"snapshot_version":2,"content":expected});
        let v2_bytes = v2.to_string();
        require(full.typing_projection_after == v2_bytes)?;
        expected["stats"] = serde_json::json!(BaseStats::unmodeled());
        let v3_bytes = serde_json::json!({"snapshot_version":3,"content":expected}).to_string();
        require(full.after == v3_bytes)?;
        let world = types::full_content(c, HistoryKind::World, WORLD_ID, None, 1).await?;
        let world_bytes = serde_json::json!({"snapshot_version":2,"content":{
            "id":WORLD_ID,"fields":self.world.fields,"revision":1,"archived_at":null,"variant_ids":[MOSSRUNNER_ID],"type_ids":[EMBER_ID]
        }}).to_string();
        require(world.before.is_none() && world.after == world_bytes)
    }
}

// Closed SQL choices, local to this one operator use case like the other fixture batches.
#[derive(Clone, Copy)]
enum Kind {
    World,
    Variant,
    Type,
    Move,
    Card,
    Field,
    Access,
}
impl Kind {
    fn tables(self) -> (&'static str, &'static str, &'static str, &'static str) {
        match self {
            Self::World => ("worlds", "world_revisions", "world_id", "''"),
            Self::Access => ("worlds", "world_access_revisions", "world_id", "''"),
            Self::Variant => ("variants", "variant_revisions", "variant_id", "world_id"),
            Self::Type => ("world_types", "world_type_versions", "type_id", "world_id"),
            Self::Move => ("moves", "move_versions", "move_id", "scope"),
            Self::Card => (
                "standalone_cards",
                "standalone_card_versions",
                "card_id",
                "scope",
            ),
            Self::Field => (
                "card_field_definitions",
                "card_field_versions",
                "field_id",
                "scope",
            ),
        }
    }
}
struct Creation {
    kind: Kind,
    id: String,
    scope: String,
    slug: String,
    after: String,
}
impl Creation {
    fn new(kind: Kind, id: &str, scope: &str, slug: &str, after: String) -> Self {
        Self {
            kind,
            id: id.into(),
            scope: scope.into(),
            slug: slug.into(),
            after,
        }
    }
    async fn present(&self, c: &mut SqliteConnection) -> Result<bool, Error> {
        let (head, versions, key, _) = self.kind.tables();
        sqlx::query_scalar(AssertSqlSafe(format!("SELECT EXISTS(SELECT 1 FROM {head} WHERE id=?1 UNION ALL SELECT 1 FROM {versions} WHERE {key}=?1)")))
            .bind(&self.id).fetch_one(c).await.map_err(content::storage_error)
    }
    async fn check_slug(&self, c: &mut SqliteConnection) -> Result<(), Error> {
        let (head, versions, _, scope) = self.kind.tables();
        let exists: bool = sqlx::query_scalar(AssertSqlSafe(format!("SELECT EXISTS(SELECT 1 FROM {head} WHERE {scope}=?1 AND slug=?2 UNION ALL SELECT 1 FROM {versions} WHERE {scope}=?1 AND revision=1 AND json_extract(after_json,'$.content.fields.slug')=?2)")))
            .bind(&self.scope).bind(&self.slug).fetch_one(c).await.map_err(content::storage_error)?;
        require(!exists)
    }
    async fn verify(&self, c: &mut SqliteConnection) -> Result<String, Error> {
        let (head, versions, key, scope) = self.kind.tables();
        let exists: bool = sqlx::query_scalar(AssertSqlSafe(format!(
            "SELECT EXISTS(SELECT 1 FROM {head} WHERE id=? AND {scope}=?)"
        )))
        .bind(&self.id)
        .bind(&self.scope)
        .fetch_one(&mut *c)
        .await
        .map_err(content::storage_error)?;
        require(exists)?;
        let audit: Option<String> = sqlx::query_scalar(AssertSqlSafe(format!("SELECT v.audit_id FROM {versions} v JOIN audit_events a ON a.id=v.audit_id WHERE v.{key}=?1 AND {scope}=?2 AND v.revision=1 AND v.previous_revision IS NULL AND v.before_json IS NULL AND v.after_json=?3 AND a.actor_user_id IS NULL AND a.system_key='bundled-fixture-loader' AND a.action=?4 AND a.reason=?5")))
            .bind(&self.id).bind(&self.scope).bind(&self.after)
            .bind(if matches!(self.kind, Kind::Access) { "own" } else { "create" }).bind(REASON)
            .fetch_optional(c).await.map_err(content::storage_error)?;
        audit.ok_or_else(baseline_error)
    }
}
