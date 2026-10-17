use game_data::{BUNDLED_FIXTURES, Catalog, authoring::ContentFields, types::TypePin};
use sqlx::{SqlSafeStr, SqlitePool};
use tempfile::TempDir;
use web_app::{
    accounts::Accounts,
    application::{
        authoring::{
            AddressKey, Authoring, Edit, Error, FormRecord, HistoryKind, SourcePin, SpeciesRecord,
            VariantRecord, WorldRecord,
        },
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
            "sqlite://{}/types.db",
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
            types: Types::new(accounts.clone()),
            accounts,
            admin,
            owner,
            editor,
            viewer,
            _dir: dir,
        }
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
        let result: String = sqlx::query_scalar("PRAGMA integrity_check")
            .fetch_one(&self.pool)
            .await
            .unwrap();
        assert_eq!(result, "ok");
    }
}
fn fields(name: &str) -> ContentFields {
    ContentFields {
        slug: "forged / ignored".into(),
        display_name: name.into(),
        description: "Typed metadata only.".into(),
    }
}
fn edit(revision: i64, name: &str) -> Edit {
    Edit {
        expected_revision: revision,
        fields: fields(name),
        reason: Some("Reviewed".into()),
    }
}
fn pin(id: &str, version: i64) -> TypePin {
    TypePin {
        type_id: id.into(),
        version,
    }
}
fn envelope(value: &impl serde::Serialize) -> String {
    serde_json::to_string_pretty(&serde_json::json!({"snapshot_version":1,"content":value}))
        .unwrap()
}

#[tokio::test]
async fn type_roles_scopes_slugs_history_and_fresh_global_eligibility() {
    let l = Lab::new().await;
    for u in [&l.owner, &l.editor, &l.viewer] {
        assert_eq!(
            l.types
                .create_type(&u.token, fields("Denied"), None)
                .await
                .unwrap_err(),
            Error::Forbidden
        );
    }
    assert_eq!(
        l.types
            .create_type("revoked", fields("Denied"), None)
            .await
            .unwrap_err(),
        Error::Unauthorized
    );
    let fire = l
        .types
        .create_type(&l.admin.token, fields("Fíre"), Some("Initial"))
        .await
        .unwrap();
    let second = l
        .types
        .create_type(&l.admin.token, fields("Fire"), None)
        .await
        .unwrap();
    assert_eq!(
        (&*fire.fields.slug, &*second.fields.slug),
        ("fire", "fire-2")
    );
    assert_eq!(
        l.types
            .resolve_type(AddressKey::Slug("fire".into()))
            .await
            .unwrap(),
        fire
    );
    assert_eq!(
        l.types
            .resolve_type(AddressKey::Id("fire".into()))
            .await
            .unwrap_err(),
        Error::NotFound
    );
    let world = l
        .author
        .create_world(&l.owner.token, fields("World"), None)
        .await
        .unwrap();
    for u in [&l.admin, &l.editor, &l.viewer] {
        assert_eq!(
            l.types
                .create_world_type(&u.token, &world.id, 1, None, fields("Denied"), None)
                .await
                .unwrap_err(),
            Error::Forbidden
        );
    }
    let local = l
        .types
        .create_world_type(
            &l.owner.token,
            &world.id,
            1,
            Some(pin(&fire.id, 1)),
            fields("Fire"),
            None,
        )
        .await
        .unwrap();
    assert_eq!(l.author.world_detail(&world.id).await.unwrap().revision, 2);
    let other = l
        .author
        .create_world(&l.admin.token, fields("Other"), None)
        .await
        .unwrap();
    assert_eq!(
        l.types
            .world_type_detail(&other.id, &local.id)
            .await
            .unwrap_err(),
        Error::NotFound
    );
    assert_eq!(
        l.types
            .resolve_world_type(
                AddressKey::Slug("world".into()),
                AddressKey::Slug("fire".into())
            )
            .await
            .unwrap()
            .1,
        local
    );
    assert_eq!(
        l.types
            .world_type_version(&world.id, &local.id, 99)
            .await
            .unwrap_err(),
        Error::NotFound
    );
    l.author
        .grant_collaborator(&l.owner.token, &world.id, 1, &l.editor.actor.user_id, None)
        .await
        .unwrap();
    let renamed = l
        .types
        .save_world_type(
            &l.editor.token,
            &world.id,
            &local.id,
            edit(1, "Local flame"),
        )
        .await
        .unwrap();
    assert_eq!(renamed.source, local.source);
    assert_eq!(l.author.world_detail(&world.id).await.unwrap().revision, 2);
    assert_eq!(
        l.types
            .world_type_version(&world.id, &local.id, 1)
            .await
            .unwrap(),
        local
    );
    let count = l.count("audit_events").await;
    l.types
        .save_world_type(&l.owner.token, &world.id, &local.id, edit(2, "Local flame"))
        .await
        .unwrap();
    assert_eq!(l.count("audit_events").await, count);
    assert_eq!(
        l.types
            .save_world_type(&l.owner.token, &world.id, &local.id, edit(1, "Local flame"))
            .await
            .unwrap_err(),
        Error::Conflict
    );
    let history = l
        .author
        .history(HistoryKind::WorldType, &local.id, None)
        .await
        .unwrap();
    assert_eq!(history[0].actor_name, "Editor");
    assert_eq!(
        history[0].before_json.as_deref(),
        Some(history[1].after_json.as_str())
    );
    sqlx::query("UPDATE users SET role='viewer' WHERE id=?")
        .bind(&l.editor.actor.user_id)
        .execute(&l.pool)
        .await
        .unwrap();
    assert_eq!(
        l.types
            .save_world_type(&l.editor.token, &world.id, &local.id, edit(2, "Denied"))
            .await
            .unwrap_err(),
        Error::Forbidden
    );
    l.accounts
        .set_disabled(&l.owner.actor.user_id, true)
        .await
        .unwrap();
    assert_eq!(
        l.types
            .save_world_type(&l.owner.token, &world.id, &local.id, edit(2, "Denied"))
            .await
            .unwrap_err(),
        Error::Unauthorized
    );
    l.integrity().await;
}

#[tokio::test]
async fn ordered_typing_distinguishes_unmodeled_typeless_and_three_slots_and_checks_stale_first() {
    let l = Lab::new().await;
    let species = l
        .author
        .create_species(&l.admin.token, fields("Creature"), None)
        .await
        .unwrap();
    let id = &species.default_form_id;
    let mut pins = Vec::new();
    for name in ["Fire", "Flying", "Dragon"] {
        pins.push(pin(
            &l.types
                .create_type(&l.admin.token, fields(name), None)
                .await
                .unwrap()
                .id,
            1,
        ));
    }
    assert_eq!(
        l.types.form_typing(id, None).await.unwrap().assignments,
        None
    );
    let typeless = l
        .types
        .save_form_typing(
            &l.admin.token,
            id,
            1,
            Some(vec![]),
            Some("Intentionally typeless"),
        )
        .await
        .unwrap();
    assert_eq!(typeless.revision, 2);
    assert_eq!(
        l.types.form_typing(id, None).await.unwrap().assignments,
        Some(vec![])
    );
    assert_eq!(
        l.types.form_typing(id, Some(1)).await.unwrap().assignments,
        None
    );
    assert_eq!(
        l.types
            .save_form_typing(&l.owner.token, id, 2, Some(pins.clone()), None)
            .await
            .unwrap_err(),
        Error::Forbidden
    );
    let typed = l
        .types
        .save_form_typing(&l.admin.token, id, 2, Some(pins.clone()), None)
        .await
        .unwrap();
    assert_eq!(typed.revision, 3);
    assert_eq!(
        l.types.form_typing(id, None).await.unwrap().pins(),
        Some(pins.clone())
    );
    let count = l.count("audit_events").await;
    l.types
        .save_form_typing(&l.admin.token, id, 3, Some(pins.clone()), None)
        .await
        .unwrap();
    assert_eq!(l.count("audit_events").await, count);
    let bad = Some(vec![pins[0].clone(), pin(&pins[0].type_id, 2)]);
    assert_eq!(
        l.types
            .save_form_typing(&l.admin.token, id, 2, bad.clone(), None)
            .await
            .unwrap_err(),
        Error::Conflict
    );
    assert!(matches!(
        l.types
            .save_form_typing(&l.admin.token, id, 3, bad, None)
            .await,
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        l.types
            .save_form_typing(
                &l.admin.token,
                id,
                3,
                Some(vec![pin(&pins[0].type_id, 99)]),
                None
            )
            .await,
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        l.types
            .save_form_typing(&l.admin.token, id, 3, Some(vec![pin("bad", 1)]), None)
            .await,
        Err(Error::Invalid(_))
    ));
    let too_many = Some((0..33).map(|n| pin(&format!("{n:032x}"), 1)).collect());
    assert!(matches!(
        l.types
            .save_form_typing(&l.admin.token, id, 3, too_many, None)
            .await,
        Err(Error::Invalid(_))
    ));
    pins.reverse();
    l.types
        .save_form_typing(&l.admin.token, id, 3, Some(pins.clone()), None)
        .await
        .unwrap();
    assert_eq!(
        l.types.form_typing(id, None).await.unwrap().pins(),
        Some(pins)
    );
    let h = l.author.history(HistoryKind::Form, id, None).await.unwrap();
    for p in h.windows(2) {
        assert_eq!(p[0].before_json.as_deref(), Some(p[1].after_json.as_str()));
    }
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&h[0].after_json).unwrap()["snapshot_version"],
        3
    );
    l.types
        .save_form_typing(&l.admin.token, id, 4, None, None)
        .await
        .unwrap();
    assert_eq!(
        l.types.form_typing(id, None).await.unwrap().assignments,
        None
    );
    l.integrity().await;
}

