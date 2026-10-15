use game_data::{
    authoring::ContentFields,
    stats::{
        BaseStats, Decimal, NumericConstraints, StatDefinition, StatMapping, StatPin, StatRoster,
    },
};
use sqlx::SqlitePool;
use tempfile::TempDir;
use web_app::{
    accounts::Accounts,
    application::{
        authoring::{
            Authoring, Edit, Error, FormRecord, HistoryKind, SourcePin, VariantRecord, WorldRecord,
        },
        stat_values::{FormStatsEdit, SourceValuesEdit, StatValues, VariantStatsEdit},
        stats::{DefinitionEdit, NewStat, Stats},
        types::Types,
    },
    auth::{Actor, Role, Subject},
    persistence,
};

struct User {
    actor: Actor,
    token: String,
}

struct Lab {
    pool: SqlitePool,
    accounts: Accounts,
    author: Authoring,
    stats: Stats,
    values: StatValues,
    types: Types,
    admin: User,
    owner: User,
    editor: User,
    viewer: User,
    _dir: TempDir,
}

async fn user(accounts: &Accounts, name: &str, role: Role) -> User {
    accounts.invite("test", name, role, None).await.unwrap();
    let actor = accounts
        .sign_in(Subject {
            provider: "test".into(),
            provider_subject: name.into(),
            display_name: name.into(),
        })
        .await
        .unwrap();
    let (token, _) = accounts.start_session(&actor.user_id).await.unwrap();
    User { actor, token }
}

impl Lab {
    async fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let pool = persistence::connect_for_migration(&format!(
            "sqlite://{}/stat-values.db",
            dir.path().display()
        ))
        .await
        .unwrap();
        persistence::migrate(&pool).await.unwrap();
        let accounts = Accounts::new(pool.clone(), 3600).unwrap();
        let admin = user(&accounts, "Admin", Role::Administrator).await;
        let owner = user(&accounts, "Owner", Role::Editor).await;
        let editor = user(&accounts, "Editor", Role::Editor).await;
        let viewer = user(&accounts, "Viewer", Role::Viewer).await;
        Self {
            pool,
            author: Authoring::new(accounts.clone()),
            stats: Stats::new(accounts.clone()),
            values: StatValues::new(accounts.clone()),
            types: Types::new(accounts.clone()),
            accounts,
            admin,
            owner,
            editor,
            viewer,
            _dir: dir,
        }
    }

    async fn form(&self) -> FormRecord {
        let species = self
            .author
            .create_species(&self.admin.token, fields("Creature"), None)
            .await
            .unwrap();
        self.author
            .form_detail(&species.default_form_id)
            .await
            .unwrap()
    }

    async fn world(&self) -> WorldRecord {
        self.author
            .create_world(&self.owner.token, fields("Lab"), None)
            .await
            .unwrap()
    }

    async fn variant(&self, world: &str, form: Option<&FormRecord>) -> VariantRecord {
        let revision = self.author.world_detail(world).await.unwrap().revision;
        self.author
            .create_variant(
                &self.owner.token,
                world,
                revision,
                form.map(|f| SourcePin {
                    form_id: f.id.clone(),
                    version: f.revision,
                }),
                fields("Local creature"),
                None,
            )
            .await
            .unwrap()
    }

    async fn count(&self) -> i64 {
        sqlx::query_scalar("SELECT count(*) FROM audit_events")
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }

    async fn definition(
        &self,
        world: Option<&str>,
        name: &str,
        source: Option<StatPin>,
        constraints: NumericConstraints,
    ) -> StatDefinition {
        let token = if world.is_some() {
            &self.owner.token
        } else {
            &self.admin.token
        };
        self.stats
            .create(
                token,
                world,
                NewStat {
                    fields: fields(name),
                    constraints,
                    source,
                    reason: None,
                },
            )
            .await
            .unwrap()
    }

    async fn change_definition(
        &self,
        old: &StatDefinition,
        constraints: NumericConstraints,
    ) -> StatDefinition {
        let token = if old.world_id.is_some() {
            &self.owner.token
        } else {
            &self.admin.token
        };
        self.stats
            .save_definition(
                token,
                old.world_id.as_deref(),
                &old.id,
                DefinitionEdit {
                    expected_revision: old.revision,
                    fields: old.fields.clone(),
                    constraints,
                    reason: None,
                },
            )
            .await
            .unwrap()
    }

    async fn roster(
        &self,
        world: &str,
        expected: i64,
        definitions: &[StatDefinition],
    ) -> StatRoster {
        self.stats
            .save_roster(
                &self.owner.token,
                world,
                expected,
                definitions.iter().map(StatDefinition::pin).collect(),
                None,
            )
            .await
            .unwrap()
    }

    async fn integrity(&self) {
        let issues: Vec<(String, i64, String, i64)> = sqlx::query_as("PRAGMA foreign_key_check")
            .fetch_all(&self.pool)
            .await
            .unwrap();
        assert!(issues.is_empty(), "{issues:?}");
        let result: String = sqlx::query_scalar("PRAGMA integrity_check")
            .fetch_one(&self.pool)
            .await
            .unwrap();
        assert_eq!(result, "ok");
    }
}

