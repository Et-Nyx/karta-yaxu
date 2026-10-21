//! Opt-in ADR0018 original examples. The typed v1 creation manifest never follows live heads.
use super::authoring::{self as author, Error, HistoryKind, VariantRecord, WorldRecord};
use crate::{
    accounts::Accounts,
    auth::Role,
    persistence::{self, authoring as content, cards, relations, stats, types},
};
use game_data::{
    authoring::ContentFields,
    card_fields::{ChoiceOption, FieldDefinition, FieldKind},
    cards::{CardComponent, CardIdentity, CardNamespace, CardRecord},
    relations::*,
    stats::{BaseStats, Decimal, NumericConstraints, StatDefinition},
    types::{TypePin, TypeRecord, TypingState, WorldTypeRecord},
};
use sqlx::{AssertSqlSafe, SqliteConnection};
use std::collections::HashSet;

pub const WORLD_ID: &str = "d1800000000040008000000000000001";
pub const FERNSTRIDER_ID: &str = "d1810000000040008000000000000001";
pub const EMBER_ID: &str = "d1810000000040008000000000000002";
pub const FOCUS_ID: &str = "d1810000000040008000000000000003";
pub const SECOND_WIND_ID: &str = "d1810000000040008000000000000004";
pub const FIELD_SATCHEL_ID: &str = "d1810000000040008000000000000005";
pub const PATIENT_ID: &str = "d1810000000040008000000000000006";
pub const RESTLESS_ID: &str = "d1810000000040008000000000000007";
pub const SHARED_EMBER_ID: &str = "d1820000000040008000000000000001";
pub const SHARED_FOCUS_ID: &str = "d1820000000040008000000000000002";
pub const SHARED_SECOND_WIND_ID: &str = "d1820000000040008000000000000003";
pub const SHARED_QUANTITY_ID: &str = "d1830000000040008000000000000001";
pub const SHARED_CONDITION_ID: &str = "d1830000000040008000000000000002";
pub const QUANTITY_ID: &str = "d1840000000040008000000000000001";
pub const CONDITION_ID: &str = "d1840000000040008000000000000002";
pub const ADJUSTMENT_ID: &str = "d1840000000040008000000000000003";
pub const INTERPRETATION_ID: &str = "d1840000000040008000000000000004";
pub const SHARED_RESOURCE_AMOUNT_ID: &str = "d1850000000040008000000000000001";
pub const RESOURCE_AMOUNT_ID: &str = "d1850000000040008000000000000002";
pub const RELATED_CARDS_ID: &str = "d1850000000040008000000000000003";
pub const INFLUENCE_ID: &str = "d1850000000040008000000000000004";
pub const SHARED_RESOURCE_COSTS_ID: &str = "d1860000000040008000000000000001";
pub const RESOURCE_COSTS_ID: &str = "d1860000000040008000000000000002";
pub const PRODUCES_ID: &str = "d1860000000040008000000000000003";
pub const GRANTED_ABILITIES_ID: &str = "d1860000000040008000000000000004";
pub const AVAILABLE_TRAITS_ID: &str = "d1860000000040008000000000000005";
pub const INFLUENCES_ID: &str = "d1860000000040008000000000000006";
pub const RESOLVE_ID: &str = "d1870000000040008000000000000001";
pub const RESOURCE_COLUMN: &str = "d1880000000040008000000000000001";
pub const AMOUNT_COLUMN: &str = "d1880000000040008000000000000002";
pub const CONDITION_COLUMN: &str = "d1880000000040008000000000000003";
pub const RELATED_COLUMN: &str = "d1880000000040008000000000000004";
pub const AFFECTED_COLUMN: &str = "d1880000000040008000000000000005";
pub const ADJUSTMENT_COLUMN: &str = "d1880000000040008000000000000006";
pub const INTERPRETATION_COLUMN: &str = "d1880000000040008000000000000007";
pub const ADD_OPTION: &str = "d18a0000000040008000000000000001";
pub const SCALE_OPTION: &str = "d18a0000000040008000000000000002";
pub const REASON: &str = "Original Card Relations Workshop v1 (ADR0018)";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkshopReport {
    pub world_id: String,
    pub created: usize,
    pub skipped: usize,
    pub cards: usize,
    pub fields: usize,
    pub layouts: usize,
    pub collections: usize,
    pub stats: usize,
}
pub struct RelationWorkshop {
    accounts: Accounts,
}
impl RelationWorkshop {
    pub fn new(accounts: Accounts) -> Self {
        Self { accounts }
    }