#[tokio::test]
async fn pinned_fields_survive_library_local_metadata_and_both_adoptions() {
    let l = Lab::new().await;
    let t = l
        .types
        .create_type(&l.admin.token, fields("Fire"), None)
        .await
        .unwrap();
    let species = l
        .author
        .create_species(&l.admin.token, fields("Creature"), None)
        .await
        .unwrap();
    let form = l
        .types
        .save_form_typing(
            &l.admin.token,
            &species.default_form_id,
            1,
            Some(vec![pin(&t.id, 1)]),
            None,
        )
        .await
        .unwrap();
    let old_shared = l.types.form_typing(&form.id, None).await.unwrap();
    let w = l
        .author
        .create_world(&l.owner.token, fields("World"), None)
        .await
        .unwrap();
    let local = l
        .types
        .create_world_type(
            &l.owner.token,
            &w.id,
            1,
            Some(pin(&t.id, 1)),
            fields("Local fire"),
            None,
        )
        .await
        .unwrap();
    let variant = l
        .author
        .create_variant(
            &l.owner.token,
            &w.id,
            2,
            Some(SourcePin {
                form_id: form.id.clone(),
                version: 2,
            }),
            fields("Local creature"),
            None,
        )
        .await
        .unwrap();
    let variant = l
        .types
        .save_variant_typing(
            &l.owner.token,
            &w.id,
            &variant.id,
            1,
            Some(vec![pin(&local.id, 1)]),
            None,
        )
        .await
        .unwrap();
    let old_local = l
        .types
        .variant_typing(&w.id, &variant.id, None)
        .await
        .unwrap();
    let t2 = l
        .types
        .save_type(&l.admin.token, &t.id, edit(1, "Flame"))
        .await
        .unwrap();
    assert_eq!(
        l.types.form_typing(&form.id, None).await.unwrap(),
        old_shared
    );
    assert_eq!(
        l.types.world_type_detail(&w.id, &local.id).await.unwrap(),
        local
    );
    let preview = l.types.adoption_preview(&w.id, &local.id, 2).await.unwrap();
    assert_eq!((preview.old_source, preview.new_source), (t.clone(), t2));
    let adopted = l
        .types
        .adopt_world_type(
            &l.owner.token,
            &w.id,
            &local.id,
            2,
            edit(1, "Independent local flame"),
        )
        .await
        .unwrap();
    assert_eq!(adopted.source, Some(pin(&t.id, 2)));
    assert_eq!(
        l.types
            .variant_typing(&w.id, &variant.id, None)
            .await
            .unwrap(),
        old_local
    );
    let saved_form = l
        .author
        .save_form(
            &l.admin.token,
            &form.id,
            1,
            edit(2, "Creature metadata edit"),
        )
        .await
        .unwrap();
    assert_eq!(
        l.types.form_typing(&form.id, None).await.unwrap(),
        old_shared
    );
    l.types
        .save_form_typing(
            &l.admin.token,
            &form.id,
            saved_form.revision,
            Some(vec![pin(&t.id, 2)]),
            None,
        )
        .await
        .unwrap();
    let preview = l
        .author
        .adoption_preview(&w.id, &variant.id, 4)
        .await
        .unwrap();
    assert_eq!(preview.old_typing, old_shared);
    assert_eq!(
        preview.new_typing.assignments.as_ref().unwrap()[0]
            .fields
            .display_name,
        "Flame"
    );
    assert_eq!(preview.local_typing, old_local);
    let adopted = l
        .author
        .adopt_variant(
            &l.owner.token,
            &w.id,
            &variant.id,
            4,
            edit(2, "Reviewed local creature"),
        )
        .await
        .unwrap();
    assert_eq!(adopted.source.unwrap().version, 4);
    assert_eq!(
        l.types
            .variant_typing(&w.id, &variant.id, None)
            .await
            .unwrap(),
        old_local
    );
    l.author
        .save_variant(
            &l.owner.token,
            &w.id,
            &variant.id,
            edit(3, "Metadata-only later"),
        )
        .await
        .unwrap();
    assert_eq!(
        l.types
            .variant_typing(&w.id, &variant.id, None)
            .await
            .unwrap(),
        old_local
    );
    assert_eq!(l.author.world_detail(&w.id).await.unwrap().revision, 3);
    // Archive current definitions through test-owned SQL; historical assignments still read.
    sqlx::query("UPDATE types SET archived_at=1 WHERE id=?")
        .bind(&t.id)
        .execute(&l.pool)
        .await
        .unwrap();
    assert_eq!(
        l.types.form_typing(&form.id, Some(2)).await.unwrap(),
        old_shared
    );
    assert!(matches!(
        l.types
            .save_form_typing(&l.admin.token, &form.id, 4, Some(vec![pin(&t.id, 1)]), None)
            .await,
        Err(Error::Invalid(_))
    ));
    l.author
        .save_form(
            &l.admin.token,
            &form.id,
            1,
            edit(4, "Archived pin preserved"),
        )
        .await
        .unwrap();
    sqlx::query("UPDATE world_types SET archived_at=1 WHERE id=?")
        .bind(&local.id)
        .execute(&l.pool)
        .await
        .unwrap();
    l.author
        .save_variant(
            &l.owner.token,
            &w.id,
            &variant.id,
            edit(4, "Archived local pin preserved"),
        )
        .await
        .unwrap();
    assert_eq!(
        l.types
            .variant_typing(&w.id, &variant.id, None)
            .await
            .unwrap(),
        old_local
    );
    l.integrity().await;
}

#[tokio::test]
async fn variant_assignments_reject_shared_and_foreign_world_exact_pins() {
    let l = Lab::new().await;
    let t = l
        .types
        .create_type(&l.admin.token, fields("Fire"), None)
        .await
        .unwrap();
    let a = l
        .author
        .create_world(&l.owner.token, fields("A"), None)
        .await
        .unwrap();
    let b = l
        .author
        .create_world(&l.owner.token, fields("B"), None)
        .await
        .unwrap();
    let ta = l
        .types
        .create_world_type(&l.owner.token, &a.id, 1, None, fields("Mist"), None)
        .await
        .unwrap();
    let tb = l
        .types
        .create_world_type(&l.owner.token, &b.id, 1, None, fields("Mist"), None)
        .await
        .unwrap();
    let v = l
        .author
        .create_variant(&l.owner.token, &a.id, 2, None, fields("Creature"), None)
        .await
        .unwrap();
    for p in [pin(&t.id, 1), pin(&tb.id, 1), pin(&ta.id, 99)] {
        assert!(matches!(
            l.types
                .save_variant_typing(&l.owner.token, &a.id, &v.id, 1, Some(vec![p]), None)
                .await,
            Err(Error::Invalid(_))
        ));
    }
    assert_eq!(
        l.types
            .save_variant_typing(&l.owner.token, &b.id, &v.id, 1, None, None)
            .await
            .unwrap_err(),
        Error::NotFound
    );
    assert_eq!(
        l.types
            .save_variant_typing(&l.admin.token, &a.id, &v.id, 1, None, None)
            .await
            .unwrap_err(),
        Error::Forbidden
    );
    l.types
        .save_variant_typing(
            &l.owner.token,
            &a.id,
            &v.id,
            1,
            Some(vec![pin(&ta.id, 1)]),
            None,
        )
        .await
        .unwrap();
    let mut tx = persistence::begin_write(&l.pool).await.unwrap();
    let error=sqlx::query("INSERT INTO variant_type_bindings(world_id,variant_id,revision,slot,type_id,type_version) VALUES (?,?,3,1,?,1)").bind(&a.id).bind(&v.id).bind(&tb.id).execute(&mut *tx).await.unwrap_err();
    assert_eq!(
        error.as_database_error().unwrap().code().as_deref(),
        Some("787")
    );
    tx.rollback().await.unwrap();
    l.integrity().await;
}

#[tokio::test]
async fn typed_mutations_roll_back_heads_audits_bindings_and_sidecars_on_late_failures() {
    for trigger in [
        "CREATE TRIGGER fault BEFORE INSERT ON form_content_snapshots WHEN NEW.revision=2 BEGIN SELECT RAISE(ABORT,'test'); END",
        "CREATE TRIGGER fault BEFORE INSERT ON form_versions WHEN NEW.revision=2 BEGIN SELECT RAISE(ABORT,'test'); END",
    ] {
        let l = Lab::new().await;
        let t = l
            .types
            .create_type(&l.admin.token, fields("Fire"), None)
            .await
            .unwrap();
        let s = l
            .author
            .create_species(&l.admin.token, fields("Creature"), None)
            .await
            .unwrap();
        let counts = [
            l.count("audit_events").await,
            l.count("form_content_snapshots").await,
            l.count("form_versions").await,
        ];
        sqlx::query(trigger).execute(&l.pool).await.unwrap();
        assert_eq!(
            l.types
                .save_form_typing(
                    &l.admin.token,
                    &s.default_form_id,
                    1,
                    Some(vec![pin(&t.id, 1)]),
                    None
                )
                .await
                .unwrap_err(),
            Error::Unavailable
        );
        assert_eq!(
            l.author
                .form_detail(&s.default_form_id)
                .await
                .unwrap()
                .revision,
            1
        );
        assert_eq!(l.count("form_type_bindings").await, 0);
        assert_eq!(
            [
                l.count("audit_events").await,
                l.count("form_content_snapshots").await,
                l.count("form_versions").await
            ],
            counts
        );
        l.integrity().await;
    }
    let l = Lab::new().await;
    let w = l
        .author
        .create_world(&l.owner.token, fields("World"), None)
        .await
        .unwrap();
    let count = l.count("audit_events").await;
    sqlx::query("CREATE TRIGGER fault BEFORE INSERT ON world_revisions WHEN NEW.revision=2 BEGIN SELECT RAISE(ABORT,'test'); END").execute(&l.pool).await.unwrap();
    assert_eq!(
        l.types
            .create_world_type(&l.owner.token, &w.id, 1, None, fields("Mist"), None)
            .await
            .unwrap_err(),
        Error::Unavailable
    );
    assert_eq!(l.count("world_types").await, 0);
    assert_eq!(l.count("world_type_versions").await, 0);
    assert_eq!(l.count("world_content_snapshots").await, 1);
    assert_eq!(l.count("audit_events").await, count);
    assert_eq!(l.author.world_detail(&w.id).await.unwrap(), w);
    l.integrity().await;
}

#[tokio::test]
async fn database_seals_require_exact_marker_two_sidecars_and_reject_late_or_orphan_bindings() {
    let l = Lab::new().await;
    let t = l
        .types
        .create_type(&l.admin.token, fields("Fire"), None)
        .await
        .unwrap();
    let s = l
        .author
        .create_species(&l.admin.token, fields("Creature"), None)
        .await
        .unwrap();
    let id = &s.default_form_id;
    for sql in [
        "UPDATE form_content_snapshots SET typing_defined=1 WHERE form_id=?",
        "DELETE FROM form_content_snapshots WHERE form_id=?",
    ] {
        assert!(sqlx::query(sql).bind(id).execute(&l.pool).await.is_err());
    }
    assert!(sqlx::query("INSERT INTO form_type_bindings(form_id,revision,slot,type_id,type_version) VALUES (?,1,1,?,1)").bind(id).bind(&t.id).execute(&l.pool).await.is_err());
    let mut tx = persistence::begin_write(&l.pool).await.unwrap();
    sqlx::query("INSERT INTO audit_events(id,system_key,action) VALUES ('aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa','bundled-fixture-loader','edit')").execute(&mut *tx).await.unwrap();
    for marker in [1, 2] {
        let sql = "INSERT INTO form_versions(form_id,species_id,species_version,revision,slug,display_name,description,is_default,archived_at,audit_id,previous_revision,before_json,after_json,content_snapshot_version) SELECT form_id,species_id,species_version,2,slug,display_name,description,is_default,archived_at,'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',1,after_json,after_json,?2 FROM form_versions WHERE form_id=?1 AND revision=1";
        assert!(
            sqlx::query(sql)
                .bind(id)
                .bind(marker)
                .execute(&mut *tx)
                .await
                .is_err()
        );
    }
    tx.rollback().await.unwrap();
    // Deferred orphan binding has a real exact Type but no corresponding owner seal.
    let mut tx = persistence::begin_write(&l.pool).await.unwrap();
    sqlx::query("INSERT INTO form_type_bindings(form_id,revision,slot,type_id,type_version) VALUES (?,99,1,?,1)").bind(id).bind(&t.id).execute(&mut *tx).await.unwrap();
    assert!(tx.commit().await.is_err());
    assert_eq!(l.count("form_type_bindings").await, 0);
    // The sidecar cannot be sealed onto an already present revision, even marker 1.
    assert!(sqlx::query("INSERT INTO form_content_snapshots(form_id,revision,typing_defined,after_json) SELECT form_id,1,0,after_json FROM form_content_snapshots WHERE form_id=?").bind(id).execute(&l.pool).await.is_err());
    // A malformed ordering cannot be sealed, and a duplicate identity is rejected by SQL.
    let mut tx = persistence::begin_write(&l.pool).await.unwrap();
    sqlx::query("INSERT INTO form_type_bindings(form_id,revision,slot,type_id,type_version) VALUES (?,2,2,?,1)").bind(id).bind(&t.id).execute(&mut *tx).await.unwrap();
    assert!(sqlx::query("INSERT INTO form_type_bindings(form_id,revision,slot,type_id,type_version) VALUES (?,2,1,?,1)").bind(id).bind(&t.id).execute(&mut *tx).await.is_err());
    let mut full: serde_json::Value = serde_json::from_str(
        &l.author
            .history_entry(HistoryKind::Form, id, 1)
            .await
            .unwrap()
            .after_json,
    )
    .unwrap();
    // Exercise the retained v2 projection's ordering seal, independently of v3.
    full["snapshot_version"] = 2.into();
    full["content"].as_object_mut().unwrap().remove("stats");
    full["content"]["revision"] = 2.into();
    full["content"]["typing"] =
        serde_json::json!({"assignments":[{"pin":pin(&t.id,1),"fields":t.fields}]});
    assert!(sqlx::query("INSERT INTO form_content_snapshots(form_id,revision,typing_defined,before_json,after_json) VALUES (?,2,1,?,?)").bind(id).bind(full.to_string()).bind(full.to_string()).execute(&mut *tx).await.is_err());
    tx.rollback().await.unwrap();
    l.types
        .save_form_typing(&l.admin.token, id, 1, Some(vec![pin(&t.id, 1)]), None)
        .await
        .unwrap();
    for sql in [
        "UPDATE form_type_bindings SET slot=2 WHERE form_id=?",
        "DELETE FROM form_type_bindings WHERE form_id=?",
    ] {
        assert!(sqlx::query(sql).bind(id).execute(&l.pool).await.is_err());
    }
    l.integrity().await;
}