fn fields(name: &str) -> ContentFields {
    ContentFields {
        slug: "ignored".into(),
        display_name: name.into(),
        description: "Base values only.".into(),
    }
}

fn decimal(value: &str) -> Decimal {
    value.parse().unwrap()
}

fn edit(revision: i64, name: &str) -> Edit {
    Edit {
        expected_revision: revision,
        fields: fields(name),
        reason: None,
    }
}

fn form_edit(revision: i64, values: Option<Vec<(StatPin, Option<Decimal>)>>) -> FormStatsEdit {
    FormStatsEdit {
        expected_revision: revision,
        values,
        reason: Some("Reviewed values".into()),
    }
}

fn variant_edit(
    revision: i64,
    roster: i64,
    values: Vec<(String, Option<Decimal>)>,
) -> VariantStatsEdit {
    VariantStatsEdit {
        expected_revision: revision,
        expected_roster_revision: roster,
        values,
        reason: Some("Reviewed roster values".into()),
    }
}

fn source_edit(
    revision: i64,
    roster: i64,
    target: i64,
    mapping: Vec<StatMapping>,
) -> SourceValuesEdit {
    SourceValuesEdit {
        expected_revision: revision,
        expected_roster_revision: roster,
        target_form_version: target,
        mapping,
        reason: Some("Reviewed shared values".into()),
    }
}

fn mapping(shared: &StatDefinition, local: &StatDefinition, clear_missing: bool) -> StatMapping {
    StatMapping {
        shared_stat_id: shared.id.clone(),
        local_stat_id: local.id.clone(),
        clear_missing,
    }
}

fn assignments(state: &BaseStats) -> Vec<(String, Option<Decimal>)> {
    state
        .assignments
        .as_ref()
        .unwrap()
        .iter()
        .map(|v| (v.definition.id.clone(), v.value))
        .collect()
}

#[tokio::test]
async fn shared_values_are_exact_optional_pinned_and_normalized_without_noop_history() {
    let l = Lab::new().await;
    let form = l.form().await;
    let signed = l
        .definition(
            None,
            "Signed",
            None,
            NumericConstraints {
                minimum: Some(decimal("-1.25")),
                maximum: Some(decimal("0")),
                whole_only: false,
            },
        )
        .await;
    let whole = l
        .definition(
            None,
            "Whole",
            None,
            NumericConstraints {
                minimum: Some(decimal("0")),
                maximum: Some(decimal("10")),
                whole_only: true,
            },
        )
        .await;
    assert_eq!(l.values.form(&form.id).await.unwrap(), BaseStats::default());
    let empty = l
        .values
        .save_form(&l.admin.token, &form.id, form_edit(1, Some(vec![])))
        .await
        .unwrap();
    assert_eq!(empty.revision, 2);
    assert_eq!(
        l.values.form(&form.id).await.unwrap().assignments,
        Some(vec![])
    );
    let values = Some(vec![
        (signed.pin(), Some(decimal("-1.250000"))),
        (whole.pin(), None),
    ]);
    let saved = l
        .values
        .save_form(&l.admin.token, &form.id, form_edit(2, values.clone()))
        .await
        .unwrap();
    assert_eq!(saved.revision, 3);
    let state = l.values.form(&form.id).await.unwrap();
    assert_eq!(state.roster, None);
    assert_eq!(
        assignments(&state),
        vec![
            (signed.id.clone(), Some(decimal("-1.25"))),
            (whole.id.clone(), None)
        ]
    );
    let count = l.count().await;
    assert_eq!(
        l.values
            .save_form(&l.admin.token, &form.id, form_edit(3, values))
            .await
            .unwrap(),
        saved
    );
    assert_eq!(l.count().await, count);
    for (pin, value) in [
        (signed.pin(), "-1.250001"),
        (signed.pin(), "0.000001"),
        (whole.pin(), "0.5"),
        (whole.pin(), "11"),
    ] {
        assert!(matches!(
            l.values
                .save_form(
                    &l.admin.token,
                    &form.id,
                    form_edit(3, Some(vec![(pin, Some(decimal(value)))]))
                )
                .await,
            Err(Error::Invalid(_))
        ));
    }
    let duplicate = Some(vec![
        (signed.pin(), None),
        (signed.pin(), Some(decimal("0"))),
    ]);
    assert!(matches!(
        l.values
            .save_form(&l.admin.token, &form.id, form_edit(3, duplicate))
            .await,
        Err(Error::Invalid(_))
    ));
    let missing = Some(vec![(
        StatPin {
            stat_id: signed.id.clone(),
            version: 999,
        },
        None,
    )]);
    assert_eq!(
        l.values
            .save_form(&l.admin.token, &form.id, form_edit(2, missing.clone()))
            .await
            .unwrap_err(),
        Error::Conflict
    );
    assert!(matches!(
        l.values
            .save_form(&l.admin.token, &form.id, form_edit(3, missing))
            .await,
        Err(Error::Invalid(_))
    ));
    assert_eq!(l.count().await, count);
    let zeroed = l
        .values
        .save_form(
            &l.admin.token,
            &form.id,
            form_edit(
                3,
                Some(vec![
                    (signed.pin(), Some(decimal("-0.0000"))),
                    (whole.pin(), Some(decimal("0"))),
                ]),
            ),
        )
        .await
        .unwrap();
    assert_eq!(zeroed.revision, 4);
    let state = l.values.form(&form.id).await.unwrap();
    assert!(
        assignments(&state)
            .iter()
            .all(|(_, v)| *v == Some(decimal("0")))
    );
    let renamed = l
        .author
        .save_form(
            &l.admin.token,
            &form.id,
            form.species_version,
            edit(4, "Renamed creature"),
        )
        .await
        .unwrap();
    assert_eq!(l.values.form(&form.id).await.unwrap(), state);
    l.types
        .save_form_typing(
            &l.admin.token,
            &form.id,
            renamed.revision,
            Some(vec![]),
            None,
        )
        .await
        .unwrap();
    assert_eq!(l.values.form(&form.id).await.unwrap(), state);
    let history = l
        .author
        .history(HistoryKind::Form, &form.id, None)
        .await
        .unwrap();
    for pair in history.windows(2) {
        assert_eq!(
            pair[0].before_json.as_deref(),
            Some(pair[1].after_json.as_str())
        );
    }
    let json: serde_json::Value = serde_json::from_str(&history[0].after_json).unwrap();
    assert_eq!(json["snapshot_version"], 3);
    assert_eq!(json["content"]["stats"]["assignments"][0]["value"], "0");
    assert_eq!(history[0].actor_name, "Admin");
    l.values
        .save_form(&l.admin.token, &form.id, form_edit(6, None))
        .await
        .unwrap();
    assert_eq!(l.values.form(&form.id).await.unwrap(), BaseStats::default());
    l.integrity().await;
}

