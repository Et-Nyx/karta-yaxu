use game_data::{authoring::ContentFields, stats::*};
use sqlx::{Executor, SqlitePool};
use tempfile::TempDir;
use web_app::{
    accounts::Accounts,
    application::{
        authoring::{AddressKey, Authoring, Error, WorldRecord},
        stats::{DefinitionEdit, NewStat, Stats},
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
    stats: Stats,
    author: Authoring,
    admin: User,
    owner: User,
    other: User,
    viewer: User,
    _dir: TempDir,
}
fn fields(name: &str) -> ContentFields {
    ContentFields {
        slug: "forged / slug".into(),
        display_name: name.into(),
        description: "Independent synthetic stat.".into(),
    }
}
fn new(name: &str) -> NewStat {
    NewStat {
        fields: fields(name),
        constraints: NumericConstraints::default(),
        source: None,
        reason: Some("Initial design".into()),
    }
}
fn edit(row: &StatDefinition) -> DefinitionEdit {
    DefinitionEdit {
        expected_revision: row.revision,
        fields: row.fields.clone(),
        constraints: row.constraints.clone(),
        reason: None,
    }
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
            "sqlite://{}/stats.db",
            dir.path().display()
        ))
        .await
        .unwrap();
        persistence::migrate(&pool).await.unwrap();
        let accounts = Accounts::new(pool.clone(), 3600).unwrap();
        let admin = user(&accounts, "Admin", Role::Administrator).await;
        let owner = user(&accounts, "Owner", Role::Editor).await;
        let other = user(&accounts, "Other", Role::Editor).await;
        let viewer = user(&accounts, "Viewer", Role::Viewer).await;
        Self {
            stats: Stats::new(accounts.clone()),
            author: Authoring::new(accounts.clone()),
            pool,
            accounts,
            admin,
            owner,
            other,
            viewer,
            _dir: dir,
        }
    }
    async fn world(&self, name: &str) -> WorldRecord {
        self.author
            .create_world(&self.owner.token, fields(name), None)
            .await
            .unwrap()
    }
    async fn shared(&self, name: &str) -> StatDefinition {
        self.stats
            .create(&self.admin.token, None, new(name))
            .await
            .unwrap()
    }
    async fn local(&self, world: &str, name: &str, source: Option<StatPin>) -> StatDefinition {
        let mut n = new(name);
        n.source = source;
        self.stats
            .create(&self.owner.token, Some(world), n)
            .await
            .unwrap()
    }
    async fn audits(&self) -> i64 {
        sqlx::query_scalar("SELECT count(*) FROM audit_events")
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
        let result: String = sqlx::query_scalar("PRAGMA integrity_check")
            .fetch_one(&self.pool)
            .await
            .unwrap();
        assert_eq!(result, "ok");
    }
}