#[tokio::test]
async fn full_history_and_typing_fail_closed_for_missing_inconsistent_or_unknown_full_snapshots() {
    let l = Lab::new().await;
    let t = l
        .types
        .create_type(&l.admin.token, fields("Fire"), None)
        .await
        .unwrap();
    let s = l
        .author
        .create_species(&l.admin.token, fields("Creature"), None)
        .await
        .unwrap();
    let id = &s.default_form_id;
    l.types
        .save_form_typing(&l.admin.token, id, 1, Some(vec![pin(&t.id, 1)]), None)
        .await
        .unwrap();
    let baseline: String = sqlx::query_scalar(
        "SELECT after_json FROM form_content_snapshots WHERE form_id=? AND revision=2",
    )
    .bind(id)
    .fetch_one(&l.pool)
    .await
    .unwrap();
    sqlx::query("DROP TRIGGER form_content_update")
        .execute(&l.pool)
        .await
        .unwrap();
    let good: serde_json::Value = serde_json::from_str(&baseline).unwrap();
    let mut cases = Vec::new();
    let mut bad = good.clone();
    bad["content"]["typing"]["assignments"][0]["fields"]["display_name"] = "Forged".into();
    cases.push(bad);
    let mut bad = good.clone();
    bad["content"]["typing"]["assignments"][0]["pin"]["version"] = 99.into();
    cases.push(bad);
    let mut bad = good.clone();
    bad["content"]["fields"]["display_name"] = "Different projection".into();
    cases.push(bad);
    let mut bad = good.clone();
    bad["content"]["extra"] = true.into();
    cases.push(bad);
    let mut bad = good.clone();
    bad["content"].as_object_mut().unwrap().remove("typing");
    cases.push(bad);
    let mut bad = good.clone();
    bad["content"]["typing"]
        .as_object_mut()
        .unwrap()
        .remove("assignments");
    cases.push(bad);
    for bad in cases {
        sqlx::query(
            "UPDATE form_content_snapshots SET after_json=? WHERE form_id=? AND revision=2",
        )
        .bind(bad.to_string())
        .bind(id)
        .execute(&l.pool)
        .await
        .unwrap();
        assert_eq!(
            l.types.form_typing(id, None).await.unwrap_err(),
            Error::Unavailable
        );
        assert_eq!(
            l.author
                .history_entry(HistoryKind::Form, id, 2)
                .await
                .unwrap_err(),
            Error::Unavailable
        );
    }
    sqlx::query("UPDATE form_content_snapshots SET after_json=? WHERE form_id=? AND revision=2")
        .bind(&baseline)
        .bind(id)
        .execute(&l.pool)
        .await
        .unwrap();
    // Inject missing sidecar in a test-only connection with FK enforcement explicitly off.
    let mut c = l.pool.acquire().await.unwrap();
    sqlx::query("DROP TRIGGER form_content_delete")
        .execute(&mut *c)
        .await
        .unwrap();
    sqlx::query("PRAGMA foreign_keys=OFF")
        .execute(&mut *c)
        .await
        .unwrap();
    // bindings have FK to sidecar, but corruption injection is allowed only here.
    sqlx::query("DELETE FROM form_content_snapshots WHERE form_id=? AND revision=2")
        .bind(id)
        .execute(&mut *c)
        .await
        .unwrap();
    sqlx::query("PRAGMA foreign_keys=ON")
        .execute(&mut *c)
        .await
        .unwrap();
    drop(c);
    assert_eq!(
        l.types.form_typing(id, None).await.unwrap_err(),
        Error::Unavailable
    );
    assert_eq!(
        l.author.form_version(id, 2).await.unwrap_err(),
        Error::Unavailable
    );
    assert_eq!(
        l.author
            .history(HistoryKind::Form, id, None)
            .await
            .unwrap_err(),
        Error::Unavailable
    );
    assert_eq!(
        l.author
            .save_form(&l.admin.token, id, 1, edit(2, "Must not hide corruption"))
            .await
            .unwrap_err(),
        Error::Unavailable
    );
    assert_eq!(l.author.form_detail(id).await.unwrap().revision, 2);
}

#[tokio::test]
async fn fixture_world_batch_is_complete_system_attributed_and_idempotent_after_edits_and_grants() {
    let l = Lab::new().await;
    let users = l.count("users").await;
    let invites = l.count("invites").await;
    let report = l
        .types
        .seed_world_fixtures(&l.owner.actor.user_id)
        .await
        .unwrap();
    assert_eq!(
        (
            report.types_created,
            report.species_created,
            report.forms_created,
            report.worlds_created,
            report.world_types_created,
            report.variants_created,
            report.skipped
        ),
        (3, 1, 2, 2, 7, 5, 0)
    );
    assert_eq!(l.count("audit_events").await, 22);
    let world = l
        .author
        .worlds()
        .await
        .unwrap()
        .into_iter()
        .find(|w| w.fields.display_name == "VariantLab")
        .unwrap();
    assert_eq!((world.revision, world.access_revision), (1, 1));
    let h = l
        .author
        .history_entry(HistoryKind::World, &world.id, 1)
        .await
        .unwrap();
    assert_eq!(h.actor_name, "bundled-fixture-loader");
    assert!(!h.after_json.contains("owner_user_id") && !h.after_json.contains("collaborator"));
    let content: serde_json::Value = serde_json::from_str(&h.after_json).unwrap();
    assert_eq!(content["content"]["type_ids"].as_array().unwrap().len(), 4);
    assert_eq!(
        content["content"]["variant_ids"].as_array().unwrap().len(),
        3
    );
    let local = l.types.world_types(&world.id).await.unwrap();
    assert!(
        local
            .iter()
            .any(|t| t.source.is_none() && t.fields.display_name == "Mist")
    );
    let variants = l.author.variants(&world.id).await.unwrap();
    let triad = variants
        .iter()
        .find(|v| v.fields.display_name == "Charizard Triad")
        .unwrap();
    assert_eq!(
        l.types
            .variant_typing(&world.id, &triad.id, None)
            .await
            .unwrap()
            .assignments
            .unwrap()
            .len(),
        3
    );
    let access = l.author.access(&l.owner.token, &world.id).await.unwrap();
    assert_eq!(access.history.len(), 1);
    assert_eq!(
        (&*access.history[0].actor_name, &*access.history[0].action),
        ("bundled-fixture-loader", "own")
    );
    assert_eq!(
        l.author
            .access(&l.admin.token, &world.id)
            .await
            .unwrap_err(),
        Error::Forbidden
    );
    l.author
        .grant_collaborator(&l.owner.token, &world.id, 1, &l.editor.actor.user_id, None)
        .await
        .unwrap();
    assert_eq!(
        l.author
            .access(&l.editor.token, &world.id)
            .await
            .unwrap_err(),
        Error::Forbidden
    );
    let edited = l
        .author
        .save_variant(
            &l.editor.token,
            &world.id,
            &triad.id,
            edit(1, "Locally edited"),
        )
        .await
        .unwrap();
    let edited_world = l
        .author
        .save_world(&l.owner.token, &world.id, edit(1, "Renamed lab"))
        .await
        .unwrap();
    let edited_type = l
        .types
        .save_world_type(
            &l.owner.token,
            &world.id,
            &local[0].id,
            edit(1, "Edited local Type"),
        )
        .await
        .unwrap();
    let shared = l.types.types().await.unwrap().remove(0);
    l.types
        .save_type(&l.admin.token, &shared.id, edit(1, "Edited reference Type"))
        .await
        .unwrap();
    let count = l.count("audit_events").await;
    let report = l
        .types
        .seed_world_fixtures(&l.owner.actor.user_id)
        .await
        .unwrap();
    assert_eq!(report.skipped, 20);
    assert_eq!((report.types_created, report.worlds_created), (0, 0));
    assert_eq!(l.count("audit_events").await, count);
    assert_eq!(
        l.author.variant_detail(&world.id, &triad.id).await.unwrap(),
        edited
    );
    assert_eq!(
        l.author.world_detail(&world.id).await.unwrap(),
        edited_world
    );
    assert_eq!(
        l.types
            .world_type_detail(&world.id, &edited_type.id)
            .await
            .unwrap(),
        edited_type
    );
    assert_eq!(
        l.author
            .access(&l.owner.token, &world.id)
            .await
            .unwrap()
            .collaborators
            .len(),
        1
    );
    assert_eq!(l.count("users").await, users);
    assert_eq!(l.count("invites").await, invites);
    let source:(String,i64)=sqlx::query_as("SELECT endpoint_kind,external_id FROM form_version_sources WHERE source_url LIKE '%10134/%'").fetch_one(&l.pool).await.unwrap();
    assert_eq!(source, ("pokemon-form".into(), 10134));
    l.integrity().await;
}

#[tokio::test]
async fn invalid_owners_and_fixture_late_faults_have_no_content_or_account_effects() {
    let l = Lab::new().await;
    for owner in ["missing", &l.viewer.actor.user_id] {
        assert!(matches!(
            l.types.seed_world_fixtures(owner).await,
            Err(Error::Invalid(_))
        ));
    }
    l.accounts
        .set_disabled(&l.owner.actor.user_id, true)
        .await
        .unwrap();
    assert!(matches!(
        l.types.seed_world_fixtures(&l.owner.actor.user_id).await,
        Err(Error::Invalid(_))
    ));
    for table in [
        "types",
        "species",
        "forms",
        "worlds",
        "world_types",
        "variants",
        "audit_events",
    ] {
        assert_eq!(l.count(table).await, 0, "{table}");
    }
    l.accounts
        .set_disabled(&l.owner.actor.user_id, false)
        .await
        .unwrap();
    sqlx::query("CREATE TRIGGER late_fault BEFORE INSERT ON world_access_revisions WHEN NEW.world_id='5d2d9dba132840f3acbaa5ed1e2b0b49' BEGIN SELECT RAISE(ABORT,'test'); END").execute(&l.pool).await.unwrap();
    assert_eq!(
        l.types
            .seed_world_fixtures(&l.owner.actor.user_id)
            .await
            .unwrap_err(),
        Error::Unavailable
    );
    for table in [
        "types",
        "type_versions",
        "species",
        "forms",
        "worlds",
        "world_types",
        "world_type_versions",
        "variants",
        "audit_events",
        "form_type_bindings",
        "variant_type_bindings",
        "type_version_sources",
        "species_version_sources",
        "form_version_sources",
        "world_content_snapshots",
    ] {
        assert_eq!(l.count(table).await, 0, "{table}");
    }
    assert_eq!(l.count("users").await, 4);
    assert_eq!(l.count("invites").await, 4);
    l.integrity().await;
}

