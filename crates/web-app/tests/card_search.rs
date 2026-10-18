use game_data::authoring::ContentFields;
use sqlx::SqlitePool;
use tempfile::TempDir;
use web_app::{
    accounts::Accounts,
    application::{
        authoring::{Authoring, Error},
        cards::*,
        moves::{Moves, NewMove},
        types::Types,
    },
    auth::{Role, Subject},
    persistence,
};

fn fields(name: &str) -> ContentFields {
    ContentFields {
        slug: "ignored".into(),
        display_name: name.into(),
        description: "Local synthetic selector test.".into(),
    }
}
struct Lab {
    _dir: TempDir,
    pool: SqlitePool,
    accounts: Accounts,
    cards: Cards,
    token: String,
    world: String,
    other: String,
}
impl Lab {
    async fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let pool = persistence::connect_for_migration(&format!(
            "sqlite://{}/search.db",
            dir.path().display()
        ))
        .await
        .unwrap();
        persistence::migrate(&pool).await.unwrap();
        let accounts = Accounts::new(pool.clone(), 3600).unwrap();
        accounts
            .invite("test", "admin", Role::Administrator, None)
            .await
            .unwrap();
        let actor = accounts
            .sign_in(Subject {
                provider: "test".into(),
                provider_subject: "admin".into(),
                display_name: "Admin".into(),
            })
            .await
            .unwrap();
        let token = accounts.start_session(&actor.user_id).await.unwrap().0;
        let authoring = Authoring::new(accounts.clone());
        let world = authoring
            .create_world(&token, fields("World"), None)
            .await
            .unwrap()
            .id;
        let other = authoring
            .create_world(&token, fields("Other"), None)
            .await
            .unwrap()
            .id;
        Self {
            _dir: dir,
            cards: Cards::new(accounts.clone()),
            pool,
            accounts,
            token,
            world,
            other,
        }
    }
    async fn card(&self, world: Option<&str>, name: &str, labels: Vec<String>) -> CardRecord {
        self.cards
            .create(
                &self.token,
                world,
                NewCard {
                    fields: fields(name),
                    source: None,
                    component: CardComponent {
                        labels: Some(labels),
                        ..Default::default()
                    },
                    reason: None,
                },
            )
            .await
            .unwrap()
    }
    async fn field(
        &self,
        world: Option<&str>,
        name: &str,
        kind: FieldKind,
        source: Option<FieldPin>,
    ) -> FieldDefinition {
        self.cards
            .create_field(
                &self.token,
                world,
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
    async fn edit_card(
        &self,
        row: &CardRecord,
        name: &str,
        archived_at: Option<i64>,
    ) -> CardRecord {
        self.cards
            .save_metadata(
                &self.token,
                &row.identity,
                CardMetadataEdit {
                    expected_revision: row.revision,
                    fields: fields(name),
                    archived_at,
                    reason: None,
                },
            )
            .await
            .unwrap()
    }
    async fn edit_field(
        &self,
        row: &FieldDefinition,
        name: &str,
        kind: FieldKind,
        archived_at: Option<i64>,
    ) -> FieldDefinition {
        self.cards
            .save_field(
                &self.token,
                row.world_id.as_deref(),
                &row.id,
                FieldEdit {
                    expected_revision: row.revision,
                    fields: fields(name),
                    kind,
                    archived_at,
                    reason: None,
                },
            )
            .await
            .unwrap()
    }
}

#[tokio::test]
async fn card_filters_precede_pagination_and_literal_search_preserves_scope() {
    let l = Lab::new().await;
    for n in 0..28 {
        l.card(None, &format!("A filler {n:02}"), vec![]).await;
    }
    let mut expected = vec![];
    for n in 0..27 {
        expected.push(
            l.card(None, &format!("Z Needle %_ {n:02}"), vec!["Role".into()])
                .await
                .pin(),
        );
    }
    l.card(None, "Z Needle %_ wrong label", vec!["role".into()])
        .await;
    l.card(None, "Z Needle XX wildcard decoy", vec!["Role".into()])
        .await;
    let archived = l
        .card(None, "Z Needle %_ archived", vec!["Role".into()])
        .await;
    l.edit_card(&archived, &archived.fields.display_name, Some(123))
        .await;
    let local = l
        .card(Some(&l.world), "Z Needle %_ local", vec!["Role".into()])
        .await;
    l.card(Some(&l.other), "Z Needle %_ foreign", vec!["Role".into()])
        .await;
    let q = CardSearch {
        query: "needle %_".into(),
        label: "Role".into(),
        namespace: Some(CardNamespace::Standalone),
        archive: ArchiveFilter::Active,
        page: 1,
    };
    let first = l.cards.search_cards(None, &q).await.unwrap();
    assert_eq!(first.items.len(), 25);
    assert!(first.has_more);
    assert_eq!(first.page, 1);
    assert_eq!(
        first
            .items
            .iter()
            .map(|r| r.pin.clone())
            .collect::<Vec<_>>(),
        expected[..25]
    );
    let second = l
        .cards
        .search_cards(
            None,
            &CardSearch {
                page: 2,
                ..q.clone()
            },
        )
        .await
        .unwrap();
    assert!(!second.has_more);
    assert_eq!(
        second
            .items
            .iter()
            .map(|r| r.pin.clone())
            .collect::<Vec<_>>(),
        expected[25..]
    );
    assert!(
        first
            .items
            .iter()
            .all(|r| r.labels == ["Role"] && r.label_count == 1)
    );
    let archived = l
        .cards
        .search_cards(
            None,
            &CardSearch {
                archive: ArchiveFilter::Archived,
                ..q.clone()
            },
        )
        .await
        .unwrap();
    assert_eq!(archived.items.len(), 1);
    assert_eq!(archived.items[0].archived_at, Some(123));
    assert_eq!(archived.items[0].current_archived_at, Some(123));
    let local_page = l.cards.search_cards(Some(&l.world), &q).await.unwrap();
    assert_eq!(local_page.items[0].pin, local.pin());
    assert_eq!(local_page.items.len(), 1);
    assert!(
        l.cards
            .search_cards(
                None,
                &CardSearch {
                    query: "' OR 1=1 --".into(),
                    ..Default::default()
                }
            )
            .await
            .unwrap()
            .items
            .is_empty()
    );
    let unicode = l.card(None, "Étoile", vec![]).await;
    assert_eq!(
        l.cards
            .search_cards(
                None,
                &CardSearch {
                    query: "ÉTOILE".into(),
                    ..Default::default()
                }
            )
            .await
            .unwrap()
            .items[0]
            .pin,
        unicode.pin()
    );
    assert!(
        l.cards
            .search_cards(
                None,
                &CardSearch {
                    query: "étoile".into(),
                    ..Default::default()
                }
            )
            .await
            .unwrap()
            .items
            .is_empty()
    );
}

#[tokio::test]
async fn field_kind_archive_and_explicit_same_world_mapping_filters_precede_page() {
    let l = Lab::new().await;
    let source = l.field(None, "Source", FieldKind::Boolean, None).await;
    let updated = l
        .edit_field(&source, "Renamed source", FieldKind::Boolean, None)
        .await;
    let unrelated = l.field(None, "Unrelated", FieldKind::Boolean, None).await;
    for n in 0..28 {
        l.field(
            Some(&l.world),
            &format!("A filler {n:02}"),
            FieldKind::Text { max_length: 100 },
            None,
        )
        .await;
    }
    let mut expected = vec![];
    for n in 0..27 {
        let pin = if n % 2 == 0 {
            source.pin()
        } else {
            updated.pin()
        };
        expected.push(
            l.field(
                Some(&l.world),
                &format!("Z Needle %_ {n:02}"),
                FieldKind::Boolean,
                Some(pin),
            )
            .await
            .pin(),
        );
    }
    l.field(
        Some(&l.world),
        "Z Needle %_ unrelated",
        FieldKind::Boolean,
        Some(unrelated.pin()),
    )
    .await;
    l.field(
        Some(&l.world),
        "Z Needle %_ no source",
        FieldKind::Boolean,
        None,
    )
    .await;
    l.field(
        Some(&l.world),
        "Z Needle %_ wrong kind",
        FieldKind::Text { max_length: 10 },
        Some(source.pin()),
    )
    .await;
    l.field(
        Some(&l.other),
        "Z Needle %_ foreign",
        FieldKind::Boolean,
        Some(source.pin()),
    )
    .await;
    l.field(None, "Z Needle %_ shared", FieldKind::Boolean, None)
        .await;
    let archived = l
        .field(
            Some(&l.world),
            "Z Needle %_ archived",
            FieldKind::Boolean,
            Some(source.pin()),
        )
        .await;
    l.edit_field(
        &archived,
        &archived.fields.display_name,
        FieldKind::Boolean,
        Some(123),
    )
    .await;
    let q = FieldSearch {
        query: "needle %_".into(),
        kind: Some(FieldKindTag::Boolean),
        archive: ArchiveFilter::Active,
        page: 1,
    };
    let first = l
        .cards
        .mapping_fields(&l.world, &source.id, &q)
        .await
        .unwrap();
    assert!(first.has_more);
    assert_eq!(
        first
            .items
            .iter()
            .map(|r| r.pin.clone())
            .collect::<Vec<_>>(),
        expected[..25]
    );
    let second = l
        .cards
        .mapping_fields(
            &l.world,
            &source.id,
            &FieldSearch {
                page: 2,
                ..q.clone()
            },
        )
        .await
        .unwrap();
    assert!(!second.has_more);
    assert_eq!(
        second
            .items
            .iter()
            .map(|r| r.pin.clone())
            .collect::<Vec<_>>(),
        expected[25..]
    );
    assert!(first.items.iter().all(
        |r| r.source.as_ref().unwrap().field_id == source.id && r.kind == FieldKindTag::Boolean
    ));
    let all = l
        .cards
        .search_fields(
            Some(&l.world),
            &FieldSearch {
                page: 2,
                ..q.clone()
            },
        )
        .await
        .unwrap();
    assert_eq!(all.items.len(), 4); // 27 explicit + unrelated + unsourced
    let archived = l
        .cards
        .mapping_fields(
            &l.world,
            &source.id,
            &FieldSearch {
                archive: ArchiveFilter::Archived,
                ..q.clone()
            },
        )
        .await
        .unwrap();
    assert_eq!(archived.items.len(), 1);
    assert_eq!(archived.items[0].archived_at, Some(123));
    let shared = l.cards.search_fields(None, &q).await.unwrap();
    assert_eq!(shared.items.len(), 1);
    assert!(shared.items[0].pin.world_id.is_none());
    assert!(
        l.cards
            .search_fields(
                Some(&l.world),
                &FieldSearch {
                    query: "' OR 1=1 --".into(),
                    ..Default::default()
                }
            )
            .await
            .unwrap()
            .items
            .is_empty()
    );
}

#[tokio::test]
async fn native_catalog_includes_only_cards_in_stable_namespace_name_id_order() {
    let l = Lab::new().await;
    let author = Authoring::new(l.accounts.clone());
    let types = Types::new(l.accounts.clone());
    let moves = Moves::new(l.accounts.clone());
    let species = author
        .create_species(&l.token, fields("Species group"), None)
        .await
        .unwrap();
    let form = author
        .create_form(&l.token, &species.id, 1, fields("A form"), None)
        .await
        .unwrap();
    let shared_type = types
        .create_type(&l.token, fields("A type"), None)
        .await
        .unwrap();
    let local_type = types
        .create_world_type(&l.token, &l.world, 1, None, fields("A local type"), None)
        .await
        .unwrap();
    let variant = author
        .create_variant(&l.token, &l.world, 2, None, fields("A variant"), None)
        .await
        .unwrap();
    let mut shared_move = None;
    for w in [None, Some(l.world.as_str())] {
        let row = moves
            .create(
                &l.token,
                w,
                NewMove {
                    fields: fields("A move"),
                    source: None,
                    attributes: None,
                    reason: None,
                },
            )
            .await
            .unwrap();
        if w.is_none() {
            shared_move = Some(row);
        }
        l.card(w, "A standalone", vec![]).await;
        l.field(w, "A field is not a card", FieldKind::Boolean, None)
            .await;
    }
    let shared = l
        .cards
        .search_cards(None, &CardSearch::default())
        .await
        .unwrap();
    assert_eq!(shared.items.len(), 5); // default + named Form, Move, standalone, Type
    assert_eq!(
        shared
            .items
            .iter()
            .map(|r| r.pin.identity.namespace)
            .collect::<Vec<_>>(),
        [
            CardNamespace::Form,
            CardNamespace::Form,
            CardNamespace::Move,
            CardNamespace::Standalone,
            CardNamespace::Type
        ]
    );
    assert!(shared.items.iter().any(|r| r.pin.identity.id == form.id));
    assert!(
        shared
            .items
            .iter()
            .any(|r| r.pin.identity.id == shared_type.id)
    );
    assert!(!shared.items.iter().any(|r| r.pin.identity.id == species.id));
    let local = l
        .cards
        .search_cards(Some(&l.world), &CardSearch::default())
        .await
        .unwrap();
    assert_eq!(
        local
            .items
            .iter()
            .map(|r| r.pin.identity.namespace)
            .collect::<Vec<_>>(),
        [
            CardNamespace::Move,
            CardNamespace::Standalone,
            CardNamespace::Type,
            CardNamespace::Variant
        ]
    );
    assert!(
        local
            .items
            .iter()
            .any(|r| r.pin.identity.id == local_type.id)
    );
    assert!(local.items.iter().any(|r| r.pin.identity.id == variant.id));
    for (w, ns) in [
        (None, CardNamespace::Variant),
        (Some(l.world.as_str()), CardNamespace::Form),
    ] {
        assert!(
            l.cards
                .search_cards(
                    w,
                    &CardSearch {
                        namespace: Some(ns),
                        ..Default::default()
                    }
                )
                .await
                .unwrap()
                .items
                .is_empty()
        );
    }
    let shared_move = shared_move.unwrap();
    let selected = l
        .cards
        .search_cards(
            None,
            &CardSearch {
                namespace: Some(CardNamespace::Move),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .items
        .remove(0);
    assert_eq!(selected.pin.identity.id, shared_move.id);
    // Native metadata rename produces a new pin; old selected coordinates stay exact.
    moves
        .save(
            &l.token,
            None,
            &shared_move.id,
            web_app::application::moves::MoveEdit {
                expected_revision: 1,
                fields: fields("Renamed move"),
                attributes: None,
                reason: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(
        l.cards
            .version(&selected.pin)
            .await
            .unwrap()
            .fields
            .display_name,
        "A move"
    );
    let renamed = l
        .cards
        .search_cards(
            None,
            &CardSearch {
                query: "Renamed move".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(renamed.items[0].pin.version, 2);
    // A corrupted current typed name must not be shown from an unchecked head.
    sqlx::query("UPDATE forms SET display_name='Corrupt head' WHERE id=?")
        .bind(&form.id)
        .execute(&l.pool)
        .await
        .unwrap();
    assert!(matches!(
        l.cards
            .search_cards(
                None,
                &CardSearch {
                    query: "Corrupt head".into(),
                    ..Default::default()
                }
            )
            .await,
        Err(Error::Unavailable)
    ));
}

#[tokio::test]
async fn revision_cursors_keep_historical_names_labels_and_current_archive_eligibility() {
    let l = Lab::new().await;
    let original = l
        .card(
            None,
            "Card 01",
            (0..30).map(|n| format!("Label {n:02}")).collect(),
        )
        .await;
    let field = l.field(None, "Field 01", FieldKind::Boolean, None).await;
    let mut card = original.clone();
    let mut definition = field.clone();
    for n in 2..=27 {
        let archive = (n == 27).then_some(123);
        card = l.edit_card(&card, &format!("Card {n:02}"), archive).await;
        definition = l
            .edit_field(
                &definition,
                &format!("Field {n:02}"),
                FieldKind::Boolean,
                archive,
            )
            .await;
    }
    let first = l
        .cards
        .card_revisions(&original.identity, None)
        .await
        .unwrap();
    assert_eq!(first.items.len(), 25);
    assert!(first.has_more);
    assert_eq!(first.items[0].fields.display_name, "Card 27");
    assert_eq!(first.items[1].fields.display_name, "Card 26");
    assert_eq!(first.items[1].archived_at, None);
    assert_eq!(first.items[1].current_archived_at, Some(123));
    assert_eq!(first.items[1].labels.len(), 25);
    assert_eq!(first.items[1].label_count, 30);
    let second = l
        .cards
        .card_revisions(
            &original.identity,
            Some(first.items.last().unwrap().pin.version),
        )
        .await
        .unwrap();
    assert!(!second.has_more);
    assert_eq!(
        second
            .items
            .iter()
            .map(|r| r.pin.version)
            .collect::<Vec<_>>(),
        [2, 1]
    );
    assert_eq!(second.items[1].fields, original.fields);
    assert_eq!(second.items[1].pin, original.pin());
    assert!(
        l.cards
            .card_revisions(&original.identity, Some(1))
            .await
            .unwrap()
            .items
            .is_empty()
    );
    let first = l
        .cards
        .field_revisions(None, &field.id, None)
        .await
        .unwrap();
    assert!(first.has_more);
    assert_eq!(first.items[1].fields.display_name, "Field 26");
    assert_eq!(first.items[1].archived_at, None);
    assert_eq!(first.items[1].current_archived_at, Some(123));
    let second = l
        .cards
        .field_revisions(
            None,
            &field.id,
            Some(first.items.last().unwrap().pin.version),
        )
        .await
        .unwrap();
    assert!(!second.has_more);
    assert_eq!(
        second
            .items
            .iter()
            .map(|r| r.pin.version)
            .collect::<Vec<_>>(),
        [2, 1]
    );
    assert_eq!(second.items[1].fields, field.fields);
    for query in ["Card 01", "Card 26"] {
        assert!(
            l.cards
                .search_cards(
                    None,
                    &CardSearch {
                        query: query.into(),
                        ..Default::default()
                    }
                )
                .await
                .unwrap()
                .items
                .is_empty()
        );
    }
    assert!(
        l.cards
            .search_fields(
                None,
                &FieldSearch {
                    query: "Field 01".into(),
                    ..Default::default()
                }
            )
            .await
            .unwrap()
            .items
            .is_empty()
    );
    let by_late_label = l
        .cards
        .search_cards(
            None,
            &CardSearch {
                label: "Label 29".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(by_late_label.items[0].pin, card.pin());
}

#[tokio::test]
async fn choice_pages_filter_in_sql_and_keep_exact_off_page_stable_ids() {
    let l = Lab::new().await;
    let options: Vec<_> = (0..31)
        .map(|n| ChoiceOption {
            id: format!("{n:032x}"),
            label: format!("Option %_ {:02}", 31 - n),
        })
        .collect();
    let mut all = vec![ChoiceOption {
        id: "f".repeat(32),
        label: "A nonmatching filler".into(),
    }];
    all.extend(options.clone());
    let d = l
        .field(None, "Choice", FieldKind::Choice { options: all }, None)
        .await;
    let first = l
        .cards
        .choice_options(&d.pin(), "OPTION %_", 1)
        .await
        .unwrap();
    assert!(first.has_more);
    assert_eq!(first.items, options[..25]);
    let second = l
        .cards
        .choice_options(&d.pin(), "option %_", 2)
        .await
        .unwrap();
    assert!(!second.has_more);
    assert_eq!(second.items, options[25..]);
    let selected = second.items.last().unwrap();
    let row = l
        .cards
        .create(
            &l.token,
            None,
            NewCard {
                fields: fields("Selected"),
                source: None,
                reason: None,
                component: CardComponent {
                    fields: Some(vec![CardField {
                        definition: d.clone(),
                        value: FieldValue::Choice(selected.id.clone()),
                    }]),
                    ..Default::default()
                },
            },
        )
        .await
        .unwrap();
    let updated = l
        .edit_field(
            &d,
            "New definition",
            FieldKind::Choice {
                options: vec![ChoiceOption {
                    id: selected.id.clone(),
                    label: "New label".into(),
                }],
            },
            None,
        )
        .await;
    assert_eq!(
        l.cards
            .choice_options(&d.pin(), "option %_ 01", 1)
            .await
            .unwrap()
            .items
            .as_slice(),
        std::slice::from_ref(selected)
    );
    assert_eq!(
        l.cards
            .choice_options(&updated.pin(), "New label", 1)
            .await
            .unwrap()
            .items[0]
            .id,
        selected.id
    );
    let saved = l.cards.detail(&row.identity).await.unwrap();
    assert_eq!(
        saved.component.fields.unwrap()[0].value,
        FieldValue::Choice(selected.id.clone())
    );
    assert!(
        l.cards
            .choice_options(&d.pin(), "' OR 1=1 --", 1)
            .await
            .unwrap()
            .items
            .is_empty()
    );
    let nonchoice = l.field(None, "Boolean", FieldKind::Boolean, None).await;
    assert!(matches!(
        l.cards.choice_options(&nonchoice.pin(), "", 1).await,
        Err(Error::Invalid(_))
    ));
    // Typed options disagreeing with the immutable snapshot fail before searching.
    sqlx::query("DROP TRIGGER card_field_options_immutable_update")
        .execute(&l.pool)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE card_field_options SET label='Corrupt' WHERE field_id=? AND revision=1 AND slot=1",
    )
    .bind(&d.id)
    .execute(&l.pool)
    .await
    .unwrap();
    assert!(matches!(
        l.cards.choice_options(&d.pin(), "no match", 1).await,
        Err(Error::Unavailable)
    ));
}

#[tokio::test]
async fn missing_current_versions_and_broken_seals_fail_instead_of_disappearing() {
    for field in [false, true] {
        let l = Lab::new().await;
        let id = if field {
            l.field(None, "Broken", FieldKind::Boolean, None).await.id
        } else {
            l.card(None, "Broken", vec![]).await.identity.id
        };
        let mut c = l.pool.acquire().await.unwrap();
        sqlx::query("PRAGMA foreign_keys=OFF")
            .execute(&mut *c)
            .await
            .unwrap();
        if field {
            sqlx::query("DROP TRIGGER card_field_versions_immutable_delete")
                .execute(&mut *c)
                .await
                .unwrap();
            sqlx::query("DELETE FROM card_field_versions WHERE field_id=?")
                .bind(&id)
                .execute(&mut *c)
                .await
                .unwrap();
        } else {
            sqlx::query("DROP TRIGGER standalone_card_versions_immutable_delete")
                .execute(&mut *c)
                .await
                .unwrap();
            sqlx::query("DELETE FROM standalone_card_versions WHERE card_id=?")
                .bind(&id)
                .execute(&mut *c)
                .await
                .unwrap();
        }
        sqlx::query("PRAGMA foreign_keys=ON")
            .execute(&mut *c)
            .await
            .unwrap();
        drop(c);
        if field {
            assert!(matches!(
                l.cards
                    .search_fields(
                        None,
                        &FieldSearch {
                            query: "even no match".into(),
                            kind: Some(FieldKindTag::Text),
                            ..Default::default()
                        }
                    )
                    .await,
                Err(Error::Unavailable)
            ));
        } else {
            assert!(matches!(
                l.cards
                    .search_cards(
                        None,
                        &CardSearch {
                            query: "even no match".into(),
                            archive: ArchiveFilter::Archived,
                            ..Default::default()
                        }
                    )
                    .await,
                Err(Error::Unavailable)
            ));
        }
    }
    let l = Lab::new().await;
    let row = l.card(None, "Broken seal", vec!["Role".into()]).await;
    sqlx::query("DROP TRIGGER card_components_sealed_update")
        .execute(&l.pool)
        .await
        .unwrap();
    sqlx::query("UPDATE card_components SET snapshot_json='{}' WHERE card_id=?")
        .bind(&row.identity.id)
        .execute(&l.pool)
        .await
        .unwrap();
    assert!(matches!(
        l.cards
            .search_cards(
                None,
                &CardSearch {
                    label: "Role".into(),
                    ..Default::default()
                }
            )
            .await,
        Err(Error::Unavailable)
    ));
}

#[tokio::test]
async fn validated_bounds_defaults_and_public_reads_do_not_write() {
    let l = Lab::new().await;
    assert_eq!(ArchiveFilter::default(), ArchiveFilter::All);
    assert_eq!(CardSearch::default().page, 1);
    assert_eq!(FieldSearch::default().page, 1);
    for (tag, text) in [
        (FieldKindTag::Number, "number"),
        (FieldKindTag::Text, "text"),
        (FieldKindTag::Boolean, "boolean"),
        (FieldKindTag::Choice, "choice"),
        (FieldKindTag::CardReference, "card_reference"),
    ] {
        assert_eq!(tag.as_str(), text);
    }
    let row = l
        .card(None, &"界".repeat(100), vec!["界".repeat(120)])
        .await;
    let d = l
        .field(None, "Choice", FieldKind::Choice { options: vec![] }, None)
        .await;
    let audits: i64 = sqlx::query_scalar("SELECT count(*) FROM audit_events")
        .fetch_one(&l.pool)
        .await
        .unwrap();
    assert_eq!(
        l.cards
            .search_cards(
                None,
                &CardSearch {
                    query: "界".repeat(100),
                    label: "界".repeat(120),
                    ..Default::default()
                }
            )
            .await
            .unwrap()
            .items[0]
            .pin,
        row.pin()
    );
    for page in [0, 10001, usize::MAX] {
        assert!(matches!(
            l.cards
                .search_cards(
                    None,
                    &CardSearch {
                        page,
                        ..Default::default()
                    }
                )
                .await,
            Err(Error::Invalid(_))
        ));
        assert!(matches!(
            l.cards
                .search_fields(
                    None,
                    &FieldSearch {
                        page,
                        ..Default::default()
                    }
                )
                .await,
            Err(Error::Invalid(_))
        ));
        assert!(matches!(
            l.cards
                .mapping_fields(
                    &l.world,
                    &d.id,
                    &FieldSearch {
                        page,
                        ..Default::default()
                    }
                )
                .await,
            Err(Error::Invalid(_))
        ));
        assert!(matches!(
            l.cards.choice_options(&d.pin(), "", page).await,
            Err(Error::Invalid(_))
        ));
    }
    assert!(matches!(
        l.cards
            .search_cards(
                None,
                &CardSearch {
                    query: "界".repeat(101),
                    ..Default::default()
                }
            )
            .await,
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        l.cards
            .search_cards(
                None,
                &CardSearch {
                    label: "界".repeat(121),
                    ..Default::default()
                }
            )
            .await,
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        l.cards
            .search_fields(
                None,
                &FieldSearch {
                    query: "界".repeat(101),
                    ..Default::default()
                }
            )
            .await,
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        l.cards.choice_options(&d.pin(), &"界".repeat(101), 1).await,
        Err(Error::Invalid(_))
    ));
    for before in [0, -1] {
        assert!(matches!(
            l.cards.card_revisions(&row.identity, Some(before)).await,
            Err(Error::Invalid(_))
        ));
        assert!(matches!(
            l.cards.field_revisions(None, &d.id, Some(before)).await,
            Err(Error::Invalid(_))
        ));
    }
    assert!(
        l.cards
            .search_cards(
                None,
                &CardSearch {
                    page: 10000,
                    ..Default::default()
                }
            )
            .await
            .unwrap()
            .items
            .is_empty()
    );
    assert!(matches!(
        l.cards
            .search_fields(Some(&"f".repeat(32)), &FieldSearch::default())
            .await,
        Err(Error::NotFound)
    ));
    l.cards.card_revisions(&row.identity, None).await.unwrap();
    l.cards.field_revisions(None, &d.id, None).await.unwrap();
    l.cards.choice_options(&d.pin(), "", 1).await.unwrap();
    let after: i64 = sqlx::query_scalar("SELECT count(*) FROM audit_events")
        .fetch_one(&l.pool)
        .await
        .unwrap();
    assert_eq!(after, audits);
}