#[tokio::test]
async fn reconciliation_is_explicit_retains_identity_and_requires_correction_for_new_constraints() {
    let l = Lab::new().await;
    let world = l.world().await;
    let kept = l
        .definition(Some(&world.id), "Kept", None, NumericConstraints::default())
        .await;
    let removed = l
        .definition(
            Some(&world.id),
            "Removed",
            None,
            NumericConstraints::default(),
        )
        .await;
    let new = l
        .definition(Some(&world.id), "New", None, NumericConstraints::default())
        .await;
    let roster = l
        .roster(&world.id, 0, &[kept.clone(), removed.clone()])
        .await;
    let variant = l.variant(&world.id, None).await;
    assert_eq!(
        l.values.variant(&world.id, &variant.id).await.unwrap(),
        BaseStats::default()
    );
    assert!(matches!(
        l.values
            .save_variant(
                &l.owner.token,
                &world.id,
                &variant.id,
                variant_edit(1, 1, vec![])
            )
            .await,
        Err(Error::Invalid(_))
    ));
    let count = l.count().await;
    let preview = l
        .values
        .preview_reconcile(&world.id, &variant.id, 1, &[])
        .await
        .unwrap();
    assert_eq!(preview.roster, roster);
    assert_eq!(
        assignments(&preview.proposed),
        vec![(kept.id.clone(), None), (removed.id.clone(), None)]
    );
    assert!(preview.removed.is_empty());
    assert_eq!(l.count().await, count);
    l.values
        .reconcile(
            &l.owner.token,
            &world.id,
            &variant.id,
            variant_edit(
                1,
                1,
                vec![
                    (kept.id.clone(), Some(decimal("-0.25"))),
                    (removed.id.clone(), Some(decimal("0"))),
                ],
            ),
        )
        .await
        .unwrap();
    let old = l.values.variant(&world.id, &variant.id).await.unwrap();
    let constrained = l
        .change_definition(
            &kept,
            NumericConstraints {
                minimum: Some(decimal("0")),
                maximum: None,
                whole_only: true,
            },
        )
        .await;
    let roster2 = l
        .roster(&world.id, 1, &[new.clone(), constrained.clone()])
        .await;
    assert_eq!(l.values.variant(&world.id, &variant.id).await.unwrap(), old);
    // Both ordinary writers preserve the older roster context and invalid-under-new constraints.
    l.author
        .save_variant(
            &l.owner.token,
            &world.id,
            &variant.id,
            edit(2, "Metadata only"),
        )
        .await
        .unwrap();
    l.types
        .save_variant_typing(
            &l.owner.token,
            &world.id,
            &variant.id,
            3,
            Some(vec![]),
            None,
        )
        .await
        .unwrap();
    assert_eq!(l.values.variant(&world.id, &variant.id).await.unwrap(), old);
    assert!(matches!(
        l.values
            .save_variant(
                &l.owner.token,
                &world.id,
                &variant.id,
                variant_edit(4, 2, vec![])
            )
            .await,
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        l.values
            .preview_reconcile(&world.id, &variant.id, 2, &[])
            .await,
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        l.values
            .reconcile(
                &l.owner.token,
                &world.id,
                &variant.id,
                variant_edit(4, 2, vec![])
            )
            .await,
        Err(Error::Invalid(_))
    ));
    let corrected = vec![(kept.id.clone(), None)];
    let preview = l
        .values
        .preview_reconcile(&world.id, &variant.id, 2, &corrected)
        .await
        .unwrap();
    assert_eq!(preview.old, old);
    assert_eq!(preview.roster, roster2);
    assert_eq!(
        assignments(&preview.proposed),
        vec![(new.id.clone(), None), (kept.id.clone(), None)]
    );
    assert_eq!(
        preview.removed,
        vec![old.assignments.as_ref().unwrap()[1].clone()]
    );
    l.values
        .reconcile(
            &l.owner.token,
            &world.id,
            &variant.id,
            variant_edit(4, 2, corrected),
        )
        .await
        .unwrap();
    assert_eq!(
        l.values.variant(&world.id, &variant.id).await.unwrap(),
        preview.proposed
    );
    let before = l.count().await;
    assert_eq!(
        l.values
            .reconcile(
                &l.owner.token,
                &world.id,
                &variant.id,
                variant_edit(5, 2, vec![])
            )
            .await
            .unwrap()
            .revision,
        5
    );
    assert_eq!(l.count().await, before);
    let saved = l
        .values
        .save_variant(
            &l.owner.token,
            &world.id,
            &variant.id,
            variant_edit(5, 2, vec![(kept.id.clone(), Some(decimal("0")))]),
        )
        .await
        .unwrap();
    assert_eq!(saved.revision, 6);
    let state = l.values.variant(&world.id, &variant.id).await.unwrap();
    assert_eq!(
        assignments(&state),
        vec![
            (new.id.clone(), None),
            (kept.id.clone(), Some(decimal("0")))
        ]
    );
    assert_eq!(state.assignments.unwrap()[1].definition, constrained);
    assert_eq!(
        l.types
            .variant_typing(&world.id, &variant.id, None)
            .await
            .unwrap()
            .assignments,
        Some(vec![])
    );
    assert_eq!(l.author.world_detail(&world.id).await.unwrap().revision, 2);
    let history = l
        .author
        .history(HistoryKind::Variant, &variant.id, None)
        .await
        .unwrap();
    for pair in history.windows(2) {
        assert_eq!(
            pair[0].before_json.as_deref(),
            Some(pair[1].after_json.as_str())
        );
    }
    l.integrity().await;
}

