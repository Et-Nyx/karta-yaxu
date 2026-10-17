use game_data::{authoring::ContentFields, card_fields::*, cards::*, stats::NumericConstraints};
use sqlx::{Executor, SqlitePool};
use tempfile::TempDir;
use web_app::{
    accounts::Accounts,
    application::{
        authoring::{AddressKey, Authoring, Edit, Error},
        cards::*,
        fixtures::{Fixtures, SeedOptions},
        moves::{MoveEdit, Moves, NewMove},
        types::Types,
    },
    auth::{Role, Subject},
    persistence,
};

fn fields(name: &str) -> ContentFields {
    ContentFields {
        slug: "ignored".into(),
        display_name: name.into(),
        description: "Synthetic card design.".into(),
    }
}
struct Lab {
    _dir: TempDir,
    pool: SqlitePool,
    accounts: Accounts,
    cards: Cards,
    admin: String,
    owner: String,
    outsider: String,
    viewer: String,
    world: String,
}
async fn user(a: &Accounts, name: &str, role: Role) -> String {
    a.invite("test", name, role, None).await.unwrap();
    let actor = a
        .sign_in(Subject {
            provider: "test".into(),
            provider_subject: name.into(),
            display_name: name.into(),
        })
        .await
        .unwrap();
    a.start_session(&actor.user_id).await.unwrap().0
}
impl Lab {
    async fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let pool = persistence::connect_for_migration(&format!(
            "sqlite://{}/cards.db",
            dir.path().display()
        ))
        .await
        .unwrap();
        persistence::migrate(&pool).await.unwrap();
        let accounts = Accounts::new(pool.clone(), 3600).unwrap();
        let admin = user(&accounts, "admin", Role::Administrator).await;
        let owner = user(&accounts, "owner", Role::Editor).await;
        let outsider = user(&accounts, "outsider", Role::Editor).await;
        let viewer = user(&accounts, "viewer", Role::Viewer).await;
        let world = Authoring::new(accounts.clone())
            .create_world(&owner, fields("World"), None)
            .await
            .unwrap()
            .id;
        Self {
            _dir: dir,
            cards: Cards::new(accounts.clone()),
            pool,
            accounts,
            admin,
            owner,
            outsider,
            viewer,
            world,
        }
    }
    async fn audits(&self) -> i64 {
        sqlx::query_scalar("SELECT count(*) FROM audit_events")
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }
    async fn field(
        &self,
        w: Option<&str>,
        name: &str,
        kind: FieldKind,
        source: Option<FieldPin>,
    ) -> FieldDefinition {
        self.cards
            .create_field(
                if w.is_some() {
                    &self.owner
                } else {
                    &self.admin
                },
                w,
                NewField {
                    fields: fields(name),
                    kind,
                    source,
                    reason: None,
                },
            )
            .await
            .unwrap()
    }
    async fn card(
        &self,
        w: Option<&str>,
        name: &str,
        source: Option<CardPin>,
        component: CardComponent,
    ) -> CardRecord {
        self.cards
            .create(
                if w.is_some() {
                    &self.owner
                } else {
                    &self.admin
                },
                w,
                NewCard {
                    fields: fields(name),
                    source,
                    component,
                    reason: None,
                },
            )
            .await
            .unwrap()
    }
    async fn save(&self, row: &CardRecord, component: CardComponent) -> CardRecord {
        self.cards
            .save_card(
                if row.identity.world_id.is_some() {
                    &self.owner
                } else {
                    &self.admin
                },
                &row.identity,
                CardEdit {
                    expected_revision: row.revision,
                    component,
                    reason: None,
                },
            )
            .await
            .unwrap()
    }
}
fn component(d: &FieldDefinition, value: FieldValue) -> CardComponent {
    CardComponent {
        fields: Some(vec![CardField {
            definition: d.clone(),
            value,
        }]),
        rules_text: Some("Inert rules, never executed.".into()),
        labels: Some(vec!["Item".into()]),
    }
}
fn field_edit(d: &FieldDefinition) -> FieldEdit {
    FieldEdit {
        expected_revision: d.revision,
        fields: d.fields.clone(),
        kind: d.kind.clone(),
        archived_at: d.archived_at,
        reason: None,
    }
}

// Compare every column, including immutable snapshot bytes, in all pre-card tables.
async fn old_table_values(pool: &SqlitePool) -> Vec<(String, Vec<String>)> {
    let tables:Vec<String>=sqlx::query_scalar("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name NOT LIKE '_sqlx_%' AND name NOT LIKE 'card_%' AND name NOT LIKE 'standalone_card%' ORDER BY name").fetch_all(pool).await.unwrap();
    let mut out = vec![];
    for table in tables {
        let names: Vec<String> =
            sqlx::query_scalar("SELECT name FROM pragma_table_info(?) ORDER BY cid")
                .bind(&table)
                .fetch_all(pool)
                .await
                .unwrap();
        let columns = names
            .iter()
            .map(|s| format!("\"{}\"", s.replace('"', "\"\"")))
            .collect::<Vec<_>>()
            .join(",");
        let rows = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT json_array({columns}) AS row FROM \"{}\" ORDER BY row",
            table.replace('"', "\"\"")
        )))
        .fetch_all(pool)
        .await
        .unwrap();
        out.push((table, rows));
    }
    out
}

#[test]
fn values_are_typed_and_absent_unmodeled_empty_false_zero_are_distinct() {
    let identity = CardIdentity {
        namespace: CardNamespace::Standalone,
        world_id: None,
        id: "a".repeat(32),
    };
    let mut d = FieldDefinition {
        id: "b".repeat(32),
        world_id: None,
        source: None,
        fields: fields("Flag"),
        revision: 1,
        archived_at: None,
        kind: FieldKind::Boolean,
    };
    d.fields.slug = "flag".into();
    let values = [
        FieldValue::Missing,
        FieldValue::Unknown,
        FieldValue::NotApplicable,
        FieldValue::Boolean(false),
    ];
    for v in values {
        let c = component(&d, v);
        c.validate(&identity).unwrap();
        assert_eq!(
            serde_json::from_str::<CardComponent>(&serde_json::to_string(&c).unwrap()).unwrap(),
            c
        );
    }
    assert!(
        component(&d, FieldValue::Number("0".parse().unwrap()))
            .validate(&identity)
            .is_err()
    );
    d.kind = FieldKind::Number(NumericConstraints {
        whole_only: true,
        minimum: Some("0".parse().unwrap()),
        maximum: Some("1".parse().unwrap()),
    });
    component(&d, FieldValue::Number("0".parse().unwrap()))
        .validate(&identity)
        .unwrap();
    assert!(
        component(&d, FieldValue::Number("0.5".parse().unwrap()))
            .validate(&identity)
            .is_err()
    );
    d.kind = FieldKind::Text { max_length: 1 };
    component(&d, FieldValue::Text(String::new()))
        .validate(&identity)
        .unwrap();
    assert!(
        component(&d, FieldValue::Text("ab".into()))
            .validate(&identity)
            .is_err()
    );
    d.kind = FieldKind::Choice {
        options: vec![ChoiceOption {
            id: "c".repeat(32),
            label: "One".into(),
        }],
    };
    assert!(
        component(&d, FieldValue::Choice("One".into()))
            .validate(&identity)
            .is_err()
    );
    component(&d, FieldValue::Choice("c".repeat(32)))
        .validate(&identity)
        .unwrap();
    assert_ne!(
        CardComponent::default(),
        CardComponent {
            fields: Some(vec![]),
            ..Default::default()
        }
    );
}