#[tokio::test]
async fn exact_constraints_canonical_noops_and_public_history() {
    let l = Lab::new().await;
    let mut n = new("  Signed   Capacity  ");
    n.constraints = NumericConstraints {
        whole_only: false,
        minimum: Some("-1.2500".parse().unwrap()),
        maximum: Some("9223372036854.775807".parse().unwrap()),
    };
    let first = l.stats.create(&l.admin.token, None, n).await.unwrap();
    assert_eq!(first.fields.slug, "signed-capacity");
    assert_eq!(first.constraints.minimum.unwrap().to_string(), "-1.25");
    let audits = l.audits().await;
    let mut e = edit(&first);
    e.fields = fields(" Signed   Capacity ");
    e.constraints.minimum = Some(Decimal::new(-12500, 4).unwrap());
    assert_eq!(
        l.stats
            .save_definition(&l.admin.token, None, &first.id, e)
            .await
            .unwrap(),
        first
    );
    assert_eq!(l.audits().await, audits);
    let mut e = edit(&first);
    e.fields.display_name = "Renamed signed stat".into();
    e.constraints = NumericConstraints {
        whole_only: true,
        minimum: Some("-9223372036854775808".parse().unwrap()),
        maximum: Some("9223372036854775807".parse().unwrap()),
    };
    e.reason = Some("Reviewed exact limits".into());
    let second = l
        .stats
        .save_definition(&l.admin.token, None, &first.id, e)
        .await
        .unwrap();
    assert_eq!(second.revision, 2);
    assert_eq!(l.stats.version(None, &first.id, 1).await.unwrap(), first);
    assert_eq!(
        l.stats
            .resolve_shared(AddressKey::Slug(second.fields.slug.clone()))
            .await
            .unwrap(),
        second
    );
    assert!(matches!(
        l.stats
            .resolve_shared(AddressKey::Slug(first.fields.slug.clone()))
            .await,
        Err(Error::NotFound)
    ));
    let history = l.stats.history(None, &first.id, None).await.unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].actor_name, "Admin");
    assert_eq!(history[0].reason.as_deref(), Some("Reviewed exact limits"));
    assert_eq!(
        history[0].before_json.as_deref(),
        Some(history[1].after_json.as_str())
    );
    assert!(history[1].after_json.contains("\"-1.25\""));
    assert_eq!(
        l.stats
            .history(None, &first.id, Some(2))
            .await
            .unwrap()
            .len(),
        1
    );
    let mut stale = edit(&first);
    stale.fields.display_name = "".into();
    assert!(matches!(
        l.stats
            .save_definition(&l.admin.token, None, &first.id, stale)
            .await,
        Err(Error::Conflict)
    ));
    let mut invalid = edit(&second);
    invalid.constraints.minimum = Some("1".parse().unwrap());
    invalid.constraints.maximum = Some("-1".parse().unwrap());
    assert!(matches!(
        l.stats
            .save_definition(&l.admin.token, None, &first.id, invalid)
            .await,
        Err(Error::Invalid(_))
    ));
    assert_eq!(l.stats.detail(None, &first.id).await.unwrap(), second);
    l.integrity().await;
}

#[tokio::test]
async fn scopes_authorization_and_locked_session_revalidation() {
    let l = Lab::new().await;
    let w = l.world("Scope one").await;
    let other_world = l.world("Scope two").await;
    for token in [&l.owner.token, &l.other.token, &l.viewer.token] {
        assert!(matches!(
            l.stats.create(token, None, new("Forbidden")).await,
            Err(Error::Forbidden)
        ));
    }
    for token in [&l.admin.token, &l.other.token, &l.viewer.token] {
        assert!(matches!(
            l.stats.create(token, Some(&w.id), new("Forbidden")).await,
            Err(Error::Forbidden)
        ));
        assert!(matches!(
            l.stats.save_roster(token, &w.id, 0, vec![], None).await,
            Err(Error::Forbidden)
        ));
    }
    assert!(matches!(
        l.stats.create("bad-session", None, new("Forbidden")).await,
        Err(Error::Unauthorized)
    ));
    let shared = l.shared("Same name").await;
    let local = l.local(&w.id, "Same name", Some(shared.pin())).await;
    let another = l.local(&other_world.id, "Same name", None).await;
    assert_eq!(shared.fields.slug, local.fields.slug);
    assert_eq!(local.fields.slug, another.fields.slug);
    assert!(matches!(
        l.stats.detail(Some(&other_world.id), &local.id).await,
        Err(Error::NotFound)
    ));
    assert!(matches!(
        l.stats.detail(None, &local.id).await,
        Err(Error::NotFound)
    ));
    let (_, resolved) = l
        .stats
        .resolve_world(
            AddressKey::Slug(w.fields.slug.clone()),
            AddressKey::Slug(local.fields.slug.clone()),
        )
        .await
        .unwrap();
    assert_eq!(resolved, local);
    let audits = l.audits().await;
    for pin in [shared.pin(), another.pin()] {
        assert!(matches!(
            l.stats
                .save_roster(&l.owner.token, &w.id, 0, vec![pin], None)
                .await,
            Err(Error::Invalid(_))
        ));
    }
    assert_eq!(l.audits().await, audits);
    let access = l.author.access(&l.owner.token, &w.id).await.unwrap();
    l.author
        .grant_collaborator(
            &l.owner.token,
            &w.id,
            access.world.access_revision,
            &l.other.actor.user_id,
            None,
        )
        .await
        .unwrap();
    let collaborator = l
        .stats
        .create(&l.other.token, Some(&w.id), new("Collaborator stat"))
        .await
        .unwrap();
    assert_eq!(
        l.stats
            .history(Some(&w.id), &collaborator.id, None)
            .await
            .unwrap()[0]
            .actor_name,
        "Other"
    );
    let access = l.author.access(&l.owner.token, &w.id).await.unwrap();
    l.author
        .revoke_collaborator(
            &l.owner.token,
            &w.id,
            access.world.access_revision,
            &l.other.actor.user_id,
            None,
        )
        .await
        .unwrap();
    assert!(matches!(
        l.stats
            .save_definition(
                &l.other.token,
                Some(&w.id),
                &collaborator.id,
                edit(&collaborator)
            )
            .await,
        Err(Error::Forbidden)
    ));
    // A valid token does not preserve permissions after the account is disabled.
    l.accounts
        .set_disabled(&l.owner.actor.user_id, true)
        .await
        .unwrap();
    assert!(matches!(
        l.stats
            .save_definition(&l.owner.token, Some(&w.id), &local.id, edit(&local))
            .await,
        Err(Error::Unauthorized)
    ));
    l.integrity().await;
}