#[tokio::test]
async fn no_roster_defined_empty_and_large_rosters_remain_distinct() {
    let l = Lab::new().await;
    let world = l.world().await;
    let variant = l.variant(&world.id, None).await;
    assert!(matches!(
        l.values
            .reconcile(
                &l.owner.token,
                &world.id,
                &variant.id,
                variant_edit(1, 0, vec![])
            )
            .await,
        Err(Error::Invalid(_))
    ));
    let empty = l.roster(&world.id, 0, &[]).await;
    l.values
        .reconcile(
            &l.owner.token,
            &world.id,
            &variant.id,
            variant_edit(1, 1, vec![]),
        )
        .await
        .unwrap();
    assert_eq!(
        l.values.variant(&world.id, &variant.id).await.unwrap(),
        BaseStats {
            roster: Some(empty.pin()),
            assignments: Some(vec![])
        }
    );
    let mut definitions = Vec::new();
    for n in 0..35 {
        definitions.push(
            l.definition(
                Some(&world.id),
                &format!("Stat {n}"),
                None,
                NumericConstraints::default(),
            )
            .await,
        );
    }
    l.roster(&world.id, 1, &definitions).await;
    l.values
        .reconcile(
            &l.owner.token,
            &world.id,
            &variant.id,
            variant_edit(2, 2, vec![]),
        )
        .await
        .unwrap();
    let state = l.values.variant(&world.id, &variant.id).await.unwrap();
    assert_eq!(state.assignments.as_ref().unwrap().len(), 35);
    assert!(state.assignments.unwrap().iter().all(|v| v.value.is_none()));
    l.integrity().await;
}

