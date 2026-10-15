use game_data::{
    authoring::ContentFields,
    charts::*,
    types::{TypePin, TypeRecord, WorldTypeRecord},
};
use sqlx::{Executor, SqlitePool};
use tempfile::TempDir;
use web_app::{
    accounts::Accounts,
    application::{
        authoring::{AddressKey, Authoring, Edit, Error, WorldRecord},
        charts::*,
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
    charts: Charts,
    types: Types,
    author: Authoring,
    admin: User,
    owner: User,
    other: User,
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
fn fields(name: &str) -> ContentFields {
    ContentFields {
        slug: "ignored / forged".into(),
        display_name: name.into(),
        description: "Synthetic test metadata.".into(),
    }
}
fn pin(t: &TypeRecord) -> TypePin {
    TypePin {
        type_id: t.id.clone(),
        version: t.revision,
    }
}
fn local_pin(t: &WorldTypeRecord) -> TypePin {
    TypePin {
        type_id: t.id.clone(),
        version: t.revision,
    }
}
fn cell(a: &str, d: &str, n: u32, den: u32) -> ChartCell {
    ChartCell {
        attacker_type_id: a.into(),
        defender_type_id: d.into(),
        factor: Factor {
            numerator: n,
            denominator: den,
        },
    }
}
fn new(name: &str, roster: Vec<TypePin>) -> NewChart {
    NewChart {
        fields: fields(name),
        roster,
        source: None,
        mapping: vec![],
        apply_source: false,
        reason: Some("Initial design".into()),
    }
}
fn definition(r: &ChartRecord) -> DefinitionEdit {
    DefinitionEdit {
        expected_revision: r.revision,
        fields: r.fields.clone(),
        roster: r.roster.iter().map(|a| a.pin.clone()).collect(),
        mapping: r.mapping.clone(),
        reason: None,
    }
}
impl Lab {
    async fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let pool = persistence::connect_for_migration(&format!(
            "sqlite://{}/charts.db",
            dir.path().display()
        ))
        .await
        .unwrap();
        persistence::migrate(&pool).await.unwrap();
        assert!(persistence::schema_ready(&pool).await.unwrap());
        let accounts = Accounts::new(pool.clone(), 3600).unwrap();
        let admin = user(&accounts, "Admin", Role::Administrator).await;
        let owner = user(&accounts, "Owner", Role::Editor).await;
        let other = user(&accounts, "Other", Role::Editor).await;
        let viewer = user(&accounts, "Viewer", Role::Viewer).await;
        Self {
            charts: Charts::new(accounts.clone()),
            types: Types::new(accounts.clone()),
            author: Authoring::new(accounts.clone()),
            accounts,
            pool,
            admin,
            owner,
            other,
            viewer,
            _dir: dir,
        }
    }
    async fn ty(&self, name: &str) -> TypeRecord {
        self.types
            .create_type(&self.admin.token, fields(name), None)
            .await
            .unwrap()
    }
    async fn world(&self, name: &str) -> WorldRecord {
        self.author
            .create_world(&self.owner.token, fields(name), None)
            .await
            .unwrap()
    }
    async fn local(&self, world: &str, source: Option<TypePin>, name: &str) -> WorldTypeRecord {
        let w = self.author.world_detail(world).await.unwrap();
        self.types
            .create_world_type(
                &self.owner.token,
                world,
                w.revision,
                source,
                fields(name),
                None,
            )
            .await
            .unwrap()
    }
    async fn shared(&self, name: &str, pins: Vec<TypePin>) -> ChartRecord {
        self.charts
            .create(&self.admin.token, None, new(name, pins))
            .await
            .unwrap()
    }
    async fn row(
        &self,
        world: Option<&str>,
        r: &ChartRecord,
        a: &str,
        cells: Vec<ChartCell>,
    ) -> ChartRecord {
        self.charts
            .save_row(
                if world.is_some() {
                    &self.owner.token
                } else {
                    &self.admin.token
                },
                world,
                &r.id,
                RowEdit {
                    expected_revision: r.revision,
                    attacker_type_id: a.into(),
                    cells,
                    reason: Some("Reviewed row".into()),
                },
            )
            .await
            .unwrap()
    }
    async fn count(&self, table: &str) -> i64 {
        sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {table}")))
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }
    async fn integrity(&self) {
        let issues: Vec<(String, i64, String, i64)> = sqlx::query_as("PRAGMA foreign_key_check")
            .fetch_all(&self.pool)
            .await
            .unwrap();
        assert!(issues.is_empty(), "{issues:?}");
        let integrity: String = sqlx::query_scalar("PRAGMA integrity_check")
            .fetch_one(&self.pool)
            .await
            .unwrap();
        assert_eq!(integrity, "ok");
    }
}

#[tokio::test]
async fn directed_unknown_neutral_zero_diagonal_and_canonical_noops() {
    let l = Lab::new().await;
    let a = l.ty("A").await;
    let b = l.ty("B").await;
    let r = l.shared("Chart", vec![pin(&a), pin(&b)]).await;
    assert_eq!(r.lookup(&a.id, &b.id), Effectiveness::Unknown);
    assert_eq!(r.lookup(&a.id, "outside"), Effectiveness::OutsideRoster);
    let r = l
        .row(
            None,
            &r,
            &a.id,
            vec![cell(&a.id, &b.id, 2, 4), cell(&a.id, &a.id, 0, 500)],
        )
        .await;
    assert_eq!(
        r.lookup(&a.id, &b.id),
        Effectiveness::Known(Factor::new(1, 2).unwrap())
    );
    assert_eq!(
        r.lookup(&a.id, &a.id),
        Effectiveness::Known(Factor::new(0, 1).unwrap())
    );
    assert_eq!(r.lookup(&b.id, &a.id), Effectiveness::Unknown);
    let audits = l.count("audit_events").await;
    let noop = l
        .row(
            None,
            &r,
            &a.id,
            vec![
                cell(&a.id, &a.id, 0, 999),
                cell(&a.id, &b.id, 500_000, 1_000_000),
            ],
        )
        .await;
    assert_eq!(noop, r);
    assert_eq!(l.count("audit_events").await, audits);
    let neutral = l
        .charts
        .fill_neutral(
            &l.admin.token,
            None,
            &r.id,
            NeutralEdit {
                expected_revision: r.revision,
                reason: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(neutral.cells.len(), 4);
    assert_eq!(neutral.lookup(&a.id, &b.id), r.lookup(&a.id, &b.id));
    assert_eq!(
        neutral.lookup(&b.id, &a.id),
        Effectiveness::Known(Factor::new(1, 1).unwrap())
    );
    assert_eq!(
        l.charts
            .fill_neutral(
                &l.admin.token,
                None,
                &r.id,
                NeutralEdit {
                    expected_revision: neutral.revision,
                    reason: None
                }
            )
            .await
            .unwrap(),
        neutral
    );
    let cleared = l.row(None, &neutral, &a.id, vec![]).await;
    assert_eq!(cleared.lookup(&a.id, &b.id), Effectiveness::Unknown);
    assert_eq!(
        cleared.lookup(&b.id, &a.id),
        Effectiveness::Known(Factor::new(1, 1).unwrap())
    );
    assert_eq!(l.charts.version(None, &r.id, 2).await.unwrap(), r);
    let h = l.charts.history_entry(None, &r.id, 2).await.unwrap();
    assert_eq!(
        (
            h.actor_name.as_str(),
            h.action.as_str(),
            h.reason.as_deref()
        ),
        ("Admin", "edit", Some("Reviewed row"))
    );
    l.integrity().await;
}

#[tokio::test]
async fn empty_drafts_shared_multiplicity_scoped_addresses_and_readonly_preview() {
    let l = Lab::new().await;
    let w = l.world("World").await;
    let before = l.count("audit_events").await;
    let draft = l
        .charts
        .preview_create(None, &new("Chárt", vec![]))
        .await
        .unwrap();
    assert!(draft.roster.is_empty());
    assert_eq!(draft.fields.slug, "chart");
    assert_eq!(l.count("audit_events").await, before);
    assert_eq!(
        l.charts.detail(None, &draft.id).await.unwrap_err(),
        Error::NotFound
    );
    let a = l.shared("Chárt", vec![]).await;
    let b = l.shared("Chart", vec![]).await;
    for (source, mapping, apply_source) in [
        (
            Some(ChartPin {
                chart_id: a.id.clone(),
                version: 1,
            }),
            vec![],
            false,
        ),
        (
            None,
            vec![ChartMapping {
                source_type_id: "a".repeat(32),
                local_type_id: "b".repeat(32),
            }],
            false,
        ),
        (None, vec![], true),
    ] {
        let mut invalid = new("Invalid shared", vec![]);
        invalid.source = source;
        invalid.mapping = mapping;
        invalid.apply_source = apply_source;
        assert!(matches!(
            l.charts.preview_create(None, &invalid).await,
            Err(Error::Invalid(_))
        ));
        assert!(matches!(
            l.charts.create(&l.admin.token, None, invalid).await,
            Err(Error::Invalid(_))
        ));
    }
    assert_eq!(b.fields.slug, "chart-2");
    assert_eq!(l.charts.charts().await.unwrap().len(), 2);
    assert_eq!(
        l.charts
            .resolve_shared(AddressKey::Slug(a.fields.slug.clone()))
            .await
            .unwrap(),
        a
    );
    assert!(l.charts.world_chart(&w.id).await.unwrap().is_none());
    let local = l
        .charts
        .create(&l.owner.token, Some(&w.id), new("Chart", vec![]))
        .await
        .unwrap();
    assert_eq!(local.fields.slug, "chart");
    assert_eq!(
        l.charts.world_chart(&w.id).await.unwrap(),
        Some(local.clone())
    );
    assert_eq!(
        l.charts
            .resolve_world(
                AddressKey::Slug(w.fields.slug.clone()),
                AddressKey::Slug("chart".into())
            )
            .await
            .unwrap()
            .1,
        local
    );
    assert_eq!(
        l.charts.detail(None, &local.id).await.unwrap_err(),
        Error::NotFound
    );
    assert_eq!(
        l.charts.detail(Some(&w.id), &a.id).await.unwrap_err(),
        Error::NotFound
    );
    assert!(matches!(
        l.charts
            .create(&l.owner.token, Some(&w.id), new("Second", vec![]))
            .await,
        Err(Error::Invalid(_))
    ));
    assert_eq!(
        serde_json::to_value(l.author.world_detail(&w.id).await.unwrap()).unwrap(),
        serde_json::to_value(&w).unwrap()
    );
    let mut rename = definition(&a);
    rename.fields = fields("Renamed");
    let renamed = l
        .charts
        .save_definition(&l.admin.token, None, &a.id, rename)
        .await
        .unwrap();
    assert_eq!(renamed.fields.slug, "renamed");
    assert_eq!(
        l.charts
            .resolve_shared(AddressKey::Slug("chart".into()))
            .await
            .unwrap_err(),
        Error::NotFound
    );
    assert_eq!(
        l.charts
            .resolve_shared(AddressKey::Id(a.id.clone()))
            .await
            .unwrap(),
        renamed
    );
    l.integrity().await;
}

#[tokio::test]
async fn stale_precedes_all_submitted_dependency_validation_and_noops_preserve_audit() {
    let l = Lab::new().await;
    let a = l.ty("A").await;
    let r = l.shared("Chart", vec![pin(&a)]).await;
    let mut noop = definition(&r);
    noop.fields.slug = "forged".into();
    let before = l.count("audit_events").await;
    assert_eq!(
        l.charts
            .save_definition(&l.admin.token, None, &r.id, noop)
            .await
            .unwrap(),
        r
    );
    assert_eq!(l.count("audit_events").await, before);
    let mut invalid = definition(&r);
    invalid.expected_revision = 0;
    invalid.roster = vec![TypePin {
        type_id: "missing".into(),
        version: -1,
    }];
    invalid.fields.display_name.clear();
    assert_eq!(
        l.charts
            .save_definition(&l.admin.token, None, &r.id, invalid)
            .await
            .unwrap_err(),
        Error::Conflict
    );
    assert_eq!(
        l.charts
            .save_row(
                &l.admin.token,
                None,
                &r.id,
                RowEdit {
                    expected_revision: 0,
                    attacker_type_id: "outside".into(),
                    cells: vec![cell("outside", "outside", 1, 0)],
                    reason: None
                }
            )
            .await
            .unwrap_err(),
        Error::Conflict
    );
    assert_eq!(
        l.charts
            .fill_neutral(
                &l.admin.token,
                None,
                &r.id,
                NeutralEdit {
                    expected_revision: 0,
                    reason: Some("\0".into())
                }
            )
            .await
            .unwrap_err(),
        Error::Conflict
    );
    assert_eq!(l.count("audit_events").await, before);
}

#[tokio::test]
async fn rows_reject_duplicates_wrong_attackers_outside_defenders_and_invalid_factors() {
    let l = Lab::new().await;
    let a = l.ty("A").await;
    let b = l.ty("B").await;
    let r = l.shared("Chart", vec![pin(&a), pin(&b)]).await;
    let before = l.count("audit_events").await;
    for cells in [
        vec![cell(&a.id, &a.id, 1, 1), cell(&a.id, &a.id, 2, 2)],
        vec![cell(&b.id, &a.id, 1, 1)],
        vec![cell(&a.id, "outside", 1, 1)],
        vec![cell(&a.id, &b.id, 1_000_001, 1)],
        vec![cell(&a.id, &b.id, 1, 0)],
    ] {
        assert!(matches!(
            l.charts
                .save_row(
                    &l.admin.token,
                    None,
                    &r.id,
                    RowEdit {
                        expected_revision: r.revision,
                        attacker_type_id: a.id.clone(),
                        cells,
                        reason: None
                    }
                )
                .await,
            Err(Error::Invalid(_))
        ));
    }
    assert_eq!(l.charts.detail(None, &r.id).await.unwrap(), r);
    assert_eq!(l.count("audit_events").await, before);
}

#[tokio::test]
async fn independent_roster_bound_exact_scope_and_metadata_identity_compatibility() {
    let l = Lab::new().await;
    let a = l.ty("A").await;
    let b = l.ty("B").await;
    let r = l.shared("Chart", vec![pin(&a), pin(&b)]).await;
    let r = l.row(None, &r, &a.id, vec![cell(&a.id, &b.id, 2, 1)]).await;
    let a2 = l
        .types
        .save_type(
            &l.admin.token,
            &a.id,
            Edit {
                expected_revision: 1,
                fields: fields("A metadata v2"),
                reason: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(l.charts.detail(None, &r.id).await.unwrap(), r);
    let mut edit = definition(&r);
    edit.roster[0] = pin(&a2);
    let updated = l
        .charts
        .save_definition(&l.admin.token, None, &r.id, edit)
        .await
        .unwrap();
    // An old exact metadata assignment still addresses the same mechanical ID.
    assert_eq!(
        updated.lookup(&a.id, &b.id),
        Effectiveness::Known(Factor::new(2, 1).unwrap())
    );
    assert_eq!(updated.roster[0].fields, a2.fields);
    assert_eq!(r.roster[0].fields, a.fields);
    let w = l.world("World").await;
    let local = l.local(&w.id, None, "Exclusive").await;
    assert!(matches!(
        l.charts
            .create(&l.owner.token, Some(&w.id), new("Wrong", vec![pin(&a)]))
            .await,
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        l.charts
            .create(&l.admin.token, None, new("Wrong", vec![local_pin(&local)]))
            .await,
        Err(Error::Invalid(_))
    ));
    let w2 = l.world("Other world").await;
    assert!(matches!(
        l.charts
            .create(
                &l.owner.token,
                Some(&w2.id),
                new("Wrong", vec![local_pin(&local)])
            )
            .await,
        Err(Error::Invalid(_))
    ));
    let mut pins = vec![];
    for n in 0..33 {
        pins.push(pin(&l.ty(&format!("Bound {n}")).await));
    }
    assert!(matches!(
        l.charts
            .create(&l.admin.token, None, new("Too many", pins.clone()))
            .await,
        Err(Error::Invalid(_))
    ));
    pins.pop();
    let max = l.shared("Bound", pins).await;
    let full = l
        .charts
        .fill_neutral(
            &l.admin.token,
            None,
            &max.id,
            NeutralEdit {
                expected_revision: 1,
                reason: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(full.cells.len(), 1024);
    assert!(full.validate().is_ok());
    let mut duplicate = definition(&updated);
    duplicate.roster.push(pin(&a));
    assert!(matches!(
        l.charts
            .save_definition(&l.admin.token, None, &r.id, duplicate)
            .await,
        Err(Error::Invalid(_))
    ));
    l.integrity().await;
}

#[tokio::test]
async fn locked_authorization_has_no_local_admin_bypass_and_observes_revocation() {
    let l = Lab::new().await;
    for u in [&l.owner, &l.other, &l.viewer] {
        assert_eq!(
            l.charts
                .create(&u.token, None, new("Denied", vec![]))
                .await
                .unwrap_err(),
            Error::Forbidden
        );
    }
    assert_eq!(
        l.charts
            .create("invalid", None, new("Denied", vec![]))
            .await
            .unwrap_err(),
        Error::Unauthorized
    );
    let w = l.world("World").await;
    for u in [&l.admin, &l.other, &l.viewer] {
        assert_eq!(
            l.charts
                .create(&u.token, Some(&w.id), new("Denied", vec![]))
                .await
                .unwrap_err(),
            Error::Forbidden
        );
    }
    let r = l
        .charts
        .create(&l.owner.token, Some(&w.id), new("Local", vec![]))
        .await
        .unwrap();
    for u in [&l.admin, &l.other, &l.viewer] {
        assert_eq!(
            l.charts
                .save_definition(&u.token, Some(&w.id), &r.id, definition(&r))
                .await
                .unwrap_err(),
            Error::Forbidden
        );
        assert_eq!(
            l.charts
                .save_row(
                    &u.token,
                    Some(&w.id),
                    &r.id,
                    RowEdit {
                        expected_revision: 1,
                        attacker_type_id: "invalid".into(),
                        cells: vec![],
                        reason: None
                    }
                )
                .await
                .unwrap_err(),
            Error::Forbidden
        );
        assert_eq!(
            l.charts
                .fill_neutral(
                    &u.token,
                    Some(&w.id),
                    &r.id,
                    NeutralEdit {
                        expected_revision: 1,
                        reason: None
                    }
                )
                .await
                .unwrap_err(),
            Error::Forbidden
        );
        assert_eq!(
            l.charts
                .adopt(
                    &u.token,
                    &w.id,
                    &r.id,
                    AdoptEdit {
                        expected_revision: 1,
                        target_version: 2,
                        mapping: vec![],
                        apply_source: false,
                        reason: None
                    }
                )
                .await
                .unwrap_err(),
            Error::Forbidden
        );
    }
    l.author
        .grant_collaborator(&l.owner.token, &w.id, 1, &l.other.actor.user_id, None)
        .await
        .unwrap();
    assert_eq!(
        l.charts
            .save_definition(&l.other.token, Some(&w.id), &r.id, definition(&r))
            .await
            .unwrap(),
        r
    );
    l.author
        .revoke_collaborator(&l.owner.token, &w.id, 2, &l.other.actor.user_id, None)
        .await
        .unwrap();
    assert_eq!(
        l.charts
            .save_definition(&l.other.token, Some(&w.id), &r.id, definition(&r))
            .await
            .unwrap_err(),
        Error::Forbidden
    );
    // Query-time eligibility: even an already issued session cannot retain its old role.
    sqlx::query("UPDATE users SET role='viewer' WHERE id=?")
        .bind(&l.owner.actor.user_id)
        .execute(&l.pool)
        .await
        .unwrap();
    assert_eq!(
        l.charts
            .save_definition(&l.owner.token, Some(&w.id), &r.id, definition(&r))
            .await
            .unwrap_err(),
        Error::Forbidden
    );
    sqlx::query("UPDATE users SET role='editor' WHERE id=?")
        .bind(&l.owner.actor.user_id)
        .execute(&l.pool)
        .await
        .unwrap();
    sqlx::query("UPDATE sessions SET revoked_at=1 WHERE user_id=?")
        .bind(&l.owner.actor.user_id)
        .execute(&l.pool)
        .await
        .unwrap();
    assert_eq!(
        l.charts
            .save_definition(&l.owner.token, Some(&w.id), &r.id, definition(&r))
            .await
            .unwrap_err(),
        Error::Unauthorized
    );
    // Disabling through the real account workflow revokes the administrator too.
    l.accounts
        .set_disabled(&l.admin.actor.user_id, true)
        .await
        .unwrap();
    assert_eq!(
        l.charts
            .create(&l.admin.token, None, new("Denied", vec![]))
            .await
            .unwrap_err(),
        Error::Unauthorized
    );
}

#[tokio::test]
async fn mapping_identity_bijection_partial_mapping_and_explicit_source_adoption() {
    let l = Lab::new().await;
    let a = l.ty("A").await;
    let b = l.ty("B").await;
    let source = l.shared("Source", vec![pin(&a), pin(&b)]).await;
    let source = l
        .row(None, &source, &a.id, vec![cell(&a.id, &b.id, 2, 1)])
        .await;
    let a2 = l
        .types
        .save_type(
            &l.admin.token,
            &a.id,
            Edit {
                expected_revision: 1,
                fields: fields("A metadata v2"),
                reason: None,
            },
        )
        .await
        .unwrap();
    let w = l.world("World").await;
    let la = l.local(&w.id, Some(pin(&a2)), "Local A").await;
    let lb = l.local(&w.id, Some(pin(&b)), "Local B").await;
    let exclusive = l.local(&w.id, None, "Exclusive").await;
    let alias = l.local(&w.id, Some(pin(&a)), "Another local A").await;
    let mappings = vec![
        ChartMapping {
            source_type_id: b.id.clone(),
            local_type_id: lb.id.clone(),
        },
        ChartMapping {
            source_type_id: a.id.clone(),
            local_type_id: la.id.clone(),
        },
    ];
    let mut create = new(
        "Local",
        vec![
            local_pin(&la),
            local_pin(&lb),
            local_pin(&exclusive),
            local_pin(&alias),
        ],
    );
    create.source = Some(ChartPin {
        chart_id: source.id.clone(),
        version: source.revision,
    });
    create.mapping = mappings.clone();
    create.apply_source = true;
    let local = l
        .charts
        .create(&l.owner.token, Some(&w.id), create.clone())
        .await
        .unwrap();
    assert_eq!(
        local.lookup(&la.id, &lb.id),
        Effectiveness::Known(Factor::new(2, 1).unwrap())
    );
    assert_eq!(local.lookup(&lb.id, &la.id), Effectiveness::Unknown);
    assert_eq!(local.lookup(&exclusive.id, &la.id), Effectiveness::Unknown);
    assert!(
        local
            .mapping
            .windows(2)
            .all(|w| w[0].source_type_id < w[1].source_type_id)
    );
    // Chart/Type source metadata versions may differ, but identities may not.
    let mut invalid = definition(&local);
    invalid.mapping.push(ChartMapping {
        source_type_id: a.id.clone(),
        local_type_id: alias.id.clone(),
    });
    assert!(matches!(
        l.charts
            .save_definition(&l.owner.token, Some(&w.id), &local.id, invalid)
            .await,
        Err(Error::Invalid(_))
    ));
    for map in [
        vec![ChartMapping {
            source_type_id: a.id.clone(),
            local_type_id: exclusive.id.clone(),
        }],
        vec![ChartMapping {
            source_type_id: a.id.clone(),
            local_type_id: lb.id.clone(),
        }],
        vec![
            ChartMapping {
                source_type_id: a.id.clone(),
                local_type_id: la.id.clone(),
            },
            ChartMapping {
                source_type_id: b.id.clone(),
                local_type_id: la.id.clone(),
            },
        ],
    ] {
        let mut invalid = definition(&local);
        invalid.mapping = map;
        assert!(matches!(
            l.charts
                .save_definition(&l.owner.token, Some(&w.id), &local.id, invalid)
                .await,
            Err(Error::Invalid(_))
        ));
    }
    let local = l
        .row(
            Some(&w.id),
            &local,
            &exclusive.id,
            vec![cell(&exclusive.id, &la.id, 3, 1)],
        )
        .await;
    let local = l
        .row(
            Some(&w.id),
            &local,
            &lb.id,
            vec![cell(&lb.id, &la.id, 4, 1)],
        )
        .await;
    let world_before = l.author.world_detail(&w.id).await.unwrap();
    let source3 = l.row(None, &source, &a.id, vec![]).await;
    assert_eq!(
        l.charts.detail(Some(&w.id), &local.id).await.unwrap(),
        local
    );
    let adopt = AdoptEdit {
        expected_revision: local.revision,
        target_version: source3.revision,
        mapping: mappings.clone(),
        apply_source: true,
        reason: Some("Reviewed source changes".into()),
    };
    let audits = l.count("audit_events").await;
    let preview = l
        .charts
        .adoption_preview(&w.id, &local.id, &adopt)
        .await
        .unwrap();
    assert_eq!(preview.entry, local);
    assert_eq!(preview.old_source, source);
    assert_eq!(preview.new_source, source3);
    assert_eq!(
        preview.proposed.lookup(&la.id, &lb.id),
        Effectiveness::Unknown
    );
    assert_eq!(
        preview.proposed.lookup(&lb.id, &la.id),
        Effectiveness::Unknown
    );
    assert_eq!(
        preview.proposed.lookup(&exclusive.id, &la.id),
        Effectiveness::Known(Factor::new(3, 1).unwrap())
    );
    assert_eq!(l.count("audit_events").await, audits);
    let adopted = l
        .charts
        .adopt(&l.owner.token, &w.id, &local.id, adopt.clone())
        .await
        .unwrap();
    assert_eq!(adopted, preview.proposed);
    assert_eq!(
        serde_json::to_value(l.author.world_detail(&w.id).await.unwrap()).unwrap(),
        serde_json::to_value(&world_before).unwrap()
    );
    assert_eq!(l.types.world_type_detail(&w.id, &la.id).await.unwrap(), la);
    assert_eq!(
        l.charts
            .version(Some(&w.id), &local.id, local.revision)
            .await
            .unwrap(),
        local
    );
    assert_eq!(
        l.charts
            .adopt(&l.owner.token, &w.id, &local.id, adopt)
            .await
            .unwrap_err(),
        Error::Conflict
    );
    // Retaining local policy advances only the explicit chart source/mapping pin.
    let source4 = l
        .row(None, &source3, &a.id, vec![cell(&a.id, &b.id, 9, 1)])
        .await;
    let retained = l
        .charts
        .adopt(
            &l.owner.token,
            &w.id,
            &local.id,
            AdoptEdit {
                expected_revision: adopted.revision,
                target_version: source4.revision,
                mapping: vec![],
                apply_source: false,
                reason: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(retained.cells, adopted.cells);
    assert!(retained.mapping.is_empty());
    assert_eq!(retained.roster, local.roster);
    let mut partial = definition(&retained);
    partial.mapping = vec![mappings[0].clone()];
    let partial = l
        .charts
        .save_definition(&l.owner.token, Some(&w.id), &local.id, partial)
        .await
        .unwrap();
    assert_eq!(partial.mapping.len(), 1);
    l.integrity().await;
}

#[tokio::test]
async fn roster_removal_preserves_retained_pairs_and_historical_cells_mappings() {
    let l = Lab::new().await;
    let a = l.ty("A").await;
    let b = l.ty("B").await;
    let source = l.shared("Source", vec![pin(&a), pin(&b)]).await;
    let w = l.world("World").await;
    let la = l.local(&w.id, Some(pin(&a)), "A").await;
    let lb = l.local(&w.id, Some(pin(&b)), "B").await;
    let mut create = new("Local", vec![local_pin(&la), local_pin(&lb)]);
    create.source = Some(ChartPin {
        chart_id: source.id.clone(),
        version: 1,
    });
    create.mapping = vec![
        ChartMapping {
            source_type_id: a.id.clone(),
            local_type_id: la.id.clone(),
        },
        ChartMapping {
            source_type_id: b.id.clone(),
            local_type_id: lb.id.clone(),
        },
    ];
    let local = l
        .charts
        .create(&l.owner.token, Some(&w.id), create)
        .await
        .unwrap();
    let local = l
        .row(
            Some(&w.id),
            &local,
            &la.id,
            vec![cell(&la.id, &la.id, 2, 1), cell(&la.id, &lb.id, 3, 1)],
        )
        .await;
    let mut edit = definition(&local);
    edit.roster.retain(|p| p.type_id != lb.id);
    let trimmed = l
        .charts
        .save_definition(&l.owner.token, Some(&w.id), &local.id, edit)
        .await
        .unwrap();
    assert_eq!(trimmed.cells, vec![cell(&la.id, &la.id, 2, 1)]);
    assert_eq!(trimmed.mapping.len(), 1);
    assert_eq!(trimmed.lookup(&la.id, &lb.id), Effectiveness::OutsideRoster);
    assert_eq!(
        l.charts
            .version(Some(&w.id), &local.id, local.revision)
            .await
            .unwrap(),
        local
    );
    let mut readd = definition(&trimmed);
    readd.roster.push(local_pin(&lb));
    let readded = l
        .charts
        .save_definition(&l.owner.token, Some(&w.id), &local.id, readd)
        .await
        .unwrap();
    assert_eq!(readded.lookup(&la.id, &lb.id), Effectiveness::Unknown);
    assert_eq!(readded.mapping.len(), 1);
    // A newer shared roster can remove Types; adoption requires explicit removal
    // of mappings to them and keeps local cells under the retain policy.
    let mut edit = definition(&source);
    edit.roster.retain(|p| p.type_id != b.id);
    let source2 = l
        .charts
        .save_definition(&l.admin.token, None, &source.id, edit)
        .await
        .unwrap();
    let mut adopt = AdoptEdit {
        expected_revision: readded.revision,
        target_version: source2.revision,
        mapping: local.mapping.clone(),
        apply_source: false,
        reason: None,
    };
    assert!(matches!(
        l.charts.adoption_preview(&w.id, &local.id, &adopt).await,
        Err(Error::Invalid(_))
    ));
    adopt.mapping = trimmed.mapping;
    let result = l
        .charts
        .adopt(&l.owner.token, &w.id, &local.id, adopt)
        .await
        .unwrap();
    assert_eq!(result.roster, readded.roster);
    assert_eq!(result.cells, readded.cells);
    l.integrity().await;
}

#[tokio::test]
async fn typed_children_are_sealed_against_late_insertion_update_and_delete() {
    let l = Lab::new().await;
    let a = l.ty("A").await;
    let b = l.ty("B").await;
    let r = l.shared("Chart", vec![pin(&a), pin(&b)]).await;
    let r = l.row(None, &r, &a.id, vec![cell(&a.id, &b.id, 2, 1)]).await;
    assert!(sqlx::query("INSERT INTO chart_roster(scope,chart_id,revision,slot,type_id,type_version,shared_type_id) VALUES ('',?,1,3,?,1,?)").bind(&r.id).bind(&a.id).bind(&a.id).execute(&l.pool).await.is_err());
    assert!(sqlx::query("INSERT INTO chart_cells(scope,chart_id,revision,attacker_type_id,defender_type_id,numerator,denominator) VALUES ('',?,1,?,?,1,1)").bind(&r.id).bind(&a.id).bind(&a.id).execute(&l.pool).await.is_err());
    for table in ["chart_versions", "chart_roster", "chart_cells"] {
        let column = if table == "chart_roster" {
            "slot"
        } else if table == "chart_cells" {
            "numerator"
        } else {
            "revision"
        };
        for sql in [
            format!("UPDATE {table} SET {column}={column}"),
            format!("DELETE FROM {table}"),
        ] {
            assert!(
                sqlx::query(sqlx::AssertSqlSafe(sql))
                    .execute(&l.pool)
                    .await
                    .is_err()
            );
        }
    }
    let w = l.world("World").await;
    let la = l.local(&w.id, Some(pin(&a)), "Local A").await;
    let mut create = new("Local", vec![local_pin(&la)]);
    create.source = Some(ChartPin {
        chart_id: r.id.clone(),
        version: r.revision,
    });
    create.mapping = vec![ChartMapping {
        source_type_id: a.id.clone(),
        local_type_id: la.id.clone(),
    }];
    let local = l
        .charts
        .create(&l.owner.token, Some(&w.id), create)
        .await
        .unwrap();
    assert!(sqlx::query("INSERT INTO chart_mapping(scope,chart_id,revision,source_chart_id,source_revision,source_type_id,local_type_id) VALUES (?,?,1,?,?,?,?)").bind(&w.id).bind(&local.id).bind(&r.id).bind(r.revision).bind(&a.id).bind(&la.id).execute(&l.pool).await.is_err());
    for sql in [
        "UPDATE chart_mapping SET source_type_id=source_type_id",
        "DELETE FROM chart_mapping",
    ] {
        assert!(l.pool.execute(sql).await.is_err());
    }
    assert_eq!(
        l.charts.detail(Some(&w.id), &local.id).await.unwrap(),
        local
    );
    l.integrity().await;
}

#[tokio::test]
async fn late_write_failure_rolls_back_head_revision_children_and_audit() {
    let l = Lab::new().await;
    let a = l.ty("A").await;
    let r = l.shared("Chart", vec![pin(&a)]).await;
    let audits = l.count("audit_events").await;
    l.pool.execute("CREATE TRIGGER chart_test_late_failure BEFORE INSERT ON chart_versions WHEN NEW.revision=2 BEGIN SELECT RAISE(ABORT,'late failure'); END").await.unwrap();
    assert_eq!(
        l.charts
            .save_row(
                &l.admin.token,
                None,
                &r.id,
                RowEdit {
                    expected_revision: 1,
                    attacker_type_id: a.id.clone(),
                    cells: vec![cell(&a.id, &a.id, 2, 1)],
                    reason: None
                }
            )
            .await
            .unwrap_err(),
        Error::Unavailable
    );
    assert_eq!(l.charts.detail(None, &r.id).await.unwrap(), r);
    assert_eq!(l.count("audit_events").await, audits);
    assert_eq!(l.count("chart_versions").await, 1);
    assert_eq!(l.count("chart_roster").await, 1);
    assert_eq!(l.count("chart_cells").await, 0);
    l.integrity().await;
}

#[tokio::test]
async fn readers_fail_closed_on_unknown_missing_noncanonical_and_disagreeing_snapshots() {
    let l = Lab::new().await;
    let a = l.ty("A").await;
    let r = l.shared("Chart", vec![pin(&a)]).await;
    let r = l.row(None, &r, &a.id, vec![cell(&a.id, &a.id, 1, 2)]).await;
    l.pool
        .execute("DROP TRIGGER chart_versions_update")
        .await
        .unwrap();
    let original: String =
        sqlx::query_scalar("SELECT after_json FROM chart_versions WHERE chart_id=? AND revision=2")
            .bind(&r.id)
            .fetch_one(&l.pool)
            .await
            .unwrap();
    let value: serde_json::Value = serde_json::from_str(&original).unwrap();
    let mut bads = vec![];
    let mut v = value.clone();
    v["content"]["extra"] = true.into();
    bads.push(v);
    let mut v = value.clone();
    v["content"].as_object_mut().unwrap().remove("source");
    bads.push(v);
    let mut v = value.clone();
    v["content"]["roster"][0]["fields"]["display_name"] = "Forged metadata".into();
    bads.push(v);
    let mut v = value.clone();
    v["content"]["cells"][0]["factor"] = serde_json::json!({"numerator":2,"denominator":4});
    bads.push(v);
    let mut v = value.clone();
    v["content"]["cells"] = serde_json::json!([]);
    bads.push(v);
    let mut v = value.clone();
    v["content"]["fields"]["description"] = "Different metadata".into();
    bads.push(v);
    for bad in bads {
        sqlx::query("UPDATE chart_versions SET after_json=? WHERE chart_id=? AND revision=2")
            .bind(bad.to_string())
            .bind(&r.id)
            .execute(&l.pool)
            .await
            .unwrap();
        assert_eq!(
            l.charts.detail(None, &r.id).await.unwrap_err(),
            Error::Unavailable
        );
        assert_eq!(
            l.charts.version(None, &r.id, 2).await.unwrap_err(),
            Error::Unavailable
        );
        assert_eq!(
            l.charts.history(None, &r.id, None).await.unwrap_err(),
            Error::Unavailable
        );
    }
    sqlx::query("UPDATE chart_versions SET after_json=? WHERE chart_id=? AND revision=2")
        .bind(&original)
        .bind(&r.id)
        .execute(&l.pool)
        .await
        .unwrap();
    assert_eq!(l.charts.detail(None, &r.id).await.unwrap(), r);
    // The current after can be correct while its before lies about the prior seal.
    sqlx::query("UPDATE chart_versions SET before_json=after_json WHERE chart_id=? AND revision=2")
        .bind(&r.id)
        .execute(&l.pool)
        .await
        .unwrap();
    assert_eq!(
        l.charts.history_entry(None, &r.id, 2).await.unwrap_err(),
        Error::Unavailable
    );
}

#[tokio::test]
async fn readers_verify_exact_dependency_metadata_and_typed_rows_not_only_json() {
    let l = Lab::new().await;
    let a = l.ty("A").await;
    let r = l.shared("Chart", vec![pin(&a)]).await;
    let r = l.row(None, &r, &a.id, vec![cell(&a.id, &a.id, 2, 1)]).await;
    l.pool
        .execute("DROP TRIGGER chart_cells_update")
        .await
        .unwrap();
    sqlx::query("UPDATE chart_cells SET numerator=3 WHERE chart_id=? AND revision=2")
        .bind(&r.id)
        .execute(&l.pool)
        .await
        .unwrap();
    assert_eq!(
        l.charts.detail(None, &r.id).await.unwrap_err(),
        Error::Unavailable
    );
    sqlx::query("UPDATE chart_cells SET numerator=2 WHERE chart_id=? AND revision=2")
        .bind(&r.id)
        .execute(&l.pool)
        .await
        .unwrap();
    l.pool
        .execute("DROP TRIGGER type_versions_update")
        .await
        .unwrap();
    sqlx::query("UPDATE type_versions SET after_json=json_set(after_json,'$.content.fields.display_name','Wrong') WHERE type_id=? AND revision=1").bind(&a.id).execute(&l.pool).await.unwrap();
    assert_eq!(
        l.charts.version(None, &r.id, 1).await.unwrap_err(),
        Error::Unavailable
    );
    assert_eq!(
        l.charts.detail(None, &r.id).await.unwrap_err(),
        Error::Unavailable
    );
}

#[tokio::test]
async fn chart_history_is_bounded_exclusive_and_exact_while_shared_mutations_require_admin() {
    let l = Lab::new().await;
    let a = l.ty("A").await;
    let initial = l.shared("History", vec![pin(&a)]).await;
    for u in [&l.owner, &l.other, &l.viewer] {
        assert_eq!(
            l.charts
                .save_definition(&u.token, None, &initial.id, definition(&initial))
                .await
                .unwrap_err(),
            Error::Forbidden
        );
        assert_eq!(
            l.charts
                .save_row(
                    &u.token,
                    None,
                    &initial.id,
                    RowEdit {
                        expected_revision: 1,
                        attacker_type_id: a.id.clone(),
                        cells: vec![],
                        reason: None
                    }
                )
                .await
                .unwrap_err(),
            Error::Forbidden
        );
        assert_eq!(
            l.charts
                .fill_neutral(
                    &u.token,
                    None,
                    &initial.id,
                    NeutralEdit {
                        expected_revision: 1,
                        reason: None
                    }
                )
                .await
                .unwrap_err(),
            Error::Forbidden
        );
    }
    let mut row = initial.clone();
    for n in 1..=54 {
        row = l
            .row(None, &row, &a.id, vec![cell(&a.id, &a.id, n, 1)])
            .await;
    }
    let first = l.charts.history(None, &row.id, None).await.unwrap();
    assert_eq!(first.len(), 50);
    assert_eq!(first.first().unwrap().revision, 55);
    assert_eq!(first.last().unwrap().revision, 6);
    let second = l.charts.history(None, &row.id, Some(6)).await.unwrap();
    assert_eq!(
        second.iter().map(|h| h.revision).collect::<Vec<_>>(),
        vec![5, 4, 3, 2, 1]
    );
    assert!(
        l.charts
            .history(None, &row.id, Some(1))
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        l.charts.history_entry(None, &row.id, 56).await.unwrap_err(),
        Error::NotFound
    );
    assert_eq!(l.charts.version(None, &row.id, 1).await.unwrap(), initial);
    assert_eq!(l.charts.detail(None, &row.id).await.unwrap(), row);
    l.integrity().await;
}

#[tokio::test]
async fn unsupported_snapshot_versions_and_missing_sealed_rows_fail_closed() {
    let l = Lab::new().await;
    let a = l.ty("A").await;
    let r = l.shared("Chart", vec![pin(&a)]).await;
    let r = l.row(None, &r, &a.id, vec![cell(&a.id, &a.id, 1, 2)]).await;
    l.pool
        .execute("DROP TRIGGER chart_versions_update")
        .await
        .unwrap();
    // Deliberately damage only this test-owned database, including bypassing its
    // version CHECK, to exercise the reader rather than just SQLite rejection.
    let mut c = l.pool.acquire().await.unwrap();
    c.execute("PRAGMA ignore_check_constraints=ON")
        .await
        .unwrap();
    sqlx::query("UPDATE chart_versions SET after_json=json_set(after_json,'$.snapshot_version',99) WHERE chart_id=? AND revision=2").bind(&r.id).execute(&mut *c).await.unwrap();
    c.execute("PRAGMA ignore_check_constraints=OFF")
        .await
        .unwrap();
    drop(c);
    assert_eq!(
        l.charts.version(None, &r.id, 2).await.unwrap_err(),
        Error::Unavailable
    );
    assert_eq!(
        l.charts.history_entry(None, &r.id, 2).await.unwrap_err(),
        Error::Unavailable
    );
    sqlx::query("UPDATE chart_versions SET after_json=json_set(after_json,'$.snapshot_version',1) WHERE chart_id=? AND revision=2").bind(&r.id).execute(&l.pool).await.unwrap();
    assert_eq!(l.charts.detail(None, &r.id).await.unwrap(), r);
    l.pool
        .execute("DROP TRIGGER chart_cells_delete")
        .await
        .unwrap();
    sqlx::query("DELETE FROM chart_cells WHERE chart_id=? AND revision=2")
        .bind(&r.id)
        .execute(&l.pool)
        .await
        .unwrap();
    assert_eq!(
        l.charts.detail(None, &r.id).await.unwrap_err(),
        Error::Unavailable
    );
    assert_eq!(
        l.charts.history(None, &r.id, None).await.unwrap_err(),
        Error::Unavailable
    );
    assert_eq!(l.charts.version(None, &r.id, 1).await.unwrap().revision, 1);
}

#[tokio::test]
async fn exact_versions_and_source_pins_check_live_slug_but_preserve_historical_names() {
    let l = Lab::new().await;
    let shared = l.shared("Original chart", vec![]).await;
    let world = l.world("World").await;
    let mut create = new("Local chart", vec![]);
    create.source = Some(ChartPin {
        chart_id: shared.id.clone(),
        version: 1,
    });
    let local = l
        .charts
        .create(&l.owner.token, Some(&world.id), create)
        .await
        .unwrap();

    // Damage only the live head in this test-owned database. Exact-version and
    // source-dependency reads must detect the same disagreement as detail reads.
    sqlx::query("UPDATE charts SET slug='inconsistent-head' WHERE id=?")
        .bind(&shared.id)
        .execute(&l.pool)
        .await
        .unwrap();
    assert_eq!(
        l.charts.detail(None, &shared.id).await.unwrap_err(),
        Error::Unavailable
    );
    assert_eq!(
        l.charts.version(None, &shared.id, 1).await.unwrap_err(),
        Error::Unavailable
    );
    assert_eq!(
        l.charts
            .detail(Some(&world.id), &local.id)
            .await
            .unwrap_err(),
        Error::Unavailable
    );

    sqlx::query("UPDATE charts SET slug=? WHERE id=?")
        .bind(&shared.fields.slug)
        .bind(&shared.id)
        .execute(&l.pool)
        .await
        .unwrap();
    let mut rename = definition(&shared);
    rename.fields = fields("Renamed chart");
    let renamed = l
        .charts
        .save_definition(&l.admin.token, None, &shared.id, rename)
        .await
        .unwrap();
    assert_eq!(renamed.revision, 2);
    assert_ne!(renamed.fields.slug, shared.fields.slug);
    assert_eq!(l.charts.detail(None, &shared.id).await.unwrap(), renamed);
    assert_eq!(l.charts.version(None, &shared.id, 1).await.unwrap(), shared);
    assert_eq!(
        l.charts.detail(Some(&world.id), &local.id).await.unwrap(),
        local
    );
    l.integrity().await;
}
