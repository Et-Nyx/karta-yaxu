use game_data::{
    authoring::ContentFields, card_fields::*, cards::*, relations::*, stats::NumericConstraints,
};
use sqlx::{Executor, SqlitePool};
use tempfile::TempDir;
use web_app::{
    accounts::Accounts,
    application::{
        authoring::{Authoring, Edit, Error},
        cards::{CardEdit, CardMetadataEdit, Cards, NewCard, NewField},
        relations::*,
        stats::{NewStat, Stats},
        types::Types,
    },
    auth::{Role, Subject},
    persistence,
};

fn fields(name: &str) -> ContentFields {
    ContentFields {
        slug: "ignored".into(),
        display_name: name.into(),
        description: "Inert record example.".into(),
    }
}
fn id(n: usize) -> String {
    format!("{n:032x}")
}
async fn user(a: &Accounts, name: &str, role: Role) -> String {
    a.invite("test", name, role, None).await.unwrap();
    let u = a
        .sign_in(Subject {
            provider: "test".into(),
            provider_subject: name.into(),
            display_name: name.into(),
        })
        .await
        .unwrap();
    a.start_session(&u.user_id).await.unwrap().0
}
struct Lab {
    _dir: TempDir,
    pool: SqlitePool,
    accounts: Accounts,
    r: Relations,
    cards: Cards,
    admin: String,
    owner: String,
    outsider: String,
    world: String,
}
impl Lab {
    async fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let pool = persistence::connect_for_migration(&format!(
            "sqlite://{}/relations.db",
            dir.path().display()
        ))
        .await
        .unwrap();
        persistence::migrate(&pool).await.unwrap();
        let accounts = Accounts::new(pool.clone(), 3600).unwrap();
        let admin = user(&accounts, "admin", Role::Administrator).await;
        let owner = user(&accounts, "owner", Role::Editor).await;
        let outsider = user(&accounts, "outsider", Role::Editor).await;
        let world = Authoring::new(accounts.clone())
            .create_world(&owner, fields("World"), None)
            .await
            .unwrap()
            .id;
        Self {
            _dir: dir,
            pool,
            r: Relations::new(accounts.clone()),
            cards: Cards::new(accounts.clone()),
            accounts,
            admin,
            owner,
            outsider,
            world,
        }
    }
    fn token(&self, w: Option<&str>) -> &str {
        if w.is_some() {
            &self.owner
        } else {
            &self.admin
        }
    }
    async fn audits(&self) -> i64 {
        sqlx::query_scalar("SELECT count(*) FROM audit_events")
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }
    async fn card(&self, w: Option<&str>, name: &str, source: Option<CardPin>) -> CardRecord {
        self.cards
            .create(
                self.token(w),
                w,
                NewCard {
                    fields: fields(name),
                    source,
                    component: CardComponent::default(),
                    reason: None,
                },
            )
            .await
            .unwrap()
    }
    async fn field(&self, w: Option<&str>, kind: FieldKind) -> FieldDefinition {
        self.cards
            .create_field(
                self.token(w),
                w,
                NewField {
                    fields: fields("Field"),
                    kind,
                    source: None,
                    reason: None,
                },
            )
            .await
            .unwrap()
    }
    async fn layout(
        &self,
        w: Option<&str>,
        columns: Vec<RecordColumn>,
        source: Option<LayoutPin>,
    ) -> LayoutRecord {
        self.r
            .create_layout(
                self.token(w),
                w,
                NewLayout {
                    fields: fields("Layout"),
                    columns,
                    source,
                    reason: None,
                },
            )
            .await
            .unwrap()
    }
    async fn collection(
        &self,
        layout: &LayoutRecord,
        source: Option<CollectionPin>,
    ) -> CollectionDefinition {
        self.r
            .create_collection(
                self.token(layout.world_id.as_deref()),
                layout.world_id.as_deref(),
                NewCollection {
                    fields: fields("Collection"),
                    layout: layout.pin(),
                    source,
                    reason: None,
                },
            )
            .await
            .unwrap()
    }
    async fn save(&self, p: &CardPin, relations: CardRelations) -> CardRelationRecord {
        self.r
            .save_card(
                self.token(p.identity.world_id.as_deref()),
                &p.identity,
                RelationEdit {
                    expected_revision: p.version,
                    relations,
                    reason: Some("Records edit".into()),
                },
            )
            .await
            .unwrap()
    }
}
fn col(n: usize, kind: ColumnKind) -> RecordColumn {
    RecordColumn {
        id: id(n),
        label: format!("Column {n}"),
        kind,
    }
}
fn refs(n: usize) -> RecordColumn {
    col(
        n,
        ColumnKind::Reference {
            allowed: vec![
                ReferenceKind::Card,
                ReferenceKind::Stat,
                ReferenceKind::Field,
            ],
        },
    )
}
fn values(d: &CollectionDefinition, entries: Vec<RecordEntry>) -> CardRelations {
    CardRelations {
        collections: Some(vec![RecordCollection {
            definition: d.clone(),
            value: CollectionValue::Entries(entries),
        }]),
    }
}
fn entry(n: usize, cells: Vec<RecordCell>) -> RecordEntry {
    RecordEntry { id: id(n), cells }
}
fn cell(n: usize, value: RecordValue) -> RecordCell {
    RecordCell {
        column_id: id(n),
        value,
    }
}
fn layout_edit(d: &LayoutRecord) -> LayoutEdit {
    LayoutEdit {
        expected_revision: d.revision,
        fields: d.fields.clone(),
        columns: d.columns.clone(),
        archived_at: d.archived_at,
        reason: None,
    }
}
fn collection_edit(d: &CollectionDefinition) -> CollectionEdit {
    CollectionEdit {
        expected_revision: d.revision,
        fields: d.fields.clone(),
        layout: d.layout.pin(),
        archived_at: d.archived_at,
        reason: None,
    }
}