#[tokio::test]
async fn source_values_require_explicit_injective_identity_mappings_and_atomic_pin_adoption() {
    let l = Lab::new().await;
    let form = l.form().await;
    let shared = l
        .definition(None, "Same name", None, NumericConstraints::default())
        .await;
    let other = l
        .definition(None, "Other", None, NumericConstraints::default())
        .await;
    let form = l
        .values
        .save_form(
            &l.admin.token,
            &form.id,
            form_edit(
                1,
                Some(vec![
                    (shared.pin(), Some(decimal("-0.125"))),
                    (other.pin(), None),
                ]),
            ),
        )
        .await
        .unwrap();
    let world = l.world().await;
    let a = l
        .definition(
            Some(&world.id),
            "Same name",
            Some(shared.pin()),
            NumericConstraints::default(),
        )
        .await;
    let b = l
        .definition(
            Some(&world.id),
            "Duplicate source",
            Some(shared.pin()),
            NumericConstraints::default(),
        )
        .await;
    let c = l
        .definition(
            Some(&world.id),
            "Other",
            Some(other.pin()),
            NumericConstraints::default(),
        )
        .await;
    let exclusive = l
        .definition(
            Some(&world.id),
            "Exclusive",
            None,
            NumericConstraints::default(),
        )
        .await;
    l.roster(
        &world.id,
        0,
        &[b.clone(), exclusive.clone(), c.clone(), a.clone()],
    )
    .await;
    let variant = l.variant(&world.id, Some(&form)).await;
    let before = l.count().await;
    let keep = l
        .values
        .preview_source(&world.id, &variant.id, form.revision, &[])
        .await
        .unwrap();
    assert_eq!(keep.proposed, BaseStats::unmodeled());
    let map = mapping(&shared, &a, false);
    let preview = l
        .values
        .preview_source(
            &world.id,
            &variant.id,
            form.revision,
            std::slice::from_ref(&map),
        )
        .await
        .unwrap();
    assert_eq!(preview.source, form);
    assert_eq!(preview.old_source, l.values.form(&form.id).await.unwrap());
    assert_eq!(preview.new_source, preview.old_source);
    assert_eq!(
        assignments(&preview.proposed),
        vec![
            (b.id.clone(), None),
            (exclusive.id.clone(), None),
            (c.id.clone(), None),
            (a.id.clone(), Some(decimal("-0.125")))
        ]
    );
    assert_eq!(l.count().await, before);
    // An explicit choice among duplicate-source locals is valid; implicit fan-out is not.
    for maps in [
        vec![map.clone(), mapping(&shared, &b, false)],
        vec![map.clone(), map.clone()],
        vec![map.clone(), mapping(&other, &a, false)],
        vec![mapping(&shared, &exclusive, false)],
        vec![mapping(&other, &a, false)],
    ] {
        assert!(matches!(
            l.values
                .preview_source(&world.id, &variant.id, form.revision, &maps)
                .await,
            Err(Error::Invalid(_))
        ));
    }
    l.values
        .apply_source(
            &l.owner.token,
            &world.id,
            &variant.id,
            source_edit(1, 1, form.revision, vec![map.clone()]),
        )
        .await
        .unwrap();
    assert_eq!(
        l.values.variant(&world.id, &variant.id).await.unwrap(),
        preview.proposed
    );
    assert_eq!(l.count().await, before + 1);
    l.values
        .save_variant(
            &l.owner.token,
            &world.id,
            &variant.id,
            variant_edit(
                2,
                1,
                vec![
                    (b.id.clone(), Some(decimal("9"))),
                    (c.id.clone(), Some(decimal("4"))),
                    (exclusive.id.clone(), Some(decimal("7"))),
                ],
            ),
        )
        .await
        .unwrap();
    let form3 = l
        .values
        .save_form(
            &l.admin.token,
            &form.id,
            form_edit(
                2,
                Some(vec![
                    (shared.pin(), Some(decimal("0"))),
                    (other.pin(), None),
                ]),
            ),
        )
        .await
        .unwrap();
    let maps = vec![map.clone(), mapping(&other, &c, false)];
    let preview = l
        .values
        .preview_source(&world.id, &variant.id, 3, &maps)
        .await
        .unwrap();
    assert_eq!(
        assignments(&preview.proposed),
        vec![
            (b.id.clone(), Some(decimal("9"))),
            (exclusive.id.clone(), Some(decimal("7"))),
            (c.id.clone(), Some(decimal("4"))),
            (a.id.clone(), Some(decimal("0")))
        ]
    );
    let before = l.count().await;
    let adopted = l
        .values
        .apply_source(
            &l.owner.token,
            &world.id,
            &variant.id,
            source_edit(3, 1, form3.revision, maps.clone()),
        )
        .await
        .unwrap();
    assert_eq!(adopted.revision, 4);
    assert_eq!(adopted.source.as_ref().unwrap().version, 3);
    assert_eq!(adopted.fields, variant.fields);
    assert_eq!(
        l.values.variant(&world.id, &variant.id).await.unwrap(),
        preview.proposed
    );
    assert_eq!(l.count().await, before + 1);
    let history = l
        .author
        .history(HistoryKind::Variant, &variant.id, None)
        .await
        .unwrap();
    assert_eq!(
        (&*history[0].actor_name, &*history[0].action),
        ("Owner", "adopt")
    );
    assert_eq!(history[0].reason.as_deref(), Some("Reviewed shared values"));
    assert_eq!(
        history[0].before_json.as_deref(),
        Some(history[1].after_json.as_str())
    );
    let json: serde_json::Value = serde_json::from_str(&history[0].after_json).unwrap();
    assert_eq!(json["content"]["source"]["version"], 3);
    assert_eq!(json["content"]["stats"]["assignments"][3]["value"], "0");
    let before = l.count().await;
    assert_eq!(
        l.values
            .apply_source(
                &l.owner.token,
                &world.id,
                &variant.id,
                source_edit(4, 1, 3, maps)
            )
            .await
            .unwrap(),
        adopted
    );
    assert_eq!(l.count().await, before);
    let cleared = vec![mapping(&other, &c, true)];
    let preview = l
        .values
        .preview_source(&world.id, &variant.id, 3, &cleared)
        .await
        .unwrap();
    assert_eq!(assignments(&preview.proposed)[2].1, None);
    l.values
        .apply_source(
            &l.owner.token,
            &world.id,
            &variant.id,
            source_edit(4, 1, 3, cleared),
        )
        .await
        .unwrap();
    assert_eq!(
        l.values.variant(&world.id, &variant.id).await.unwrap(),
        preview.proposed
    );
    // Absent assignments in a newer source are Missing, with the same explicit clear policy.
    l.values
        .save_form(&l.admin.token, &form.id, form_edit(3, None))
        .await
        .unwrap();
    let keep = l
        .values
        .preview_source(&world.id, &variant.id, 4, std::slice::from_ref(&map))
        .await
        .unwrap();
    assert_eq!(keep.proposed, keep.local);
    let clear = l
        .values
        .preview_source(&world.id, &variant.id, 4, &[mapping(&shared, &a, true)])
        .await
        .unwrap();
    assert_eq!(assignments(&clear.proposed)[3].1, None);
    assert!(matches!(
        l.values
            .preview_source(&world.id, &variant.id, 2, &[])
            .await,
        Err(Error::Invalid(_))
    ));
    l.integrity().await;
}