#[tokio::test]
async fn standalone_exact_history_noops_conflicts_and_atomic_late_failure() {
    let l = Lab::new().await;
    let d = l.field(None, "Enabled", FieldKind::Boolean, None).await;
    let row = l
        .card(
            None,
            "Item",
            None,
            component(&d, FieldValue::Boolean(false)),
        )
        .await;
    assert_eq!(l.cards.detail(&row.identity).await.unwrap(), row);
    assert!(!row.legacy_unmodeled);
    let before = l.audits().await;
    assert_eq!(l.save(&row, row.component.clone()).await, row);
    assert_eq!(l.audits().await, before);
    let mut c = row.component.clone();
    c.rules_text = Some("New prose".into());
    let next = l.save(&row, c).await;
    assert_eq!(next.revision, 2);
    assert_eq!(l.audits().await, before + 1);
    assert_eq!(l.cards.version(&row.pin()).await.unwrap(), row);
    assert_eq!(
        l.cards
            .save_card(
                &l.admin,
                &row.identity,
                CardEdit {
                    expected_revision: 1,
                    component: CardComponent {
                        rules_text: Some("\0".into()),
                        ..Default::default()
                    },
                    reason: None
                }
            )
            .await,
        Err(Error::Conflict)
    );
    l.pool.execute("CREATE TRIGGER fail_card_seal BEFORE UPDATE ON card_components WHEN NEW.audit_id IS NOT NULL BEGIN SELECT RAISE(ABORT,'late failure'); END").await.unwrap();
    let mut c = next.component.clone();
    c.labels = Some(vec!["Changed".into()]);
    let audits = l.audits().await;
    assert!(
        l.cards
            .save_card(
                &l.admin,
                &next.identity,
                CardEdit {
                    expected_revision: next.revision,
                    component: c,
                    reason: None
                }
            )
            .await
            .is_err()
    );
    assert_eq!(l.cards.detail(&next.identity).await.unwrap(), next);
    assert_eq!(l.audits().await, audits);
    assert!(persistence::schema_ready(&l.pool).await.unwrap());
}