#[tokio::test]
async fn fixture_owner_drift_partial_baselines_and_provenance_drift_fail_without_repair() {
    let l = Lab::new().await;
    l.types
        .seed_world_fixtures(&l.owner.actor.user_id)
        .await
        .unwrap();
    let count = l.count("audit_events").await;
    assert!(matches!(
        l.types.seed_world_fixtures(&l.admin.actor.user_id).await,
        Err(Error::Invalid(_))
    ));
    assert_eq!(l.count("audit_events").await, count);
    sqlx::query("DROP TRIGGER type_source_update")
        .execute(&l.pool)
        .await
        .unwrap();
    sqlx::query("UPDATE type_version_sources SET external_id=999")
        .execute(&l.pool)
        .await
        .unwrap();
    assert!(matches!(
        l.types.seed_reference_fixtures().await,
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        l.types.seed_world_fixtures(&l.owner.actor.user_id).await,
        Err(Error::Invalid(_))
    ));
    assert_eq!(l.count("audit_events").await, count);
    let l = Lab::new().await;
    // A manifest identity occupied by authored content is a collision, not a seed baseline.
    let t = l
        .types
        .create_type(&l.admin.token, fields("Custom"), None)
        .await
        .unwrap();
    sqlx::query("DROP TRIGGER type_identity")
        .execute(&l.pool)
        .await
        .unwrap();
    let mut c = l.pool.acquire().await.unwrap();
    sqlx::query("PRAGMA foreign_keys=OFF")
        .execute(&mut *c)
        .await
        .unwrap();
    sqlx::query("UPDATE types SET id='6b4c13114c374eb6891469592b1743de' WHERE id=?")
        .bind(&t.id)
        .execute(&mut *c)
        .await
        .unwrap();
    sqlx::query("PRAGMA foreign_keys=ON")
        .execute(&mut *c)
        .await
        .unwrap();
    drop(c);
    let count = l.count("audit_events").await;
    assert!(matches!(
        l.types.seed_world_fixtures(&l.owner.actor.user_id).await,
        Err(Error::Invalid(_))
    ));
    assert_eq!(l.count("types").await, 1);
    assert_eq!(l.count("worlds").await, 0);
    assert_eq!(l.count("audit_events").await, count);
}

#[tokio::test]
async fn new_type_history_is_bounded_and_checks_exact_typed_snapshot_agreement() {
    let l = Lab::new().await;
    let mut t = l
        .types
        .create_type(&l.admin.token, fields("Type"), None)
        .await
        .unwrap();
    for n in 0..52 {
        t = l
            .types
            .save_type(
                &l.admin.token,
                &t.id,
                edit(t.revision, &format!("Type {n}")),
            )
            .await
            .unwrap();
    }
    let first = l
        .author
        .history(HistoryKind::Type, &t.id, None)
        .await
        .unwrap();
    assert_eq!(first.len(), 50);
    let next = l
        .author
        .history(
            HistoryKind::Type,
            &t.id,
            Some(first.last().unwrap().revision),
        )
        .await
        .unwrap();
    assert_eq!(next.len(), 3);
    assert_eq!(
        l.author
            .history_entry(HistoryKind::Type, &t.id, 999)
            .await
            .unwrap_err(),
        Error::NotFound
    );
    sqlx::query("DROP TRIGGER type_versions_update")
        .execute(&l.pool)
        .await
        .unwrap();
    sqlx::query("UPDATE type_versions SET after_json=json_set(after_json,'$.content.fields.display_name','Valid but inconsistent') WHERE type_id=? AND revision=1").bind(&t.id).execute(&l.pool).await.unwrap();
    assert_eq!(
        l.types.type_version(&t.id, 1).await.unwrap_err(),
        Error::Unavailable
    );
    assert_eq!(
        l.author
            .history_entry(HistoryKind::Type, &t.id, 1)
            .await
            .unwrap_err(),
        Error::Unavailable
    );
}