#[tokio::test]
async fn source_copy_validates_local_constraints_and_rejects_old_context_while_default_adoption_keeps_values()
 {
    let l = Lab::new().await;
    let form = l.form().await;
    let shared = l
        .definition(None, "Shared", None, NumericConstraints::default())
        .await;
    let form = l
        .values
        .save_form(
            &l.admin.token,
            &form.id,
            form_edit(1, Some(vec![(shared.pin(), Some(decimal("-0.5")))])),
        )
        .await
        .unwrap();
    let world = l.world().await;
    let local = l
        .definition(
            Some(&world.id),
            "Local",
            Some(shared.pin()),
            NumericConstraints {
                whole_only: true,
                minimum: Some(decimal("0")),
                maximum: None,
            },
        )
        .await;
    l.roster(&world.id, 0, std::slice::from_ref(&local)).await;
    let variant = l.variant(&world.id, Some(&form)).await;
    let maps = vec![mapping(&shared, &local, false)];
    assert!(matches!(
        l.values
            .preview_source(&world.id, &variant.id, 2, &maps)
            .await,
        Err(Error::Invalid(_))
    ));
    let count = l.count().await;
    assert!(matches!(
        l.values
            .apply_source(
                &l.owner.token,
                &world.id,
                &variant.id,
                source_edit(1, 1, 2, maps.clone())
            )
            .await,
        Err(Error::Invalid(_))
    ));
    assert_eq!(
        l.author
            .variant_detail(&world.id, &variant.id)
            .await
            .unwrap(),
        variant
    );
    assert_eq!(
        l.values.variant(&world.id, &variant.id).await.unwrap(),
        BaseStats::default()
    );
    assert_eq!(l.count().await, count);
    l.values
        .reconcile(
            &l.owner.token,
            &world.id,
            &variant.id,
            variant_edit(1, 1, vec![(local.id.clone(), Some(decimal("4")))]),
        )
        .await
        .unwrap();
    let saved = l.values.variant(&world.id, &variant.id).await.unwrap();
    let newer = l
        .values
        .save_form(
            &l.admin.token,
            &form.id,
            form_edit(2, Some(vec![(shared.pin(), Some(decimal("5")))])),
        )
        .await
        .unwrap();
    let changed = l
        .change_definition(&local, NumericConstraints::default())
        .await;
    l.roster(&world.id, 1, &[changed]).await;
    let count = l.count().await;
    assert!(matches!(
        l.values
            .preview_source(&world.id, &variant.id, 3, &maps)
            .await,
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        l.values
            .apply_source(
                &l.owner.token,
                &world.id,
                &variant.id,
                source_edit(2, 2, 3, maps)
            )
            .await,
        Err(Error::Invalid(_))
    ));
    assert_eq!(l.count().await, count);
    l.author
        .adopt_variant(
            &l.owner.token,
            &world.id,
            &variant.id,
            newer.revision,
            edit(2, "Keep my local values"),
        )
        .await
        .unwrap();
    assert_eq!(
        l.values.variant(&world.id, &variant.id).await.unwrap(),
        saved
    );
    assert_eq!(
        l.author
            .variant_detail(&world.id, &variant.id)
            .await
            .unwrap()
            .source
            .unwrap()
            .version,
        3
    );
    l.integrity().await;
}