    /// Trusted local operator authority; the owner is not attributed as the importer.
    pub async fn seed(&self, owner: &str) -> Result<WorkshopReport, Error> {
        if !game_data::types::valid_id(owner) {
            return Err(owner_error());
        }
        persistence::require_schema(self.accounts.pool())
            .await
            .map_err(|_| Error::Unavailable)?;
        let m = Manifest::v1(owner)?;
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
        let roots = m.creations()?;
        let mut present = 0;
        for r in &roots {
            present += usize::from(r.present(&mut tx).await?);
        }
        require(present == 0 || present == roots.len())?;
        if present == 0 {
            for r in &roots {
                r.check_slug(&mut tx).await?;
            }
            m.create(&mut tx).await?;
        }
        m.verify(&mut tx).await?;
        tx.commit().await.map_err(content::storage_error)?;
        Ok(WorkshopReport {
            world_id: WORLD_ID.into(),
            created: if present == 0 { roots.len() } else { 0 },
            skipped: present,
            cards: m.cards.len(),
            fields: m.fields.len(),
            layouts: m.layouts.len(),
            collections: m.collections.len(),
            stats: 1,
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
        "relation_workshop",
        "The Card Relations Workshop v1 creation baseline is incomplete, differs from the original examples, has a different owner, or collides with existing content. No Workshop content was changed.",
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
fn number(coefficient: i64, scale: u8) -> Decimal {
    Decimal::new(coefficient, scale).expect("exact original manifest number")
}
fn cell(column: &str, value: RecordValue) -> RecordCell {
    RecordCell {
        column_id: column.into(),
        value,
    }
}
fn entry(n: u8, cells: Vec<RecordCell>) -> RecordEntry {
    RecordEntry {
        id: format!("d18900000000400080000000000000{n:02x}"),
        cells,
    }
}
fn reference(c: &CardRecord) -> RecordValue {
    RecordValue::Reference(DefinitionPin::Card(c.pin()))
}

struct Manifest {
    world: WorldRecord,
    fields: Vec<FieldDefinition>,
    layouts: Vec<LayoutRecord>,
    collections: Vec<CollectionDefinition>,
    stat: StatDefinition,
    cards: Vec<CardRecord>,
    relations: Vec<CardRelations>, // aligned with cards, in dependency order
    variant: VariantRecord,
    shared_ember: TypeRecord,
    ember: WorldTypeRecord,
}
impl Manifest {
    fn v1(owner: &str) -> Result<Self, Error> {
        let world = WorldRecord {
            id: WORLD_ID.into(),
            owner_user_id: owner.into(),
            revision: 1,
            access_revision: 1,
            archived_at: None,
            fields: fields(
                "Card Relations Workshop",
                "Original flat record examples: resources, abilities, items and personality traits. Names, labels, conditions, amounts and influences are inert design data, not final lore or executable mechanics. Source copying requires explicit collection and column mappings and retains exact referenced targets, including shared targets; no automatic local retargeting occurs.",
            ),
        };
        let quantity = FieldKind::Number(NumericConstraints {
            whole_only: true,
            minimum: Some(number(0, 0)),
            maximum: None,
        });
        let text = FieldKind::Text { max_length: 2000 };
        let exact = NumericConstraints {
            whole_only: false,
            minimum: None,
            maximum: None,
        };
        let mut definitions: Vec<FieldDefinition> = vec![];
        for (id, shared, name, kind, source_index) in [
            (SHARED_QUANTITY_ID, true, "Quantity", quantity.clone(), None),
            (SHARED_CONDITION_ID, true, "Condition", text.clone(), None),
            (QUANTITY_ID, false, "Quantity", quantity, Some(0)),
            (CONDITION_ID, false, "Condition", text, Some(1)),
            (
                ADJUSTMENT_ID,
                false,
                "Adjustment",
                FieldKind::Number(exact.clone()),
                None,
            ),
            (
                INTERPRETATION_ID,
                false,
                "Interpretation",
                FieldKind::Choice {
                    options: vec![
                        ChoiceOption {
                            id: ADD_OPTION.into(),
                            label: "Illustrative Add".into(),
                        },
                        ChoiceOption {
                            id: SCALE_OPTION.into(),
                            label: "Illustrative Scale".into(),
                        },
                    ],
                },
                None,
            ),
        ] {
            let d = FieldDefinition {
                id: id.into(),
                world_id: (!shared).then(|| WORLD_ID.into()),
                source: source_index.map(|i: usize| definitions[i].pin()),
                fields: fields(
                    name,
                    "Reusable original example field. Interpretation labels and conditions do not execute.",
                ),
                revision: 1,
                archived_at: None,
                kind,
            };
            d.validate().map_err(Error::Invalid)?;
            definitions.push(d);
        }
        let scalar = |id: &str, label: &str, index: usize| RecordColumn {
            id: id.into(),
            label: label.into(),
            kind: ColumnKind::Field(definitions[index].clone()),
        };
        let refs = |id: &str, label: &str, allowed| RecordColumn {
            id: id.into(),
            label: label.into(),
            kind: ColumnKind::Reference { allowed },
        };
        let mut layouts: Vec<LayoutRecord> = vec![];
        for (id, shared, name, columns) in [
            (
                SHARED_RESOURCE_AMOUNT_ID,
                true,
                "Resource Amount",
                vec![
                    refs(RESOURCE_COLUMN, "Resource", vec![ReferenceKind::Card]),
                    scalar(AMOUNT_COLUMN, "Amount", 0),
                    scalar(CONDITION_COLUMN, "Condition", 1),
                ],
            ),
            (
                RESOURCE_AMOUNT_ID,
                false,
                "Resource Amount",
                vec![
                    refs(RESOURCE_COLUMN, "Resource", vec![ReferenceKind::Card]),
                    scalar(AMOUNT_COLUMN, "Amount", 2),
                    scalar(CONDITION_COLUMN, "Condition", 3),
                ],
            ),
            (
                RELATED_CARDS_ID,
                false,
                "Related Cards",
                vec![
                    refs(RELATED_COLUMN, "Related", vec![ReferenceKind::Card]),
                    scalar(CONDITION_COLUMN, "Condition", 3),
                ],
            ),
            (
                INFLUENCE_ID,
                false,
                "Influence",
                vec![
                    refs(
                        AFFECTED_COLUMN,
                        "Affected",
                        vec![
                            ReferenceKind::Stat,
                            ReferenceKind::Field,
                            ReferenceKind::Card,
                        ],
                    ),
                    scalar(ADJUSTMENT_COLUMN, "Adjustment", 4),
                    scalar(INTERPRETATION_COLUMN, "Interpretation", 5),
                    scalar(CONDITION_COLUMN, "Condition", 3),
                ],
            ),
        ] {
            let d = LayoutRecord {
                id: id.into(),
                world_id: (!shared).then(|| WORLD_ID.into()),
                source: (id == RESOURCE_AMOUNT_ID).then(|| layouts[0].pin()),
                fields: fields(
                    name,
                    "Flat original example layout; columns have no built-in mechanics.",
                ),
                revision: 1,
                archived_at: None,
                columns,
            };
            d.validate().map_err(Error::Invalid)?;
            layouts.push(d);
        }
        let mut collections: Vec<CollectionDefinition> = vec![];
        for (id, name, index) in [
            (SHARED_RESOURCE_COSTS_ID, "Resource Costs", 0),
            (RESOURCE_COSTS_ID, "Resource Costs", 1),
            (PRODUCES_ID, "Produces", 1),
            (GRANTED_ABILITIES_ID, "Granted Abilities", 2),
            (AVAILABLE_TRAITS_ID, "Available Traits", 2),
            (INFLUENCES_ID, "Influences", 3),
        ] {
            let d = CollectionDefinition {
                id: id.into(),
                world_id: layouts[index].world_id.clone(),
                source: (id == RESOURCE_COSTS_ID).then(|| collections[0].pin()),
                fields: fields(
                    name,
                    "Independent original example collection; its name does not execute behavior.",
                ),
                revision: 1,
                archived_at: None,
                layout: layouts[index].clone(),
            };
            d.validate().map_err(Error::Invalid)?;
            collections.push(d);
        }
        let stat = StatDefinition {
            id: RESOLVE_ID.into(),
            world_id: Some(WORLD_ID.into()),
            source: None,
            fields: fields(
                "Resolve",
                "An exact independent local Stat. No creature roster or computed modifier is implied.",
            ),
            revision: 1,
            archived_at: None,
            constraints: exact,
        };
        stat.validate().map_err(Error::Invalid)?;
        let card = |id: &str, namespace, shared: bool, name: &str, labels: &[&str], prose: &str| {
            CardRecord {
                identity: CardIdentity {
                    namespace,
                    world_id: (!shared).then(|| WORLD_ID.into()),
                    id: id.into(),
                },
                revision: 1,
                fields: fields(name, prose),
                archived_at: None,
                source: None,
                component: CardComponent {
                    fields: None,
                    rules_text: Some(prose.into()),
                    labels: Some(labels.iter().map(|s| (*s).into()).collect()),
                },
                legacy_unmodeled: false,
            }
        };
        let shared_ember_card = card(
            SHARED_EMBER_ID,
            CardNamespace::Type,
            true,
            "Ember",
            &["Type", "Resource", "Attachment"],
            "Ember can be described as a resource or an attachment in this world. Neither use changes typing or executes spending.",
        );
        let shared_focus = card(
            SHARED_FOCUS_ID,
            CardNamespace::Standalone,
            true,
            "Focus",
            &["Resource"],
            "An illustrative resource, without a balance or spending behavior.",
        );
        let shared_wind = card(
            SHARED_SECOND_WIND_ID,
            CardNamespace::Standalone,
            true,
            "Second Wind",
            &["Ability"],
            "An illustrative ability costing Ember x2 and Focus x1. No cost is evaluated or paid.",
        );
        let mut ember_card = card(
            EMBER_ID,
            CardNamespace::Type,
            false,
            "Ember",
            &["Type", "Resource", "Attachment"],
            &shared_ember_card.fields.description,
        );
        ember_card.source = Some(shared_ember_card.pin());
        let mut focus = card(
            FOCUS_ID,
            CardNamespace::Standalone,
            false,
            "Focus",
            &["Resource"],
            &shared_focus.fields.description,
        );
        focus.source = Some(shared_focus.pin());
        let mut wind = card(
            SECOND_WIND_ID,
            CardNamespace::Standalone,
            false,
            "Second Wind",
            &["Ability"],
            &shared_wind.fields.description,
        );
        wind.source = Some(shared_wind.pin());
        let satchel = card(
            FIELD_SATCHEL_ID,
            CardNamespace::Standalone,
            false,
            "Field Satchel",
            &["Item"],
            "An item with an authored Granted Abilities relationship to Second Wind; no inventory or effect execution.",
        );
        let patient = card(
            PATIENT_ID,
            CardNamespace::Standalone,
            false,
            "Patient",
            &["Nature", "Trait"],
            "Patient pauses to observe before acting and listens carefully to companions. Optional influences describe independent illustrative Add +2 Resolve and Scale 0.5 Quantity intentions; neither is computed.",
        );
        let restless = card(
            RESTLESS_ID,
            CardNamespace::Standalone,
            false,
            "Restless",
            &["Trait"],
            "Restless enjoys exploring and finds long pauses difficult. This is descriptive personality prose only.",
        );
        let runner = card(
            FERNSTRIDER_ID,
            CardNamespace::Variant,
            false,
            "Fernstrider",
            &["Creature"],
            "An original native Variant without a shared Form source. Patient and Restless are available trait options, not automatic assignments.",
        );
        let known = |index: usize, entries| RecordCollection {
            definition: collections[index].clone(),
            value: CollectionValue::Entries(entries),
        };
        let costs = |ember: &CardRecord, focus: &CardRecord, index| CardRelations {
            collections: Some(vec![known(
                index,
                vec![
                    entry(
                        1,
                        vec![
                            cell(RESOURCE_COLUMN, reference(ember)),
                            cell(AMOUNT_COLUMN, RecordValue::Number(number(2, 0))),
                        ],
                    ),
                    entry(
                        2,
                        vec![
                            cell(RESOURCE_COLUMN, reference(focus)),
                            cell(AMOUNT_COLUMN, RecordValue::Number(number(1, 0))),
                            cell(CONDITION_COLUMN, RecordValue::Text(String::new())),
                        ],
                    ),
                ],
            )]),
        };
        let shared_costs = costs(&shared_ember_card, &shared_focus, 0);
        let local_costs = costs(&ember_card, &focus, 1);
        let produces = CardRelations {
            collections: Some(vec![known(
                2,
                vec![
                    entry(
                        1,
                        vec![
                            cell(RESOURCE_COLUMN, reference(&focus)),
                            cell(AMOUNT_COLUMN, RecordValue::Number(number(0, 0))),
                            cell(
                                CONDITION_COLUMN,
                                RecordValue::Text("While resting; illustrative only".into()),
                            ),
                        ],
                    ),
                    entry(
                        2,
                        vec![
                            cell(RESOURCE_COLUMN, reference(&focus)),
                            cell(AMOUNT_COLUMN, RecordValue::Unknown),
                            cell(
                                CONDITION_COLUMN,
                                RecordValue::Text("After travel; illustrative only".into()),
                            ),
                        ],
                    ),
                    entry(
                        3,
                        vec![
                            cell(RESOURCE_COLUMN, RecordValue::Missing),
                            cell(AMOUNT_COLUMN, RecordValue::NotApplicable),
                            cell(CONDITION_COLUMN, RecordValue::Missing),
                        ],
                    ),
                ],
            )]),
        };
        let grants = CardRelations {
            collections: Some(vec![known(
                3,
                vec![entry(1, vec![cell(RELATED_COLUMN, reference(&wind))])],
            )]),
        };
        let influences = CardRelations {
            collections: Some(vec![known(
                5,
                vec![
                    entry(
                        1,
                        vec![
                            cell(
                                AFFECTED_COLUMN,
                                RecordValue::Reference(DefinitionPin::Stat {
                                    world_id: Some(WORLD_ID.into()),
                                    stat_id: RESOLVE_ID.into(),
                                    version: 1,
                                }),
                            ),
                            cell(ADJUSTMENT_COLUMN, RecordValue::Number(number(2, 0))),
                            cell(
                                INTERPRETATION_COLUMN,
                                RecordValue::Choice(ADD_OPTION.into()),
                            ),
                        ],
                    ),
                    entry(
                        2,
                        vec![
                            cell(
                                AFFECTED_COLUMN,
                                RecordValue::Reference(DefinitionPin::Field(definitions[2].pin())),
                            ),
                            cell(ADJUSTMENT_COLUMN, RecordValue::Number(number(5, 1))),
                            cell(
                                INTERPRETATION_COLUMN,
                                RecordValue::Choice(SCALE_OPTION.into()),
                            ),
                            cell(CONDITION_COLUMN, RecordValue::Unknown),
                        ],
                    ),
                ],
            )]),
        };
        let traits = CardRelations {
            collections: Some(vec![known(
                4,
                vec![
                    entry(1, vec![cell(RELATED_COLUMN, reference(&patient))]),
                    entry(
                        2,
                        vec![
                            cell(RELATED_COLUMN, reference(&restless)),
                            cell(CONDITION_COLUMN, RecordValue::NotApplicable),
                        ],
                    ),
                ],
            )]),
        };
        let variant = VariantRecord {
            id: FERNSTRIDER_ID.into(),
            world_id: WORLD_ID.into(),
            source: None,
            fields: runner.fields.clone(),
            revision: 1,
            archived_at: None,
        };
        let shared_ember = TypeRecord {
            id: SHARED_EMBER_ID.into(),
            fields: shared_ember_card.fields.clone(),
            revision: 1,
            archived_at: None,
        };
        let ember = WorldTypeRecord {
            id: EMBER_ID.into(),
            world_id: WORLD_ID.into(),
            source: Some(TypePin {
                type_id: SHARED_EMBER_ID.into(),
                version: 1,
            }),
            fields: ember_card.fields.clone(),
            revision: 1,
            archived_at: None,
        };
        let cards = vec![
            shared_ember_card,
            shared_focus,
            shared_wind,
            ember_card,
            focus,
            wind,
            satchel,
            patient,
            restless,
            runner,
        ];
        let relations = vec![
            CardRelations::default(),
            CardRelations::default(),
            shared_costs,
            CardRelations::default(),
            produces,
            local_costs,
            grants,
            influences,
            CardRelations::default(),
            traits,
        ];
        for (c, r) in cards.iter().zip(&relations) {
            c.validate().map_err(Error::Invalid)?;
            r.validate(&c.identity).map_err(Error::Invalid)?;
        }
        let m = Self {
            world,
            fields: definitions,
            layouts,
            collections,
            stat,
            cards,
            relations,
            variant,
            shared_ember,
            ember,
        };
        let roots = m.creations()?;
        let ids: HashSet<_> = roots.iter().map(|r| &r.id).collect();
        require(ids.len() == roots.len())?;
        Ok(m)
    }

    async fn create(&self, c: &mut SqliteConnection) -> Result<(), Error> {
        content::put_world_head(c, &self.world).await?;
        for d in &self.fields {
            cards::put_field_as(c, d, None, "create", Some(REASON)).await?;
        }
        stats::put_definition(c, &self.stat, None, "create", Some(REASON)).await?;
        for d in &self.layouts {
            relations::put_layout_as(c, d, None, "create", Some(REASON)).await?;
        }
        for d in &self.collections {
            relations::put_collection_as(c, d, None, "create", Some(REASON)).await?;
        }
        for (row, r) in self.cards.iter().zip(&self.relations) {
            relations::stage(c, &row.pin(), r).await?;
            match row.identity.namespace {
                CardNamespace::Standalone => {
                    cards::put_card_as(c, row, None, "create", Some(REASON)).await?
                }
                CardNamespace::Type => {
                    cards::stage(c, &row.pin(), &row.component).await?;
                    if row.identity.world_id.is_none() {
                        types::put_type(
                            c,
                            &self.shared_ember,
                            revision(author::snapshot(&self.shared_ember)?, "create"),
                        )
                        .await?;
                    } else {
                        types::put_world_type(
                            c,
                            &self.ember,
                            revision(author::snapshot(&self.ember)?, "create"),
                        )
                        .await?;
                    }
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
                _ => return Err(baseline_error()),
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
            None,
            &self.world.fields,
            author::snapshot(
                &serde_json::json!({"id":WORLD_ID,"fields":self.world.fields,"revision":1,"archived_at":null,"variant_ids":[FERNSTRIDER_ID]}),
            )?,
        )];
        for d in &self.fields {
            out.push(Creation::new(
                Kind::Field,
                &d.id,
                d.world_id.as_deref(),
                &d.fields,
                author::snapshot(d)?,
            ));
        }
        for d in &self.layouts {
            out.push(Creation::new(
                Kind::Layout,
                &d.id,
                d.world_id.as_deref(),
                &d.fields,
                author::snapshot(d)?,
            ));
        }
        for d in &self.collections {
            out.push(Creation::new(
                Kind::Collection,
                &d.id,
                d.world_id.as_deref(),
                &d.fields,
                author::snapshot(d)?,
            ));
        }
        out.push(Creation::new(
            Kind::Stat,
            &self.stat.id,
            self.stat.world_id.as_deref(),
            &self.stat.fields,
            author::snapshot(&self.stat)?,
        ));
        for row in &self.cards {
            let (kind, after) = match row.identity.namespace {
                CardNamespace::Standalone => (Kind::Card, author::snapshot(row)?),
                CardNamespace::Type if row.identity.world_id.is_none() => {
                    (Kind::SharedType, author::snapshot(&self.shared_ember)?)
                }
                CardNamespace::Type => (Kind::Type, author::snapshot(&self.ember)?),
                CardNamespace::Variant => (Kind::Variant, author::snapshot(&self.variant)?),
                _ => return Err(baseline_error()),
            };
            out.push(Creation::new(
                kind,
                &row.identity.id,
                row.identity.world_id.as_deref(),
                &row.fields,
                after,
            ));
        }
        Ok(out)
    }
    async fn verify(&self, c: &mut SqliteConnection) -> Result<(), Error> {
        require(content::world(c, WORLD_ID).await?.owner_user_id == self.world.owner_user_id)?;
        let roots = self.creations()?;
        let mut audits = HashSet::new();
        for root in &roots {
            require(audits.insert(root.verify(c).await?))?;
        }
        let access = Creation::new(
            Kind::Access,
            WORLD_ID,
            None,
            &self.world.fields,
            author::snapshot(
                &serde_json::json!({"world_id":WORLD_ID,"owner_user_id":self.world.owner_user_id,"access_revision":1,"collaborator_ids":[]}),
            )?,
        );
        require(audits.insert(access.verify(c).await?))?;
        require(audits.len() == roots.len() + 1)?;
        for d in &self.fields {
            require(cards::field_version(c, d.world_id.as_deref(), &d.id, 1).await? == *d)?;
        }
        for d in &self.layouts {
            require(relations::layout_version(c, &d.pin()).await? == *d)?;
        }
        for d in &self.collections {
            require(relations::collection_version(c, &d.pin()).await? == *d)?;
        }
        require(stats::definition_version(c, Some(WORLD_ID), RESOLVE_ID, 1).await? == self.stat)?;
        for (row, r) in self.cards.iter().zip(&self.relations) {
            require(cards::card_version(c, &row.pin()).await? == *row)?;
            let scalar:String=sqlx::query_scalar("SELECT snapshot_json FROM card_components WHERE namespace=? AND scope=? AND card_id=? AND revision=1").bind(row.identity.namespace.as_str()).bind(row.identity.world_id.as_deref().unwrap_or("")).bind(&row.identity.id).fetch_one(&mut *c).await.map_err(content::storage_error)?;
            require(scalar == author::snapshot(row)?)?;
            let expected = CardRelationRecord {
                card: row.pin(),
                relations: r.clone(),
                legacy_unmodeled: false,
            };
            require(relations::card_version(c, &row.pin()).await? == expected)?;
            let seal:String=sqlx::query_scalar("SELECT snapshot_json FROM card_relation_versions WHERE namespace=? AND scope=? AND card_id=? AND revision=1").bind(row.identity.namespace.as_str()).bind(row.identity.world_id.as_deref().unwrap_or("")).bind(&row.identity.id).fetch_one(&mut *c).await.map_err(content::storage_error)?;
            require(seal == author::snapshot(&expected)?)?;
        }
        require(types::type_version(c, SHARED_EMBER_ID, 1).await? == self.shared_ember)?;
        require(types::world_type_version(c, WORLD_ID, EMBER_ID, 1).await? == self.ember)?;
        let full =
            types::full_content(c, HistoryKind::Variant, FERNSTRIDER_ID, Some(WORLD_ID), 1).await?;
        require(
            full.before.is_none()
                && full.typing == TypingState::unmodeled()
                && full.stats == BaseStats::unmodeled(),
        )?;
        let mut expected = serde_json::to_value(&self.variant).map_err(|_| Error::Unavailable)?;
        expected["typing"] = serde_json::json!(TypingState::unmodeled());
        let v2 = serde_json::json!({"snapshot_version":2,"content":expected}).to_string();
        require(full.typing_projection_after == v2)?;
        expected["stats"] = serde_json::json!(BaseStats::unmodeled());
        let v3 = serde_json::json!({"snapshot_version":3,"content":expected}).to_string();
        require(full.after == v3)?;
        let world = types::full_content(c, HistoryKind::World, WORLD_ID, None, 1).await?;
        let bytes=serde_json::json!({"snapshot_version":2,"content":{"id":WORLD_ID,"fields":self.world.fields,"revision":1,"archived_at":null,"variant_ids":[FERNSTRIDER_ID],"type_ids":[EMBER_ID]}}).to_string();
        require(world.before.is_none() && world.after == bytes)
    }
}

// Closed SQL identifiers belong to this single fixture use case, not a repository framework.
#[derive(Clone, Copy)]
enum Kind {
    World,
    Access,
    Variant,
    SharedType,
    Type,
    Card,
    Field,
    Layout,
    Collection,
    Stat,
}
impl Kind {
    fn tables(self) -> (&'static str, &'static str, &'static str, &'static str) {
        match self {
            Self::World => ("worlds", "world_revisions", "world_id", "''"),
            Self::Access => ("worlds", "world_access_revisions", "world_id", "''"),
            Self::Variant => ("variants", "variant_revisions", "variant_id", "world_id"),
            Self::SharedType => ("types", "type_versions", "type_id", "''"),
            Self::Type => ("world_types", "world_type_versions", "type_id", "world_id"),
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
            Self::Layout => ("record_layouts", "record_layout_versions", "id", "scope"),
            Self::Collection => (
                "record_collection_definitions",
                "record_collection_versions",
                "id",
                "scope",
            ),
            Self::Stat => ("stat_definitions", "stat_versions", "stat_id", "scope"),
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
    fn new(
        kind: Kind,
        id: &str,
        scope: Option<&str>,
        fields: &ContentFields,
        after: String,
    ) -> Self {
        Self {
            kind,
            id: id.into(),
            scope: scope.unwrap_or("").into(),
            slug: fields.slug.clone(),
            after,
        }
    }
    async fn present(&self, c: &mut SqliteConnection) -> Result<bool, Error> {
        let (h, v, key, _) = self.kind.tables();
        sqlx::query_scalar(AssertSqlSafe(format!("SELECT EXISTS(SELECT 1 FROM {h} WHERE id=?1 UNION ALL SELECT 1 FROM {v} WHERE {key}=?1)"))).bind(&self.id).fetch_one(c).await.map_err(content::storage_error)
    }
    async fn check_slug(&self, c: &mut SqliteConnection) -> Result<(), Error> {
        let (h, v, _, scope) = self.kind.tables();
        let exists:bool=sqlx::query_scalar(AssertSqlSafe(format!("SELECT EXISTS(SELECT 1 FROM {h} WHERE {scope}=?1 AND slug=?2 UNION ALL SELECT 1 FROM {v} WHERE {scope}=?1 AND revision=1 AND json_extract(after_json,'$.content.fields.slug')=?2)"))).bind(&self.scope).bind(&self.slug).fetch_one(c).await.map_err(content::storage_error)?;
        require(!exists)
    }
    async fn verify(&self, c: &mut SqliteConnection) -> Result<String, Error> {
        let (h, v, key, scope) = self.kind.tables();
        let exists: bool = sqlx::query_scalar(AssertSqlSafe(format!(
            "SELECT EXISTS(SELECT 1 FROM {h} WHERE id=? AND {scope}=?)"
        )))
        .bind(&self.id)
        .bind(&self.scope)
        .fetch_one(&mut *c)
        .await
        .map_err(content::storage_error)?;
        require(exists)?;
        let audit:Option<String>=sqlx::query_scalar(AssertSqlSafe(format!("SELECT v.audit_id FROM {v} v JOIN audit_events a ON a.id=v.audit_id WHERE v.{key}=?1 AND {scope}=?2 AND v.revision=1 AND v.previous_revision IS NULL AND v.before_json IS NULL AND v.after_json=?3 AND a.actor_user_id IS NULL AND a.system_key='bundled-fixture-loader' AND a.action=?4 AND a.reason=?5"))).bind(&self.id).bind(&self.scope).bind(&self.after).bind(if matches!(self.kind,Kind::Access){"own"}else{"create"}).bind(REASON).fetch_optional(c).await.map_err(content::storage_error)?;
        audit.ok_or_else(baseline_error)
    }
}