#[tokio::test]
async fn populated_0002_upgrade_preserves_v1_bytes_pins_users_sessions_audit_and_catalog_baselines()
{
    let dir = tempfile::tempdir().unwrap();
    let pool = persistence::connect_for_migration(&format!(
        "sqlite://{}/upgrade.db",
        dir.path().display()
    ))
    .await
    .unwrap();
    let migrations = vec![
        sqlx::migrate::Migration::new(
            1,
            "auth".into(),
            sqlx::migrate::MigrationType::Simple,
            include_str!("../migrations/0001_auth.sql").into_sql_str(),
            false,
        ),
        sqlx::migrate::Migration::new(
            2,
            "world authoring".into(),
            sqlx::migrate::MigrationType::Simple,
            include_str!("../migrations/0002_world_authoring.sql").into_sql_str(),
            false,
        ),
    ];
    sqlx::migrate::Migrator {
        migrations: std::borrow::Cow::Owned(migrations),
        ..sqlx::migrate::Migrator::DEFAULT
    }
    .run(&pool)
    .await
    .unwrap();
    let accounts = Accounts::new(pool.clone(), 3600).unwrap();
    let owner = user(&accounts, "Legacy admin", Role::Administrator).await;
    accounts
        .invite("test", "Pending invite", Role::Editor, None)
        .await
        .unwrap();
    let catalog = Catalog::from_json(BUNDLED_FIXTURES).unwrap();
    let mut tx = persistence::begin_write(&pool).await.unwrap();
    let mut default_id = String::new();
    let mut old_form_after = String::new();
    for (i, creature) in catalog.creatures().iter().enumerate() {
        let fields = ContentFields {
            slug: creature.slug().into(),
            display_name: creature.display_name().into(),
            description: creature.description().into(),
        };
        let s = SpeciesRecord {
            id: creature.id().into(),
            fields: fields.clone(),
            revision: 1,
            default_form_id: format!("b{i:031x}"),
            archived_at: None,
        };
        let f = FormRecord {
            id: s.default_form_id.clone(),
            species_id: s.id.clone(),
            species_version: 1,
            fields,
            revision: 1,
            is_default: true,
            archived_at: None,
        };
        sqlx::query("INSERT INTO species(id,slug,display_name,description,revision,default_form_id) VALUES (?,?,?,?,1,?)").bind(&s.id).bind(&s.fields.slug).bind(&s.fields.display_name).bind(&s.fields.description).bind(&f.id).execute(&mut *tx).await.unwrap();
        let a = format!("a{:031x}", i * 2);
        let b = format!("a{:031x}", i * 2 + 1);
        for audit in [&a, &b] {
            sqlx::query("INSERT INTO audit_events(id,system_key,action) VALUES (?,'bundled-fixture-loader','create')").bind(audit).execute(&mut *tx).await.unwrap();
        }
        sqlx::query("INSERT INTO species_versions(species_id,revision,slug,display_name,description,default_form_id,audit_id,after_json) VALUES (?,1,?,?,?,?,?,?)").bind(&s.id).bind(&s.fields.slug).bind(&s.fields.display_name).bind(&s.fields.description).bind(&f.id).bind(a).bind(envelope(&s)).execute(&mut *tx).await.unwrap();
        sqlx::query("INSERT INTO forms(id,species_id,species_version,slug,display_name,description,revision,is_default) VALUES (?,?,1,?,?,?,1,1)").bind(&f.id).bind(&s.id).bind(&f.fields.slug).bind(&f.fields.display_name).bind(&f.fields.description).execute(&mut *tx).await.unwrap();
        sqlx::query("INSERT INTO form_versions(form_id,species_id,species_version,revision,slug,display_name,description,is_default,audit_id,after_json) VALUES (?,?,1,1,?,?,?,1,?,?)").bind(&f.id).bind(&s.id).bind(&f.fields.slug).bind(&f.fields.display_name).bind(&f.fields.description).bind(b).bind(envelope(&f)).execute(&mut *tx).await.unwrap();
        if i == 0 {
            default_id = f.id.clone();
            old_form_after = envelope(&f);
        }
    }
    let w = WorldRecord {
        id: "cccccccccccccccccccccccccccccccc".into(),
        owner_user_id: owner.actor.user_id.clone(),
        fields: ContentFields {
            slug: "legacy-world".into(),
            display_name: "Legacy world".into(),
            description: "Legacy world.".into(),
        },
        revision: 1,
        access_revision: 1,
        archived_at: None,
    };
    let v = VariantRecord {
        id: "dddddddddddddddddddddddddddddddd".into(),
        world_id: w.id.clone(),
        source: Some(SourcePin {
            form_id: default_id.clone(),
            version: 1,
        }),
        fields: ContentFields {
            slug: "legacy-variant".into(),
            display_name: "Legacy variant".into(),
            description: "Legacy variant.".into(),
        },
        revision: 1,
        archived_at: None,
    };
    sqlx::query("INSERT INTO worlds(id,owner_user_id,slug,display_name,description,revision,access_revision) VALUES (?,?,?,?,?,1,1)").bind(&w.id).bind(&w.owner_user_id).bind(&w.fields.slug).bind(&w.fields.display_name).bind(&w.fields.description).execute(&mut *tx).await.unwrap();
    for (audit, action) in [
        ("eeeeeeeeeeeeeeeeeeeeeeeeeeeeeee1", "create"),
        ("eeeeeeeeeeeeeeeeeeeeeeeeeeeeeee2", "own"),
        ("eeeeeeeeeeeeeeeeeeeeeeeeeeeeeee3", "create"),
    ] {
        sqlx::query("INSERT INTO audit_events(id,actor_user_id,action) VALUES (?,?,?)")
            .bind(audit)
            .bind(&owner.actor.user_id)
            .bind(action)
            .execute(&mut *tx)
            .await
            .unwrap();
    }
    sqlx::query("INSERT INTO world_revisions(world_id,revision,audit_id,after_json) VALUES (?,1,'eeeeeeeeeeeeeeeeeeeeeeeeeeeeeee1',?)").bind(&w.id).bind(envelope(&serde_json::json!({"id":w.id,"fields":w.fields,"revision":1,"archived_at":null,"variant_ids":[v.id]}))).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO world_access_revisions(world_id,revision,audit_id,after_json) VALUES (?,1,'eeeeeeeeeeeeeeeeeeeeeeeeeeeeeee2',?)").bind(&w.id).bind(envelope(&serde_json::json!({"world_id":w.id,"owner_user_id":w.owner_user_id,"access_revision":1,"collaborator_ids":[]}))).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO variants(id,world_id,source_form_id,source_form_version,slug,display_name,description,revision) VALUES (?,?,?,1,?,?,?,1)").bind(&v.id).bind(&w.id).bind(&default_id).bind(&v.fields.slug).bind(&v.fields.display_name).bind(&v.fields.description).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO variant_revisions(world_id,variant_id,revision,audit_id,after_json) VALUES (?,?,1,'eeeeeeeeeeeeeeeeeeeeeeeeeeeeeee3',?)").bind(&w.id).bind(&v.id).bind(envelope(&v)).execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
    async fn retained(pool: &SqlitePool) -> Vec<String> {
        let mut values = Vec::new();
        for sql in [
            "SELECT json_array(species_id,revision,before_json,after_json,audit_id) FROM species_versions ORDER BY species_id,revision",
            "SELECT json_array(form_id,revision,before_json,after_json,audit_id) FROM form_versions ORDER BY form_id,revision",
            "SELECT json_array(world_id,revision,before_json,after_json,audit_id) FROM world_revisions ORDER BY world_id,revision",
            "SELECT json_array(world_id,variant_id,revision,before_json,after_json,audit_id) FROM variant_revisions ORDER BY world_id,variant_id,revision",
            "SELECT json_array(world_id,revision,before_json,after_json,audit_id) FROM world_access_revisions ORDER BY world_id,revision",
            "SELECT json_array(id,actor_user_id,system_key,occurred_at,action,reason) FROM audit_events ORDER BY id",
            "SELECT json_array(id,provider,provider_subject,display_name,role,is_disabled,created_at,updated_at) FROM users ORDER BY id",
            "SELECT json_array(token_hash,user_id,created_at,expires_at,revoked_at) FROM sessions ORDER BY token_hash",
            "SELECT json_array(id,provider_subject,role,note,consumed_at,consumed_by) FROM invites ORDER BY id",
        ] {
            values.extend(
                sqlx::query_scalar::<_, String>(sql)
                    .fetch_all(pool)
                    .await
                    .unwrap(),
            );
        }
        values
    }
    let baseline = retained(&pool).await;
    assert!(!persistence::schema_ready(&pool).await.unwrap());
    persistence::migrate(&pool).await.unwrap();
    assert!(persistence::schema_ready(&pool).await.unwrap());
    assert_eq!(retained(&pool).await, baseline);
    let markers:Vec<i64>=sqlx::query_scalar("SELECT content_snapshot_version FROM form_versions UNION ALL SELECT content_snapshot_version FROM world_revisions UNION ALL SELECT content_snapshot_version FROM variant_revisions").fetch_all(&pool).await.unwrap();
    assert!(markers.iter().all(|m| *m == 1));
    assert_eq!(
        accounts.authenticate(&owner.token).await.unwrap(),
        Some(owner.actor.clone())
    );
    let author = Authoring::new(accounts.clone());
    let types = Types::new(accounts);
    assert_eq!(
        author.seed_catalog(&catalog).await.unwrap().species_skipped,
        3
    );
    assert_eq!(
        types
            .form_typing(&default_id, None)
            .await
            .unwrap()
            .assignments,
        None
    );
    assert_eq!(
        types
            .variant_typing(&w.id, &v.id, None)
            .await
            .unwrap()
            .assignments,
        None
    );
    assert_eq!(author.variant_detail(&w.id, &v.id).await.unwrap(), v);
    assert_eq!(
        author
            .history_entry(HistoryKind::Form, &default_id, 1)
            .await
            .unwrap()
            .after_json,
        old_form_after
    );
    author
        .save_form(&owner.token, &default_id, 1, edit(1, "Migrated metadata"))
        .await
        .unwrap();
    let h = author
        .history_entry(HistoryKind::Form, &default_id, 2)
        .await
        .unwrap();
    assert_eq!(h.before_json, Some(old_form_after.clone()));
    assert_eq!(
        author
            .history_entry(HistoryKind::Form, &default_id, 1)
            .await
            .unwrap()
            .after_json,
        old_form_after
    );
    assert_eq!(
        types
            .form_typing(&default_id, None)
            .await
            .unwrap()
            .assignments,
        None
    );
    author
        .save_variant(&owner.token, &w.id, &v.id, edit(1, "Migrated variant"))
        .await
        .unwrap();
    author
        .save_world(&owner.token, &w.id, edit(1, "Migrated world"))
        .await
        .unwrap();
    assert_eq!(
        author.variant_detail(&w.id, &v.id).await.unwrap().source,
        v.source
    );
    assert_eq!(
        author.seed_catalog(&catalog).await.unwrap().species_skipped,
        3
    );
    // An additive sidecar cannot be attached to a retained marker-1 revision.
    let full = author
        .history_entry(HistoryKind::Form, &default_id, 2)
        .await
        .unwrap()
        .after_json;
    assert!(sqlx::query("INSERT INTO form_content_snapshots(form_id,revision,typing_defined,after_json) VALUES (?,1,0,?)").bind(&default_id).bind(full).execute(&pool).await.is_err());
    // Marker-1 history is strictly the old schema; typing cannot be retrofitted into it.
    sqlx::query("DROP TRIGGER form_versions_update")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE form_versions SET after_json=json_set(after_json,'$.content.typing',json('{\"assignments\":null}')) WHERE form_id=? AND revision=1").bind(&default_id).execute(&pool).await.unwrap();
    assert_eq!(
        author
            .history_entry(HistoryKind::Form, &default_id, 1)
            .await
            .unwrap_err(),
        Error::Unavailable
    );
    assert_eq!(
        types.form_typing(&default_id, Some(1)).await.unwrap_err(),
        Error::Unavailable
    );
    sqlx::query("UPDATE form_versions SET after_json=? WHERE form_id=? AND revision=1")
        .bind(&old_form_after)
        .bind(&default_id)
        .execute(&pool)
        .await
        .unwrap();
    // Retained marker-1 snapshots gain the same stable reference checks as v2.
    let foreign_world = author
        .create_world(&owner.token, fields("Foreign world"), None)
        .await
        .unwrap();
    let foreign_entry = author
        .create_variant(
            &owner.token,
            &foreign_world.id,
            1,
            None,
            fields("Foreign entry"),
            None,
        )
        .await
        .unwrap();
    let world_before: String = sqlx::query_scalar(
        "SELECT after_json FROM world_revisions WHERE world_id=? AND revision=1",
    )
    .bind(&w.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    sqlx::query("DROP TRIGGER world_revisions_update")
        .execute(&pool)
        .await
        .unwrap();
    for id in [
        foreign_entry.id.as_str(),
        "00000000000000000000000000000001",
    ] {
        sqlx::query("UPDATE world_revisions SET after_json=json_set(after_json,'$.content.variant_ids',json(?)) WHERE world_id=? AND revision=1")
            .bind(serde_json::json!([id]).to_string()).bind(&w.id).execute(&pool).await.unwrap();
        assert_eq!(
            author
                .history_entry(HistoryKind::World, &w.id, 1)
                .await
                .unwrap_err(),
            Error::Unavailable
        );
        assert_eq!(
            author
                .history(HistoryKind::World, &w.id, None)
                .await
                .unwrap_err(),
            Error::Unavailable
        );
        assert_eq!(
            author
                .save_world(
                    &owner.token,
                    &w.id,
                    edit(2, "Cannot hide legacy membership corruption")
                )
                .await
                .unwrap_err(),
            Error::Unavailable
        );
    }
    sqlx::query("UPDATE world_revisions SET after_json=? WHERE world_id=? AND revision=1")
        .bind(world_before)
        .bind(&w.id)
        .execute(&pool)
        .await
        .unwrap();
    let variant_before: String = sqlx::query_scalar(
        "SELECT after_json FROM variant_revisions WHERE variant_id=? AND revision=1",
    )
    .bind(&v.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    sqlx::query("DROP TRIGGER variant_revisions_update")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE variant_revisions SET after_json=json_set(after_json,'$.content.source.version',999) WHERE variant_id=? AND revision=1").bind(&v.id).execute(&pool).await.unwrap();
    assert_eq!(
        author
            .history_entry(HistoryKind::Variant, &v.id, 1)
            .await
            .unwrap_err(),
        Error::Unavailable
    );
    assert_eq!(
        types
            .variant_typing(&w.id, &v.id, Some(1))
            .await
            .unwrap_err(),
        Error::Unavailable
    );
    assert_eq!(
        author
            .save_variant(
                &owner.token,
                &w.id,
                &v.id,
                edit(2, "Cannot hide legacy pin corruption")
            )
            .await
            .unwrap_err(),
        Error::Unavailable
    );
    sqlx::query("UPDATE variant_revisions SET after_json=? WHERE variant_id=? AND revision=1")
        .bind(variant_before)
        .bind(&v.id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        author
            .history_entry(HistoryKind::World, &w.id, 1)
            .await
            .is_ok()
    );
    assert!(
        author
            .history_entry(HistoryKind::Variant, &v.id, 1)
            .await
            .is_ok()
    );
    let issues: Vec<(String, i64, String, i64)> = sqlx::query_as("PRAGMA foreign_key_check")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert!(issues.is_empty());
}

#[tokio::test]
async fn all_affected_aggregates_fail_closed_without_full_seals_and_on_private_or_typing_drift() {
    let l = Lab::new().await;
    let w = l
        .author
        .create_world(&l.owner.token, fields("World"), None)
        .await
        .unwrap();
    let t = l
        .types
        .create_world_type(&l.owner.token, &w.id, 1, None, fields("Mist"), None)
        .await
        .unwrap();
    let v = l
        .author
        .create_variant(&l.owner.token, &w.id, 2, None, fields("Creature"), None)
        .await
        .unwrap();
    l.types
        .save_variant_typing(
            &l.owner.token,
            &w.id,
            &v.id,
            1,
            Some(vec![pin(&t.id, 1)]),
            None,
        )
        .await
        .unwrap();
    let count = l.count("audit_events").await;
    for (kind, id, revision, table, key, trigger, bad_path) in [
        (
            HistoryKind::World,
            &w.id,
            3,
            "world_content_snapshots",
            "world_id",
            "world_content_update",
            "$.content.owner_user_id",
        ),
        (
            HistoryKind::Variant,
            &v.id,
            2,
            "variant_content_snapshots",
            "variant_id",
            "variant_content_update",
            "$.content.typing.assignments[0].pin.type_id",
        ),
    ] {
        let baseline: String = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT after_json FROM {table} WHERE {key}=? AND revision=?"
        )))
        .bind(id)
        .bind(revision)
        .fetch_one(&l.pool)
        .await
        .unwrap();
        sqlx::query(sqlx::AssertSqlSafe(format!("DROP TRIGGER {trigger}")))
            .execute(&l.pool)
            .await
            .unwrap();
        let sql = format!(
            "UPDATE {table} SET after_json=json_set(after_json,?1,?2) WHERE {key}=?3 AND revision=?4"
        );
        sqlx::query(sqlx::AssertSqlSafe(sql))
            .bind(bad_path)
            .bind(&l.owner.actor.user_id)
            .bind(id)
            .bind(revision)
            .execute(&l.pool)
            .await
            .unwrap();
        assert_eq!(
            l.author
                .history_entry(kind, id, revision)
                .await
                .unwrap_err(),
            Error::Unavailable
        );
        match kind {
            HistoryKind::World => assert_eq!(
                l.author
                    .save_world(&l.owner.token, id, edit(revision, "Must fail"))
                    .await
                    .unwrap_err(),
                Error::Unavailable
            ),
            HistoryKind::Variant => assert_eq!(
                l.types.variant_typing(&w.id, id, None).await.unwrap_err(),
                Error::Unavailable
            ),
            _ => unreachable!(),
        }
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "UPDATE {table} SET after_json=? WHERE {key}=? AND revision=?"
        )))
        .bind(baseline)
        .bind(id)
        .bind(revision)
        .execute(&l.pool)
        .await
        .unwrap();
    }
    for sql in [
        "INSERT INTO world_revisions(world_id,revision,previous_revision,audit_id,before_json,after_json) SELECT world_id,4,3,audit_id,after_json,after_json FROM world_revisions WHERE world_id=? AND revision=3",
        "INSERT INTO variant_revisions(world_id,variant_id,revision,previous_revision,audit_id,before_json,after_json) SELECT world_id,variant_id,3,2,audit_id,after_json,after_json FROM variant_revisions WHERE variant_id=? AND revision=2",
    ] {
        let id = if sql.contains("INSERT INTO world_revisions") {
            &w.id
        } else {
            &v.id
        };
        let e = sqlx::query(sql)
            .bind(id)
            .execute(&l.pool)
            .await
            .unwrap_err();
        assert_eq!(
            e.as_database_error().unwrap().code().as_deref(),
            Some("1811")
        );
    }
    assert_eq!(l.count("audit_events").await, count);
    l.integrity().await;
}