#[tokio::test]
async fn permissions_are_rechecked_for_noops_collaboration_revocation_and_disabled_sessions() {
    let l = Lab::new().await;
    let form = l.form().await;
    let world = l.world().await;
    let variant = l.variant(&world.id, Some(&form)).await;
    l.roster(&world.id, 0, &[]).await;
    for user in [&l.owner, &l.editor, &l.viewer] {
        assert_eq!(
            l.values
                .save_form(&user.token, &form.id, form_edit(1, None))
                .await
                .unwrap_err(),
            Error::Forbidden
        );
    }
    for user in [&l.admin, &l.editor, &l.viewer] {
        assert_eq!(
            l.values
                .reconcile(
                    &user.token,
                    &world.id,
                    &variant.id,
                    variant_edit(1, 1, vec![])
                )
                .await
                .unwrap_err(),
            Error::Forbidden
        );
        assert_eq!(
            l.values
                .apply_source(
                    &user.token,
                    &world.id,
                    &variant.id,
                    source_edit(1, 1, 1, vec![])
                )
                .await
                .unwrap_err(),
            Error::Forbidden
        );
    }
    l.author
        .grant_collaborator(&l.owner.token, &world.id, 1, &l.editor.actor.user_id, None)
        .await
        .unwrap();
    l.values
        .reconcile(
            &l.editor.token,
            &world.id,
            &variant.id,
            variant_edit(1, 1, vec![]),
        )
        .await
        .unwrap();
    assert_eq!(
        l.author
            .history_entry(HistoryKind::Variant, &variant.id, 2)
            .await
            .unwrap()
            .actor_name,
        "Editor"
    );
    l.author
        .revoke_collaborator(&l.owner.token, &world.id, 2, &l.editor.actor.user_id, None)
        .await
        .unwrap();
    assert_eq!(
        l.values
            .save_variant(
                &l.editor.token,
                &world.id,
                &variant.id,
                variant_edit(2, 1, vec![])
            )
            .await
            .unwrap_err(),
        Error::Forbidden
    );
    l.author
        .grant_collaborator(&l.owner.token, &world.id, 3, &l.editor.actor.user_id, None)
        .await
        .unwrap();
    sqlx::query("UPDATE users SET role='viewer' WHERE id=?")
        .bind(&l.editor.actor.user_id)
        .execute(&l.pool)
        .await
        .unwrap();
    assert_eq!(
        l.values
            .save_variant(
                &l.editor.token,
                &world.id,
                &variant.id,
                variant_edit(2, 1, vec![])
            )
            .await
            .unwrap_err(),
        Error::Forbidden
    );
    l.accounts
        .set_disabled(&l.owner.actor.user_id, true)
        .await
        .unwrap();
    assert_eq!(
        l.values
            .save_variant(
                &l.owner.token,
                &world.id,
                &variant.id,
                variant_edit(2, 1, vec![])
            )
            .await
            .unwrap_err(),
        Error::Unauthorized
    );
    l.accounts
        .set_disabled(&l.owner.actor.user_id, false)
        .await
        .unwrap();
    assert_eq!(
        l.values
            .reconcile(
                &l.owner.token,
                &world.id,
                &variant.id,
                variant_edit(2, 1, vec![])
            )
            .await
            .unwrap_err(),
        Error::Unauthorized
    );
    assert_eq!(
        l.values
            .save_form("revoked", &form.id, form_edit(1, None))
            .await
            .unwrap_err(),
        Error::Unauthorized
    );
    l.integrity().await;
}

#[tokio::test]
async fn creature_and_current_roster_conflicts_precede_invalid_dependencies_and_reasons() {
    let l = Lab::new().await;
    let form = l.form().await;
    let world = l.world().await;
    let local = l
        .definition(
            Some(&world.id),
            "Local",
            None,
            NumericConstraints::default(),
        )
        .await;
    l.roster(&world.id, 0, std::slice::from_ref(&local)).await;
    let variant = l.variant(&world.id, Some(&form)).await;
    l.values
        .reconcile(
            &l.owner.token,
            &world.id,
            &variant.id,
            variant_edit(1, 1, vec![]),
        )
        .await
        .unwrap();
    l.roster(&world.id, 1, &[]).await;
    let count = l.count().await;
    for (expected, roster) in [(1, 2), (2, 1)] {
        let mut bad = variant_edit(
            expected,
            roster,
            vec![("fabricated".into(), Some(decimal("-1")))],
        );
        bad.reason = Some("x".repeat(1001));
        assert_eq!(
            l.values
                .save_variant(&l.owner.token, &world.id, &variant.id, bad.clone())
                .await
                .unwrap_err(),
            Error::Conflict
        );
        assert_eq!(
            l.values
                .reconcile(&l.owner.token, &world.id, &variant.id, bad)
                .await
                .unwrap_err(),
            Error::Conflict
        );
        let mut source = source_edit(
            expected,
            roster,
            999,
            vec![StatMapping {
                shared_stat_id: "bad".into(),
                local_stat_id: "bad".into(),
                clear_missing: true,
            }],
        );
        source.reason = Some("x".repeat(1001));
        assert_eq!(
            l.values
                .apply_source(&l.owner.token, &world.id, &variant.id, source)
                .await
                .unwrap_err(),
            Error::Conflict
        );
    }
    assert_eq!(l.count().await, count);
    assert_eq!(
        l.values
            .preview_reconcile(&world.id, &variant.id, 1, &[])
            .await
            .unwrap_err(),
        Error::Conflict
    );
    // Correct context still rejects foreign/duplicate identity updates and bad reasons.
    l.values
        .reconcile(
            &l.owner.token,
            &world.id,
            &variant.id,
            variant_edit(2, 2, vec![]),
        )
        .await
        .unwrap();
    assert!(matches!(
        l.values
            .save_variant(
                &l.owner.token,
                &world.id,
                &variant.id,
                variant_edit(3, 2, vec![(local.id.clone(), None)])
            )
            .await,
        Err(Error::Invalid(_))
    ));
    let mut bad_reason = variant_edit(3, 2, vec![]);
    bad_reason.reason = Some("x".repeat(1001));
    assert!(matches!(
        l.values
            .save_variant(&l.owner.token, &world.id, &variant.id, bad_reason)
            .await,
        Err(Error::Invalid(_))
    ));
    let foreign_world = l
        .author
        .create_world(&l.owner.token, fields("Foreign"), None)
        .await
        .unwrap();
    let foreign = l
        .definition(
            Some(&foreign_world.id),
            "Local",
            None,
            NumericConstraints::default(),
        )
        .await;
    assert!(matches!(
        l.values
            .save_form(
                &l.admin.token,
                &form.id,
                form_edit(1, Some(vec![(foreign.pin(), None)]))
            )
            .await,
        Err(Error::Invalid(_))
    ));
    assert_eq!(
        l.values
            .variant(&foreign_world.id, &variant.id)
            .await
            .unwrap_err(),
        Error::NotFound
    );
    l.roster(&world.id, 2, std::slice::from_ref(&local)).await;
    assert!(matches!(
        l.values
            .reconcile(
                &l.owner.token,
                &world.id,
                &variant.id,
                variant_edit(
                    3,
                    3,
                    vec![
                        (local.id.clone(), None),
                        (local.id.clone(), Some(decimal("0")))
                    ]
                )
            )
            .await,
        Err(Error::Invalid(_))
    ));
    l.integrity().await;
}