#[tokio::test]
async fn states_absence_empty_false_zero_repeated_fields_and_exact_history() {
    let l = Lab::new().await;
    let f = l.field(None, FieldKind::Boolean).await;
    let layout = l
        .layout(
            None,
            vec![
                col(1, ColumnKind::Field(f.clone())),
                col(2, ColumnKind::Field(f)),
            ],
            None,
        )
        .await;
    let d = l.collection(&layout, None).await;
    let c = l.card(None, "Card", None).await;
    let unmodeled = l.r.card(&c.pin()).await.unwrap();
    assert!(!unmodeled.legacy_unmodeled);
    assert_eq!(unmodeled.relations, CardRelations::default());
    let entries = vec![
        entry(1, vec![]),
        entry(2, vec![cell(1, RecordValue::Missing)]),
        entry(3, vec![cell(1, RecordValue::Unknown)]),
        entry(4, vec![cell(1, RecordValue::NotApplicable)]),
        entry(
            5,
            vec![
                cell(1, RecordValue::Boolean(false)),
                cell(2, RecordValue::Boolean(false)),
            ],
        ),
    ];
    let saved = l.save(&c.pin(), values(&d, entries)).await;
    assert_eq!(saved.card.version, 2);
    assert_eq!(l.r.card(&saved.card).await.unwrap(), saved);
    assert_eq!(l.r.card(&c.pin()).await.unwrap(), unmodeled);
    let mut p = saved.card;
    for state in [
        CollectionValue::Missing,
        CollectionValue::Unknown,
        CollectionValue::NotApplicable,
        CollectionValue::Entries(vec![]),
    ] {
        let x = l
            .save(
                &p,
                CardRelations {
                    collections: Some(vec![RecordCollection {
                        definition: d.clone(),
                        value: state,
                    }]),
                },
            )
            .await;
        p = x.card;
    }
    let empty = l
        .save(
            &p,
            CardRelations {
                collections: Some(vec![]),
            },
        )
        .await;
    assert_ne!(empty.relations, CardRelations::default());
    let history = l.r.card_history(&c.identity, None).await.unwrap();
    assert_eq!(history.len(), 7);
    assert_eq!(history[0].actor_name, "admin");
    assert_eq!(history[0].reason.as_deref(), Some("Records edit"));
    let public = serde_json::to_value(history).unwrap().to_string();
    assert!(!public.contains("actor_user_id"));
    assert!(!public.contains("snapshot"));
}

#[tokio::test]
async fn strict_scalar_constraints_and_structural_validation() {
    let l = Lab::new().await;
    let number = l
        .field(
            None,
            FieldKind::Number(NumericConstraints {
                whole_only: true,
                minimum: Some("0".parse().unwrap()),
                maximum: Some("2".parse().unwrap()),
            }),
        )
        .await;
    let text = l.field(None, FieldKind::Text { max_length: 2000 }).await;
    let choice = l
        .field(
            None,
            FieldKind::Choice {
                options: vec![ChoiceOption {
                    id: id(42),
                    label: "Adjust".into(),
                }],
            },
        )
        .await;
    let layout = l
        .layout(
            None,
            vec![
                col(1, ColumnKind::Field(number)),
                col(2, ColumnKind::Field(text)),
                col(3, ColumnKind::Field(choice)),
            ],
            None,
        )
        .await;
    let d = l.collection(&layout, None).await;
    let c = l.card(None, "Card", None).await;
    let good = values(
        &d,
        vec![entry(
            1,
            vec![
                cell(1, RecordValue::Number("0".parse().unwrap())),
                cell(2, RecordValue::Text("😀".repeat(2000))),
                cell(3, RecordValue::Choice(id(42))),
            ],
        )],
    );
    good.validate(&c.identity).unwrap();
    for (n, value) in [
        (1, RecordValue::Number("0.5".parse().unwrap())),
        (1, RecordValue::Number("3".parse().unwrap())),
        (2, RecordValue::Text("\r\n".into())),
        (2, RecordValue::Text("😀".repeat(2001))),
        (3, RecordValue::Choice("Adjust".into())),
        (1, RecordValue::Boolean(false)),
    ] {
        assert!(
            values(&d, vec![entry(1, vec![cell(n, value)])])
                .validate(&c.identity)
                .is_err()
        );
    }
    for es in [
        vec![entry(1, vec![cell(9, RecordValue::Missing)])],
        vec![entry(1, vec![]), entry(1, vec![])],
        vec![entry(
            1,
            vec![cell(1, RecordValue::Missing), cell(1, RecordValue::Unknown)],
        )],
    ] {
        assert!(values(&d, es).validate(&c.identity).is_err());
    }
    let json = serde_json::to_value(&good).unwrap();
    let mut bad = json.clone();
    bad["extra"] = true.into();
    assert!(serde_json::from_value::<CardRelations>(bad).is_err());
    let mut bad = json;
    bad["collections"][0]["value"]["entries"][0]["cells"][0]["value"]["extra"] = true.into();
    assert!(serde_json::from_value::<CardRelations>(bad).is_err());
    let saved = l.save(&c.pin(), good).await;
    assert_eq!(l.r.card(&saved.card).await.unwrap(), saved);
}

#[tokio::test]
async fn all_three_exact_targets_repeated_occurrences_and_cyclic_identities() {
    let l = Lab::new().await;
    let layout = l.layout(None, vec![refs(1)], None).await;
    let d = l.collection(&layout, None).await;
    let a = l.card(None, "A", None).await;
    let b = l.card(None, "B", None).await;
    let f = l.field(None, FieldKind::Boolean).await;
    let s = Stats::new(l.accounts.clone())
        .create(
            &l.admin,
            None,
            NewStat {
                fields: fields("Influence"),
                constraints: NumericConstraints::default(),
                source: None,
                reason: None,
            },
        )
        .await
        .unwrap();
    let pins = vec![
        DefinitionPin::Card(b.pin()),
        DefinitionPin::Card(b.pin()),
        DefinitionPin::Stat {
            world_id: None,
            stat_id: s.id,
            version: s.revision,
        },
        DefinitionPin::Field(f.pin()),
    ];
    let a = l
        .save(
            &a.pin(),
            values(
                &d,
                pins.into_iter()
                    .enumerate()
                    .map(|(n, p)| entry(n + 1, vec![cell(1, RecordValue::Reference(p))]))
                    .collect(),
            ),
        )
        .await;
    let b = l
        .save(
            &b.pin(),
            values(
                &d,
                vec![entry(
                    1,
                    vec![cell(
                        1,
                        RecordValue::Reference(DefinitionPin::Card(a.card.clone())),
                    )],
                )],
            ),
        )
        .await;
    assert_eq!(l.r.card(&a.card).await.unwrap(), a);
    assert_eq!(l.r.card(&b.card).await.unwrap(), b);
    assert_eq!(
        l.cards.version(&b.card).await.unwrap().revision,
        b.card.version
    );
}