#[tokio::test]
async fn variable_ordered_rosters_are_independent_and_pin_exact_history() {
    let l = Lab::new().await;
    let w = l.world("Large roster").await;
    assert!(l.stats.world_roster(&w.id).await.unwrap().is_none());
    let mut entries = Vec::new();
    for n in 0..35 {
        entries.push(l.local(&w.id, &format!("Stat {n}"), None).await);
    }
    assert!(l.stats.world_roster(&w.id).await.unwrap().is_none());
    let roster = l
        .stats
        .save_roster(
            &l.owner.token,
            &w.id,
            0,
            entries.iter().map(StatDefinition::pin).collect(),
            None,
        )
        .await
        .unwrap();
    assert_eq!(roster.entries, entries);
    assert_eq!(roster.entries.len(), 35);
    let audits = l.audits().await;
    assert_eq!(
        l.stats
            .save_roster(
                &l.owner.token,
                &w.id,
                1,
                entries.iter().map(StatDefinition::pin).collect(),
                Some("No-op".into())
            )
            .await
            .unwrap(),
        roster
    );
    assert_eq!(l.audits().await, audits);
    let mut e = edit(&entries[0]);
    e.constraints.maximum = Some("-5.125".parse().unwrap());
    let changed = l
        .stats
        .save_definition(&l.owner.token, Some(&w.id), &entries[0].id, e)
        .await
        .unwrap();
    assert_eq!(
        l.stats.world_roster(&w.id).await.unwrap(),
        Some(roster.clone())
    );
    let mut pins: Vec<_> = entries.iter().map(StatDefinition::pin).collect();
    pins[0] = changed.pin();
    pins.reverse();
    let next = l
        .stats
        .save_roster(&l.owner.token, &w.id, 1, pins, None)
        .await
        .unwrap();
    assert_eq!(next.revision, 2);
    assert_eq!(next.entries.last(), Some(&changed));
    assert_eq!(
        l.stats.roster_version(&w.id, &roster.id, 1).await.unwrap(),
        roster
    );
    assert!(matches!(
        l.stats
            .save_roster(
                &l.owner.token,
                &w.id,
                1,
                vec![StatPin {
                    stat_id: "invalid".into(),
                    version: 0
                }],
                Some("\0".into())
            )
            .await,
        Err(Error::Conflict)
    ));
    assert!(matches!(
        l.stats
            .save_roster(
                &l.owner.token,
                &w.id,
                2,
                vec![changed.pin(), changed.pin()],
                None
            )
            .await,
        Err(Error::Invalid(_))
    ));
    let empty = l
        .stats
        .save_roster(&l.owner.token, &w.id, 2, vec![], None)
        .await
        .unwrap();
    assert_eq!(empty.revision, 3);
    assert!(empty.entries.is_empty());
    let history = l.stats.roster_history(&w.id, None).await.unwrap();
    assert_eq!(history.len(), 3);
    assert_eq!(
        history[0].before_json.as_deref(),
        Some(history[1].after_json.as_str())
    );
    assert_eq!(
        l.author.world_detail(&w.id).await.unwrap().revision,
        w.revision
    );
    let mut invalid = roster.clone();
    invalid.entries.push(roster.entries[0].clone());
    assert!(invalid.validate().is_err());
    let mut state = BaseStats {
        roster: Some(roster.pin()),
        assignments: Some(vec![StatValue {
            definition: entries[0].clone(),
            value: Some("-7.125".parse().unwrap()),
        }]),
    };
    assert!(state.validate().is_ok());
    state.assignments.as_mut().unwrap()[0].definition = changed;
    state.assignments.as_mut().unwrap()[0].value = Some("0".parse().unwrap());
    assert!(state.validate().is_err());
    state.assignments.as_mut().unwrap()[0].value = None;
    assert!(state.validate().is_ok());
    l.integrity().await;
}