#[tokio::test]
async fn native_move_and_type_writers_preserve_component_and_share_audit() {
    let l = Lab::new().await;
    let d = l
        .field(
            None,
            "Cost",
            FieldKind::Number(NumericConstraints::default()),
            None,
        )
        .await;
    let moves = Moves::new(l.accounts.clone());
    let m = moves
        .create(
            &l.admin,
            None,
            NewMove {
                fields: fields("Move"),
                source: None,
                attributes: None,
                reason: None,
            },
        )
        .await
        .unwrap();
    let i = CardIdentity {
        namespace: CardNamespace::Move,
        world_id: None,
        id: m.id.clone(),
    };
    let c = l.cards.detail(&i).await.unwrap();
    let n = l
        .save(&c, component(&d, FieldValue::Number("0".parse().unwrap())))
        .await;
    assert_eq!(n.revision, m.revision + 1);
    let native = moves.detail(None, &m.id).await.unwrap();
    assert_eq!(native.revision, n.revision);
    let before = l.audits().await;
    let mut f = native.fields.clone();
    f.description = "Old editor metadata".into();
    let native = moves
        .save(
            &l.admin,
            None,
            &m.id,
            MoveEdit {
                expected_revision: native.revision,
                fields: f,
                attributes: None,
                reason: None,
            },
        )
        .await
        .unwrap();
    let read = l.cards.detail(&i).await.unwrap();
    assert_eq!(read.revision, native.revision);
    assert_eq!(read.component, n.component);
    assert_eq!(l.audits().await, before + 1);
    let same:bool=sqlx::query_scalar("SELECT c.audit_id=v.audit_id FROM card_components c JOIN move_versions v ON c.scope=v.scope AND c.card_id=v.move_id AND c.revision=v.revision WHERE c.namespace='move' AND c.card_id=? AND c.revision=?").bind(&m.id).bind(native.revision).fetch_one(&l.pool).await.unwrap();
    assert!(same);
    assert_eq!(
        moves
            .save(
                &l.admin,
                None,
                &m.id,
                MoveEdit {
                    expected_revision: m.revision,
                    fields: m.fields,
                    attributes: None,
                    reason: None
                }
            )
            .await,
        Err(Error::Conflict)
    );
    let types = Types::new(l.accounts.clone());
    let t = types
        .create_type(&l.admin, fields("Type"), None)
        .await
        .unwrap();
    let i = CardIdentity {
        namespace: CardNamespace::Type,
        world_id: None,
        id: t.id.clone(),
    };
    let c = l.cards.detail(&i).await.unwrap();
    let c = l.save(&c, component(&d, FieldValue::Unknown)).await;
    let t = types.type_detail(&t.id).await.unwrap();
    let mut f = t.fields.clone();
    f.description = "Native change".into();
    types
        .save_type(
            &l.admin,
            &t.id,
            Edit {
                expected_revision: t.revision,
                fields: f,
                reason: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(l.cards.detail(&i).await.unwrap().component, c.component);
}

#[tokio::test]
async fn permissions_revocation_scopes_and_exact_reference_retention() {
    let l = Lab::new().await;
    let d = l
        .field(Some(&l.world), "Target", FieldKind::CardReference, None)
        .await;
    let target = l
        .card(None, "Shared target", None, CardComponent::default())
        .await;
    let row = l
        .card(
            Some(&l.world),
            "Local",
            None,
            component(&d, FieldValue::CardReference(target.pin())),
        )
        .await;
    for token in [&l.admin, &l.outsider, &l.viewer] {
        assert_eq!(
            l.cards
                .save_card(
                    token,
                    &row.identity,
                    CardEdit {
                        expected_revision: row.revision,
                        component: CardComponent::default(),
                        reason: None
                    }
                )
                .await,
            Err(Error::Forbidden)
        );
    }
    let foreign = Authoring::new(l.accounts.clone())
        .create_world(&l.owner, fields("Foreign"), None)
        .await
        .unwrap();
    let other = l
        .card(Some(&foreign.id), "Other", None, CardComponent::default())
        .await;
    assert!(matches!(
        l.cards
            .save_card(
                &l.owner,
                &row.identity,
                CardEdit {
                    expected_revision: row.revision,
                    component: component(&d, FieldValue::CardReference(other.pin())),
                    reason: None
                }
            )
            .await,
        Err(Error::Invalid(_))
    ));
    let archived = l
        .cards
        .save_metadata(
            &l.admin,
            &target.identity,
            CardMetadataEdit {
                expected_revision: target.revision,
                fields: target.fields.clone(),
                archived_at: Some(1800000000),
                reason: None,
            },
        )
        .await
        .unwrap();
    let mut c = row.component.clone();
    c.rules_text = Some("Unrelated edit".into());
    let retained = l.save(&row, c).await;
    assert_eq!(
        retained.component.fields.as_ref().unwrap()[0].value,
        FieldValue::CardReference(target.pin())
    );
    assert!(matches!(
        l.cards
            .create(
                &l.owner,
                Some(&l.world),
                NewCard {
                    fields: fields("Bad new pin"),
                    source: None,
                    component: component(&d, FieldValue::CardReference(archived.pin())),
                    reason: None
                }
            )
            .await,
        Err(Error::Invalid(_))
    ));
    l.accounts.sign_out(&l.owner).await.unwrap();
    assert_eq!(
        l.cards
            .save_card(
                &l.owner,
                &retained.identity,
                CardEdit {
                    expected_revision: retained.revision,
                    component: CardComponent::default(),
                    reason: None
                }
            )
            .await,
        Err(Error::Unauthorized)
    );
}

#[tokio::test]
async fn source_fields_require_reviewed_injective_identity_mappings_and_keep_local_adoption() {
    let l = Lab::new().await;
    let shared = l.field(None, "Flag", FieldKind::Boolean, None).await;
    let local = l
        .field(
            Some(&l.world),
            "Local flag",
            FieldKind::Boolean,
            Some(shared.pin()),
        )
        .await;
    let source = l
        .card(
            None,
            "Source",
            None,
            component(&shared, FieldValue::Boolean(false)),
        )
        .await;
    let row = l
        .card(
            Some(&l.world),
            "Local",
            Some(source.pin()),
            CardComponent::default(),
        )
        .await;
    assert_eq!(row.component, CardComponent::default());
    let mapping = FieldMapping {
        shared_field_id: shared.id.clone(),
        local_field: local.pin(),
    };
    assert!(matches!(
        l.cards
            .preview_source_fields(
                &row.identity,
                row.revision,
                &[mapping.clone(), mapping.clone()]
            )
            .await,
        Err(Error::Invalid(_))
    ));
    let p = l
        .cards
        .preview_source_fields(&row.identity, row.revision, std::slice::from_ref(&mapping))
        .await
        .unwrap();
    assert!(p.unmapped.is_empty());
    let copied = l
        .cards
        .copy_source_fields(&l.owner, &p, None)
        .await
        .unwrap();
    assert_eq!(
        copied.component.fields.as_ref().unwrap()[0].value,
        FieldValue::Boolean(false)
    );
    assert_eq!(copied.component.rules_text, row.component.rules_text);
    assert_eq!(
        l.cards.copy_source_fields(&l.owner, &p, None).await,
        Err(Error::Conflict)
    );
    let mut edit = field_edit(&shared);
    edit.fields.description = "Source revision two".into();
    let shared2 = l
        .cards
        .save_field(&l.admin, None, &shared.id, edit)
        .await
        .unwrap();
    let adopted = l
        .cards
        .adopt_field(
            &l.owner,
            &l.world,
            &local.id,
            shared2.revision,
            field_edit(&local),
        )
        .await
        .unwrap();
    assert_eq!(adopted.fields, local.fields);
    assert_eq!(adopted.kind, local.kind);
    assert_eq!(adopted.source, Some(shared2.pin()));
    let mut c = source.component.clone();
    c.rules_text = Some("New source rules".into());
    let source2 = l.save(&source, c).await;
    let updated = l
        .cards
        .adopt_card(
            &l.owner,
            &copied.identity,
            copied.revision,
            source2.revision,
            None,
        )
        .await
        .unwrap();
    assert_eq!(updated.component, copied.component);
    assert_eq!(updated.source, Some(source2.pin()));
}

#[tokio::test]
async fn complete_fixtures_keep_65_objects_69_audits_and_all_native_components() {
    let l = Lab::new().await;
    let a = l.accounts.authenticate(&l.owner).await.unwrap().unwrap();
    let fixtures = Fixtures::new(l.accounts.clone());
    let options = || SeedOptions {
        owner_user_id: Some(a.user_id.as_str()),
        ..Default::default()
    };
    let before = l.audits().await;
    let report = fixtures.seed(options()).await.unwrap().fixtures;
    assert_eq!(
        report.move_attributes_created
            + report.shared_moves_created
            + report.world_moves_created
            + report.rulesets_created
            + report.learnsets_created
            + report.movesets_created
            + report.shared_stats_created
            + report.world_stats_created
            + report.stat_rosters_created
            + report.shared_charts_created
            + report.world_charts_created
            + report.types_created
            + report.species_created
            + report.forms_created
            + report.worlds_created
            + report.world_types_created
            + report.variants_created,
        65
    );
    assert_eq!(l.audits().await - before, 69);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM card_components")
        .fetch_one(&l.pool)
        .await
        .unwrap();
    assert!(count > 0);
    for page in 1..=4 {
        for c in l.cards.list(None, page).await.unwrap() {
            assert!(!c.legacy_unmodeled);
        }
    }
    assert_eq!(fixtures.seed(options()).await.unwrap().fixtures.skipped, 65);
    assert_eq!(l.audits().await - before, 69);
    let authored:i64=sqlx::query_scalar("SELECT (SELECT count(*) FROM standalone_cards)+(SELECT count(*) FROM card_field_definitions)").fetch_one(&l.pool).await.unwrap();
    assert_eq!(authored, 0);
}

#[tokio::test]
async fn field_kind_changes_never_reinterpret_old_values_and_archived_exact_pins_survive() {
    let l = Lab::new().await;
    let d = l
        .field(
            None,
            "Choice",
            FieldKind::Choice {
                options: vec![ChoiceOption {
                    id: "a".repeat(32),
                    label: "First".into(),
                }],
            },
            None,
        )
        .await;
    let row = l
        .card(
            None,
            "Old choice",
            None,
            component(&d, FieldValue::Choice("a".repeat(32))),
        )
        .await;
    let mut edit = field_edit(&d);
    edit.kind = FieldKind::Boolean;
    let changed = l
        .cards
        .save_field(&l.admin, None, &d.id, edit)
        .await
        .unwrap();
    assert_eq!(l.cards.detail(&row.identity).await.unwrap(), row);
    assert_eq!(l.cards.field_version(&d.pin()).await.unwrap(), d);
    assert!(matches!(
        l.cards
            .save_card(
                &l.admin,
                &row.identity,
                CardEdit {
                    expected_revision: row.revision,
                    component: component(&changed, FieldValue::Choice("a".repeat(32))),
                    reason: None
                }
            )
            .await,
        Err(Error::Invalid(_))
    ));
    let mut edit = field_edit(&changed);
    edit.archived_at = Some(1800000000);
    l.cards
        .save_field(&l.admin, None, &changed.id, edit)
        .await
        .unwrap();
    let mut c = row.component.clone();
    c.rules_text = Some("Retained choice".into());
    let updated = l.save(&row, c).await;
    assert_eq!(updated.component.fields.as_ref().unwrap()[0].definition, d);
    assert!(matches!(
        l.cards
            .create(
                &l.admin,
                None,
                NewCard {
                    fields: fields("New selection"),
                    source: None,
                    component: row.component,
                    reason: None
                }
            )
            .await,
        Err(Error::Invalid(_))
    ));
}

#[tokio::test]
async fn form_variant_and_local_type_native_editors_carry_custom_fields() {
    use web_app::application::stat_values::{FormStatsEdit, StatValues};
    let l = Lab::new().await;
    let actor = l.accounts.authenticate(&l.owner).await.unwrap().unwrap();
    Fixtures::new(l.accounts.clone())
        .seed(SeedOptions {
            owner_user_id: Some(&actor.user_id),
            ..Default::default()
        })
        .await
        .unwrap();
    let d = l.field(None, "Shared flag", FieldKind::Boolean, None).await;
    let author = Authoring::new(l.accounts.clone());
    let types = Types::new(l.accounts.clone());
    let forms: Vec<String> = sqlx::query_scalar("SELECT id FROM forms ORDER BY id LIMIT 1")
        .fetch_all(&l.pool)
        .await
        .unwrap();
    let i = CardIdentity {
        namespace: CardNamespace::Form,
        world_id: None,
        id: forms[0].clone(),
    };
    let c = l.cards.detail(&i).await.unwrap();
    let c = l.save(&c, component(&d, FieldValue::Boolean(false))).await;
    let species_version: i64 = sqlx::query_scalar("SELECT species_version FROM forms WHERE id=?")
        .bind(&i.id)
        .fetch_one(&l.pool)
        .await
        .unwrap();
    let mut f = c.fields.clone();
    f.description = "Native form metadata".into();
    let form = author
        .save_form(
            &l.admin,
            &i.id,
            species_version,
            Edit {
                expected_revision: c.revision,
                fields: f,
                reason: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(l.cards.detail(&i).await.unwrap().component, c.component);
    let form = types
        .save_form_typing(&l.admin, &i.id, form.revision, Some(vec![]), None)
        .await
        .unwrap();
    assert_eq!(l.cards.detail(&i).await.unwrap().component, c.component);
    StatValues::new(l.accounts.clone())
        .save_form(
            &l.admin,
            &i.id,
            FormStatsEdit {
                expected_revision: form.revision,
                values: Some(vec![]),
                reason: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(l.cards.detail(&i).await.unwrap().component, c.component);
    let (world, id): (String, String) =
        sqlx::query_as("SELECT world_id,id FROM variants ORDER BY id LIMIT 1")
            .fetch_one(&l.pool)
            .await
            .unwrap();
    let local = l
        .field(Some(&world), "Local flag", FieldKind::Boolean, None)
        .await;
    let i = CardIdentity {
        namespace: CardNamespace::Variant,
        world_id: Some(world.clone()),
        id,
    };
    let c = l.cards.detail(&i).await.unwrap();
    let c = l
        .save(&c, component(&local, FieldValue::Boolean(false)))
        .await;
    let mut f = c.fields.clone();
    f.description = "Native variant metadata".into();
    let v = author
        .save_variant(
            &l.owner,
            &world,
            &i.id,
            Edit {
                expected_revision: c.revision,
                fields: f,
                reason: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(l.cards.detail(&i).await.unwrap().component, c.component);
    types
        .save_variant_typing(&l.owner, &world, &i.id, v.revision, Some(vec![]), None)
        .await
        .unwrap();
    assert_eq!(l.cards.detail(&i).await.unwrap().component, c.component);
    let id: String =
        sqlx::query_scalar("SELECT id FROM world_types WHERE world_id=? ORDER BY id LIMIT 1")
            .bind(&world)
            .fetch_one(&l.pool)
            .await
            .unwrap();
    let i = CardIdentity {
        namespace: CardNamespace::Type,
        world_id: Some(world.clone()),
        id,
    };
    let c = l.cards.detail(&i).await.unwrap();
    let c = l.save(&c, component(&local, FieldValue::Unknown)).await;
    let mut f = c.fields.clone();
    f.description = "Native local Type metadata".into();
    types
        .save_world_type(
            &l.owner,
            &world,
            &i.id,
            Edit {
                expected_revision: c.revision,
                fields: f,
                reason: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(l.cards.detail(&i).await.unwrap().component, c.component);
}

#[tokio::test]
async fn cyclic_identity_references_are_exact_pins_and_read_without_recursive_walks() {
    let l = Lab::new().await;
    let d = l
        .field(None, "Reference", FieldKind::CardReference, None)
        .await;
    let a = l.card(None, "A", None, CardComponent::default()).await;
    let b = l
        .card(
            None,
            "B",
            None,
            component(&d, FieldValue::CardReference(a.pin())),
        )
        .await;
    let a2 = l
        .save(&a, component(&d, FieldValue::CardReference(b.pin())))
        .await;
    assert_eq!(l.cards.version(&a2.pin()).await.unwrap(), a2);
    assert_eq!(l.cards.version(&b.pin()).await.unwrap(), b);
    assert_eq!(l.cards.version(&a.pin()).await.unwrap(), a);
    assert_eq!(field_dependency_pins(&a2.component), vec![d.pin()]);
    assert_eq!(component_ref_pins(&a2.component), vec![b.pin()]);
    assert_eq!(component_ref_pins(&b.component), vec![a.pin()]);
}

#[tokio::test]
async fn corrupt_native_companions_fail_reads_and_roll_back_old_editor_writes() {
    let l = Lab::new().await;
    let moves = Moves::new(l.accounts.clone());
    let m = moves
        .create(
            &l.admin,
            None,
            NewMove {
                fields: fields("Move"),
                source: None,
                attributes: None,
                reason: None,
            },
        )
        .await
        .unwrap();
    let i = CardIdentity {
        namespace: CardNamespace::Move,
        world_id: None,
        id: m.id.clone(),
    };
    l.pool
        .execute("DROP TRIGGER card_components_sealed_update")
        .await
        .unwrap();
    sqlx::query("UPDATE card_components SET rules_text='corrupt typed companion' WHERE namespace='move' AND card_id=?").bind(&m.id).execute(&l.pool).await.unwrap();
    assert_eq!(l.cards.detail(&i).await, Err(Error::Unavailable));
    let before = l.audits().await;
    let mut f = m.fields.clone();
    f.description = "Must roll back".into();
    assert_eq!(
        moves
            .save(
                &l.admin,
                None,
                &m.id,
                MoveEdit {
                    expected_revision: m.revision,
                    fields: f,
                    attributes: None,
                    reason: None
                }
            )
            .await,
        Err(Error::Unavailable)
    );
    assert_eq!(moves.detail(None, &m.id).await.unwrap(), m);
    assert_eq!(l.audits().await, before);
}

#[tokio::test]
async fn collaborator_permissions_and_account_disable_are_rechecked_under_writer_lock() {
    let l = Lab::new().await;
    let row = l
        .card(Some(&l.world), "Owned card", None, CardComponent::default())
        .await;
    let outsider = l.accounts.authenticate(&l.outsider).await.unwrap().unwrap();
    let author = Authoring::new(l.accounts.clone());
    let access = author.access(&l.owner, &l.world).await.unwrap();
    author
        .grant_collaborator(
            &l.owner,
            &l.world,
            access.world.access_revision,
            &outsider.user_id,
            None,
        )
        .await
        .unwrap();
    let edited = l
        .cards
        .save_card(
            &l.outsider,
            &row.identity,
            CardEdit {
                expected_revision: row.revision,
                component: CardComponent {
                    rules_text: Some("Collaborator edit".into()),
                    ..Default::default()
                },
                reason: None,
            },
        )
        .await
        .unwrap();
    let actor:String=sqlx::query_scalar("SELECT a.actor_user_id FROM card_components c JOIN audit_events a ON a.id=c.audit_id WHERE c.namespace='standalone' AND c.scope=? AND c.card_id=? AND c.revision=?").bind(&l.world).bind(&edited.identity.id).bind(edited.revision).fetch_one(&l.pool).await.unwrap();
    assert_eq!(actor, outsider.user_id);
    let access = author.access(&l.owner, &l.world).await.unwrap();
    author
        .revoke_collaborator(
            &l.owner,
            &l.world,
            access.world.access_revision,
            &outsider.user_id,
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        l.cards
            .save_card(
                &l.outsider,
                &edited.identity,
                CardEdit {
                    expected_revision: edited.revision,
                    component: CardComponent::default(),
                    reason: None
                }
            )
            .await,
        Err(Error::Forbidden)
    );
    assert_eq!(
        l.cards
            .create_field(
                &l.owner,
                None,
                NewField {
                    fields: fields("Shared forbidden"),
                    kind: FieldKind::Boolean,
                    source: None,
                    reason: None
                }
            )
            .await,
        Err(Error::Forbidden)
    );
    let owner = l.accounts.authenticate(&l.owner).await.unwrap().unwrap();
    l.accounts.set_disabled(&owner.user_id, true).await.unwrap();
    assert_eq!(
        l.cards
            .save_card(
                &l.owner,
                &edited.identity,
                CardEdit {
                    expected_revision: edited.revision,
                    component: CardComponent::default(),
                    reason: None
                }
            )
            .await,
        Err(Error::Unauthorized)
    );
}

#[tokio::test]
async fn native_adoption_cannot_commit_a_source_with_missing_post_boundary_component() {
    use web_app::application::moves::MoveAdoptEdit;
    let l = Lab::new().await;
    let moves = Moves::new(l.accounts.clone());
    let source = moves
        .create(
            &l.admin,
            None,
            NewMove {
                fields: fields("Source"),
                source: None,
                attributes: None,
                reason: None,
            },
        )
        .await
        .unwrap();
    let local = moves
        .create(
            &l.owner,
            Some(&l.world),
            NewMove {
                fields: fields("Local"),
                source: Some(source.pin()),
                attributes: None,
                reason: None,
            },
        )
        .await
        .unwrap();
    let mut f = source.fields.clone();
    f.description = "Source two".into();
    let source = moves
        .save(
            &l.admin,
            None,
            &source.id,
            MoveEdit {
                expected_revision: source.revision,
                fields: f,
                attributes: None,
                reason: None,
            },
        )
        .await
        .unwrap();
    l.pool
        .execute("DROP TRIGGER card_components_sealed_delete")
        .await
        .unwrap();
    sqlx::query("DELETE FROM card_components WHERE namespace='move' AND scope='' AND card_id=? AND revision=?").bind(&source.id).bind(source.revision).execute(&l.pool).await.unwrap();
    let before = l.audits().await;
    assert_eq!(
        moves
            .adopt(
                &l.owner,
                &l.world,
                &local.id,
                MoveAdoptEdit {
                    expected_revision: local.revision,
                    target_version: source.revision,
                    fields: local.fields.clone(),
                    apply_source: false,
                    mappings: vec![],
                    reason: None
                }
            )
            .await,
        Err(Error::Unavailable)
    );
    assert_eq!(
        moves.detail(Some(&l.world), &local.id).await.unwrap(),
        local
    );
    assert_eq!(l.audits().await, before);
}

#[tokio::test]
async fn local_move_metadata_and_native_source_adoption_keep_card_components() {
    use web_app::application::moves::MoveAdoptEdit;
    let l = Lab::new().await;
    let moves = Moves::new(l.accounts.clone());
    let d = l
        .field(
            Some(&l.world),
            "Local text",
            FieldKind::Text { max_length: 20 },
            None,
        )
        .await;
    let source = moves
        .create(
            &l.admin,
            None,
            NewMove {
                fields: fields("Shared move"),
                source: None,
                attributes: None,
                reason: None,
            },
        )
        .await
        .unwrap();
    let local = moves
        .create(
            &l.owner,
            Some(&l.world),
            NewMove {
                fields: fields("Local move"),
                source: Some(source.pin()),
                attributes: None,
                reason: None,
            },
        )
        .await
        .unwrap();
    let i = CardIdentity {
        namespace: CardNamespace::Move,
        world_id: Some(l.world.clone()),
        id: local.id.clone(),
    };
    let c = l.cards.detail(&i).await.unwrap();
    let c = l
        .save(&c, component(&d, FieldValue::Text(String::new())))
        .await;
    let mut f = source.fields.clone();
    f.description = "New shared description".into();
    let source = moves
        .save(
            &l.admin,
            None,
            &source.id,
            MoveEdit {
                expected_revision: source.revision,
                fields: f,
                attributes: None,
                reason: None,
            },
        )
        .await
        .unwrap();
    moves
        .adopt(
            &l.owner,
            &l.world,
            &local.id,
            MoveAdoptEdit {
                expected_revision: c.revision,
                target_version: source.revision,
                fields: c.fields.clone(),
                apply_source: false,
                mappings: vec![],
                reason: None,
            },
        )
        .await
        .unwrap();
    let adopted = l.cards.detail(&i).await.unwrap();
    assert_eq!(adopted.component, c.component);
    assert_eq!(adopted.source.as_ref().unwrap().version, source.revision);
    let mut f = adopted.fields.clone();
    f.description = "Local metadata".into();
    moves
        .save(
            &l.owner,
            Some(&l.world),
            &local.id,
            MoveEdit {
                expected_revision: adopted.revision,
                fields: f,
                attributes: None,
                reason: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(l.cards.detail(&i).await.unwrap().component, c.component);
}

#[tokio::test]
async fn populated_upgrade_records_legacy_boundary_without_rewriting_old_values_or_audits() {
    let dir = tempfile::tempdir().unwrap();
    let pool = persistence::connect_for_migration(&format!(
        "sqlite://{}/upgrade.db",
        dir.path().display()
    ))
    .await
    .unwrap();
    for migration in [
        include_str!("../migrations/0001_auth.sql"),
        include_str!("../migrations/0002_world_authoring.sql"),
        include_str!("../migrations/0003_types.sql"),
        include_str!("../migrations/0004_charts.sql"),
        include_str!("../migrations/0005_stats.sql"),
        include_str!("../migrations/0006_stat_content.sql"),
        include_str!("../migrations/0007_moves_rulesets.sql"),
        include_str!("../migrations/0008_learning.sql"),
    ] {
        pool.execute(sqlx::raw_sql(migration)).await.unwrap();
    }
    let accounts = Accounts::new(pool.clone(), 3600).unwrap();
    let admin = user(&accounts, "admin", Role::Administrator).await;
    let world = Authoring::new(accounts.clone())
        .create_world(&admin, fields("Legacy world"), None)
        .await
        .unwrap();
    let actor = accounts.authenticate(&admin).await.unwrap().unwrap();
    let id = "a".repeat(32);
    let row = web_app::application::types::TypeRecord {
        id: id.clone(),
        fields: ContentFields {
            slug: "old".into(),
            display_name: "Old".into(),
            description: "Original bytes".into(),
        },
        revision: 1,
        archived_at: None,
    };
    let after = serde_json::json!({"snapshot_version":1,"content":row}).to_string();
    let mut tx = persistence::begin_write(&pool).await.unwrap();
    sqlx::query(
        "INSERT INTO audit_events(id,actor_user_id,action) VALUES ('old-audit',?,'create')",
    )
    .bind(&actor.user_id)
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::query("INSERT INTO types(id,slug,display_name,description,revision) VALUES (?,'old','Old','Original bytes',1)").bind(&id).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO type_versions(type_id,revision,slug,display_name,description,audit_id,after_json) VALUES (?,1,'old','Old','Original bytes','old-audit',?)").bind(&id).bind(&after).execute(&mut *tx).await.unwrap();
    // Native IDs are only qualified, never globally unique across kind or scope.
    {
        let w: Option<&str> = None;
        let m = game_data::moves::MoveRecord {
            id: id.clone(),
            world_id: w.map(str::to_owned),
            source: None,
            fields: row.fields.clone(),
            revision: 1,
            archived_at: None,
            attributes: None,
            mappings: vec![],
        };
        let audit = "old-shared-move";
        sqlx::query("INSERT INTO audit_events(id,actor_user_id,action) VALUES (?,?,'create')")
            .bind(audit)
            .bind(&actor.user_id)
            .execute(&mut *tx)
            .await
            .unwrap();
        sqlx::query("INSERT INTO moves(scope,id,world_id,slug,revision) VALUES (?,?,?,'old',1)")
            .bind("")
            .bind(&id)
            .bind(w)
            .execute(&mut *tx)
            .await
            .unwrap();
        sqlx::query("INSERT INTO move_versions(scope,move_id,revision,slug,display_name,description,attributes_defined,audit_id,after_json) VALUES (?,?,1,'old','Old','Original bytes',0,?,?)").bind("").bind(&id).bind(audit).bind(serde_json::json!({"snapshot_version":1,"content":m}).to_string()).execute(&mut *tx).await.unwrap();
    }
    let local_type = web_app::application::types::WorldTypeRecord {
        id: id.clone(),
        world_id: world.id.clone(),
        source: None,
        fields: row.fields.clone(),
        revision: 1,
        archived_at: None,
    };
    sqlx::query(
        "INSERT INTO audit_events(id,actor_user_id,action) VALUES ('old-local-type',?,'create')",
    )
    .bind(&actor.user_id)
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::query("INSERT INTO world_types(id,world_id,slug,display_name,description,revision) VALUES (?,?,'old','Old','Original bytes',1)").bind(&id).bind(&world.id).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO world_type_versions(world_id,type_id,revision,slug,display_name,description,audit_id,after_json) VALUES (?,?,1,'old','Old','Original bytes','old-local-type',?)").bind(&world.id).bind(&id).bind(serde_json::json!({"snapshot_version":1,"content":local_type}).to_string()).execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
    let all_old = old_table_values(&pool).await;
    let old: Vec<(String, String, i64)> =
        sqlx::query_as("SELECT type_id,after_json,revision FROM type_versions")
            .fetch_all(&pool)
            .await
            .unwrap();
    let audits: i64 = sqlx::query_scalar("SELECT count(*) FROM audit_events")
        .fetch_one(&pool)
        .await
        .unwrap();
    pool.execute(sqlx::raw_sql(include_str!("../migrations/0009_cards.sql")))
        .await
        .unwrap();
    assert_eq!(old_table_values(&pool).await, all_old);
    assert_eq!(
        sqlx::query_as::<_, (String, String, i64)>(
            "SELECT type_id,after_json,revision FROM type_versions"
        )
        .fetch_all(&pool)
        .await
        .unwrap(),
        old
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM audit_events")
            .fetch_one(&pool)
            .await
            .unwrap(),
        audits
    );
    let cards = Cards::new(accounts.clone());
    let i = CardIdentity {
        namespace: CardNamespace::Type,
        world_id: None,
        id,
    };
    let legacy = cards.detail(&i).await.unwrap();
    let legacy_history = cards.history(&i, None).await.unwrap();
    assert_eq!(legacy_history.len(), 1);
    assert_eq!(
        (
            legacy_history[0].revision,
            legacy_history[0].actor_name.as_str(),
            legacy_history[0].action.as_str()
        ),
        (1, "admin", "create")
    );
    for (namespace, w) in [
        (CardNamespace::Move, None),
        (CardNamespace::Type, Some(world.id.clone())),
    ] {
        let move_identity = CardIdentity {
            namespace,
            world_id: w,
            id: i.id.clone(),
        };
        let m = cards.detail(&move_identity).await.unwrap();
        assert!(m.legacy_unmodeled);
        assert_eq!(m.identity, move_identity);
        assert_ne!(m.identity, i);
    }
    assert!(legacy.legacy_unmodeled);
    assert_eq!(legacy.component, CardComponent::default());
    let upgraded = cards
        .save_card(
            &admin,
            &i,
            CardEdit {
                expected_revision: 1,
                component: CardComponent {
                    fields: Some(vec![]),
                    ..Default::default()
                },
                reason: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(upgraded.revision, 2);
    assert!(!upgraded.legacy_unmodeled);
    assert_eq!(cards.version(&legacy.pin()).await.unwrap(), legacy);
    let upgraded_history = cards.history(&i, None).await.unwrap();
    assert_eq!(
        upgraded_history
            .iter()
            .map(|e| (e.revision, e.actor_name.as_str(), e.action.as_str()))
            .collect::<Vec<_>>(),
        vec![(2, "admin", "edit"), (1, "admin", "create")]
    );
    assert!(
        pool.execute("DELETE FROM card_components WHERE revision=2")
            .await
            .is_err()
    );
    pool.execute("DROP TRIGGER card_components_sealed_delete")
        .await
        .unwrap();
    pool.execute("DELETE FROM card_components WHERE revision=2")
        .await
        .unwrap();
    assert_eq!(cards.detail(&i).await, Err(Error::Unavailable));
}

#[tokio::test]
async fn attributed_history_is_public_bounded_and_cursor_paginated_for_cards_and_fields() {
    let l = Lab::new().await;
    let outsider = l.accounts.authenticate(&l.outsider).await.unwrap().unwrap();
    let author = Authoring::new(l.accounts.clone());
    let access = author.access(&l.owner, &l.world).await.unwrap();
    author
        .grant_collaborator(
            &l.owner,
            &l.world,
            access.world.access_revision,
            &outsider.user_id,
            None,
        )
        .await
        .unwrap();
    let mut card = l
        .card(
            Some(&l.world),
            "History card",
            None,
            CardComponent::default(),
        )
        .await;
    let mut definition = l
        .field(Some(&l.world), "History field", FieldKind::Boolean, None)
        .await;
    for revision in 2..=28 {
        let token = if revision % 2 == 0 {
            &l.outsider
        } else {
            &l.owner
        };
        card = l
            .cards
            .save_card(
                token,
                &card.identity,
                CardEdit {
                    expected_revision: card.revision,
                    component: CardComponent {
                        rules_text: Some(format!("Revision {revision}")),
                        ..Default::default()
                    },
                    reason: Some(format!("Card reason {revision}")),
                },
            )
            .await
            .unwrap();
        let mut edit = field_edit(&definition);
        edit.fields.description = format!("Revision {revision}");
        edit.reason = Some(format!("Field reason {revision}"));
        definition = l
            .cards
            .save_field(token, Some(&l.world), &definition.id, edit)
            .await
            .unwrap();
    }
    let access = author.access(&l.owner, &l.world).await.unwrap();
    author
        .revoke_collaborator(
            &l.owner,
            &l.world,
            access.world.access_revision,
            &outsider.user_id,
            None,
        )
        .await
        .unwrap();
    l.accounts
        .set_disabled(&outsider.user_id, true)
        .await
        .unwrap();
    let cards = l.cards.history(&card.identity, None).await.unwrap();
    let fields = l
        .cards
        .field_history(Some(&l.world), &definition.id, None)
        .await
        .unwrap();
    for entries in [&cards, &fields] {
        assert_eq!(
            entries.iter().map(|e| e.revision).collect::<Vec<_>>(),
            (4..=28).rev().collect::<Vec<_>>()
        );
        assert_eq!(entries[0].actor_name, "outsider");
        assert_eq!(entries[1].actor_name, "owner");
        assert!(
            entries
                .iter()
                .all(|e| e.action == "edit" && !e.occurred_at.is_empty())
        );
        let value = serde_json::to_value(&entries[0]).unwrap();
        assert_eq!(
            value
                .as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            vec!["action", "actor_name", "occurred_at", "reason", "revision"]
        );
    }
    assert_eq!(cards[0].reason.as_deref(), Some("Card reason 28"));
    assert_eq!(fields[0].reason.as_deref(), Some("Field reason 28"));
    let cards = l.cards.history(&card.identity, Some(4)).await.unwrap();
    let fields = l
        .cards
        .field_history(Some(&l.world), &definition.id, Some(4))
        .await
        .unwrap();
    for entries in [&cards, &fields] {
        assert_eq!(
            entries.iter().map(|e| e.revision).collect::<Vec<_>>(),
            vec![3, 2, 1]
        );
        assert_eq!(entries[2].actor_name, "owner");
        assert_eq!(entries[2].action, "create");
        assert_eq!(entries[2].reason, None);
    }
    assert!(
        l.cards
            .history(&card.identity, Some(1))
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        l.cards
            .field_history(Some(&l.world), &definition.id, Some(1))
            .await
            .unwrap()
            .is_empty()
    );
    assert!(matches!(
        l.cards.history(&card.identity, Some(0)).await,
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        l.cards
            .field_history(Some(&l.world), &definition.id, Some(-1))
            .await,
        Err(Error::Invalid(_))
    ));
}

#[tokio::test]
async fn history_uses_native_audits_and_checks_every_returned_exact_revision() {
    let l = Lab::new().await;
    let types = Types::new(l.accounts.clone());
    let native = types
        .create_type(&l.admin, fields("Native history"), Some("Native creation"))
        .await
        .unwrap();
    let identity = CardIdentity {
        namespace: CardNamespace::Type,
        world_id: None,
        id: native.id.clone(),
    };
    let card = l.cards.detail(&identity).await.unwrap();
    let mut component = card.component.clone();
    component.rules_text = Some("Card revision".into());
    let card = l
        .cards
        .save_card(
            &l.admin,
            &identity,
            CardEdit {
                expected_revision: card.revision,
                component,
                reason: Some("Custom edit".into()),
            },
        )
        .await
        .unwrap();
    let mut edit = Edit {
        expected_revision: card.revision,
        fields: card.fields.clone(),
        reason: Some("Native edit".into()),
    };
    edit.fields.description = "Typed native edit".into();
    types.save_type(&l.admin, &identity.id, edit).await.unwrap();
    let history = l.cards.history(&identity, None).await.unwrap();
    assert_eq!(
        history
            .iter()
            .map(|e| (
                e.revision,
                e.actor_name.as_str(),
                e.action.as_str(),
                e.reason.as_deref()
            ))
            .collect::<Vec<_>>(),
        vec![
            (3, "admin", "edit", Some("Native edit")),
            (2, "admin", "edit", Some("Custom edit")),
            (1, "admin", "create", Some("Native creation"))
        ]
    );
    l.pool
        .execute("DROP TRIGGER card_components_sealed_update")
        .await
        .unwrap();
    sqlx::query("UPDATE card_components SET rules_text='Damaged earlier component' WHERE namespace='type' AND card_id=? AND revision=1").bind(&identity.id).execute(&l.pool).await.unwrap();
    // Current specialized metadata is a valid projection; historical common
    // reads cannot skip the damaged earlier component or invent an empty one.
    assert_eq!(l.cards.detail(&identity).await.unwrap().revision, 3);
    assert_eq!(
        l.cards.history(&identity, None).await,
        Err(Error::Unavailable)
    );
    assert_eq!(
        l.cards.history(&identity, Some(2)).await,
        Err(Error::Unavailable)
    );
}

#[tokio::test]
async fn scoped_resolvers_support_slugs_and_ids_without_ambiguous_form_slug_lookup() {
    let l = Lab::new().await;
    let types = Types::new(l.accounts.clone());
    let moves = Moves::new(l.accounts.clone());
    let shared = l
        .card(None, "Same name", None, CardComponent::default())
        .await;
    let local = l
        .card(Some(&l.world), "Same name", None, CardComponent::default())
        .await;
    let shared_field = l.field(None, "Same name", FieldKind::Boolean, None).await;
    let local_field = l
        .field(Some(&l.world), "Same name", FieldKind::Boolean, None)
        .await;
    let native_type = types
        .create_type(&l.admin, fields("Same name"), None)
        .await
        .unwrap();
    let native_move = moves
        .create(
            &l.admin,
            None,
            NewMove {
                fields: fields("Same name"),
                source: None,
                attributes: None,
                reason: None,
            },
        )
        .await
        .unwrap();
    for card in [&shared, &local] {
        for key in [
            AddressKey::Id(card.identity.id.clone()),
            AddressKey::Slug(card.fields.slug.clone()),
        ] {
            assert_eq!(
                l.cards
                    .resolve_card(
                        card.identity.world_id.as_deref(),
                        CardNamespace::Standalone,
                        key
                    )
                    .await
                    .unwrap(),
                *card
            );
        }
    }
    for definition in [&shared_field, &local_field] {
        for key in [
            AddressKey::Id(definition.id.clone()),
            AddressKey::Slug(definition.fields.slug.clone()),
        ] {
            assert_eq!(
                l.cards
                    .resolve_field(definition.world_id.as_deref(), key)
                    .await
                    .unwrap(),
                *definition
            );
        }
    }
    for (namespace, id, slug) in [
        (
            CardNamespace::Type,
            &native_type.id,
            &native_type.fields.slug,
        ),
        (
            CardNamespace::Move,
            &native_move.id,
            &native_move.fields.slug,
        ),
    ] {
        assert_eq!(
            l.cards
                .resolve_card(None, namespace, AddressKey::Slug(slug.clone()))
                .await
                .unwrap()
                .identity
                .id,
            *id
        );
    }
    let foreign = Authoring::new(l.accounts.clone())
        .create_world(&l.owner, fields("Foreign"), None)
        .await
        .unwrap();
    assert_eq!(
        l.cards
            .resolve_card(
                Some(&foreign.id),
                CardNamespace::Standalone,
                AddressKey::Slug(local.fields.slug.clone())
            )
            .await,
        Err(Error::NotFound)
    );
    assert_eq!(
        l.cards
            .resolve_field(
                Some(&foreign.id),
                AddressKey::Slug(local_field.fields.slug.clone())
            )
            .await,
        Err(Error::NotFound)
    );
    assert_eq!(
        l.cards
            .resolve_card(
                None,
                CardNamespace::Standalone,
                AddressKey::Id(shared.fields.slug.clone())
            )
            .await,
        Err(Error::NotFound)
    );
    assert!(matches!(
        l.cards
            .resolve_card(
                None,
                CardNamespace::Form,
                AddressKey::Slug("default".into())
            )
            .await,
        Err(Error::Invalid(_))
    ));
    let actor = l.accounts.authenticate(&l.owner).await.unwrap().unwrap();
    Fixtures::new(l.accounts.clone())
        .seed(SeedOptions {
            owner_user_id: Some(&actor.user_id),
            ..Default::default()
        })
        .await
        .unwrap();
    let forms: Vec<(String, String)> =
        sqlx::query_as("SELECT id,slug FROM forms ORDER BY id LIMIT 2")
            .fetch_all(&l.pool)
            .await
            .unwrap();
    for (id, slug) in forms {
        assert_eq!(
            l.cards
                .resolve_card(None, CardNamespace::Form, AddressKey::Id(id.clone()))
                .await
                .unwrap()
                .identity
                .id,
            id
        );
        assert!(matches!(
            l.cards
                .resolve_card(None, CardNamespace::Form, AddressKey::Slug(slug))
                .await,
            Err(Error::Invalid(_))
        ));
    }
    let mut fields = local.fields.clone();
    fields.display_name = "Renamed".into();
    let renamed = l
        .cards
        .save_metadata(
            &l.owner,
            &local.identity,
            CardMetadataEdit {
                expected_revision: local.revision,
                fields,
                archived_at: Some(1800000000),
                reason: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(
        l.cards
            .resolve_card(
                Some(&l.world),
                CardNamespace::Standalone,
                AddressKey::Slug(local.fields.slug)
            )
            .await,
        Err(Error::NotFound)
    );
    assert_eq!(
        l.cards
            .resolve_card(
                Some(&l.world),
                CardNamespace::Standalone,
                AddressKey::Slug(renamed.fields.slug.clone())
            )
            .await
            .unwrap(),
        renamed
    );
}

#[tokio::test]
async fn direct_reference_and_source_checks_reconstruct_typed_target_children() {
    let l = Lab::new().await;
    let types = Types::new(l.accounts.clone());
    let definition = l.field(None, "Flag", FieldKind::Boolean, None).await;
    let reference = l
        .field(None, "Reference", FieldKind::CardReference, None)
        .await;
    let native = types
        .create_type(&l.admin, fields("Typed target"), None)
        .await
        .unwrap();
    let identity = CardIdentity {
        namespace: CardNamespace::Type,
        world_id: None,
        id: native.id.clone(),
    };
    let target = l.cards.detail(&identity).await.unwrap();
    let target = l
        .save(&target, component(&definition, FieldValue::Boolean(false)))
        .await;
    let referrer = l
        .card(
            None,
            "Referrer",
            None,
            component(&reference, FieldValue::CardReference(target.pin())),
        )
        .await;
    let world = Authoring::new(l.accounts.clone())
        .world_detail(&l.world)
        .await
        .unwrap();
    let local_type = types
        .create_world_type(
            &l.owner,
            &l.world,
            world.revision,
            Some(game_data::types::TypePin {
                type_id: native.id.clone(),
                version: target.revision,
            }),
            fields("Local target"),
            None,
        )
        .await
        .unwrap();
    let local_identity = CardIdentity {
        namespace: CardNamespace::Type,
        world_id: Some(l.world.clone()),
        id: local_type.id,
    };
    l.pool
        .execute("DROP TRIGGER card_component_fields_immutable_update")
        .await
        .unwrap();
    sqlx::query("UPDATE card_component_fields SET boolean_value=1 WHERE namespace='type' AND scope='' AND card_id=? AND revision=?").bind(&target.identity.id).bind(target.revision).execute(&l.pool).await.unwrap();
    assert_eq!(
        types.type_detail(&native.id).await.unwrap().revision,
        target.revision
    );
    assert_eq!(
        l.cards.version(&target.pin()).await,
        Err(Error::Unavailable)
    );
    assert_eq!(
        l.cards.version(&referrer.pin()).await,
        Err(Error::Unavailable)
    );
    assert_eq!(
        l.cards.detail(&local_identity).await,
        Err(Error::Unavailable)
    );
}

#[tokio::test]
async fn field_predecessor_typed_header_and_audit_cannot_be_hidden_by_matching_before_bytes() {
    let l = Lab::new().await;
    let first = l.field(None, "Predecessor", FieldKind::Boolean, None).await;
    let mut edit = field_edit(&first);
    edit.fields.description = "Second revision".into();
    let second = l
        .cards
        .save_field(&l.admin, None, &first.id, edit)
        .await
        .unwrap();
    l.pool
        .execute("DROP TRIGGER card_field_versions_immutable_update")
        .await
        .unwrap();
    let mut forged = first.clone();
    forged.fields.description = "Forged snapshot, not typed header".into();
    let bytes = serde_json::json!({"snapshot_version":1,"content":forged}).to_string();
    sqlx::query(
        "UPDATE card_field_versions SET after_json=? WHERE scope='' AND field_id=? AND revision=1",
    )
    .bind(&bytes)
    .bind(&first.id)
    .execute(&l.pool)
    .await
    .unwrap();
    sqlx::query(
        "UPDATE card_field_versions SET before_json=? WHERE scope='' AND field_id=? AND revision=2",
    )
    .bind(&bytes)
    .bind(&first.id)
    .execute(&l.pool)
    .await
    .unwrap();
    assert_eq!(
        l.cards.field_version(&second.pin()).await,
        Err(Error::Unavailable)
    );
    assert_eq!(
        l.cards.field(None, &first.id).await,
        Err(Error::Unavailable)
    );
    assert_eq!(
        l.cards.field_history(None, &first.id, None).await,
        Err(Error::Unavailable)
    );
    // Restore consistent snapshot headers, then independently damage the original
    // audit action: the predecessor must still be verified, not just byte-matched.
    let bytes = serde_json::json!({"snapshot_version":1,"content":first}).to_string();
    sqlx::query(
        "UPDATE card_field_versions SET after_json=? WHERE scope='' AND field_id=? AND revision=1",
    )
    .bind(&bytes)
    .bind(&first.id)
    .execute(&l.pool)
    .await
    .unwrap();
    sqlx::query(
        "UPDATE card_field_versions SET before_json=? WHERE scope='' AND field_id=? AND revision=2",
    )
    .bind(&bytes)
    .bind(&first.id)
    .execute(&l.pool)
    .await
    .unwrap();
    l.pool.execute("DROP TRIGGER audit_update").await.unwrap();
    sqlx::query("UPDATE audit_events SET action='edit' WHERE id=(SELECT audit_id FROM card_field_versions WHERE scope='' AND field_id=? AND revision=1)").bind(&first.id).execute(&l.pool).await.unwrap();
    assert_eq!(
        l.cards.field_version(&second.pin()).await,
        Err(Error::Unavailable)
    );
}

#[tokio::test]
async fn exact_self_reference_is_verified_without_recursively_walking_the_cycle() {
    let l = Lab::new().await;
    let definition = l
        .field(None, "Self reference", FieldKind::CardReference, None)
        .await;
    let mut row = l
        .card(
            None,
            "Cycle",
            None,
            component(&definition, FieldValue::Unknown),
        )
        .await;
    row.component.fields.as_mut().unwrap()[0].value = FieldValue::CardReference(row.pin());
    // Construct an internally consistent exact cycle in this test-owned DB.
    // Ordinary authoring selects existing pins; capture/package input may cycle.
    for trigger in [
        "card_components_sealed_update",
        "card_component_fields_immutable_update",
        "standalone_card_versions_immutable_update",
    ] {
        l.pool
            .execute(sqlx::AssertSqlSafe(format!("DROP TRIGGER {trigger}")))
            .await
            .unwrap();
    }
    let bytes = serde_json::json!({"snapshot_version":1,"content":row}).to_string();
    let mut tx = persistence::begin_write(&l.pool).await.unwrap();
    sqlx::query("UPDATE card_component_fields SET state='card_reference',ref_namespace='standalone',ref_scope='',ref_id=?,ref_revision=1 WHERE namespace='standalone' AND scope='' AND card_id=? AND revision=1").bind(&row.identity.id).bind(&row.identity.id).execute(&mut *tx).await.unwrap();
    sqlx::query("UPDATE card_components SET snapshot_json=? WHERE namespace='standalone' AND scope='' AND card_id=? AND revision=1").bind(&bytes).bind(&row.identity.id).execute(&mut *tx).await.unwrap();
    sqlx::query("UPDATE standalone_card_versions SET after_json=? WHERE scope='' AND card_id=? AND revision=1").bind(&bytes).bind(&row.identity.id).execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
    assert_eq!(
        tokio::time::timeout(
            std::time::Duration::from_secs(5),
            l.cards.version(&row.pin())
        )
        .await
        .unwrap()
        .unwrap(),
        row
    );
}

async fn card_storage_counts(pool: &SqlitePool) -> Vec<i64> {
    // Acquiring a fresh writer also proves cancellation/error cleanup has
    // released the prior transaction, rather than hiding uncommitted rows.
    let mut tx = persistence::begin_write(pool).await.unwrap();
    let mut counts = vec![];
    for table in [
        "card_components",
        "card_component_labels",
        "card_component_fields",
        "standalone_cards",
        "standalone_card_versions",
        "card_field_definitions",
        "card_field_versions",
        "card_field_options",
        "type_versions",
    ] {
        counts.push(
            sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {table}")))
                .fetch_one(&mut *tx)
                .await
                .unwrap(),
        );
    }
    counts.push(
        sqlx::query_scalar(
            "SELECT count(*) FROM card_components WHERE audit_id IS NULL OR snapshot_json IS NULL",
        )
        .fetch_one(&mut *tx)
        .await
        .unwrap(),
    );
    tx.rollback().await.unwrap();
    counts
}

#[tokio::test]
async fn service_failures_roll_back_all_staged_components_children_heads_and_audits() {
    let l = Lab::new().await;
    let definition = l.field(None, "Staged flag", FieldKind::Boolean, None).await;
    let native = Types::new(l.accounts.clone())
        .create_type(&l.admin, fields("Native"), None)
        .await
        .unwrap();
    let identity = CardIdentity {
        namespace: CardNamespace::Type,
        world_id: None,
        id: native.id,
    };
    let before = card_storage_counts(&l.pool).await;
    let audits = l.audits().await;
    l.pool.execute("CREATE TRIGGER reject_final_seal BEFORE UPDATE ON card_components WHEN NEW.audit_id IS NOT NULL BEGIN SELECT RAISE(ABORT,'late seal failure'); END").await.unwrap();
    let proposal = component(&definition, FieldValue::Boolean(false));
    assert!(
        l.cards
            .create(
                &l.admin,
                None,
                NewCard {
                    fields: fields("Failed standalone"),
                    source: None,
                    component: proposal.clone(),
                    reason: None
                }
            )
            .await
            .is_err()
    );
    assert_eq!(card_storage_counts(&l.pool).await, before);
    assert_eq!(l.audits().await, audits);
    assert!(
        l.cards
            .save_card(
                &l.admin,
                &identity,
                CardEdit {
                    expected_revision: 1,
                    component: proposal.clone(),
                    reason: None
                }
            )
            .await
            .is_err()
    );
    assert_eq!(card_storage_counts(&l.pool).await, before);
    assert_eq!(l.audits().await, audits);
    assert_eq!(l.cards.detail(&identity).await.unwrap().revision, 1);
    l.pool
        .execute("DROP TRIGGER reject_final_seal")
        .await
        .unwrap();
    let saved = l
        .cards
        .save_card(
            &l.admin,
            &identity,
            CardEdit {
                expected_revision: 1,
                component: proposal,
                reason: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(saved.revision, 2);
    let before = card_storage_counts(&l.pool).await;
    let audits = l.audits().await;
    l.pool.execute("CREATE TRIGGER reject_field_version BEFORE INSERT ON card_field_versions BEGIN SELECT RAISE(ABORT,'late field failure'); END").await.unwrap();
    assert!(
        l.cards
            .create_field(
                &l.admin,
                None,
                NewField {
                    fields: fields("Failed choice"),
                    kind: FieldKind::Choice {
                        options: vec![ChoiceOption {
                            id: "a".repeat(32),
                            label: "Staged option".into()
                        }]
                    },
                    source: None,
                    reason: None
                }
            )
            .await
            .is_err()
    );
    assert_eq!(card_storage_counts(&l.pool).await, before);
    assert_eq!(l.audits().await, audits);
    assert_eq!(*before.last().unwrap(), 0);
}