#[tokio::test]
async fn deferred_sidecar_marker_fk_rejects_orphans_and_marker_one_even_without_insert_guard() {
    let l = Lab::new().await;
    let s = l
        .author
        .create_species(&l.admin.token, fields("Creature"), None)
        .await
        .unwrap();
    let id = &s.default_form_id;
    let old: String = sqlx::query_scalar(
        "SELECT after_json FROM form_content_snapshots WHERE form_id=? AND revision=1",
    )
    .bind(id)
    .fetch_one(&l.pool)
    .await
    .unwrap();
    let mut after: serde_json::Value = serde_json::from_str(&old).unwrap();
    after["content"]["revision"] = 2.into();
    let mut tx = persistence::begin_write(&l.pool).await.unwrap();
    sqlx::query("INSERT INTO form_content_snapshots(form_id,revision,typing_defined,before_json,after_json) VALUES (?,2,0,?,?)").bind(id).bind(&old).bind(after.to_string()).execute(&mut *tx).await.unwrap();
    let error = tx.commit().await.unwrap_err();
    assert_eq!(
        error.as_database_error().unwrap().code().as_deref(),
        Some("787")
    );
    assert_eq!(l.count("form_content_snapshots").await, 1);
    // Isolate the deferred constant-marker FK independently from its insert guard.
    sqlx::query("DROP TRIGGER form_content_guard")
        .execute(&l.pool)
        .await
        .unwrap();
    sqlx::query("DROP TRIGGER form_stat_guard")
        .execute(&l.pool)
        .await
        .unwrap();
    let mut tx = persistence::begin_write(&l.pool).await.unwrap();
    sqlx::query("INSERT INTO form_content_snapshots(form_id,revision,typing_defined,before_json,after_json) VALUES (?,2,0,?,?)").bind(id).bind(&old).bind(after.to_string()).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO audit_events(id,system_key,action) VALUES ('aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa','bundled-fixture-loader','edit')").execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO form_versions(form_id,species_id,species_version,revision,slug,display_name,description,is_default,archived_at,audit_id,previous_revision,before_json,after_json,content_snapshot_version) SELECT form_id,species_id,species_version,2,slug,display_name,description,is_default,archived_at,'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',1,after_json,json_set(after_json,'$.content.revision',2),1 FROM form_versions WHERE form_id=? AND revision=1").bind(id).execute(&mut *tx).await.unwrap();
    let error = tx.commit().await.unwrap_err();
    assert_eq!(
        error.as_database_error().unwrap().code().as_deref(),
        Some("787")
    );
    assert_eq!(l.count("form_versions").await, 1);
    assert_eq!(l.count("form_content_snapshots").await, 1);
    l.integrity().await;
}

#[tokio::test]
async fn type_authorization_waits_for_the_writer_and_observes_revocation() {
    let l = Lab::new().await;
    let t = l
        .types
        .create_type(&l.admin.token, fields("Fire"), None)
        .await
        .unwrap();
    let mut tx = persistence::begin_write(&l.pool).await.unwrap();
    let service = l.types.clone();
    let token = l.admin.token.clone();
    let id = t.id.clone();
    let blocked = tokio::spawn(async move {
        service
            .save_type(&token, &id, edit(1, "Denied after revocation"))
            .await
    });
    persistence::revoke_user_sessions(&mut *tx, &l.admin.actor.user_id, 1)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert_eq!(blocked.await.unwrap().unwrap_err(), Error::Unauthorized);
    assert_eq!(l.types.type_detail(&t.id).await.unwrap(), t);
    let world = l
        .author
        .create_world(&l.owner.token, fields("World"), None)
        .await
        .unwrap();
    l.author
        .grant_collaborator(&l.owner.token, &world.id, 1, &l.editor.actor.user_id, None)
        .await
        .unwrap();
    let mut tx = persistence::begin_write(&l.pool).await.unwrap();
    let service = l.types.clone();
    let token = l.editor.token.clone();
    let world_id = world.id.clone();
    let blocked = tokio::spawn(async move {
        service
            .create_world_type(
                &token,
                &world_id,
                1,
                None,
                fields("Denied after grant removal"),
                None,
            )
            .await
    });
    sqlx::query("DELETE FROM world_collaborators WHERE world_id=? AND user_id=?")
        .bind(&world.id)
        .bind(&l.editor.actor.user_id)
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert_eq!(blocked.await.unwrap().unwrap_err(), Error::Forbidden);
    assert_eq!(l.count("world_types").await, 0);
    l.integrity().await;
}

#[tokio::test]
async fn fixture_creation_membership_and_private_ownership_baselines_are_checked_without_repair() {
    let l = Lab::new().await;
    l.types
        .seed_world_fixtures(&l.owner.actor.user_id)
        .await
        .unwrap();
    let world = l.author.worlds().await.unwrap().remove(0);
    let count = l.count("audit_events").await;
    let baseline = l
        .author
        .history_entry(HistoryKind::World, &world.id, 1)
        .await
        .unwrap()
        .after_json;
    sqlx::query("DROP TRIGGER world_content_update")
        .execute(&l.pool)
        .await
        .unwrap();
    sqlx::query("UPDATE world_content_snapshots SET after_json=json_set(after_json,'$.content.type_ids',json('[]')) WHERE world_id=? AND revision=1").bind(&world.id).execute(&l.pool).await.unwrap();
    assert!(matches!(
        l.types.seed_world_fixtures(&l.owner.actor.user_id).await,
        Err(Error::Invalid(_))
    ));
    assert_eq!(l.count("audit_events").await, count);
    sqlx::query("UPDATE world_content_snapshots SET after_json=? WHERE world_id=? AND revision=1")
        .bind(baseline)
        .bind(&world.id)
        .execute(&l.pool)
        .await
        .unwrap();
    sqlx::query("DROP TRIGGER access_revisions_update")
        .execute(&l.pool)
        .await
        .unwrap();
    sqlx::query("UPDATE world_access_revisions SET after_json=json_set(after_json,'$.content.owner_user_id',?) WHERE world_id=? AND revision=1").bind(&l.admin.actor.user_id).bind(&world.id).execute(&l.pool).await.unwrap();
    assert!(matches!(
        l.types.seed_world_fixtures(&l.owner.actor.user_id).await,
        Err(Error::Invalid(_))
    ));
    assert_eq!(l.count("audit_events").await, count);
}

// Construct matching legacy/full JSON with ordinary schema guards and FKs intact.
// Variant source pins live in JSON history but in typed columns on the current head.
async fn append_variant_snapshot(l: &Lab, head: &VariantRecord, snapshot: &VariantRecord) {
    assert_eq!(
        (head.id.as_str(), head.revision),
        (snapshot.id.as_str(), snapshot.revision)
    );
    let previous = head.revision - 1;
    let mut tx = persistence::begin_write(&l.pool).await.unwrap();
    let before: String = sqlx::query_scalar("SELECT after_json FROM variant_content_snapshots WHERE world_id=? AND variant_id=? AND revision=?")
        .bind(&head.world_id).bind(&head.id).bind(previous).fetch_one(&mut *tx).await.unwrap();
    let projection_before: String = sqlx::query_scalar(
        "SELECT after_json FROM variant_revisions WHERE world_id=? AND variant_id=? AND revision=?",
    )
    .bind(&head.world_id)
    .bind(&head.id)
    .bind(previous)
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    let mut after = serde_json::json!({"snapshot_version":2,"content":snapshot});
    after["content"]["typing"] = serde_json::json!({"assignments":null});
    sqlx::query("UPDATE variants SET revision=?,slug=?,display_name=?,description=?,source_form_id=?,source_form_version=?,archived_at=? WHERE world_id=? AND id=?")
        .bind(head.revision).bind(&head.fields.slug).bind(&head.fields.display_name).bind(&head.fields.description)
        .bind(head.source.as_ref().map(|pin| &pin.form_id)).bind(head.source.as_ref().map(|pin| pin.version)).bind(head.archived_at)
        .bind(&head.world_id).bind(&head.id).execute(&mut *tx).await.unwrap();
    let audit = format!("ee{:030x}", head.revision);
    sqlx::query("INSERT INTO audit_events(id,actor_user_id,action) VALUES (?,?,'edit')")
        .bind(&audit)
        .bind(&l.owner.actor.user_id)
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("INSERT INTO variant_content_snapshots(world_id,variant_id,revision,typing_defined,before_json,after_json) VALUES (?,?,?,0,?,?)")
        .bind(&head.world_id).bind(&head.id).bind(head.revision).bind(before).bind(after.to_string()).execute(&mut *tx).await.unwrap();
    let authoritative_before: String = sqlx::query_scalar("SELECT after_json FROM variant_stat_snapshots WHERE world_id=? AND variant_id=? AND revision=?")
        .bind(&head.world_id).bind(&head.id).bind(previous).fetch_one(&mut *tx).await.unwrap();
    after["snapshot_version"] = 3.into();
    after["content"]["stats"] = serde_json::json!({"roster":null,"assignments":null});
    sqlx::query("INSERT INTO variant_stat_snapshots(world_id,variant_id,revision,stats_defined,before_json,after_json) VALUES (?,?,?,0,?,?)")
        .bind(&head.world_id).bind(&head.id).bind(head.revision).bind(authoritative_before).bind(after.to_string()).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO variant_revisions(world_id,variant_id,revision,previous_revision,audit_id,before_json,after_json,content_snapshot_version,stat_snapshot_version) VALUES (?,?,?,?,?,?,?,2,3)")
        .bind(&head.world_id).bind(&head.id).bind(head.revision).bind(previous).bind(&audit).bind(projection_before).bind(envelope(snapshot)).execute(&mut *tx).await.unwrap();
    // Raw historical provisioning must include the same-revision card evidence.
    // Use the historical record, not the deliberately different current head.
    let card = game_data::cards::CardRecord {
        identity: game_data::cards::CardIdentity {
            namespace: game_data::cards::CardNamespace::Variant,
            world_id: Some(snapshot.world_id.clone()),
            id: snapshot.id.clone(),
        },
        revision: snapshot.revision,
        fields: snapshot.fields.clone(),
        archived_at: snapshot.archived_at,
        source: snapshot.source.as_ref().map(|p| game_data::cards::CardPin {
            identity: game_data::cards::CardIdentity {
                namespace: game_data::cards::CardNamespace::Form,
                world_id: None,
                id: p.form_id.clone(),
            },
            version: p.version,
        }),
        component: Default::default(),
        legacy_unmodeled: false,
    };
    sqlx::query("INSERT INTO card_components(namespace,scope,card_id,revision,fields_defined,rules_text,labels_defined,audit_id,snapshot_json) VALUES ('variant',?,?,?,0,NULL,0,?,?)").bind(&head.world_id).bind(&head.id).bind(head.revision).bind(&audit).bind(envelope(&card)).execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
    l.integrity().await;
}