#[tokio::test]
async fn adoption_changes_only_definition_and_preserves_historical_constraints() {
    let l = Lab::new().await;
    let w = l.world("Adoption").await;
    let source = l.shared("Shared stat").await;
    let local = l.local(&w.id, "Local stat", Some(source.pin())).await;
    let roster = l
        .stats
        .save_roster(&l.owner.token, &w.id, 0, vec![local.pin()], None)
        .await
        .unwrap();
    let mut e = edit(&source);
    e.fields.display_name = "Updated source".into();
    e.constraints = NumericConstraints {
        whole_only: true,
        minimum: Some("-10".parse().unwrap()),
        maximum: Some("20".parse().unwrap()),
    };
    let newer = l
        .stats
        .save_definition(&l.admin.token, None, &source.id, e)
        .await
        .unwrap();
    assert_eq!(l.stats.detail(Some(&w.id), &local.id).await.unwrap(), local);
    let mut selected = edit(&local);
    selected.fields = newer.fields.clone();
    selected.constraints = newer.constraints.clone();
    selected.reason = Some("Explicit adoption".into());
    let preview = l
        .stats
        .adoption_preview(&w.id, &local.id, 2, &selected)
        .await
        .unwrap();
    assert_eq!(preview.old_source, source);
    assert_eq!(preview.new_source, newer);
    assert_eq!(preview.entry, local);
    let adopted = l
        .stats
        .adopt(&l.owner.token, &w.id, &local.id, 2, selected)
        .await
        .unwrap();
    assert_eq!(adopted.source, Some(newer.pin()));
    assert_eq!(adopted.constraints, newer.constraints);
    assert_eq!(l.stats.world_roster(&w.id).await.unwrap(), Some(roster));
    assert_eq!(
        l.stats.version(Some(&w.id), &local.id, 1).await.unwrap(),
        local
    );
    let history = l.stats.history(Some(&w.id), &local.id, None).await.unwrap();
    assert_eq!(history[0].action, "adopt");
    assert_eq!(history[0].actor_name, "Owner");
    assert!(matches!(
        l.stats
            .adopt(&l.owner.token, &w.id, &local.id, 2, edit(&adopted))
            .await,
        Err(Error::Invalid(_))
    ));
    l.integrity().await;
}