#[tokio::test]
async fn late_failures_roll_back_values_source_pin_seals_and_audit_together() {
    for trigger in [
        "CREATE TRIGGER fault BEFORE INSERT ON form_stat_snapshots WHEN NEW.revision=2 BEGIN SELECT RAISE(ABORT,'test'); END",
        "CREATE TRIGGER fault BEFORE INSERT ON form_versions WHEN NEW.revision=2 BEGIN SELECT RAISE(ABORT,'test'); END",
    ] {
        let l = Lab::new().await;
        let form = l.form().await;
        let shared = l
            .definition(None, "Shared", None, NumericConstraints::default())
            .await;
        let count = l.count().await;
        sqlx::query(trigger).execute(&l.pool).await.unwrap();
        assert_eq!(
            l.values
                .save_form(
                    &l.admin.token,
                    &form.id,
                    form_edit(1, Some(vec![(shared.pin(), Some(decimal("-1.5")))]))
                )
                .await
                .unwrap_err(),
            Error::Unavailable
        );
        assert_eq!(l.author.form_detail(&form.id).await.unwrap(), form);
        assert_eq!(l.values.form(&form.id).await.unwrap(), BaseStats::default());
        assert_eq!(l.count().await, count);
        let orphans: i64 = sqlx::query_scalar("SELECT (SELECT count(*) FROM form_stat_bindings WHERE revision=2)+(SELECT count(*) FROM form_stat_snapshots WHERE revision=2)+(SELECT count(*) FROM form_content_snapshots WHERE revision=2)").fetch_one(&l.pool).await.unwrap();
        assert_eq!(orphans, 0);
        l.integrity().await;
    }
    for trigger in [
        "CREATE TRIGGER fault BEFORE INSERT ON variant_stat_snapshots WHEN NEW.revision=2 BEGIN SELECT RAISE(ABORT,'test'); END",
        "CREATE TRIGGER fault BEFORE INSERT ON variant_revisions WHEN NEW.revision=2 BEGIN SELECT RAISE(ABORT,'test'); END",
    ] {
        let l = Lab::new().await;
        let form = l.form().await;
        let shared = l
            .definition(None, "Shared", None, NumericConstraints::default())
            .await;
        let world = l.world().await;
        let local = l
            .definition(
                Some(&world.id),
                "Local",
                Some(shared.pin()),
                NumericConstraints::default(),
            )
            .await;
        l.roster(&world.id, 0, std::slice::from_ref(&local)).await;
        let variant = l.variant(&world.id, Some(&form)).await;
        let newer = l
            .values
            .save_form(
                &l.admin.token,
                &form.id,
                form_edit(1, Some(vec![(shared.pin(), Some(decimal("-1.5")))])),
            )
            .await
            .unwrap();
        let count = l.count().await;
        sqlx::query(trigger).execute(&l.pool).await.unwrap();
        assert_eq!(
            l.values
                .apply_source(
                    &l.owner.token,
                    &world.id,
                    &variant.id,
                    source_edit(1, 1, newer.revision, vec![mapping(&shared, &local, false)])
                )
                .await
                .unwrap_err(),
            Error::Unavailable
        );
        assert_eq!(
            l.author
                .variant_detail(&world.id, &variant.id)
                .await
                .unwrap(),
            variant
        );
        assert_eq!(
            l.values.variant(&world.id, &variant.id).await.unwrap(),
            BaseStats::default()
        );
        assert_eq!(l.count().await, count);
        let orphans: i64 = sqlx::query_scalar("SELECT (SELECT count(*) FROM variant_stat_bindings WHERE revision=2)+(SELECT count(*) FROM variant_stat_snapshots WHERE revision=2)+(SELECT count(*) FROM variant_content_snapshots WHERE revision=2)").fetch_one(&l.pool).await.unwrap();
        assert_eq!(orphans, 0);
        l.integrity().await;
    }
}