#[tokio::test]
async fn sealed_variant_source_pins_require_the_exact_form_even_in_older_history() {
    let l = Lab::new().await;
    let species = l
        .author
        .create_species(&l.admin.token, fields("Source"), None)
        .await
        .unwrap();
    l.author
        .save_form(
            &l.admin.token,
            &species.default_form_id,
            1,
            edit(1, "Source version two"),
        )
        .await
        .unwrap();
    let world = l
        .author
        .create_world(&l.owner.token, fields("World"), None)
        .await
        .unwrap();
    let entry = l
        .author
        .create_variant(
            &l.owner.token,
            &world.id,
            1,
            Some(SourcePin {
                form_id: species.default_form_id,
                version: 1,
            }),
            fields("Local"),
            None,
        )
        .await
        .unwrap();
    let mut head = entry.clone();
    head.revision = 2;
    let mut broken = head.clone();
    broken.source.as_mut().unwrap().version = 999;
    append_variant_snapshot(&l, &head, &broken).await;
    // The head's typed FK remains valid, and both JSON schemas and all SQL constraints hold.
    assert_eq!(
        l.author.variant_detail(&world.id, &entry.id).await.unwrap(),
        head
    );
    let count = l.count("audit_events").await;
    for revision in [None, Some(2)] {
        assert_eq!(
            l.types
                .variant_typing(&world.id, &entry.id, revision)
                .await
                .unwrap_err(),
            Error::Unavailable
        );
    }
    assert_eq!(
        l.author
            .history_entry(HistoryKind::Variant, &entry.id, 2)
            .await
            .unwrap_err(),
        Error::Unavailable
    );
    assert_eq!(
        l.author
            .history(HistoryKind::Variant, &entry.id, None)
            .await
            .unwrap_err(),
        Error::Unavailable
    );
    assert_eq!(
        l.author
            .save_variant(&l.owner.token, &world.id, &entry.id, edit(2, "Must fail"))
            .await
            .unwrap_err(),
        Error::Unavailable
    );
    assert_eq!(
        l.author
            .adopt_variant(
                &l.owner.token,
                &world.id,
                &entry.id,
                2,
                edit(2, "Must fail")
            )
            .await
            .unwrap_err(),
        Error::Unavailable
    );
    assert_eq!(
        l.author
            .adoption_preview(&world.id, &entry.id, 2)
            .await
            .unwrap_err(),
        Error::Unavailable
    );
    assert_eq!(
        l.types
            .save_variant_typing(&l.owner.token, &world.id, &entry.id, 2, Some(vec![]), None)
            .await
            .unwrap_err(),
        Error::Unavailable
    );
    assert_eq!(l.count("audit_events").await, count);
    // A later valid head must not hide the nonexistent source in an older sealed revision.
    head.revision = 3;
    append_variant_snapshot(&l, &head, &head).await;
    assert_eq!(
        l.author
            .history_entry(HistoryKind::Variant, &entry.id, 2)
            .await
            .unwrap_err(),
        Error::Unavailable
    );
    assert_eq!(
        l.author
            .history_entry(HistoryKind::Variant, &entry.id, 3)
            .await
            .unwrap_err(),
        Error::Unavailable
    );
    assert_eq!(
        l.types
            .variant_typing(&world.id, &entry.id, None)
            .await
            .unwrap_err(),
        Error::Unavailable
    );
    assert_eq!(
        l.author
            .save_variant(
                &l.owner.token,
                &world.id,
                &entry.id,
                edit(3, "Must still fail")
            )
            .await
            .unwrap_err(),
        Error::Unavailable
    );
    assert!(
        l.author
            .history_entry(HistoryKind::Variant, &entry.id, 1)
            .await
            .is_ok()
    );
    l.integrity().await;
}

#[tokio::test]
async fn variant_head_projection_is_checked_without_reinterpreting_historical_local_values() {
    let l = Lab::new().await;
    let species = l
        .author
        .create_species(&l.admin.token, fields("Source"), None)
        .await
        .unwrap();
    l.author
        .save_form(
            &l.admin.token,
            &species.default_form_id,
            1,
            edit(1, "Source version two"),
        )
        .await
        .unwrap();
    let world = l
        .author
        .create_world(&l.owner.token, fields("World"), None)
        .await
        .unwrap();
    let entry = l
        .author
        .create_variant(
            &l.owner.token,
            &world.id,
            1,
            Some(SourcePin {
                form_id: species.default_form_id,
                version: 1,
            }),
            fields("Local"),
            None,
        )
        .await
        .unwrap();
    let mut head = entry.clone();
    head.revision = 2;
    let mut historical = head.clone();
    historical.source.as_mut().unwrap().version = 2; // real exact source, distinct from head
    historical.fields.display_name = "Historical local name".into();
    historical.fields.slug = "historical-local-name".into();
    append_variant_snapshot(&l, &head, &historical).await;
    assert_eq!(
        l.author
            .history_entry(HistoryKind::Variant, &entry.id, 2)
            .await
            .unwrap_err(),
        Error::Unavailable
    );
    assert_eq!(
        l.types
            .variant_typing(&world.id, &entry.id, None)
            .await
            .unwrap_err(),
        Error::Unavailable
    );
    assert_eq!(
        l.author
            .save_variant(&l.owner.token, &world.id, &entry.id, edit(2, "Must fail"))
            .await
            .unwrap_err(),
        Error::Unavailable
    );
    head.revision = 3;
    head.fields.display_name = "Current local name".into();
    head.fields.slug = "current-local-name".into();
    append_variant_snapshot(&l, &head, &head).await;
    let old = l
        .author
        .history_entry(HistoryKind::Variant, &entry.id, 2)
        .await
        .unwrap();
    let old: serde_json::Value = serde_json::from_str(&old.after_json).unwrap();
    assert_eq!(old["content"]["source"]["version"], 2);
    assert_eq!(
        old["content"]["fields"]["display_name"],
        "Historical local name"
    );
    assert_eq!(
        l.author
            .history(HistoryKind::Variant, &entry.id, None)
            .await
            .unwrap()
            .len(),
        3
    );
    assert_eq!(
        l.types
            .variant_typing(&world.id, &entry.id, Some(2))
            .await
            .unwrap()
            .assignments,
        None
    );
    l.author
        .save_variant(
            &l.owner.token,
            &world.id,
            &entry.id,
            edit(3, "Another current name"),
        )
        .await
        .unwrap();
    assert!(
        l.author
            .history_entry(HistoryKind::Variant, &entry.id, 2)
            .await
            .is_ok()
    );
    l.integrity().await;
}

#[tokio::test]
async fn sealed_world_membership_rejects_foreign_and_missing_variants_without_schema_corruption() {
    let l = Lab::new().await;
    let foreign = l
        .author
        .create_world(&l.owner.token, fields("Foreign world"), None)
        .await
        .unwrap();
    let foreign_variant = l
        .author
        .create_variant(
            &l.owner.token,
            &foreign.id,
            1,
            None,
            fields("Foreign entry"),
            None,
        )
        .await
        .unwrap();
    for (i, bad_id) in [
        foreign_variant.id.as_str(),
        "00000000000000000000000000000001",
    ]
    .into_iter()
    .enumerate()
    {
        let world = l
            .author
            .create_world(&l.owner.token, fields(&format!("World {i}")), None)
            .await
            .unwrap();
        let own = l
            .author
            .create_variant(
                &l.owner.token,
                &world.id,
                1,
                None,
                fields("Own entry"),
                None,
            )
            .await
            .unwrap();
        let before = l
            .author
            .history_entry(HistoryKind::World, &world.id, 2)
            .await
            .unwrap()
            .after_json;
        let mut after: serde_json::Value = serde_json::from_str(&before).unwrap();
        after["content"]["revision"] = 3.into();
        after["content"]["variant_ids"] = serde_json::json!([bad_id]);
        let mut projection = after.clone();
        projection["snapshot_version"] = 1.into();
        projection["content"]
            .as_object_mut()
            .unwrap()
            .remove("type_ids");
        let mut tx = persistence::begin_write(&l.pool).await.unwrap();
        let projection_before: String = sqlx::query_scalar(
            "SELECT after_json FROM world_revisions WHERE world_id=? AND revision=2",
        )
        .bind(&world.id)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
        sqlx::query("UPDATE worlds SET revision=3 WHERE id=?")
            .bind(&world.id)
            .execute(&mut *tx)
            .await
            .unwrap();
        let audit = format!("dd{i:030x}");
        sqlx::query("INSERT INTO audit_events(id,actor_user_id,action) VALUES (?,?,'edit')")
            .bind(&audit)
            .bind(&l.owner.actor.user_id)
            .execute(&mut *tx)
            .await
            .unwrap();
        sqlx::query("INSERT INTO world_content_snapshots(world_id,revision,before_json,after_json) VALUES (?,3,?,?)").bind(&world.id).bind(before).bind(after.to_string()).execute(&mut *tx).await.unwrap();
        sqlx::query("INSERT INTO world_revisions(world_id,revision,previous_revision,audit_id,before_json,after_json,content_snapshot_version) VALUES (?,3,2,?,?,?,2)").bind(&world.id).bind(audit).bind(projection_before).bind(projection.to_string()).execute(&mut *tx).await.unwrap();
        tx.commit().await.unwrap();
        l.integrity().await;
        let count = l.count("audit_events").await;
        assert_eq!(
            l.author
                .history_entry(HistoryKind::World, &world.id, 3)
                .await
                .unwrap_err(),
            Error::Unavailable
        );
        assert_eq!(
            l.author
                .history(HistoryKind::World, &world.id, None)
                .await
                .unwrap_err(),
            Error::Unavailable
        );
        assert_eq!(
            l.author
                .save_world(&l.owner.token, &world.id, edit(3, "Must fail"))
                .await
                .unwrap_err(),
            Error::Unavailable
        );
        assert_eq!(
            l.author
                .create_variant(
                    &l.owner.token,
                    &world.id,
                    3,
                    None,
                    fields("Must roll back"),
                    None
                )
                .await
                .unwrap_err(),
            Error::Unavailable
        );
        assert_eq!(
            l.types
                .create_world_type(
                    &l.owner.token,
                    &world.id,
                    3,
                    None,
                    fields("Must roll back"),
                    None
                )
                .await
                .unwrap_err(),
            Error::Unavailable
        );
        assert_eq!(l.count("audit_events").await, count);
        assert_eq!(l.author.variants(&world.id).await.unwrap(), vec![own]);
    }
}