#[tokio::test]
async fn immutable_seals_and_failed_writes_rollback_all_rows() {
    let l = Lab::new().await;
    let w = l.world("Seals").await;
    let stat = l.local(&w.id, "Sealed stat", None).await;
    let roster = l
        .stats
        .save_roster(&l.owner.token, &w.id, 0, vec![stat.pin()], None)
        .await
        .unwrap();
    for sql in [
        "UPDATE stat_versions SET whole_only=1",
        "DELETE FROM stat_versions",
        "UPDATE stat_roster_versions SET revision=5",
        "DELETE FROM stat_roster_versions",
        "UPDATE stat_roster_entries SET slot=2",
        "DELETE FROM stat_roster_entries",
    ] {
        assert!(l.pool.execute(sql).await.is_err(), "{sql}");
    }
    assert!(sqlx::query("INSERT INTO stat_roster_entries(world_id,roster_id,revision,slot,scope,stat_id,stat_version) VALUES (?,?,1,2,?,?,1)").bind(&w.id).bind(&roster.id).bind(&w.id).bind(&stat.id).execute(&l.pool).await.is_err());
    let audits = l.audits().await;
    l.pool.execute("CREATE TRIGGER reject_stat_revision BEFORE INSERT ON stat_versions WHEN NEW.revision=2 BEGIN SELECT RAISE(ABORT,'test failed seal'); END").await.unwrap();
    let mut e = edit(&stat);
    e.fields.display_name = "Failed update".into();
    assert!(
        l.stats
            .save_definition(&l.owner.token, Some(&w.id), &stat.id, e)
            .await
            .is_err()
    );
    assert_eq!(l.stats.detail(Some(&w.id), &stat.id).await.unwrap(), stat);
    assert_eq!(l.audits().await, audits);
    l.pool.execute("CREATE TRIGGER reject_roster_revision BEFORE INSERT ON stat_roster_versions WHEN NEW.revision=2 BEGIN SELECT RAISE(ABORT,'test failed seal'); END").await.unwrap();
    assert!(
        l.stats
            .save_roster(&l.owner.token, &w.id, 1, vec![], None)
            .await
            .is_err()
    );
    assert_eq!(l.stats.world_roster(&w.id).await.unwrap(), Some(roster));
    assert_eq!(l.audits().await, audits);
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM stat_roster_entries WHERE revision=2")
            .fetch_one(&l.pool)
            .await
            .unwrap();
    assert_eq!(count, 0);
    l.integrity().await;
}

#[tokio::test]
async fn readers_fail_closed_on_typed_snapshot_head_and_dependency_mismatch() {
    let l = Lab::new().await;
    let w = l.world("Corruption").await;
    let first = l.local(&w.id, "Exact stat", None).await;
    let roster = l
        .stats
        .save_roster(&l.owner.token, &w.id, 0, vec![first.pin()], None)
        .await
        .unwrap();
    let mut e = edit(&first);
    e.constraints.minimum = Some("-0.25".parse().unwrap());
    let second = l
        .stats
        .save_definition(&l.owner.token, Some(&w.id), &first.id, e)
        .await
        .unwrap();
    // Simulate storage damage only in this test-owned database.
    l.pool
        .execute("DROP TRIGGER stat_versions_update")
        .await
        .unwrap();
    sqlx::query("UPDATE stat_versions SET minimum_coefficient=-26 WHERE stat_id=? AND revision=2")
        .bind(&first.id)
        .execute(&l.pool)
        .await
        .unwrap();
    assert!(matches!(
        l.stats.detail(Some(&w.id), &first.id).await,
        Err(Error::Unavailable)
    ));
    assert!(matches!(
        l.stats.version(Some(&w.id), &first.id, 1).await,
        Err(Error::Unavailable)
    ));
    assert!(matches!(
        l.stats.roster_version(&w.id, &roster.id, 1).await,
        Err(Error::Unavailable)
    ));
    sqlx::query("UPDATE stat_versions SET minimum_coefficient=-25 WHERE stat_id=? AND revision=2")
        .bind(&first.id)
        .execute(&l.pool)
        .await
        .unwrap();
    assert_eq!(
        l.stats.detail(Some(&w.id), &first.id).await.unwrap(),
        second
    );
    sqlx::query("UPDATE stat_versions SET before_json=json_set(before_json,'$.content.fields.display_name','Forged predecessor') WHERE stat_id=? AND revision=2").bind(&first.id).execute(&l.pool).await.unwrap();
    assert!(matches!(
        l.stats.detail(Some(&w.id), &first.id).await,
        Err(Error::Unavailable)
    ));
}