#[tokio::test]
async fn permission_expected_first_revocation_and_noop() {
    let l = Lab::new().await;
    let c = l.card(Some(&l.world), "Card", None).await;
    let before = l.audits().await;
    let old = l.r.card(&c.pin()).await.unwrap();
    let same = l.save(&c.pin(), CardRelations::default()).await;
    assert_eq!(same, old);
    assert_eq!(l.audits().await, before);
    let bad = CardRelations {
        collections: Some(vec![RecordCollection {
            definition: CollectionDefinition {
                id: "bad".into(),
                world_id: None,
                source: None,
                fields: fields("Bad"),
                revision: -1,
                archived_at: None,
                layout: LayoutRecord {
                    id: "bad".into(),
                    world_id: None,
                    source: None,
                    fields: fields("Bad"),
                    revision: 0,
                    archived_at: None,
                    columns: vec![],
                },
            },
            value: CollectionValue::Missing,
        }]),
    };
    for token in [&l.admin, &l.outsider] {
        assert_eq!(
            l.r.save_card(
                token,
                &c.identity,
                RelationEdit {
                    expected_revision: 999,
                    relations: bad.clone(),
                    reason: None
                }
            )
            .await,
            Err(Error::Forbidden)
        );
    }
    assert_eq!(
        l.r.save_card(
            &l.owner,
            &c.identity,
            RelationEdit {
                expected_revision: 999,
                relations: bad,
                reason: None
            }
        )
        .await,
        Err(Error::Conflict)
    );
    let a = l.accounts.authenticate(&l.outsider).await.unwrap().unwrap();
    let author = Authoring::new(l.accounts.clone());
    let access = author.access(&l.owner, &l.world).await.unwrap();
    author
        .grant_collaborator(
            &l.owner,
            &l.world,
            access.world.access_revision,
            &a.user_id,
            None,
        )
        .await
        .unwrap();
    let saved =
        l.r.save_card(
            &l.outsider,
            &c.identity,
            RelationEdit {
                expected_revision: 1,
                relations: CardRelations {
                    collections: Some(vec![]),
                },
                reason: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(
        l.r.card_history(&c.identity, None).await.unwrap()[0].actor_name,
        "outsider"
    );
    l.accounts.set_disabled(&a.user_id, true).await.unwrap();
    assert_eq!(
        l.r.save_card(
            &l.outsider,
            &c.identity,
            RelationEdit {
                expected_revision: saved.card.version,
                relations: CardRelations::default(),
                reason: None
            }
        )
        .await,
        Err(Error::Unauthorized)
    );
}

#[tokio::test]
async fn definition_edits_are_independent_and_archived_exact_pins_survive() {
    let l = Lab::new().await;
    let layout = l.layout(None, vec![refs(1)], None).await;
    let d = l.collection(&layout, None).await;
    let c = l.card(None, "Consumer", None).await;
    let target = l.card(None, "Target", None).await;
    let x = values(
        &d,
        vec![entry(
            1,
            vec![cell(
                1,
                RecordValue::Reference(DefinitionPin::Card(target.pin())),
            )],
        )],
    );
    let saved = l.save(&c.pin(), x.clone()).await;
    let mut edit = layout_edit(&layout);
    edit.columns.clear();
    edit.archived_at = Some(1);
    let changed =
        l.r.save_layout(&l.admin, None, &layout.id, edit)
            .await
            .unwrap();
    let mut edit = collection_edit(&d);
    edit.archived_at = Some(1);
    l.r.save_collection(&l.admin, None, &d.id, edit)
        .await
        .unwrap();
    l.cards
        .save_metadata(
            &l.admin,
            &target.identity,
            CardMetadataEdit {
                expected_revision: target.revision,
                fields: target.fields.clone(),
                archived_at: Some(1),
                reason: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(l.r.card(&saved.card).await.unwrap(), saved);
    assert_eq!(l.r.layout_version(&layout.pin()).await.unwrap(), layout);
    assert!(changed.columns.is_empty());
    let metadata = l
        .cards
        .save_metadata(
            &l.admin,
            &c.identity,
            CardMetadataEdit {
                expected_revision: saved.card.version,
                fields: fields("Rename"),
                archived_at: None,
                reason: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(l.r.card(&metadata.pin()).await.unwrap().relations, x);
    let other = l.card(None, "New consumer", None).await;
    assert!(matches!(
        l.r.save_card(
            &l.admin,
            &other.identity,
            RelationEdit {
                expected_revision: 1,
                relations: x,
                reason: None
            }
        )
        .await,
        Err(Error::Invalid(_))
    ));
}

#[tokio::test]
async fn new_archived_reference_occurrences_are_rejected_but_retained_cells_survive() {
    let l = Lab::new().await;
    let layout = l.layout(None, vec![refs(1)], None).await;
    let d = l.collection(&layout, None).await;
    let c = l.card(None, "Consumer", None).await;
    let target = l.card(None, "Target", None).await;
    let e = entry(
        1,
        vec![cell(
            1,
            RecordValue::Reference(DefinitionPin::Card(target.pin())),
        )],
    );
    let saved = l.save(&c.pin(), values(&d, vec![e.clone()])).await;
    l.cards
        .save_metadata(
            &l.admin,
            &target.identity,
            CardMetadataEdit {
                expected_revision: 1,
                fields: target.fields,
                archived_at: Some(1),
                reason: None,
            },
        )
        .await
        .unwrap();
    let mut repeat = e.clone();
    repeat.id = id(2);
    assert!(matches!(
        l.r.save_card(
            &l.admin,
            &c.identity,
            RelationEdit {
                expected_revision: saved.card.version,
                relations: values(&d, vec![e.clone(), repeat]),
                reason: None
            }
        )
        .await,
        Err(Error::Invalid(_))
    ));
    let updated = l
        .save(&saved.card, values(&d, vec![e, entry(3, vec![])]))
        .await;
    assert_eq!(updated.card.version, 3);
}

#[tokio::test]
async fn scalar_metadata_and_native_type_writers_carry_records_one_audit() {
    let l = Lab::new().await;
    let layout = l.layout(None, vec![], None).await;
    let d = l.collection(&layout, None).await;
    let t = Types::new(l.accounts.clone())
        .create_type(&l.admin, fields("Type"), None)
        .await
        .unwrap();
    let p = CardPin {
        identity: CardIdentity {
            namespace: CardNamespace::Type,
            world_id: None,
            id: t.id.clone(),
        },
        version: 1,
    };
    let x = values(&d, vec![entry(1, vec![])]);
    let saved = l.save(&p, x.clone()).await;
    let before = l.audits().await;
    let c = l
        .cards
        .save_card(
            &l.admin,
            &p.identity,
            CardEdit {
                expected_revision: saved.card.version,
                component: CardComponent {
                    rules_text: Some("Scalar prose".into()),
                    ..Default::default()
                },
                reason: None,
            },
        )
        .await
        .unwrap();
    let t = Types::new(l.accounts.clone())
        .save_type(
            &l.admin,
            &t.id,
            Edit {
                expected_revision: c.revision,
                fields: fields("Renamed type"),
                reason: None,
            },
        )
        .await
        .unwrap();
    let p = CardPin {
        version: t.revision,
        ..p
    };
    assert_eq!(l.r.card(&p).await.unwrap().relations, x);
    assert_eq!(l.audits().await, before + 2);
    let same:bool=sqlx::query_scalar("SELECT r.audit_id=c.audit_id AND r.audit_id=t.audit_id FROM card_relation_versions r JOIN card_components c USING(namespace,scope,card_id,revision) JOIN type_versions t ON t.type_id=r.card_id AND t.revision=r.revision WHERE r.namespace='type' AND r.card_id=? AND r.revision=?").bind(&t.id).bind(t.revision).fetch_one(&l.pool).await.unwrap();
    assert!(same);
}

#[tokio::test]
async fn late_relation_seal_failure_rolls_back_native_scalar_heads_and_audit() {
    let l = Lab::new().await;
    let c = l.card(None, "Card", None).await;
    let before = l.audits().await;
    l.pool.execute("CREATE TRIGGER fail_relation_seal BEFORE UPDATE ON card_relation_versions WHEN NEW.audit_id IS NOT NULL BEGIN SELECT RAISE(ABORT,'late failure'); END").await.unwrap();
    assert!(
        l.r.save_card(
            &l.admin,
            &c.identity,
            RelationEdit {
                expected_revision: 1,
                relations: CardRelations {
                    collections: Some(vec![])
                },
                reason: None
            }
        )
        .await
        .is_err()
    );
    assert_eq!(l.audits().await, before);
    assert_eq!(l.cards.detail(&c.identity).await.unwrap(), c);
    assert_eq!(
        l.r.card(&c.pin()).await.unwrap().relations,
        CardRelations::default()
    );
    let staged: i64 =
        sqlx::query_scalar("SELECT count(*) FROM card_relation_versions WHERE revision>1")
            .fetch_one(&l.pool)
            .await
            .unwrap();
    assert_eq!(staged, 0);
}

#[tokio::test]
async fn reconciliation_requires_injective_columns_and_binds_complete_review() {
    let l = Lab::new().await;
    let f = l.field(None, FieldKind::Boolean).await;
    let layout = l
        .layout(
            None,
            vec![
                col(1, ColumnKind::Field(f.clone())),
                col(2, ColumnKind::Field(f.clone())),
            ],
            None,
        )
        .await;
    let d = l.collection(&layout, None).await;
    let c = l.card(None, "Card", None).await;
    let saved = l
        .save(
            &c.pin(),
            values(
                &d,
                vec![
                    entry(
                        1,
                        vec![
                            cell(1, RecordValue::Boolean(false)),
                            cell(2, RecordValue::Unknown),
                        ],
                    ),
                    entry(2, vec![]),
                ],
            ),
        )
        .await;
    let mut edit = layout_edit(&layout);
    edit.columns = vec![col(3, ColumnKind::Field(f))];
    let layout2 =
        l.r.save_layout(&l.admin, None, &layout.id, edit)
            .await
            .unwrap();
    let mut edit = collection_edit(&d);
    edit.layout = layout2.pin();
    let d2 =
        l.r.save_collection(&l.admin, None, &d.id, edit)
            .await
            .unwrap();
    let mapping = ColumnMapping {
        source_column_id: id(1),
        local_column_id: id(3),
    };
    assert!(
        l.r.preview_reconcile_card(
            &c.identity,
            saved.card.version,
            &d.id,
            &d2.pin(),
            &[mapping.clone(), mapping.clone()]
        )
        .await
        .is_err()
    );
    let preview =
        l.r.preview_reconcile_card(
            &c.identity,
            saved.card.version,
            &d.id,
            &d2.pin(),
            &[mapping],
        )
        .await
        .unwrap();
    assert_eq!(preview.unmapped_cells.len(), 1);
    assert_eq!(preview.removed_cells.len(), 1);
    let mut forged = preview.clone();
    forged.removed.clear();
    assert_eq!(
        l.r.confirm_card(&l.admin, &forged, None).await,
        Err(Error::Conflict)
    );
    let result = l.r.confirm_card(&l.admin, &preview, None).await.unwrap();
    assert_eq!(result, preview.proposed);
    assert_eq!(
        l.r.confirm_card(&l.admin, &preview, None).await,
        Err(Error::Conflict)
    );
    let CollectionValue::Entries(es) = &result.relations.collections.as_ref().unwrap()[0].value
    else {
        panic!()
    };
    assert_eq!(es[0].id, id(1));
    assert_eq!(es[1].id, id(2));
    assert!(es[1].cells.is_empty());
    assert_eq!(es[0].cells, vec![cell(3, RecordValue::Boolean(false))]);
}

#[tokio::test]
async fn source_copy_and_keep_local_adoption_require_both_source_identities() {
    let l = Lab::new().await;
    let shared = l.layout(None, vec![refs(1)], None).await;
    let sd = l.collection(&shared, None).await;
    let source = l.card(None, "Source", None).await;
    let target = l.card(None, "Target", None).await;
    let source = l
        .save(
            &source.pin(),
            values(
                &sd,
                vec![entry(
                    1,
                    vec![cell(
                        1,
                        RecordValue::Reference(DefinitionPin::Card(target.pin())),
                    )],
                )],
            ),
        )
        .await;
    let local = l
        .layout(Some(&l.world), vec![refs(2)], Some(shared.pin()))
        .await;
    let ld = l.collection(&local, Some(sd.pin())).await;
    let c = l
        .card(Some(&l.world), "Local", Some(source.card.clone()))
        .await;
    let mappings = vec![CollectionMapping {
        shared_collection_id: sd.id.clone(),
        local_collection: ld.pin(),
        columns: vec![ColumnMapping {
            source_column_id: id(1),
            local_column_id: id(2),
        }],
    }];
    let p =
        l.r.preview_source_collections(&c.identity, 1, &mappings)
            .await
            .unwrap();
    assert!(p.unmapped.is_empty());
    let copied = l.r.confirm_card(&l.owner, &p, None).await.unwrap();
    assert_eq!(copied, p.proposed);
    let mut edit = layout_edit(&shared);
    edit.fields.description = "Shared changed".into();
    let shared2 =
        l.r.save_layout(&l.admin, None, &shared.id, edit)
            .await
            .unwrap();
    let adopted =
        l.r.adopt_layout(
            &l.owner,
            &l.world,
            &local.id,
            shared2.revision,
            layout_edit(&local),
        )
        .await
        .unwrap();
    assert_eq!(adopted.columns, local.columns);
    assert_eq!(adopted.fields, local.fields);
    assert_eq!(adopted.source, Some(shared2.pin()));
    let mut edit = collection_edit(&sd);
    edit.fields.description = "Shared collection changed".into();
    let sd2 =
        l.r.save_collection(&l.admin, None, &sd.id, edit)
            .await
            .unwrap();
    let adopted =
        l.r.adopt_collection(
            &l.owner,
            &l.world,
            &ld.id,
            sd2.revision,
            collection_edit(&ld),
        )
        .await
        .unwrap();
    assert_eq!(adopted.layout, ld.layout);
    assert_eq!(adopted.fields, ld.fields);
    assert_eq!(adopted.source, Some(sd2.pin()));
    assert_eq!(l.r.card(&copied.card).await.unwrap(), copied);
    let wrong = l.layout(Some(&l.world), vec![refs(2)], None).await;
    let wrong = l.collection(&wrong, Some(sd.pin())).await;
    let bad = vec![CollectionMapping {
        local_collection: wrong.pin(),
        ..mappings[0].clone()
    }];
    assert!(
        l.r.preview_source_collections(&c.identity, copied.card.version, &bad)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn missing_and_forged_record_seals_fail_scalar_reads_and_reference_validation() {
    let l = Lab::new().await;
    let c = l.card(None, "Card", None).await;
    l.pool
        .execute("DROP TRIGGER card_relation_versions_update")
        .await
        .unwrap();
    sqlx::query("UPDATE card_relation_versions SET collections_defined=1 WHERE card_id=?")
        .bind(&c.identity.id)
        .execute(&l.pool)
        .await
        .unwrap();
    assert_eq!(l.cards.detail(&c.identity).await, Err(Error::Unavailable));
    assert_eq!(l.r.card(&c.pin()).await, Err(Error::Unavailable));
    let target = l.card(None, "Target", None).await;
    l.pool
        .execute("DROP TRIGGER card_relation_versions_delete")
        .await
        .unwrap();
    sqlx::query("DELETE FROM card_relation_versions WHERE card_id=?")
        .bind(&target.identity.id)
        .execute(&l.pool)
        .await
        .unwrap();
    assert_eq!(
        l.cards.version(&target.pin()).await,
        Err(Error::Unavailable)
    );
    let layout = l.layout(None, vec![refs(1)], None).await;
    let d = l.collection(&layout, None).await;
    let a = l.card(None, "A", None).await;
    let before = l.audits().await;
    assert!(
        l.r.save_card(
            &l.admin,
            &a.identity,
            RelationEdit {
                expected_revision: 1,
                relations: values(
                    &d,
                    vec![entry(
                        1,
                        vec![cell(
                            1,
                            RecordValue::Reference(DefinitionPin::Card(target.pin()))
                        )]
                    )]
                ),
                reason: None
            }
        )
        .await
        .is_err()
    );
    assert_eq!(l.audits().await, before);
}

#[tokio::test]
async fn forged_typed_cells_layout_children_and_predecessor_snapshots_fail_closed() {
    let l = Lab::new().await;
    let f = l.field(None, FieldKind::Boolean).await;
    let layout = l
        .layout(None, vec![col(1, ColumnKind::Field(f))], None)
        .await;
    let d = l.collection(&layout, None).await;
    let c = l.card(None, "Card", None).await;
    let saved = l
        .save(
            &c.pin(),
            values(
                &d,
                vec![entry(1, vec![cell(1, RecordValue::Boolean(false))])],
            ),
        )
        .await;
    l.pool
        .execute("DROP TRIGGER card_relation_cells_update")
        .await
        .unwrap();
    sqlx::query("UPDATE card_relation_cells SET boolean_value=1 WHERE card_id=?")
        .bind(&c.identity.id)
        .execute(&l.pool)
        .await
        .unwrap();
    assert_eq!(l.r.card(&saved.card).await, Err(Error::Unavailable));
    assert_eq!(l.cards.version(&saved.card).await, Err(Error::Unavailable));
    l.pool
        .execute("DROP TRIGGER record_layout_columns_update")
        .await
        .unwrap();
    sqlx::query("UPDATE record_layout_columns SET label='Forged' WHERE layout_id=?")
        .bind(&layout.id)
        .execute(&l.pool)
        .await
        .unwrap();
    assert_eq!(
        l.r.layout_version(&layout.pin()).await,
        Err(Error::Unavailable)
    );
    assert_eq!(
        l.r.collection_version(&d.pin()).await,
        Err(Error::Unavailable)
    );
    let layout = l.layout(None, vec![], None).await;
    let mut edit = layout_edit(&layout);
    edit.fields.description = "Next".into();
    let next =
        l.r.save_layout(&l.admin, None, &layout.id, edit)
            .await
            .unwrap();
    l.pool
        .execute("DROP TRIGGER record_layout_versions_update")
        .await
        .unwrap();
    sqlx::query("UPDATE record_layout_versions SET before_json='{}' WHERE id=? AND revision=2")
        .bind(&next.id)
        .execute(&l.pool)
        .await
        .unwrap();
    assert_eq!(
        l.r.layout_version(&next.pin()).await,
        Err(Error::Unavailable)
    );
}

#[tokio::test]
async fn literal_catalog_search_pages_and_public_attributed_history_are_bounded() {
    let l = Lab::new().await;
    let mut layout = l.layout(None, vec![], None).await;
    for n in 2..=28 {
        let mut edit = layout_edit(&layout);
        edit.fields.description = format!("Revision {n}");
        edit.reason = Some(format!("Reason {n}"));
        layout =
            l.r.save_layout(&l.admin, None, &layout.id, edit)
                .await
                .unwrap();
    }
    let first = l.r.layout_history(None, &layout.id, None).await.unwrap();
    assert_eq!(first.len(), 25);
    assert_eq!(first[0].revision, 28);
    assert_eq!(first[0].reason.as_deref(), Some("Reason 28"));
    let rest = l.r.layout_history(None, &layout.id, Some(4)).await.unwrap();
    assert_eq!(rest.len(), 3);
    assert_eq!(rest[2].action, "create");
    for n in 0..27 {
        l.r.create_layout(
            &l.admin,
            None,
            NewLayout {
                fields: fields(&format!("Literal % {n:02}")),
                source: None,
                columns: vec![],
                reason: None,
            },
        )
        .await
        .unwrap();
    }
    assert_eq!(l.r.layouts(None, "%", 1).await.unwrap().len(), 25);
    assert_eq!(l.r.layouts(None, "%", 2).await.unwrap().len(), 2);
    assert!(l.r.layouts(None, "_", 1).await.unwrap().is_empty());
    assert!(persistence::schema_ready(&l.pool).await.unwrap());
}

// Upgrade evidence compares every old typed column and snapshot byte, including
// scalar components/standalone cards which did not exist before migration 0009.
async fn table_values(pool: &SqlitePool) -> Vec<(String, Vec<String>)> {
    let tables:Vec<String>=sqlx::query_scalar("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name NOT LIKE '_sqlx_%' ORDER BY name").fetch_all(pool).await.unwrap();
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

#[tokio::test]
async fn populated_0010_upgrade_preserves_old_bytes_and_marks_standalone_and_native_versions() {
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
        include_str!("../migrations/0009_cards.sql"),
    ] {
        pool.execute(sqlx::raw_sql(migration)).await.unwrap();
    }
    let accounts = Accounts::new(pool.clone(), 3600).unwrap();
    let admin = user(&accounts, "admin", Role::Administrator).await;
    let actor = accounts.authenticate(&admin).await.unwrap().unwrap();
    let mut f = fields("Legacy");
    f.slug = "legacy".into();
    let old = CardRecord {
        identity: CardIdentity {
            namespace: CardNamespace::Standalone,
            world_id: None,
            id: id(100),
        },
        revision: 1,
        fields: f.clone(),
        archived_at: None,
        source: None,
        component: CardComponent {
            rules_text: Some("Preserve exact scalar bytes".into()),
            ..Default::default()
        },
        legacy_unmodeled: false,
    };
    let snapshot = serde_json::json!({"snapshot_version":1,"content":old}).to_string();
    let mut tx = persistence::begin_write(&pool).await.unwrap();
    for audit in ["old-card", "old-type"] {
        sqlx::query("INSERT INTO audit_events(id,actor_user_id,action) VALUES (?,?,'create')")
            .bind(audit)
            .bind(&actor.user_id)
            .execute(&mut *tx)
            .await
            .unwrap();
    }
    sqlx::query("INSERT INTO standalone_cards(scope,id,slug,revision) VALUES ('',?,'legacy',1)")
        .bind(&old.identity.id)
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("INSERT INTO standalone_card_versions(scope,card_id,revision,slug,display_name,description,audit_id,after_json) VALUES ('',?,1,?,?,?,'old-card',?)").bind(&old.identity.id).bind(&f.slug).bind(&f.display_name).bind(&f.description).bind(&snapshot).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO card_components(namespace,scope,card_id,revision,fields_defined,rules_text,labels_defined,audit_id,snapshot_json) VALUES ('standalone','',?,1,0,?,0,'old-card',?)").bind(&old.identity.id).bind(&old.component.rules_text).bind(&snapshot).execute(&mut *tx).await.unwrap();
    let native = web_app::application::types::TypeRecord {
        id: id(101),
        revision: 1,
        fields: f.clone(),
        archived_at: None,
    };
    let native_snapshot = serde_json::json!({"snapshot_version":1,"content":native}).to_string();
    sqlx::query("INSERT INTO types(id,slug,display_name,description,revision) VALUES (?,?,?,?,1)")
        .bind(&native.id)
        .bind(&f.slug)
        .bind(&f.display_name)
        .bind(&f.description)
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("INSERT INTO type_versions(type_id,revision,slug,display_name,description,audit_id,after_json) VALUES (?,1,?,?,?,'old-type',?)").bind(&native.id).bind(&f.slug).bind(&f.display_name).bind(&f.description).bind(native_snapshot).execute(&mut *tx).await.unwrap();
    let card = CardRecord {
        identity: CardIdentity {
            namespace: CardNamespace::Type,
            world_id: None,
            id: native.id.clone(),
        },
        revision: 1,
        fields: f,
        archived_at: None,
        source: None,
        component: CardComponent::default(),
        legacy_unmodeled: false,
    };
    sqlx::query("INSERT INTO card_components(namespace,scope,card_id,revision,fields_defined,labels_defined,audit_id,snapshot_json) VALUES ('type','',?,1,0,0,'old-type',?)").bind(&native.id).bind(serde_json::json!({"snapshot_version":1,"content":card}).to_string()).execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
    let before = table_values(&pool).await;
    pool.execute(sqlx::raw_sql(include_str!(
        "../migrations/0010_records.sql"
    )))
    .await
    .unwrap();
    let after = table_values(&pool).await;
    for (table, rows) in before {
        assert_eq!(
            &rows,
            &after.iter().find(|(n, _)| n == &table).unwrap().1,
            "{table}"
        );
    }
    let r = Relations::new(accounts.clone());
    let cards = Cards::new(accounts);
    assert_eq!(cards.version(&old.pin()).await.unwrap(), old);
    assert_eq!(cards.version(&card.pin()).await.unwrap(), card);
    for p in [old.pin(), card.pin()] {
        let legacy = r.card(&p).await.unwrap();
        assert!(legacy.legacy_unmodeled);
        assert_eq!(legacy.relations, CardRelations::default());
        let next = r
            .save_card(
                &admin,
                &p.identity,
                RelationEdit {
                    expected_revision: 1,
                    relations: CardRelations {
                        collections: Some(vec![]),
                    },
                    reason: None,
                },
            )
            .await
            .unwrap();
        assert_eq!(next.card.version, 2);
        assert!(!next.legacy_unmodeled);
        assert_eq!(r.card(&p).await.unwrap(), legacy);
    }
    assert!(
        pool.execute("UPDATE card_relation_legacy_boundaries SET maximum_revision=2")
            .await
            .is_err()
    );
}

#[tokio::test]
async fn reference_allowlists_scope_and_exact_embedded_equality_are_enforced() {
    let l = Lab::new().await;
    let scalar = l.field(None, FieldKind::Boolean).await;
    for allowed in [vec![], vec![ReferenceKind::Card, ReferenceKind::Card]] {
        assert!(matches!(
            l.r.create_layout(
                &l.admin,
                None,
                NewLayout {
                    fields: fields("Bad"),
                    source: None,
                    columns: vec![col(1, ColumnKind::Reference { allowed })],
                    reason: None
                }
            )
            .await,
            Err(Error::Invalid(_))
        ));
    }
    assert!(
        l.r.create_layout(
            &l.owner,
            Some(&l.world),
            NewLayout {
                fields: fields("Wrong scope"),
                source: None,
                columns: vec![col(1, ColumnKind::Field(scalar.clone()))],
                reason: None
            }
        )
        .await
        .is_err()
    );
    let layout = l
        .layout(
            Some(&l.world),
            vec![col(
                1,
                ColumnKind::Reference {
                    allowed: vec![ReferenceKind::Card],
                },
            )],
            None,
        )
        .await;
    let d = l.collection(&layout, None).await;
    let c = l.card(Some(&l.world), "Local", None).await;
    assert!(
        values(
            &d,
            vec![entry(
                1,
                vec![cell(
                    1,
                    RecordValue::Reference(DefinitionPin::Field(scalar.pin()))
                )]
            )]
        )
        .validate(&c.identity)
        .is_err()
    );
    let foreign = Authoring::new(l.accounts.clone())
        .create_world(&l.owner, fields("Foreign"), None)
        .await
        .unwrap();
    let other = l.card(Some(&foreign.id), "Other", None).await;
    assert!(
        values(
            &d,
            vec![entry(
                1,
                vec![cell(
                    1,
                    RecordValue::Reference(DefinitionPin::Card(other.pin()))
                )]
            )]
        )
        .validate(&c.identity)
        .is_err()
    );
    let mut forged = d.clone();
    forged.fields.description = "Forged embedded definition".into();
    let before = l.audits().await;
    assert_eq!(
        l.r.save_card(
            &l.owner,
            &c.identity,
            RelationEdit {
                expected_revision: 1,
                relations: values(&forged, vec![]),
                reason: None
            }
        )
        .await,
        Err(Error::Unavailable)
    );
    assert_eq!(l.audits().await, before);
    // Shared rows cannot point into a local world, even using a structurally valid pin.
    let shared = l.layout(None, vec![refs(1)], None).await;
    let sd = l.collection(&shared, None).await;
    let sc = l.card(None, "Shared", None).await;
    assert!(
        values(
            &sd,
            vec![entry(
                1,
                vec![cell(
                    1,
                    RecordValue::Reference(DefinitionPin::Card(c.pin()))
                )]
            )]
        )
        .validate(&sc.identity)
        .is_err()
    );
}

#[tokio::test]
async fn fixture_baselines_and_native_form_stats_typing_variant_writers_preserve_relations() {
    use web_app::application::{
        card_workshop::CardWorkshop,
        fixtures::{Fixtures, SeedOptions},
        stat_values::{FormStatsEdit, StatValues},
    };
    let l = Lab::new().await;
    let actor = l.accounts.authenticate(&l.owner).await.unwrap().unwrap();
    let before = l.audits().await;
    let fixtures = Fixtures::new(l.accounts.clone());
    fixtures
        .seed(SeedOptions {
            owner_user_id: Some(&actor.user_id),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(l.audits().await - before, 69);
    assert_eq!(
        fixtures
            .seed(SeedOptions {
                owner_user_id: Some(&actor.user_id),
                ..Default::default()
            })
            .await
            .unwrap()
            .fixtures
            .skipped,
        65
    );
    let before = l.audits().await;
    let report = CardWorkshop::new(l.accounts.clone())
        .seed(&actor.user_id)
        .await
        .unwrap();
    assert_eq!(report.created, 15);
    assert_eq!(l.audits().await - before, 16);
    assert_eq!(
        CardWorkshop::new(l.accounts.clone())
            .seed(&actor.user_id)
            .await
            .unwrap()
            .skipped,
        15
    );
    let form: String = sqlx::query_scalar("SELECT id FROM forms ORDER BY id LIMIT 1")
        .fetch_one(&l.pool)
        .await
        .unwrap();
    let identity = CardIdentity {
        namespace: CardNamespace::Form,
        world_id: None,
        id: form.clone(),
    };
    let card = l.cards.detail(&identity).await.unwrap();
    let layout = l.layout(None, vec![], None).await;
    let d = l.collection(&layout, None).await;
    let x = values(&d, vec![entry(1, vec![])]);
    let saved = l.save(&card.pin(), x.clone()).await;
    let author = Authoring::new(l.accounts.clone());
    let species_version: i64 = sqlx::query_scalar("SELECT species_version FROM forms WHERE id=?")
        .bind(&form)
        .fetch_one(&l.pool)
        .await
        .unwrap();
    let native = author
        .save_form(
            &l.admin,
            &form,
            species_version,
            Edit {
                expected_revision: saved.card.version,
                fields: fields("New form name"),
                reason: None,
            },
        )
        .await
        .unwrap();
    let native = Types::new(l.accounts.clone())
        .save_form_typing(&l.admin, &form, native.revision, Some(vec![]), None)
        .await
        .unwrap();
    StatValues::new(l.accounts.clone())
        .save_form(
            &l.admin,
            &form,
            FormStatsEdit {
                expected_revision: native.revision,
                values: Some(vec![]),
                reason: None,
            },
        )
        .await
        .unwrap();
    let card = l.cards.detail(&identity).await.unwrap();
    assert_eq!(l.r.card(&card.pin()).await.unwrap().relations, x);
    let (world, id): (String, String) =
        sqlx::query_as("SELECT world_id,id FROM variants ORDER BY id LIMIT 1")
            .fetch_one(&l.pool)
            .await
            .unwrap();
    let identity = CardIdentity {
        namespace: CardNamespace::Variant,
        world_id: Some(world.clone()),
        id,
    };
    let card = l.cards.detail(&identity).await.unwrap();
    let layout = l.layout(Some(&world), vec![], None).await;
    let d = l.collection(&layout, None).await;
    let x = values(&d, vec![entry(1, vec![])]);
    let saved = l.save(&card.pin(), x.clone()).await;
    let v = author
        .save_variant(
            &l.owner,
            &world,
            &identity.id,
            Edit {
                expected_revision: saved.card.version,
                fields: fields("Local metadata"),
                reason: None,
            },
        )
        .await
        .unwrap();
    let v = Types::new(l.accounts.clone())
        .save_variant_typing(
            &l.owner,
            &world,
            &identity.id,
            v.revision,
            Some(vec![]),
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        l.r.card(&CardPin {
            identity,
            version: v.revision
        })
        .await
        .unwrap()
        .relations,
        x
    );
}

#[tokio::test]
async fn native_move_metadata_and_source_adoption_preserve_relation_companion() {
    use web_app::application::moves::{MoveAdoptEdit, MoveEdit, Moves, NewMove};
    let l = Lab::new().await;
    let moves = Moves::new(l.accounts.clone());
    let shared = moves
        .create(
            &l.admin,
            None,
            NewMove {
                fields: fields("Shared"),
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
                source: Some(shared.pin()),
                attributes: None,
                reason: None,
            },
        )
        .await
        .unwrap();
    let identity = CardIdentity {
        namespace: CardNamespace::Move,
        world_id: Some(l.world.clone()),
        id: local.id.clone(),
    };
    let layout = l.layout(Some(&l.world), vec![], None).await;
    let d = l.collection(&layout, None).await;
    let x = values(&d, vec![entry(1, vec![])]);
    let saved = l
        .save(
            &CardPin {
                identity: identity.clone(),
                version: local.revision,
            },
            x.clone(),
        )
        .await;
    let shared = moves
        .save(
            &l.admin,
            None,
            &shared.id,
            MoveEdit {
                expected_revision: shared.revision,
                fields: fields("Shared two"),
                attributes: None,
                reason: None,
            },
        )
        .await
        .unwrap();
    let local = moves
        .adopt(
            &l.owner,
            &l.world,
            &local.id,
            MoveAdoptEdit {
                expected_revision: saved.card.version,
                target_version: shared.revision,
                fields: local.fields,
                apply_source: false,
                mappings: vec![],
                reason: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(
        l.r.card(&CardPin {
            identity: identity.clone(),
            version: local.revision
        })
        .await
        .unwrap()
        .relations,
        x
    );
    let local = moves
        .save(
            &l.owner,
            Some(&l.world),
            &local.id,
            MoveEdit {
                expected_revision: local.revision,
                fields: fields("Local two"),
                attributes: None,
                reason: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(
        l.r.card(&CardPin {
            identity,
            version: local.revision
        })
        .await
        .unwrap()
        .relations,
        x
    );
}

#[tokio::test]
async fn definition_noops_stale_dependencies_and_late_failures_preserve_heads_and_attribution() {
    let l = Lab::new().await;
    let layout = l.layout(Some(&l.world), vec![], None).await;
    let d = l.collection(&layout, None).await;
    let before = l.audits().await;
    assert_eq!(
        l.r.save_layout(&l.owner, Some(&l.world), &layout.id, layout_edit(&layout))
            .await
            .unwrap(),
        layout
    );
    assert_eq!(
        l.r.save_collection(&l.owner, Some(&l.world), &d.id, collection_edit(&d))
            .await
            .unwrap(),
        d
    );
    assert_eq!(l.audits().await, before);
    let mut stale = layout_edit(&layout);
    stale.expected_revision = 999;
    stale.columns = vec![col(1, ColumnKind::Reference { allowed: vec![] })];
    assert_eq!(
        l.r.save_layout(&l.owner, Some(&l.world), &layout.id, stale)
            .await,
        Err(Error::Conflict)
    );
    let mut stale = collection_edit(&d);
    stale.expected_revision = 999;
    stale.layout.layout_id = "invalid".into();
    assert_eq!(
        l.r.save_collection(&l.owner, Some(&l.world), &d.id, stale)
            .await,
        Err(Error::Conflict)
    );
    l.pool.execute("CREATE TRIGGER fail_collection BEFORE INSERT ON record_collection_versions WHEN NEW.revision>1 BEGIN SELECT RAISE(ABORT,'late failure'); END").await.unwrap();
    let mut edit = collection_edit(&d);
    edit.fields.display_name = "Rename collection".into();
    assert!(
        l.r.save_collection(&l.owner, Some(&l.world), &d.id, edit.clone())
            .await
            .is_err()
    );
    assert_eq!(l.r.collection(Some(&l.world), &d.id).await.unwrap(), d);
    assert_eq!(l.audits().await, before);
    l.pool
        .execute("DROP TRIGGER fail_collection")
        .await
        .unwrap();
    let next =
        l.r.save_collection(&l.owner, Some(&l.world), &d.id, edit)
            .await
            .unwrap();
    assert_ne!(next.fields.slug, d.fields.slug);
    assert_eq!(next.revision, 2);
    let history =
        l.r.collection_history(Some(&l.world), &d.id, None)
            .await
            .unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].actor_name, "owner");
    assert_eq!(history[1].action, "create");
    assert_eq!(l.r.collection_version(&d.pin()).await.unwrap(), d);
    // An ignored scalar seal update must also prevent a native/card write from
    // committing, even when the relation seal itself could have been written.
    let c = l.card(None, "Card", None).await;
    let before = l.audits().await;
    l.pool.execute("CREATE TRIGGER ignore_scalar_seal BEFORE UPDATE ON card_components WHEN NEW.audit_id IS NOT NULL BEGIN SELECT RAISE(IGNORE); END").await.unwrap();
    assert!(
        l.r.save_card(
            &l.admin,
            &c.identity,
            RelationEdit {
                expected_revision: 1,
                relations: CardRelations {
                    collections: Some(vec![])
                },
                reason: None
            }
        )
        .await
        .is_err()
    );
    assert_eq!(l.audits().await, before);
    assert_eq!(l.cards.detail(&c.identity).await.unwrap(), c);
}