#[tokio::test]
async fn historical_world_membership_retains_archived_identities_and_smaller_member_sets() {
    let l = Lab::new().await;
    let world = l
        .author
        .create_world(&l.owner.token, fields("World"), None)
        .await
        .unwrap();
    let local_type = l
        .types
        .create_world_type(&l.owner.token, &world.id, 1, None, fields("Mist"), None)
        .await
        .unwrap();
    let entry = l
        .author
        .create_variant(&l.owner.token, &world.id, 2, None, fields("First"), None)
        .await
        .unwrap();
    let baseline = l
        .author
        .history_entry(HistoryKind::World, &world.id, 3)
        .await
        .unwrap()
        .after_json;
    l.author
        .create_variant(&l.owner.token, &world.id, 3, None, fields("Second"), None)
        .await
        .unwrap();
    // Only identity/scope are relevant to membership reads, including retained archived entries.
    sqlx::query(
        "UPDATE variants SET archived_at=1,display_name='Renamed current entry' WHERE id=?",
    )
    .bind(&entry.id)
    .execute(&l.pool)
    .await
    .unwrap();
    sqlx::query(
        "UPDATE world_types SET archived_at=1,display_name='Renamed current Type' WHERE id=?",
    )
    .bind(&local_type.id)
    .execute(&l.pool)
    .await
    .unwrap();
    assert_eq!(
        l.author
            .history_entry(HistoryKind::World, &world.id, 3)
            .await
            .unwrap()
            .after_json,
        baseline
    );
    assert!(
        l.author
            .history_entry(HistoryKind::World, &world.id, 1)
            .await
            .is_ok()
    );
    l.author
        .save_world(&l.owner.token, &world.id, edit(4, "World metadata edit"))
        .await
        .unwrap();
    assert_eq!(
        l.author
            .history_entry(HistoryKind::World, &world.id, 3)
            .await
            .unwrap()
            .after_json,
        baseline
    );
    l.integrity().await;
}

#[tokio::test]
async fn world_and_variant_missing_v2_sidecars_fail_history_typing_and_writes_closed() {
    for kind in [HistoryKind::World, HistoryKind::Variant] {
        let l = Lab::new().await;
        let world = l
            .author
            .create_world(&l.owner.token, fields("World"), None)
            .await
            .unwrap();
        let entry = l
            .author
            .create_variant(&l.owner.token, &world.id, 1, None, fields("Entry"), None)
            .await
            .unwrap();
        let (table, trigger, id, revision) = match kind {
            HistoryKind::World => (
                "world_content_snapshots",
                "world_content_delete",
                &world.id,
                2,
            ),
            HistoryKind::Variant => (
                "variant_content_snapshots",
                "variant_content_delete",
                &entry.id,
                1,
            ),
            _ => unreachable!(),
        };
        let mut c = l.pool.acquire().await.unwrap();
        sqlx::query(sqlx::AssertSqlSafe(format!("DROP TRIGGER {trigger}")))
            .execute(&mut *c)
            .await
            .unwrap();
        // Test-only corruption of an otherwise valid temporary database.
        sqlx::query("PRAGMA foreign_keys=OFF")
            .execute(&mut *c)
            .await
            .unwrap();
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DELETE FROM {table} WHERE world_id=? AND revision=?"
        )))
        .bind(&world.id)
        .bind(revision)
        .execute(&mut *c)
        .await
        .unwrap();
        sqlx::query("PRAGMA foreign_keys=ON")
            .execute(&mut *c)
            .await
            .unwrap();
        drop(c);
        let count = l.count("audit_events").await;
        assert_eq!(
            l.author
                .history_entry(kind, id, revision)
                .await
                .unwrap_err(),
            Error::Unavailable
        );
        assert_eq!(
            l.author.history(kind, id, None).await.unwrap_err(),
            Error::Unavailable
        );
        match kind {
            HistoryKind::World => {
                assert_eq!(
                    l.author
                        .save_world(&l.owner.token, &world.id, edit(2, "Must fail"))
                        .await
                        .unwrap_err(),
                    Error::Unavailable
                );
                assert_eq!(
                    l.types
                        .create_world_type(
                            &l.owner.token,
                            &world.id,
                            2,
                            None,
                            fields("Must roll back"),
                            None
                        )
                        .await
                        .unwrap_err(),
                    Error::Unavailable
                );
                assert_eq!(
                    l.author
                        .create_variant(
                            &l.owner.token,
                            &world.id,
                            2,
                            None,
                            fields("Must roll back"),
                            None
                        )
                        .await
                        .unwrap_err(),
                    Error::Unavailable
                );
                assert_eq!(l.count("world_types").await, 0);
            }
            HistoryKind::Variant => {
                assert_eq!(
                    l.types
                        .variant_typing(&world.id, &entry.id, None)
                        .await
                        .unwrap_err(),
                    Error::Unavailable
                );
                assert_eq!(
                    l.types
                        .variant_typing(&world.id, &entry.id, Some(1))
                        .await
                        .unwrap_err(),
                    Error::Unavailable
                );
                assert_eq!(
                    l.author
                        .save_variant(&l.owner.token, &world.id, &entry.id, edit(1, "Must fail"))
                        .await
                        .unwrap_err(),
                    Error::Unavailable
                );
                assert_eq!(
                    l.types
                        .save_variant_typing(
                            &l.owner.token,
                            &world.id,
                            &entry.id,
                            1,
                            Some(vec![]),
                            None
                        )
                        .await
                        .unwrap_err(),
                    Error::Unavailable
                );
            }
            _ => unreachable!(),
        }
        assert_eq!(l.count("audit_events").await, count);
        assert_eq!(l.count("variants").await, 1);
    }
}

#[tokio::test]
async fn sidecar_before_tags_require_explicit_supported_integer_versions() {
    let l = Lab::new().await;
    let species = l
        .author
        .create_species(&l.admin.token, fields("Source"), None)
        .await
        .unwrap();
    let world = l
        .author
        .create_world(&l.owner.token, fields("World"), None)
        .await
        .unwrap();
    let entry = l
        .author
        .create_variant(&l.owner.token, &world.id, 1, None, fields("Entry"), None)
        .await
        .unwrap();
    for (kind, id, revision, sql) in [
        (
            HistoryKind::Form,
            &species.default_form_id,
            1,
            "INSERT INTO form_content_snapshots(form_id,revision,typing_defined,before_json,after_json) VALUES (?1,2,0,?2,?3)",
        ),
        (
            HistoryKind::Variant,
            &entry.id,
            1,
            "INSERT INTO variant_content_snapshots(world_id,variant_id,revision,typing_defined,before_json,after_json) VALUES (?4,?1,2,0,?2,?3)",
        ),
        (
            HistoryKind::World,
            &world.id,
            2,
            "INSERT INTO world_content_snapshots(world_id,revision,before_json,after_json) VALUES (?1,3,?2,?3)",
        ),
    ] {
        let baseline = l
            .author
            .history_entry(kind, id, revision)
            .await
            .unwrap()
            .after_json;
        let mut after: serde_json::Value = serde_json::from_str(&baseline).unwrap();
        after["snapshot_version"] = 2.into();
        after["content"].as_object_mut().unwrap().remove("stats");
        after["content"]["revision"] = (revision + 1).into();
        let mut tx = persistence::begin_write(&l.pool).await.unwrap();
        for bad in [
            serde_json::json!({}),
            serde_json::json!({"snapshot_version":null}),
            serde_json::json!({"snapshot_version":3}),
            serde_json::json!({"snapshot_version":"1"}),
            serde_json::json!({"snapshot_version":true}),
            serde_json::json!({"snapshot_version":1.0}),
        ] {
            let query = sqlx::query(sql)
                .bind(id)
                .bind(bad.to_string())
                .bind(after.to_string());
            let query = if matches!(kind, HistoryKind::Variant) {
                query.bind(&world.id)
            } else {
                query
            };
            let error = query.execute(&mut *tx).await.unwrap_err();
            assert_eq!(
                error.as_database_error().unwrap().code().as_deref(),
                Some("275"),
                "{kind:?}: {bad}"
            );
        }
        // Both legacy-v1 and full-v2 before snapshots remain accepted by the SQL seal.
        for version in [1, 2] {
            let before = serde_json::json!({"snapshot_version":version,"content":{}});
            let query = sqlx::query(sql)
                .bind(id)
                .bind(before.to_string())
                .bind(after.to_string());
            let query = if matches!(kind, HistoryKind::Variant) {
                query.bind(&world.id)
            } else {
                query
            };
            query.execute(&mut *tx).await.unwrap();
            tx.rollback().await.unwrap();
            tx = persistence::begin_write(&l.pool).await.unwrap();
        }
        tx.rollback().await.unwrap();
    }
    l.integrity().await;
}

#[tokio::test]
async fn unmodeled_and_defined_empty_typing_survive_metadata_edits_and_source_adoption() {
    let l = Lab::new().await;
    let species = l
        .author
        .create_species(&l.admin.token, fields("Source"), None)
        .await
        .unwrap();
    l.author
        .save_form(
            &l.admin.token,
            &species.default_form_id,
            1,
            edit(1, "Source two"),
        )
        .await
        .unwrap();
    l.types
        .save_form_typing(
            &l.admin.token,
            &species.default_form_id,
            2,
            Some(vec![]),
            None,
        )
        .await
        .unwrap();
    l.author
        .save_form(
            &l.admin.token,
            &species.default_form_id,
            1,
            edit(3, "Typeless metadata"),
        )
        .await
        .unwrap();
    assert_eq!(
        l.types
            .form_typing(&species.default_form_id, None)
            .await
            .unwrap()
            .assignments,
        Some(vec![])
    );
    assert_eq!(
        l.types
            .form_typing(&species.default_form_id, Some(2))
            .await
            .unwrap()
            .assignments,
        None
    );
    let world = l
        .author
        .create_world(&l.owner.token, fields("World"), None)
        .await
        .unwrap();
    for (i, pins) in [None, Some(vec![])].into_iter().enumerate() {
        let mut entry = l
            .author
            .create_variant(
                &l.owner.token,
                &world.id,
                1 + i as i64,
                Some(SourcePin {
                    form_id: species.default_form_id.clone(),
                    version: 1,
                }),
                fields(&format!("Entry {i}")),
                None,
            )
            .await
            .unwrap();
        entry = l
            .types
            .save_variant_typing(
                &l.owner.token,
                &world.id,
                &entry.id,
                entry.revision,
                pins.clone(),
                None,
            )
            .await
            .unwrap();
        entry = l
            .author
            .save_variant(
                &l.owner.token,
                &world.id,
                &entry.id,
                edit(entry.revision, "Metadata edit"),
            )
            .await
            .unwrap();
        let preview = l
            .author
            .adoption_preview(&world.id, &entry.id, 4)
            .await
            .unwrap();
        assert_eq!(preview.old_typing.assignments, None);
        assert_eq!(preview.new_typing.assignments, Some(vec![]));
        assert_eq!(preview.local_typing.pins(), pins);
        entry = l
            .author
            .adopt_variant(
                &l.owner.token,
                &world.id,
                &entry.id,
                4,
                edit(entry.revision, "Reviewed adoption"),
            )
            .await
            .unwrap();
        assert_eq!(
            l.types
                .variant_typing(&world.id, &entry.id, None)
                .await
                .unwrap()
                .pins(),
            pins
        );
        assert_eq!(entry.source.as_ref().unwrap().version, 4);
        let before = l.count("audit_events").await;
        l.types
            .save_variant_typing(
                &l.owner.token,
                &world.id,
                &entry.id,
                entry.revision,
                pins,
                None,
            )
            .await
            .unwrap();
        assert_eq!(l.count("audit_events").await, before);
    }
    l.integrity().await;
}
