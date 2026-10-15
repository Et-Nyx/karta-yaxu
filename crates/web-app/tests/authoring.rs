use game_data::{BUNDLED_FIXTURES, Catalog, authoring::ContentFields};
use sqlx::{SqlSafeStr, SqlitePool};
use tempfile::TempDir;
use web_app::{
    accounts::Accounts,
    application::authoring::{AddressKey, Authoring, Edit, Error, HistoryKind, SourcePin},
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
    service: Authoring,
    admin: User,
    owner: User,
    editor: User,
    other_admin: User,
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
            "sqlite://{}/test.db",
            dir.path().display()
        ))
        .await
        .unwrap();
        persistence::migrate(&pool).await.unwrap();
        let accounts = Accounts::new(pool.clone(), 3600).unwrap();
        let admin = user(&accounts, "Alice", Role::Administrator).await;
        let owner = user(&accounts, "Owner", Role::Editor).await;
        let editor = user(&accounts, "Collaborator", Role::Editor).await;
        let other_admin = user(&accounts, "OtherAdmin", Role::Administrator).await;
        let viewer = user(&accounts, "Viewer", Role::Viewer).await;
        Self {
            _dir: dir,
            pool,
            service: Authoring::new(accounts.clone()),
            accounts,
            admin,
            owner,
            editor,
            other_admin,
            viewer,
        }
    }
    async fn count(&self, table: &'static str) -> i64 {
        sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {table}")))
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }
}
fn fields(slug: &str) -> ContentFields {
    ContentFields {
        slug: slug.into(),
        display_name: slug.into(),
        description: "First paragraph.\n\nSecond paragraph.".into(),
    }
}
fn edit(revision: i64, slug: &str) -> Edit {
    Edit {
        expected_revision: revision,
        fields: fields(slug),
        reason: Some("Reviewed change".into()),
    }
}
fn bundled() -> Catalog {
    Catalog::from_json(BUNDLED_FIXTURES).unwrap()
}

#[tokio::test]
async fn generated_addresses_ignore_forgery_and_rename_without_changing_identity_or_pins() {
    let l = Lab::new().await;
    let mut submitted = fields("forged / BAD");
    submitted.display_name = " Éclair 🦄 ".into();
    let s = l
        .service
        .create_species(&l.admin.token, submitted.clone(), None)
        .await
        .unwrap();
    assert_eq!(s.fields.slug, "eclair-unicorn");
    assert_eq!(s.fields.display_name, "Éclair 🦄");
    let default = l.service.form_detail(&s.default_form_id).await.unwrap();
    assert_eq!(default.fields, s.fields);
    let w = l
        .service
        .create_world(&l.owner.token, submitted.clone(), None)
        .await
        .unwrap();
    assert_eq!(w.fields.slug, "eclair-unicorn");
    let f = l
        .service
        .create_form(&l.admin.token, &s.id, 1, submitted.clone(), None)
        .await
        .unwrap();
    assert_eq!(f.fields.slug, "eclair-unicorn-2");
    let pin = SourcePin {
        form_id: f.id.clone(),
        version: 1,
    };
    let v = l
        .service
        .create_variant(&l.owner.token, &w.id, 1, Some(pin.clone()), submitted, None)
        .await
        .unwrap();
    assert_eq!(v.fields.slug, "eclair-unicorn");

    // Whitespace-only name normalization and forged slugs are true no-ops.
    let unchanged = |revision, fields: &ContentFields| {
        let mut fields = fields.clone();
        fields.slug = "ignore-me".into();
        fields.display_name = format!(" {} ", fields.display_name);
        Edit {
            expected_revision: revision,
            fields,
            reason: None,
        }
    };
    let baseline = l.count("audit_events").await;
    assert_eq!(
        l.service
            .save_species(&l.admin.token, &s.id, unchanged(1, &s.fields))
            .await
            .unwrap(),
        s
    );
    assert_eq!(
        l.service
            .save_form(&l.admin.token, &f.id, 1, unchanged(1, &f.fields))
            .await
            .unwrap(),
        f
    );
    assert_eq!(
        l.service
            .save_world(&l.owner.token, &w.id, unchanged(2, &w.fields))
            .await
            .unwrap()
            .fields,
        w.fields
    );
    assert_eq!(
        l.service
            .save_variant(&l.owner.token, &w.id, &v.id, unchanged(1, &v.fields))
            .await
            .unwrap(),
        v
    );
    assert_eq!(l.count("audit_events").await, baseline);

    // Change only names: the old submitted slug must not dictate the new address.
    let rename = |revision, fields: &ContentFields, name: &str| {
        let mut fields = fields.clone();
        fields.display_name = name.into();
        Edit {
            expected_revision: revision,
            fields,
            reason: Some("Rename".into()),
        }
    };
    let s2 = l
        .service
        .save_species(&l.admin.token, &s.id, rename(1, &s.fields, "北亰"))
        .await
        .unwrap();
    assert_eq!(s2.id, s.id);
    assert_eq!(s2.default_form_id, s.default_form_id);
    assert_eq!(s2.fields.slug, "bei-jing");
    assert_eq!(l.service.form_detail(&default.id).await.unwrap(), default);
    let f2 = l
        .service
        .save_form(&l.admin.token, &f.id, 2, rename(1, &f.fields, "げんまい茶"))
        .await
        .unwrap();
    assert_eq!(f2.id, f.id);
    assert_eq!(f2.species_id, s.id);
    assert_eq!(f2.fields.slug, "genmaicha");
    let w2 = l
        .service
        .save_world(&l.owner.token, &w.id, rename(2, &w.fields, "New world"))
        .await
        .unwrap();
    assert_eq!(w2.id, w.id);
    assert_eq!(w2.owner_user_id, w.owner_user_id);
    assert_eq!(w2.fields.slug, "new-world");
    assert_eq!(l.service.variant_detail(&w.id, &v.id).await.unwrap(), v);
    let v2 = l
        .service
        .save_variant(
            &l.owner.token,
            &w.id,
            &v.id,
            rename(1, &v.fields, "... !!!"),
        )
        .await
        .unwrap();
    assert_eq!(v2.id, v.id);
    assert_eq!(v2.world_id, w.id);
    assert_eq!(v2.fields.slug, "object");
    assert_eq!(v2.source, Some(pin));
    assert_eq!(l.service.form_version(&f.id, 1).await.unwrap(), f);
    assert_eq!(l.service.species_version(&s.id, 1).await.unwrap(), s);
    let before_stale = l.count("audit_events").await;
    assert_eq!(
        l.service
            .save_variant(&l.owner.token, &w.id, &v.id, rename(1, &v.fields, "stale"))
            .await
            .unwrap_err(),
        Error::Conflict
    );
    assert_eq!(l.service.variant_detail(&w.id, &v.id).await.unwrap(), v2);
    assert_eq!(l.count("audit_events").await, before_stale);

    // Pin-only adoption retains the address despite different source names/slugs.
    let adopted = l
        .service
        .adopt_variant(&l.owner.token, &w.id, &v.id, 2, unchanged(2, &v2.fields))
        .await
        .unwrap();
    assert_eq!(adopted.fields, v2.fields);
    assert_eq!(adopted.source.unwrap().version, 2);
    let form_pin_only = l
        .service
        .save_form(
            &l.admin.token,
            &default.id,
            2,
            unchanged(1, &default.fields),
        )
        .await
        .unwrap();
    assert_eq!(form_pin_only.fields, default.fields);
    assert_eq!(form_pin_only.species_version, 2);
}

#[tokio::test]
async fn duplicate_species_names_allocate_default_form_addresses_in_each_new_scope() {
    let l = Lab::new().await;
    let first = l
        .service
        .create_species(&l.admin.token, fields("Emberling"), Some("Initial species"))
        .await
        .unwrap();
    let second = l
        .service
        .create_species(&l.admin.token, fields("Emberling"), Some("Initial species"))
        .await
        .unwrap();
    assert_ne!(first.id, second.id);
    assert_ne!(first.default_form_id, second.default_form_id);
    assert_eq!(l.count("species").await, 2);
    assert_eq!(l.count("species_versions").await, 2);
    assert_eq!(l.count("forms").await, 2);
    assert_eq!(l.count("form_versions").await, 2);
    assert_eq!(l.count("audit_events").await, 4);
    for (species, slug) in [(&first, "emberling"), (&second, "emberling-2")] {
        assert_eq!(species.fields.slug, slug);
        assert_eq!(species.revision, 1);
        let form = l
            .service
            .form_detail(&species.default_form_id)
            .await
            .unwrap();
        assert_eq!(form.species_id, species.id);
        assert_eq!((form.species_version, form.revision), (1, 1));
        assert!(form.is_default);
        assert_eq!(form.fields.slug, "emberling");
        assert_eq!(form.fields.display_name, species.fields.display_name);
        assert_eq!(form.fields.description, species.fields.description);
        assert_eq!(l.service.form_version(&form.id, 1).await.unwrap(), form);
        assert_eq!(
            l.service
                .resolve_form(
                    AddressKey::Slug(slug.into()),
                    AddressKey::Slug("emberling".into())
                )
                .await
                .unwrap(),
            (species.clone(), form.clone())
        );
        for (kind, id, expected_slug) in [
            (HistoryKind::Species, &species.id, slug),
            (HistoryKind::Form, &form.id, "emberling"),
        ] {
            let history = l.service.history_entry(kind, id, 1).await.unwrap();
            assert_eq!(history.actor_name, "Alice");
            assert_eq!(history.action, "create");
            assert_eq!(history.reason.as_deref(), Some("Initial species"));
            assert!(history.previous_revision.is_none());
            assert!(history.before_json.is_none());
            let content = snapshot_content(&history.after_json);
            assert_eq!(content["id"], *id);
            assert_eq!(content["fields"]["slug"], expected_slug);
        }
    }
    let world = l
        .service
        .create_world(&l.owner.token, fields("world"), None)
        .await
        .unwrap();
    for (species, revision) in [(&first, 1), (&second, 2)] {
        let pin = SourcePin {
            form_id: species.default_form_id.clone(),
            version: 1,
        };
        let variant = l
            .service
            .create_variant(
                &l.owner.token,
                &world.id,
                revision,
                Some(pin.clone()),
                fields("Emberling"),
                None,
            )
            .await
            .unwrap();
        assert_eq!(variant.source, Some(pin));
        assert_eq!(
            l.service
                .form_version(&species.default_form_id, 1)
                .await
                .unwrap()
                .species_id,
            species.id
        );
    }
}

#[tokio::test]
async fn collisions_use_existing_scopes_and_preserve_suffixes_on_description_edits() {
    let l = Lab::new().await;
    let a = l
        .service
        .create_species(&l.admin.token, fields("same"), None)
        .await
        .unwrap();
    let b = l
        .service
        .create_species(&l.admin.token, fields("same"), None)
        .await
        .unwrap();
    assert_eq!(a.fields.slug, "same");
    assert_eq!(b.fields.slug, "same-2");
    let c = l
        .service
        .create_species(&l.admin.token, fields("same-2"), None)
        .await
        .unwrap();
    assert_eq!(c.fields.slug, "same-2-2");
    let a2 = l
        .service
        .save_species(&l.admin.token, &a.id, edit(1, "same-2"))
        .await
        .unwrap();
    assert_eq!(a2.fields.slug, "same-2-3");
    // Self is excluded when a name change transliterates to the same base.
    let a3 = l
        .service
        .save_species(&l.admin.token, &a.id, edit(2, "SAME-2"))
        .await
        .unwrap();
    assert_eq!(a3.fields.slug, "same-2-3");
    let free = l
        .service
        .create_species(&l.admin.token, fields("same"), None)
        .await
        .unwrap();
    assert_eq!(free.fields.slug, "same");
    let fa = l
        .service
        .create_form(&l.admin.token, &a.id, 1, fields("same"), None)
        .await
        .unwrap();
    assert_eq!(fa.fields.slug, "same-2"); // the original default form still owns same
    let fb = l
        .service
        .create_form(&l.admin.token, &b.id, 1, fields("same"), None)
        .await
        .unwrap();
    assert_eq!(fb.fields.slug, "same-2"); // each species' default form owns its own same
    let fa2 = l
        .service
        .create_form(&l.admin.token, &a.id, 1, fields("same"), None)
        .await
        .unwrap();
    assert_eq!(fa2.fields.slug, "same-3");
    let wa = l
        .service
        .create_world(&l.owner.token, fields("same"), None)
        .await
        .unwrap();
    let wb = l
        .service
        .create_world(&l.owner.token, fields("same"), None)
        .await
        .unwrap();
    assert_eq!(wa.fields.slug, "same");
    assert_eq!(wb.fields.slug, "same-2");
    let va = l
        .service
        .create_variant(&l.owner.token, &wa.id, 1, None, fields("same"), None)
        .await
        .unwrap();
    let va2 = l
        .service
        .create_variant(&l.owner.token, &wa.id, 2, None, fields("same"), None)
        .await
        .unwrap();
    let vb = l
        .service
        .create_variant(&l.owner.token, &wb.id, 1, None, fields("same"), None)
        .await
        .unwrap();
    assert_eq!(va.fields.slug, "same");
    assert_eq!(va2.fields.slug, "same-2");
    assert_eq!(vb.fields.slug, "same");
    let desc = |revision, fields: &ContentFields| {
        let mut fields = fields.clone();
        fields.slug = "forged".into();
        fields.description = "Description-only edit".into();
        Edit {
            expected_revision: revision,
            fields,
            reason: None,
        }
    };
    assert_eq!(
        l.service
            .save_species(&l.admin.token, &b.id, desc(1, &b.fields))
            .await
            .unwrap()
            .fields
            .slug,
        "same-2"
    );
    assert_eq!(
        l.service
            .save_form(&l.admin.token, &fa2.id, 1, desc(1, &fa2.fields))
            .await
            .unwrap()
            .fields
            .slug,
        "same-3"
    );
    assert_eq!(
        l.service
            .save_world(&l.owner.token, &wb.id, desc(2, &wb.fields))
            .await
            .unwrap()
            .fields
            .slug,
        "same-2"
    );
    assert_eq!(
        l.service
            .save_variant(&l.owner.token, &wa.id, &va2.id, desc(1, &va2.fields))
            .await
            .unwrap()
            .fields
            .slug,
        "same-2"
    );

    let long = fields(&"é".repeat(100));
    let first = l
        .service
        .create_world(&l.owner.token, long.clone(), None)
        .await
        .unwrap();
    let second = l
        .service
        .create_world(&l.owner.token, long, None)
        .await
        .unwrap();
    assert_eq!(first.fields.slug, "e".repeat(80));
    assert_eq!(second.fields.slug, format!("{}-2", "e".repeat(78)));
    assert!(second.fields.normalized().is_ok());
}

#[tokio::test]
async fn explicit_legacy_fixture_addresses_survive_unchanged_names_and_history_reads() {
    let l = Lab::new().await;
    let mut data: serde_json::Value = serde_json::from_str(BUNDLED_FIXTURES).unwrap();
    data[0]["slug"] = "manual-legacy-77".into();
    data[0]["display_name"] = "Éclair".into();
    let fixture_id = data[0]["id"].as_str().unwrap().to_owned();
    let catalog = Catalog::from_json(&data.to_string()).unwrap();
    l.service.seed_catalog(&catalog).await.unwrap();
    let s = l.service.species_detail(&fixture_id).await.unwrap();
    assert_eq!(s.fields.slug, "manual-legacy-77");
    let mut fields = s.fields.clone();
    fields.slug = "ignored".into();
    fields.display_name = " Éclair ".into();
    fields.description = "Description change".into();
    let e = Edit {
        expected_revision: 1,
        fields,
        reason: None,
    };
    let saved = l
        .service
        .save_species(&l.admin.token, &s.id, e.clone())
        .await
        .unwrap();
    let form = l
        .service
        .save_form(&l.admin.token, &s.default_form_id, 1, e)
        .await
        .unwrap();
    assert_eq!(saved.fields.slug, s.fields.slug);
    assert_eq!(form.fields.slug, s.fields.slug);
    assert_eq!(
        l.service
            .seed_catalog(&catalog)
            .await
            .unwrap()
            .species_skipped,
        3
    );
    assert_eq!(l.service.species_version(&s.id, 1).await.unwrap(), s);
    let history = l
        .service
        .history(HistoryKind::Species, &s.id, None)
        .await
        .unwrap();
    assert_snapshot_continuity(&history);
    assert_eq!(
        snapshot_content(&history[1].after_json)["fields"]["slug"],
        "manual-legacy-77"
    );
    let renamed = l
        .service
        .save_species(&l.admin.token, &s.id, edit(2, "Éclair deluxe"))
        .await
        .unwrap();
    assert_eq!(renamed.fields.slug, "eclair-deluxe");
    assert_eq!(renamed.id, s.id);
    assert_eq!(
        l.service
            .seed_catalog(&catalog)
            .await
            .unwrap()
            .species_skipped,
        3
    );
    assert_eq!(
        l.service.form_detail(&s.default_form_id).await.unwrap(),
        form
    );
}

#[tokio::test]
async fn concurrent_creates_and_renames_allocate_distinct_addresses() {
    let l = Lab::new().await;
    let (one, two) = tokio::join!(
        l.service
            .create_species(&l.admin.token, fields("racing"), None),
        l.service
            .create_species(&l.admin.token, fields("racing"), None),
    );
    let (one, two) = (one.unwrap(), two.unwrap());
    assert_ne!(one.id, two.id);
    let mut slugs = vec![one.fields.slug.clone(), two.fields.slug.clone()];
    slugs.sort();
    assert_eq!(slugs, ["racing", "racing-2"]);
    let (a, b) = tokio::join!(
        l.service
            .save_species(&l.admin.token, &one.id, edit(1, "new race")),
        l.service
            .save_species(&l.admin.token, &two.id, edit(1, "new race")),
    );
    let (a, b) = (a.unwrap(), b.unwrap());
    let mut slugs = vec![a.fields.slug, b.fields.slug];
    slugs.sort();
    assert_eq!(slugs, ["new-race", "new-race-2"]);
    assert_eq!(l.count("species_versions").await, 4);
    assert_eq!(l.count("form_versions").await, 2);
    assert_eq!(l.count("audit_events").await, 6);
    let (wa, wb) = tokio::join!(
        l.service
            .create_world(&l.owner.token, fields("world race"), None),
        l.service
            .create_world(&l.owner.token, fields("world race"), None),
    );
    let (wa, wb) = (wa.unwrap(), wb.unwrap());
    assert_ne!(wa.fields.slug, wb.fields.slug);
    assert_eq!(l.count("world_revisions").await, 2);
    assert_eq!(l.count("world_access_revisions").await, 2);
}

#[tokio::test]
async fn exhausted_candidate_search_is_a_name_error_and_leaves_no_partial_mutation() {
    let l = Lab::new().await;
    let w = l
        .service
        .create_world(&l.owner.token, fields("template"), None)
        .await
        .unwrap();
    let world_json = l
        .service
        .history_entry(HistoryKind::World, &w.id, 1)
        .await
        .unwrap()
        .after_json;
    let access_json = l
        .service
        .access(&l.owner.token, &w.id)
        .await
        .unwrap()
        .history[0]
        .after_json
        .clone();
    // Bulk-build valid test-owned worlds with attributed revisions, not 10,000 HTTP calls.
    let mut tx = persistence::begin_write(&l.pool).await.unwrap();
    sqlx::query("WITH RECURSIVE numbers(n) AS (VALUES(1) UNION ALL SELECT n+1 FROM numbers WHERE n < 10000) INSERT INTO worlds(id, owner_user_id, slug, display_name, description, revision, access_revision) SELECT printf('%032x',n), ?1, CASE n WHEN 1 THEN 'crowded' ELSE 'crowded-' || n END, 'crowded', ?2, 1, 1 FROM numbers")
        .bind(&l.owner.actor.user_id).bind(&w.fields.description).execute(&mut *tx).await.unwrap();
    sqlx::query("WITH RECURSIVE numbers(n) AS (VALUES(1) UNION ALL SELECT n+1 FROM numbers WHERE n < 10000) INSERT INTO audit_events(id, actor_user_id, action) SELECT printf('%032x',100000+n), ?1, 'create' FROM numbers UNION ALL SELECT printf('%032x',200000+n), ?1, 'own' FROM numbers")
        .bind(&l.owner.actor.user_id).execute(&mut *tx).await.unwrap();
    sqlx::query("WITH RECURSIVE numbers(n) AS (VALUES(1) UNION ALL SELECT n+1 FROM numbers WHERE n < 10000) INSERT INTO world_content_snapshots(world_id, revision, after_json) SELECT printf('%032x',n), 1, json_set(?1, '$.content.id', printf('%032x',n), '$.content.fields.display_name', 'crowded', '$.content.fields.slug', CASE n WHEN 1 THEN 'crowded' ELSE 'crowded-' || n END) FROM numbers")
        .bind(&world_json).execute(&mut *tx).await.unwrap();
    sqlx::query("WITH RECURSIVE numbers(n) AS (VALUES(1) UNION ALL SELECT n+1 FROM numbers WHERE n < 10000) INSERT INTO world_revisions(world_id,revision,audit_id,after_json,content_snapshot_version) SELECT s.world_id,1,printf('%032x',100000+n),json_remove(json_set(s.after_json,'$.snapshot_version',1),'$.content.type_ids'),2 FROM numbers JOIN world_content_snapshots s ON s.world_id=printf('%032x',n)")
        .execute(&mut *tx).await.unwrap();
    sqlx::query("WITH RECURSIVE numbers(n) AS (VALUES(1) UNION ALL SELECT n+1 FROM numbers WHERE n < 10000) INSERT INTO world_access_revisions(world_id, revision, audit_id, after_json) SELECT printf('%032x',n), 1, printf('%032x',200000+n), json_set(?1, '$.content.world_id', printf('%032x',n)) FROM numbers")
        .bind(access_json).execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
    let before = l.count("audit_events").await;
    assert!(
        matches!(l.service.create_world(&l.owner.token, fields("crowded"), None).await, Err(Error::Invalid(e)) if e.len() == 1 && e.contains_key("display_name"))
    );
    assert!(
        matches!(l.service.save_world(&l.owner.token, &w.id, edit(1, "crowded")).await, Err(Error::Invalid(e)) if e.len() == 1 && e.contains_key("display_name"))
    );
    assert_eq!(
        l.service
            .save_world(&l.owner.token, &w.id, edit(0, "crowded"))
            .await
            .unwrap_err(),
        Error::Conflict
    );
    assert_eq!(
        l.service
            .create_world("revoked", fields("crowded"), None)
            .await
            .unwrap_err(),
        Error::Unauthorized
    );
    assert!(
        matches!(l.service.create_world(&l.owner.token, fields("crowded"), Some(&"x".repeat(501))).await, Err(Error::Invalid(e)) if e.contains_key("reason") && !e.contains_key("display_name"))
    );
    assert_eq!(l.service.world_detail(&w.id).await.unwrap(), w);
    assert_eq!(l.count("worlds").await, 10001);
    assert_eq!(l.count("world_revisions").await, 10001);
    assert_eq!(l.count("world_access_revisions").await, 10001);
    assert_eq!(l.count("audit_events").await, before);
    // Removing the last candidate from the scope makes exactly candidate 10,000 available.
    sqlx::query("UPDATE worlds SET slug = 'moved' WHERE id = printf('%032x',10000)")
        .execute(&l.pool)
        .await
        .unwrap();
    let last = l
        .service
        .create_world(&l.owner.token, fields("crowded"), None)
        .await
        .unwrap();
    assert_eq!(last.fields.slug, "crowded-10000");
}

#[tokio::test]
async fn read_address_resolvers_are_strict_current_and_parent_scoped() {
    let l = Lab::new().await;
    let s = l
        .service
        .create_species(&l.admin.token, fields("species"), None)
        .await
        .unwrap();
    let t = l
        .service
        .create_species(&l.admin.token, fields("other"), None)
        .await
        .unwrap();
    let f = l
        .service
        .create_form(&l.admin.token, &s.id, 1, fields("shared"), None)
        .await
        .unwrap();
    let g = l
        .service
        .create_form(&l.admin.token, &t.id, 1, fields("shared"), None)
        .await
        .unwrap();
    let w = l
        .service
        .create_world(&l.owner.token, fields("world"), None)
        .await
        .unwrap();
    let x = l
        .service
        .create_world(&l.owner.token, fields("other"), None)
        .await
        .unwrap();
    let v = l
        .service
        .create_variant(
            &l.owner.token,
            &w.id,
            1,
            Some(SourcePin {
                form_id: f.id.clone(),
                version: 1,
            }),
            fields("shared"),
            None,
        )
        .await
        .unwrap();
    let z = l
        .service
        .create_variant(&l.owner.token, &x.id, 1, None, fields("shared"), None)
        .await
        .unwrap();
    let id = |s: &str| AddressKey::Id(s.into());
    let slug = |s: &str| AddressKey::Slug(s.into());
    for key in [id(&s.id), slug("species")] {
        assert_eq!(l.service.resolve_species(key).await.unwrap(), s);
    }
    for key in [id(&w.id), slug("world")] {
        assert_eq!(
            l.service.resolve_world(key).await.unwrap(),
            l.service.world_detail(&w.id).await.unwrap()
        );
    }
    for parent in [id(&s.id), slug("species")] {
        for child in [id(&f.id), slug("shared")] {
            assert_eq!(
                l.service.resolve_form(parent.clone(), child).await.unwrap(),
                (s.clone(), f.clone())
            );
        }
    }
    assert_eq!(
        l.service
            .resolve_form(slug("other"), slug("shared"))
            .await
            .unwrap()
            .1,
        g
    );
    assert_eq!(
        l.service.resolve_legacy_form(&f.id).await.unwrap(),
        (s.clone(), f.clone())
    );
    assert_eq!(
        l.service.resolve_legacy_form("shared").await.unwrap_err(),
        Error::NotFound
    );
    assert_eq!(
        l.service
            .resolve_variant(slug("other"), slug("shared"))
            .await
            .unwrap()
            .1,
        z
    );
    for parent in [id(&w.id), slug("world")] {
        for child in [id(&v.id), slug("shared")] {
            assert_eq!(
                l.service
                    .resolve_variant(parent.clone(), child)
                    .await
                    .unwrap()
                    .1,
                v
            );
        }
    }
    assert_eq!(
        l.service
            .resolve_form(id(&t.id), id(&f.id))
            .await
            .unwrap_err(),
        Error::NotFound
    );
    assert_eq!(
        l.service
            .resolve_variant(id(&x.id), id(&v.id))
            .await
            .unwrap_err(),
        Error::NotFound
    );
    assert_eq!(
        l.service.resolve_species(id("species")).await.unwrap_err(),
        Error::NotFound
    );
    assert_eq!(
        l.service.resolve_world(id("world")).await.unwrap_err(),
        Error::NotFound
    );
    assert_eq!(
        l.service
            .resolve_form(id(&s.id), id("shared"))
            .await
            .unwrap_err(),
        Error::NotFound
    );
    assert_eq!(
        l.service
            .resolve_variant(id(&w.id), id("shared"))
            .await
            .unwrap_err(),
        Error::NotFound
    );
    assert_eq!(
        l.service.species_detail("species").await.unwrap_err(),
        Error::NotFound
    );
    assert_eq!(
        l.service.world_detail("world").await.unwrap_err(),
        Error::NotFound
    );
    assert_eq!(
        l.service.form_detail("shared").await.unwrap_err(),
        Error::NotFound
    );
    assert_eq!(
        l.service.variant_detail(&w.id, "shared").await.unwrap_err(),
        Error::NotFound
    );
    assert_eq!(
        l.service
            .resolve_species(slug("' OR 1=1 --"))
            .await
            .unwrap_err(),
        Error::NotFound
    );

    let s2 = l
        .service
        .save_species(&l.admin.token, &s.id, edit(1, "renamed species"))
        .await
        .unwrap();
    let w2 = l
        .service
        .save_world(&l.owner.token, &w.id, edit(2, "renamed world"))
        .await
        .unwrap();
    assert_eq!(
        l.service
            .resolve_form(slug("renamed-species"), slug("shared"))
            .await
            .unwrap(),
        (s2.clone(), f.clone())
    );
    assert_eq!(
        l.service.resolve_legacy_form(&f.id).await.unwrap(),
        (s2, f.clone())
    );
    assert_eq!(
        l.service
            .resolve_variant(slug("renamed-world"), slug("shared"))
            .await
            .unwrap(),
        (w2, v.clone())
    );
    assert_eq!(
        l.service
            .resolve_species(slug("species"))
            .await
            .unwrap_err(),
        Error::NotFound
    );
    assert_eq!(
        l.service.resolve_world(slug("world")).await.unwrap_err(),
        Error::NotFound
    );
    assert_eq!(l.service.form_detail(&f.id).await.unwrap(), f);
    assert_eq!(l.service.variant_detail(&w.id, &v.id).await.unwrap(), v);

    // A 32hex slug which equals another record's id is still unambiguous.
    let shadow = l
        .service
        .create_species(&l.admin.token, fields(&t.id), None)
        .await
        .unwrap();
    assert_eq!(l.service.resolve_species(id(&t.id)).await.unwrap(), t);
    assert_eq!(
        l.service.resolve_species(slug(&t.id)).await.unwrap(),
        shadow
    );
    let hex_slug = "00000000000000000000000000000001";
    let hex = l
        .service
        .create_world(&l.owner.token, fields(hex_slug), None)
        .await
        .unwrap();
    assert_eq!(l.service.resolve_world(slug(hex_slug)).await.unwrap(), hex);
    assert_eq!(
        l.service.resolve_world(id(hex_slug)).await.unwrap_err(),
        Error::NotFound
    );

    // Resolution reads no grants/access history, even if private history is broken.
    sqlx::query("DROP TRIGGER access_revisions_update")
        .execute(&l.pool)
        .await
        .unwrap();
    sqlx::query("UPDATE world_access_revisions SET after_json = '{\"snapshot_version\":1}' WHERE world_id = ?").bind(&w.id).execute(&l.pool).await.unwrap();
    assert!(
        l.service
            .resolve_variant(id(&w.id), id(&v.id))
            .await
            .is_ok()
    );
    assert_eq!(
        l.service.access(&l.owner.token, &w.id).await.unwrap_err(),
        Error::Unavailable
    );
}

#[tokio::test]
async fn nested_address_reads_keep_one_snapshot_during_competing_writes() {
    let l = Lab::new().await;
    let s = l
        .service
        .create_species(&l.admin.token, fields("initial"), None)
        .await
        .unwrap();
    let w = l
        .service
        .create_world(&l.owner.token, fields("initial"), None)
        .await
        .unwrap();
    let v = l
        .service
        .create_variant(&l.owner.token, &w.id, 1, None, fields("initial"), None)
        .await
        .unwrap();
    let pool = l.pool.clone();
    let species_id = s.id.clone();
    let form_id = s.default_form_id.clone();
    let world_id = w.id.clone();
    let variant_id = v.id.clone();
    let writer = tokio::spawn(async move {
        // Inject paired fixture changes in one transaction so a split read would
        // expose mismatched descriptions. Only these temporary test rows change.
        for n in 0..100 {
            let mut tx = persistence::begin_write(&pool).await.unwrap();
            let description = format!("Snapshot generation {n}");
            for (sql, id) in [
                (
                    "UPDATE species SET description = ? WHERE id = ?",
                    &species_id,
                ),
                ("UPDATE forms SET description = ? WHERE id = ?", &form_id),
                ("UPDATE worlds SET description = ? WHERE id = ?", &world_id),
                (
                    "UPDATE variants SET description = ? WHERE id = ?",
                    &variant_id,
                ),
            ] {
                sqlx::query(sql)
                    .bind(&description)
                    .bind(id)
                    .execute(&mut *tx)
                    .await
                    .unwrap();
            }
            tx.commit().await.unwrap();
            tokio::task::yield_now().await;
        }
    });
    for _ in 0..100 {
        let (form, legacy, variant) = tokio::join!(
            l.service.resolve_form(
                AddressKey::Slug("initial".into()),
                AddressKey::Slug("initial".into())
            ),
            l.service.resolve_legacy_form(&s.default_form_id),
            l.service.resolve_variant(
                AddressKey::Slug("initial".into()),
                AddressKey::Id(v.id.clone())
            ),
        );
        let (species, form) = form.unwrap();
        let (legacy_species, legacy_form) = legacy.unwrap();
        let (world, variant) = variant.unwrap();
        assert_eq!(species.fields.description, form.fields.description);
        assert_eq!(
            legacy_species.fields.description,
            legacy_form.fields.description
        );
        assert_eq!(world.fields.description, variant.fields.description);
    }
    writer.await.unwrap();
}

#[tokio::test]
async fn direct_service_roles_ownership_and_private_access() {
    let l = Lab::new().await;
    assert_eq!(
        l.service
            .create_species(&l.owner.token, fields("denied"), None)
            .await
            .unwrap_err(),
        Error::Forbidden
    );
    assert_eq!(
        l.service
            .create_world(&l.viewer.token, fields("denied"), None)
            .await
            .unwrap_err(),
        Error::Forbidden
    );
    assert_eq!(
        l.service
            .create_world("invalid", fields("denied"), None)
            .await
            .unwrap_err(),
        Error::Unauthorized
    );
    let s = l
        .service
        .create_species(
            &l.admin.token,
            fields("species"),
            Some("Initial library entry"),
        )
        .await
        .unwrap();
    assert_eq!(l.service.forms(&s.id).await.unwrap().len(), 1);
    assert!(
        l.service
            .form_detail(&s.default_form_id)
            .await
            .unwrap()
            .is_default
    );
    assert_eq!(
        l.service
            .save_species(&l.editor.token, &s.id, edit(1, "denied"))
            .await
            .unwrap_err(),
        Error::Forbidden
    );
    assert_eq!(
        l.service
            .create_form(&l.editor.token, &s.id, 1, fields("denied"), None)
            .await
            .unwrap_err(),
        Error::Forbidden
    );
    assert_eq!(
        l.service
            .save_form(&l.viewer.token, &s.default_form_id, 1, edit(1, "denied"))
            .await
            .unwrap_err(),
        Error::Forbidden
    );
    let w = l
        .service
        .create_world(&l.owner.token, fields("world"), None)
        .await
        .unwrap();
    assert_eq!(w.owner_user_id, l.owner.actor.user_id);
    for u in [&l.admin, &l.other_admin, &l.editor, &l.viewer] {
        assert_eq!(
            l.service
                .save_world(&u.token, &w.id, edit(1, "denied"))
                .await
                .unwrap_err(),
            Error::Forbidden
        );
        assert_eq!(
            l.service.access(&u.token, &w.id).await.unwrap_err(),
            Error::Forbidden
        );
        assert_eq!(
            l.service
                .grant_collaborator(&u.token, &w.id, 1, &l.editor.actor.user_id, None)
                .await
                .unwrap_err(),
            Error::Forbidden
        );
    }
    assert!(matches!(
        l.service
            .grant_collaborator(&l.owner.token, &w.id, 1, &l.viewer.actor.user_id, None)
            .await,
        Err(Error::Invalid(_))
    ));
    l.accounts
        .set_disabled(&l.editor.actor.user_id, true)
        .await
        .unwrap();
    assert!(matches!(
        l.service
            .grant_collaborator(&l.owner.token, &w.id, 1, &l.editor.actor.user_id, None)
            .await,
        Err(Error::Invalid(_))
    ));
    l.accounts
        .set_disabled(&l.editor.actor.user_id, false)
        .await
        .unwrap();
    let (editor_token, _) = l
        .accounts
        .start_session(&l.editor.actor.user_id)
        .await
        .unwrap();
    l.service
        .grant_collaborator(
            &l.owner.token,
            &w.id,
            1,
            &l.editor.actor.user_id,
            Some("Team member"),
        )
        .await
        .unwrap();
    let current = l.service.world_detail(&w.id).await.unwrap();
    assert_eq!((current.revision, current.access_revision), (1, 2));
    assert_eq!(
        l.service.access(&editor_token, &w.id).await.unwrap_err(),
        Error::Forbidden
    );
    let caps = l
        .service
        .capabilities(Some(&editor_token), Some(&w.id))
        .await
        .unwrap();
    assert!(caps.can_edit_world && !caps.can_manage_access && !caps.can_edit_library);
    l.service
        .save_world(&editor_token, &w.id, edit(1, "edited"))
        .await
        .unwrap();
    l.service
        .revoke_collaborator(&l.owner.token, &w.id, 2, &l.editor.actor.user_id, None)
        .await
        .unwrap();
    assert_eq!(
        l.service
            .save_world(&editor_token, &w.id, edit(2, "denied"))
            .await
            .unwrap_err(),
        Error::Forbidden
    );
    assert_eq!(
        l.service.world_detail(&w.id).await.unwrap().access_revision,
        3
    );
    let access = l.service.access(&l.owner.token, &w.id).await.unwrap();
    assert_eq!(access.history.len(), 3);
    assert!(access.collaborators.is_empty());
    for h in l
        .service
        .history(HistoryKind::World, &w.id, None)
        .await
        .unwrap()
    {
        assert!(
            !h.after_json.contains("collaborator")
                && !h.after_json.contains("owner_user_id")
                && !h.after_json.contains("access_revision")
        );
        assert!(!h.after_json.contains(&l.editor.actor.user_id));
    }
    l.accounts.sign_out(&l.owner.token).await.unwrap();
    assert_eq!(
        l.service
            .save_world(&l.owner.token, &w.id, edit(2, "denied"))
            .await
            .unwrap_err(),
        Error::Unauthorized
    );
    assert!(
        !l.service
            .capabilities(Some(&l.owner.token), Some(&w.id))
            .await
            .unwrap()
            .authenticated
    );
    assert!(
        !l.service
            .capabilities(None, None)
            .await
            .unwrap()
            .authenticated
    );
}

#[tokio::test]
async fn scoped_variants_exact_dependency_pins_and_explicit_adoption() {
    let l = Lab::new().await;
    let s = l
        .service
        .create_species(&l.admin.token, fields("species"), None)
        .await
        .unwrap();
    let pin = SourcePin {
        form_id: s.default_form_id.clone(),
        version: 1,
    };
    let a = l
        .service
        .create_world(&l.owner.token, fields("a"), None)
        .await
        .unwrap();
    let b = l
        .service
        .create_world(&l.other_admin.token, fields("b"), None)
        .await
        .unwrap();
    let v = l
        .service
        .create_variant(
            &l.owner.token,
            &a.id,
            1,
            Some(pin.clone()),
            fields("local-one"),
            None,
        )
        .await
        .unwrap();
    l.service
        .create_variant(
            &l.owner.token,
            &a.id,
            2,
            Some(pin.clone()),
            fields("local-two"),
            None,
        )
        .await
        .unwrap();
    let exclusive = l
        .service
        .create_variant(&l.owner.token, &a.id, 3, None, fields("exclusive"), None)
        .await
        .unwrap();
    l.service
        .create_variant(
            &l.other_admin.token,
            &b.id,
            1,
            Some(pin.clone()),
            fields("local-one"),
            None,
        )
        .await
        .unwrap();
    assert_eq!(l.service.variants(&a.id).await.unwrap().len(), 3);
    assert_eq!(l.service.availability(&s.id).await.unwrap().len(), 2);
    assert_eq!(
        l.service.variant_detail(&b.id, &v.id).await.unwrap_err(),
        Error::NotFound
    );
    assert_eq!(
        l.service
            .save_variant(&l.other_admin.token, &b.id, &v.id, edit(1, "spoof"))
            .await
            .unwrap_err(),
        Error::NotFound
    );
    assert_eq!(
        l.service
            .save_variant(&l.admin.token, &a.id, &v.id, edit(1, "spoof"))
            .await
            .unwrap_err(),
        Error::Forbidden
    );
    assert_eq!(
        l.service
            .create_variant(&l.owner.token, &a.id, 1, None, fields("stale"), None)
            .await
            .unwrap_err(),
        Error::Conflict
    );
    let duplicate = l
        .service
        .create_variant(&l.owner.token, &a.id, 4, None, fields("local-one"), None)
        .await
        .unwrap();
    assert_eq!(duplicate.fields.slug, "local-one-2");
    assert_ne!(duplicate.id, v.id);
    let s2 = l
        .service
        .save_species(&l.admin.token, &s.id, edit(1, "species-new"))
        .await
        .unwrap();
    assert_eq!(s2.revision, 2);
    let f = l.service.form_detail(&s.default_form_id).await.unwrap();
    assert_eq!(f.species_version, 1);
    l.service
        .save_form(&l.admin.token, &f.id, 2, edit(1, "new-form"))
        .await
        .unwrap();
    assert_eq!(l.service.variant_detail(&a.id, &v.id).await.unwrap(), v);
    let preview = l.service.adoption_preview(&a.id, &v.id, 2).await.unwrap();
    assert_eq!(preview.old_species, s);
    assert_eq!(preview.new_species, s2);
    assert_eq!(preview.old_source.species_version, 1);
    assert_eq!(preview.new_source.species_version, 2);
    assert_eq!(preview.entry.fields, fields("local-one"));
    assert!(matches!(
        l.service.adoption_preview(&a.id, &exclusive.id, 2).await,
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        l.service.adoption_preview(&a.id, &v.id, 1).await,
        Err(Error::Invalid(_))
    ));
    l.service
        .save_variant(&l.owner.token, &a.id, &v.id, edit(1, "local-reviewed"))
        .await
        .unwrap();
    assert_eq!(
        l.service
            .adopt_variant(&l.owner.token, &a.id, &v.id, 2, edit(1, "stale"))
            .await
            .unwrap_err(),
        Error::Conflict
    );
    let adopted = l
        .service
        .adopt_variant(&l.owner.token, &a.id, &v.id, 2, edit(2, "local-reviewed"))
        .await
        .unwrap();
    assert_eq!(adopted.source.unwrap().version, 2);
    assert_eq!(adopted.fields, fields("local-reviewed"));
    assert_eq!(l.service.world_detail(&a.id).await.unwrap().revision, 5);
    assert_eq!(l.service.world_detail(&b.id).await.unwrap().revision, 2);
    assert_eq!(l.service.form_version(&f.id, 1).await.unwrap(), f);
    let h = l
        .service
        .history_entry(HistoryKind::Variant, &v.id, 3)
        .await
        .unwrap();
    assert_eq!(
        (
            h.actor_name.as_str(),
            h.action.as_str(),
            h.previous_revision
        ),
        ("Owner", "adopt", Some(2))
    );
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&h.after_json).unwrap()["snapshot_version"],
        2
    );
}

#[tokio::test]
async fn stale_noops_validation_and_bounded_attributed_history() {
    let l = Lab::new().await;
    let mut s = l
        .service
        .create_species(&l.admin.token, fields("s"), Some("Created"))
        .await
        .unwrap();
    let initial_count = l.count("audit_events").await;
    let noop = Edit {
        expected_revision: 1,
        fields: s.fields.clone(),
        reason: Some("No change".into()),
    };
    assert_eq!(
        l.service
            .save_species(&l.admin.token, &s.id, noop.clone())
            .await
            .unwrap(),
        s
    );
    assert_eq!(l.count("audit_events").await, initial_count);
    let mut bad = fields("BAD SLUG");
    bad.display_name = "".into();
    bad.description = "\t".into();
    let original = bad.clone();
    let invalid_edit = Edit {
        expected_revision: 1,
        fields: bad,
        reason: Some("x".repeat(501)),
    };
    assert!(
        matches!(l.service.save_species(&l.admin.token, &s.id, invalid_edit).await, Err(Error::Invalid(e)) if e.len() == 3 && e.contains_key("display_name") && e.contains_key("description") && e.contains_key("reason"))
    );
    assert_eq!(original.slug, "BAD SLUG");
    assert_eq!(l.service.species_detail(&s.id).await.unwrap(), s);
    let mut normalized = fields("s");
    normalized.description = "One\r\n\r\nTwo\rThree".into();
    s = l
        .service
        .save_species(
            &l.admin.token,
            &s.id,
            Edit {
                expected_revision: 1,
                fields: normalized,
                reason: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(s.fields.description, "One\n\nTwo\nThree");
    assert_eq!(
        l.service
            .save_species(&l.admin.token, &s.id, noop)
            .await
            .unwrap_err(),
        Error::Conflict
    );
    for n in 0..55 {
        s = l
            .service
            .save_species(&l.admin.token, &s.id, edit(s.revision, &format!("s-{n}")))
            .await
            .unwrap();
    }
    let first_page = l
        .service
        .history(HistoryKind::Species, &s.id, None)
        .await
        .unwrap();
    assert_eq!(first_page.len(), 50);
    let next_page = l
        .service
        .history(
            HistoryKind::Species,
            &s.id,
            Some(first_page.last().unwrap().revision),
        )
        .await
        .unwrap();
    assert_eq!(next_page.len(), 7);
    let initial = next_page.last().unwrap();
    assert_eq!(initial.actor_name, "Alice");
    assert_eq!(initial.reason.as_deref(), Some("Created"));
    assert!(initial.before_json.is_none());
    assert!(initial.occurred_at.ends_with('Z'));
    assert_eq!(
        l.service
            .history_entry(HistoryKind::Species, &s.id, 999)
            .await
            .unwrap_err(),
        Error::NotFound
    );
}

#[tokio::test]
async fn fixture_seed_baselines_survive_edits_and_drift_rolls_back_batch() {
    let l = Lab::new().await;
    let users = l.count("users").await;
    let report = l.service.seed_catalog(&bundled()).await.unwrap();
    assert_eq!(
        (
            report.species_created,
            report.forms_created,
            report.species_skipped
        ),
        (3, 3, 0)
    );
    let s = l.service.species().await.unwrap().remove(0);
    l.service
        .save_species(&l.admin.token, &s.id, edit(1, "locally-edited"))
        .await
        .unwrap();
    l.service
        .save_form(&l.admin.token, &s.default_form_id, 1, edit(1, "local-form"))
        .await
        .unwrap();
    let audit_count = l.count("audit_events").await;
    let report = l.service.seed_catalog(&bundled()).await.unwrap();
    assert_eq!(
        (
            report.species_created,
            report.forms_created,
            report.species_skipped
        ),
        (0, 0, 3)
    );
    assert_eq!(l.count("audit_events").await, audit_count);
    assert_eq!(l.count("users").await, users);
    let h = l
        .service
        .history_entry(HistoryKind::Species, &s.id, 1)
        .await
        .unwrap();
    assert_eq!(h.actor_name, "bundled-fixture-loader");
    let mut value: serde_json::Value = serde_json::from_str(BUNDLED_FIXTURES).unwrap();
    value[0]["description"] = "Drifted description".into();
    value.as_array_mut().unwrap().push(serde_json::json!({"id":"00000000000000000000000000000001", "slug":"a-new", "display_name":"A newly seeded definition", "description":"New baseline"}));
    assert!(matches!(
        l.service
            .seed_catalog(&Catalog::from_json(&value.to_string()).unwrap())
            .await,
        Err(Error::Invalid(_))
    ));
    assert_eq!(l.count("species").await, 3);
    assert_eq!(l.count("forms").await, 3);
    assert_eq!(l.count("audit_events").await, audit_count);
    let mut collision: serde_json::Value = serde_json::from_str(BUNDLED_FIXTURES).unwrap();
    collision[0]["id"] = "00000000000000000000000000000002".into();
    assert!(matches!(
        l.service
            .seed_catalog(&Catalog::from_json(&collision.to_string()).unwrap())
            .await,
        Err(Error::Invalid(_))
    ));
    assert_eq!(l.count("species").await, 3);
    let l2 = Lab::new().await;
    let first = bundled().creatures()[0].id().to_owned();
    let c = bundled();
    let fixture = &c.creatures()[0];
    let authored = l2
        .service
        .create_species(
            &l2.admin.token,
            ContentFields {
                slug: fixture.slug().into(),
                display_name: fixture.display_name().into(),
                description: fixture.description().into(),
            },
            None,
        )
        .await
        .unwrap();
    let mut collision: serde_json::Value = serde_json::from_str(BUNDLED_FIXTURES).unwrap();
    for row in collision.as_array_mut().unwrap() {
        if row["id"] == first {
            row["id"] = authored.id.clone().into();
        }
    }
    assert!(matches!(
        l2.service
            .seed_catalog(&Catalog::from_json(&collision.to_string()).unwrap())
            .await,
        Err(Error::Invalid(_))
    ));
    assert_eq!(l2.count("species").await, 1);
}

#[tokio::test]
async fn revision_audit_default_form_and_ownership_faults_roll_back() {
    for trigger in [
        "CREATE TRIGGER fault BEFORE INSERT ON species_versions BEGIN SELECT RAISE(ABORT, 'test fault'); END",
        "CREATE TRIGGER fault BEFORE INSERT ON audit_events BEGIN SELECT RAISE(ABORT, 'test fault'); END",
        "CREATE TRIGGER fault BEFORE INSERT ON form_versions BEGIN SELECT RAISE(ABORT, 'test fault'); END",
    ] {
        let l = Lab::new().await;
        sqlx::query(trigger).execute(&l.pool).await.unwrap();
        assert_eq!(
            l.service
                .create_species(&l.admin.token, fields("atomic"), None)
                .await
                .unwrap_err(),
            Error::Unavailable
        );
        for table in [
            "species",
            "species_versions",
            "forms",
            "form_versions",
            "audit_events",
        ] {
            assert_eq!(l.count(table).await, 0, "{table}");
        }
    }
    let l = Lab::new().await;
    sqlx::query("CREATE TRIGGER fault BEFORE INSERT ON world_access_revisions BEGIN SELECT RAISE(ABORT, 'test fault'); END").execute(&l.pool).await.unwrap();
    assert_eq!(
        l.service
            .create_world(&l.owner.token, fields("atomic"), None)
            .await
            .unwrap_err(),
        Error::Unavailable
    );
    for table in [
        "worlds",
        "world_revisions",
        "world_access_revisions",
        "audit_events",
    ] {
        assert_eq!(l.count(table).await, 0);
    }
}

#[tokio::test]
async fn variant_membership_edits_and_grants_roll_back_after_late_faults() {
    let l = Lab::new().await;
    let w = l
        .service
        .create_world(&l.owner.token, fields("world"), None)
        .await
        .unwrap();
    let baseline = l.count("audit_events").await;
    sqlx::query("CREATE TRIGGER fault_world BEFORE INSERT ON world_revisions BEGIN SELECT RAISE(ABORT, 'test fault'); END").execute(&l.pool).await.unwrap();
    assert_eq!(
        l.service
            .create_variant(&l.owner.token, &w.id, 1, None, fields("variant"), None)
            .await
            .unwrap_err(),
        Error::Unavailable
    );
    assert_eq!(l.count("variants").await, 0);
    assert_eq!(l.count("variant_revisions").await, 0);
    assert_eq!(l.service.world_detail(&w.id).await.unwrap(), w);
    assert_eq!(l.count("audit_events").await, baseline);
    sqlx::query("DROP TRIGGER fault_world")
        .execute(&l.pool)
        .await
        .unwrap();
    let v = l
        .service
        .create_variant(&l.owner.token, &w.id, 1, None, fields("variant"), None)
        .await
        .unwrap();
    let baseline = l.count("audit_events").await;
    sqlx::query("CREATE TRIGGER fault_variant BEFORE INSERT ON variant_revisions BEGIN SELECT RAISE(ABORT, 'test fault'); END").execute(&l.pool).await.unwrap();
    assert_eq!(
        l.service
            .save_variant(&l.owner.token, &w.id, &v.id, edit(1, "changed"))
            .await
            .unwrap_err(),
        Error::Unavailable
    );
    assert_eq!(l.service.variant_detail(&w.id, &v.id).await.unwrap(), v);
    assert_eq!(l.count("audit_events").await, baseline);
    sqlx::query("DROP TRIGGER fault_variant")
        .execute(&l.pool)
        .await
        .unwrap();
    sqlx::query("CREATE TRIGGER fault_access BEFORE INSERT ON world_access_revisions BEGIN SELECT RAISE(ABORT, 'test fault'); END").execute(&l.pool).await.unwrap();
    assert_eq!(
        l.service
            .grant_collaborator(&l.owner.token, &w.id, 1, &l.editor.actor.user_id, None)
            .await
            .unwrap_err(),
        Error::Unavailable
    );
    assert_eq!(l.count("world_collaborators").await, 0);
    assert_eq!(l.count("audit_events").await, baseline);
    assert_eq!(
        l.service.world_detail(&w.id).await.unwrap().access_revision,
        1
    );
    sqlx::query("DROP TRIGGER fault_access")
        .execute(&l.pool)
        .await
        .unwrap();
    l.service
        .grant_collaborator(&l.owner.token, &w.id, 1, &l.editor.actor.user_id, None)
        .await
        .unwrap();
    let baseline = l.count("audit_events").await;
    sqlx::query("CREATE TRIGGER fault_revoke BEFORE DELETE ON world_collaborators BEGIN SELECT RAISE(ABORT, 'test fault'); END").execute(&l.pool).await.unwrap();
    assert_eq!(
        l.service
            .revoke_collaborator(&l.owner.token, &w.id, 2, &l.editor.actor.user_id, None)
            .await
            .unwrap_err(),
        Error::Unavailable
    );
    assert_eq!(l.count("world_collaborators").await, 1);
    assert_eq!(l.count("audit_events").await, baseline);
}

#[tokio::test]
async fn authorization_waits_for_writer_and_sees_revoked_session_and_role() {
    let l = Lab::new().await;
    let w = l
        .service
        .create_world(&l.owner.token, fields("world"), None)
        .await
        .unwrap();
    let mut tx = persistence::begin_write(&l.pool).await.unwrap();
    let service = l.service.clone();
    let token = l.owner.token.clone();
    let id = w.id.clone();
    let write = tokio::spawn(async move {
        service
            .save_world(&token, &id, edit(1, "racing-write"))
            .await
    });
    persistence::revoke_user_sessions(&mut *tx, &l.owner.actor.user_id, 1)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert_eq!(write.await.unwrap().unwrap_err(), Error::Unauthorized);
    let (token, _) = l
        .accounts
        .start_session(&l.owner.actor.user_id)
        .await
        .unwrap();
    sqlx::query("UPDATE users SET role = 'viewer' WHERE id = ?")
        .bind(&l.owner.actor.user_id)
        .execute(&l.pool)
        .await
        .unwrap();
    assert_eq!(
        l.service
            .save_world(&token, &w.id, edit(1, "demoted"))
            .await
            .unwrap_err(),
        Error::Forbidden
    );
    assert_eq!(l.service.world_detail(&w.id).await.unwrap(), w);
    l.accounts
        .set_disabled(&l.owner.actor.user_id, true)
        .await
        .unwrap();
    assert_eq!(
        l.service.world_detail(&w.id).await.unwrap().owner_user_id,
        l.owner.actor.user_id
    );
    assert_eq!(
        l.service
            .save_world(&l.admin.token, &w.id, edit(1, "recovery-bypass"))
            .await
            .unwrap_err(),
        Error::Forbidden
    );
}

#[tokio::test]
async fn schema_retains_versions_and_enforces_scope_parent_and_exact_references() {
    let l = Lab::new().await;
    let s = l
        .service
        .create_species(&l.admin.token, fields("s"), None)
        .await
        .unwrap();
    let t = l
        .service
        .create_species(&l.admin.token, fields("t"), None)
        .await
        .unwrap();
    let w = l
        .service
        .create_world(&l.owner.token, fields("w"), None)
        .await
        .unwrap();
    let v = l
        .service
        .create_variant(
            &l.owner.token,
            &w.id,
            1,
            Some(SourcePin {
                form_id: s.default_form_id.clone(),
                version: 1,
            }),
            fields("v"),
            None,
        )
        .await
        .unwrap();
    for sql in [
        "UPDATE species_versions SET description = 'rewrite' WHERE species_id = ?",
        "DELETE FROM species_versions WHERE species_id = ?",
    ] {
        assert!(sqlx::query(sql).bind(&s.id).execute(&l.pool).await.is_err());
    }
    for sql in [
        "UPDATE form_versions SET description = 'rewrite' WHERE form_id = ?",
        "DELETE FROM form_versions WHERE form_id = ?",
    ] {
        assert!(
            sqlx::query(sql)
                .bind(&s.default_form_id)
                .execute(&l.pool)
                .await
                .is_err()
        );
    }
    assert!(
        sqlx::query("UPDATE forms SET species_id = ? WHERE id = ?")
            .bind(&t.id)
            .bind(&s.default_form_id)
            .execute(&l.pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("UPDATE forms SET species_version = 99 WHERE id = ?")
            .bind(&s.default_form_id)
            .execute(&l.pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("UPDATE variants SET source_form_version = NULL WHERE id = ?")
            .bind(&v.id)
            .execute(&l.pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("UPDATE variants SET source_form_version = 99 WHERE id = ?")
            .bind(&v.id)
            .execute(&l.pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("UPDATE worlds SET owner_user_id = ? WHERE id = ?")
            .bind(&l.admin.actor.user_id)
            .bind(&w.id)
            .execute(&l.pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("DELETE FROM species WHERE id = ?")
            .bind(&s.id)
            .execute(&l.pool)
            .await
            .is_err()
    );
    let other = l
        .service
        .create_world(&l.owner.token, fields("other"), None)
        .await
        .unwrap();
    assert!(
        sqlx::query("UPDATE variants SET world_id = ? WHERE id = ?")
            .bind(&other.id)
            .bind(&v.id)
            .execute(&l.pool)
            .await
            .is_err()
    );
    let issues: Vec<(String, i64, String, i64)> = sqlx::query_as("PRAGMA foreign_key_check")
        .fetch_all(&l.pool)
        .await
        .unwrap();
    assert!(issues.is_empty());
}

#[tokio::test]
async fn migration_one_upgrade_preserves_users_invites_and_sessions() {
    let dir = tempfile::tempdir().unwrap();
    let pool = persistence::connect_for_migration(&format!(
        "sqlite://{}/upgrade.db",
        dir.path().display()
    ))
    .await
    .unwrap();
    let migration = sqlx::migrate::Migration::new(
        1,
        "auth".into(),
        sqlx::migrate::MigrationType::Simple,
        include_str!("../migrations/0001_auth.sql").into_sql_str(),
        false,
    );
    let migrator = sqlx::migrate::Migrator {
        migrations: std::borrow::Cow::Owned(vec![migration]),
        ..sqlx::migrate::Migrator::DEFAULT
    };
    migrator.run(&pool).await.unwrap();
    let accounts = Accounts::new(pool.clone(), 3600).unwrap();
    let alice = user(&accounts, "Alice", Role::Administrator).await;
    accounts
        .invite("test", "Pending", Role::Editor, None)
        .await
        .unwrap();
    assert!(!persistence::schema_ready(&pool).await.unwrap());
    persistence::migrate(&pool).await.unwrap();
    assert!(persistence::schema_ready(&pool).await.unwrap());
    assert_eq!(
        accounts.authenticate(&alice.token).await.unwrap(),
        Some(alice.actor)
    );
    assert_eq!(accounts.users().await.unwrap().len(), 1);
    assert_eq!(accounts.invites().await.unwrap().len(), 2);
    let service = Authoring::new(accounts);
    assert!(service.species().await.unwrap().is_empty());
    assert!(service.worlds().await.unwrap().is_empty());
}

#[tokio::test]
async fn aggregate_noops_check_revisions_first_and_do_not_add_history() {
    let l = Lab::new().await;
    let s = l
        .service
        .create_species(&l.admin.token, fields("s"), None)
        .await
        .unwrap();
    let w = l
        .service
        .create_world(&l.owner.token, fields("w"), None)
        .await
        .unwrap();
    let v = l
        .service
        .create_variant(&l.owner.token, &w.id, 1, None, fields("v"), None)
        .await
        .unwrap();
    let noop = |revision, fields| Edit {
        expected_revision: revision,
        fields,
        reason: None,
    };
    let baseline = l.count("audit_events").await;
    l.service
        .save_form(
            &l.admin.token,
            &s.default_form_id,
            1,
            noop(1, s.fields.clone()),
        )
        .await
        .unwrap();
    l.service
        .save_world(&l.owner.token, &w.id, noop(2, w.fields.clone()))
        .await
        .unwrap();
    l.service
        .save_variant(&l.owner.token, &w.id, &v.id, noop(1, v.fields.clone()))
        .await
        .unwrap();
    l.service
        .revoke_collaborator(&l.owner.token, &w.id, 1, &l.editor.actor.user_id, None)
        .await
        .unwrap();
    assert_eq!(l.count("audit_events").await, baseline);
    assert_eq!(
        l.service
            .save_form(
                &l.admin.token,
                &s.default_form_id,
                1,
                noop(0, s.fields.clone())
            )
            .await
            .unwrap_err(),
        Error::Conflict
    );
    assert_eq!(
        l.service
            .save_world(&l.owner.token, &w.id, noop(1, w.fields.clone()))
            .await
            .unwrap_err(),
        Error::Conflict
    );
    assert_eq!(
        l.service
            .save_variant(&l.owner.token, &w.id, &v.id, noop(0, v.fields.clone()))
            .await
            .unwrap_err(),
        Error::Conflict
    );
    l.service
        .grant_collaborator(&l.owner.token, &w.id, 1, &l.editor.actor.user_id, None)
        .await
        .unwrap();
    let baseline = l.count("audit_events").await;
    l.service
        .grant_collaborator(&l.owner.token, &w.id, 2, &l.editor.actor.user_id, None)
        .await
        .unwrap();
    assert_eq!(l.count("audit_events").await, baseline);
    assert_eq!(
        l.service
            .grant_collaborator(&l.owner.token, &w.id, 1, &l.editor.actor.user_id, None)
            .await
            .unwrap_err(),
        Error::Conflict
    );
    assert_eq!(
        l.service
            .revoke_collaborator(&l.owner.token, &w.id, 1, &l.editor.actor.user_id, None)
            .await
            .unwrap_err(),
        Error::Conflict
    );
    assert_eq!(l.service.world_detail(&w.id).await.unwrap().revision, 2);
}

#[tokio::test]
async fn concurrent_edits_and_grant_revocation_are_serialized() {
    let l = Lab::new().await;
    let w = l
        .service
        .create_world(&l.owner.token, fields("w"), None)
        .await
        .unwrap();
    l.service
        .grant_collaborator(&l.owner.token, &w.id, 1, &l.editor.actor.user_id, None)
        .await
        .unwrap();
    let a = l.service.clone();
    let b = l.service.clone();
    let token_a = l.owner.token.clone();
    let token_b = token_a.clone();
    let id_a = w.id.clone();
    let id_b = id_a.clone();
    let one = tokio::spawn(async move { a.save_world(&token_a, &id_a, edit(1, "winner-a")).await });
    let two = tokio::spawn(async move { b.save_world(&token_b, &id_b, edit(1, "winner-b")).await });
    let results = [one.await.unwrap(), two.await.unwrap()];
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|r| matches!(r, Err(Error::Conflict)))
            .count(),
        1
    );
    let mut tx = persistence::begin_write(&l.pool).await.unwrap();
    let service = l.service.clone();
    let token = l.editor.token.clone();
    let id = w.id.clone();
    let blocked = tokio::spawn(async move {
        service
            .save_world(&token, &id, edit(2, "revoked-write"))
            .await
    });
    // Simulate a competing transaction's grant removal before releasing the writer.
    sqlx::query("DELETE FROM world_collaborators WHERE world_id = ? AND user_id = ?")
        .bind(&w.id)
        .bind(&l.editor.actor.user_id)
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert_eq!(blocked.await.unwrap().unwrap_err(), Error::Forbidden);
    assert_eq!(l.service.world_detail(&w.id).await.unwrap().revision, 2);
    // Grant alone does not bypass current global eligibility.
    l.service
        .grant_collaborator(&l.owner.token, &w.id, 2, &l.editor.actor.user_id, None)
        .await
        .unwrap();
    sqlx::query("UPDATE users SET role = 'viewer' WHERE id = ?")
        .bind(&l.editor.actor.user_id)
        .execute(&l.pool)
        .await
        .unwrap();
    assert_eq!(
        l.service
            .save_world(&l.editor.token, &w.id, edit(2, "demoted-collaborator"))
            .await
            .unwrap_err(),
        Error::Forbidden
    );
    l.accounts
        .set_disabled(&l.editor.actor.user_id, true)
        .await
        .unwrap();
    assert_eq!(
        l.service
            .save_world(&l.editor.token, &w.id, edit(2, "disabled-collaborator"))
            .await
            .unwrap_err(),
        Error::Unauthorized
    );
    l.service
        .revoke_collaborator(&l.owner.token, &w.id, 3, &l.editor.actor.user_id, None)
        .await
        .unwrap();
}

#[tokio::test]
async fn invalid_stored_content_and_snapshots_fail_closed() {
    let l = Lab::new().await;
    let s = l
        .service
        .create_species(&l.admin.token, fields("s"), None)
        .await
        .unwrap();
    sqlx::query("UPDATE species SET slug = 'UPPERCASE' WHERE id = ?")
        .bind(&s.id)
        .execute(&l.pool)
        .await
        .unwrap();
    assert_eq!(
        l.service.species_detail(&s.id).await.unwrap_err(),
        Error::Unavailable
    );
    // Historical typed rows are independent of current values.
    assert_eq!(l.service.species_version(&s.id, 1).await.unwrap(), s);
    let mut tx = persistence::begin_write(&l.pool).await.unwrap();
    sqlx::query("INSERT INTO audit_events(id, actor_user_id, action) VALUES ('00000000000000000000000000000001', ?, 'edit')").bind(&l.admin.actor.user_id).execute(&mut *tx).await.unwrap();
    assert!(sqlx::query("INSERT INTO species_versions(species_id, revision, slug, display_name, description, default_form_id, audit_id, previous_revision, before_json, after_json) VALUES (?, 2, 'valid', 'Valid', 'Valid', ?, '00000000000000000000000000000001', 1, '{\"snapshot_version\":1}', '{}')").bind(&s.id).bind(&s.default_form_id).execute(&mut *tx).await.is_err());
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn missing_input_pins_are_field_errors_and_stale_commands_take_precedence() {
    let l = Lab::new().await;
    let s = l
        .service
        .create_species(&l.admin.token, fields("s"), None)
        .await
        .unwrap();
    let w = l
        .service
        .create_world(&l.owner.token, fields("w"), None)
        .await
        .unwrap();
    let baseline = l.count("audit_events").await;
    assert!(
        matches!(l.service.create_form(&l.admin.token, &s.id, 99, fields("new-form"), None).await, Err(Error::Invalid(e)) if e.contains_key("species_version"))
    );
    assert_eq!(
        l.service
            .save_form(&l.admin.token, &s.default_form_id, 99, edit(0, "new-form"))
            .await
            .unwrap_err(),
        Error::Conflict
    );
    assert!(
        matches!(l.service.create_variant(&l.owner.token, &w.id, 1, Some(SourcePin { form_id: s.default_form_id.clone(), version: 99 }), fields("v"), None).await, Err(Error::Invalid(e)) if e.contains_key("source_pin"))
    );
    assert_eq!(l.count("audit_events").await, baseline);
    let v = l
        .service
        .create_variant(
            &l.owner.token,
            &w.id,
            1,
            Some(SourcePin {
                form_id: s.default_form_id.clone(),
                version: 1,
            }),
            fields("v"),
            None,
        )
        .await
        .unwrap();
    let baseline = l.count("audit_events").await;
    assert!(
        matches!(l.service.adopt_variant(&l.owner.token, &w.id, &v.id, 99, edit(1, "v")).await, Err(Error::Invalid(e)) if e.contains_key("target_version"))
    );
    assert_eq!(
        l.service
            .adopt_variant(&l.owner.token, &w.id, &v.id, 99, edit(0, "v"))
            .await
            .unwrap_err(),
        Error::Conflict
    );
    assert_eq!(l.count("audit_events").await, baseline);
    assert_eq!(l.service.variant_detail(&w.id, &v.id).await.unwrap(), v);
}

#[tokio::test]
async fn paragraph_fixtures_seed_exact_canonical_baselines() {
    let l = Lab::new().await;
    let mut data: serde_json::Value = serde_json::from_str(BUNDLED_FIXTURES).unwrap();
    data[0]["description"] = "First paragraph.\n\nSecond paragraph.".into();
    let catalog = Catalog::from_json(&data.to_string()).unwrap();
    let report = l.service.seed_catalog(&catalog).await.unwrap();
    assert_eq!(report.species_created, 3);
    let fixture = &catalog.creatures()[0];
    let species = l.service.species_detail(fixture.id()).await.unwrap();
    assert_eq!(species.fields.description, fixture.description());
    assert_eq!(
        l.service
            .form_detail(&species.default_form_id)
            .await
            .unwrap()
            .fields,
        species.fields
    );
    let h = l
        .service
        .history_entry(HistoryKind::Species, &species.id, 1)
        .await
        .unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&h.after_json).unwrap()["content"]["fields"]["description"],
        fixture.description()
    );
    assert_eq!(
        l.service
            .seed_catalog(&catalog)
            .await
            .unwrap()
            .species_skipped,
        3
    );
}

#[tokio::test]
async fn every_revision_table_rejects_null_predecessor_and_rolls_back() {
    let l = Lab::new().await;
    let s = l
        .service
        .create_species(&l.admin.token, fields("s"), None)
        .await
        .unwrap();
    let w = l
        .service
        .create_world(&l.owner.token, fields("w"), None)
        .await
        .unwrap();
    let vw = l
        .service
        .create_world(&l.owner.token, fields("variant-world"), None)
        .await
        .unwrap();
    let v = l
        .service
        .create_variant(&l.owner.token, &vw.id, 1, None, fields("v"), None)
        .await
        .unwrap();
    let species = l
        .service
        .history_entry(HistoryKind::Species, &s.id, 1)
        .await
        .unwrap();
    let form = l
        .service
        .history_entry(HistoryKind::Form, &s.default_form_id, 1)
        .await
        .unwrap();
    let world = l
        .service
        .history_entry(HistoryKind::World, &w.id, 1)
        .await
        .unwrap();
    let variant = l
        .service
        .history_entry(HistoryKind::Variant, &v.id, 1)
        .await
        .unwrap();
    let access = l
        .service
        .access(&l.owner.token, &w.id)
        .await
        .unwrap()
        .history
        .remove(0);
    let cases = [
        (
            "species_versions",
            "UPDATE species SET revision = 2 WHERE id = ?",
            "INSERT INTO species_versions(species_id, revision, slug, display_name, description, default_form_id, archived_at, audit_id, previous_revision, before_json, after_json) SELECT species_id, 2, slug, display_name, description, default_form_id, archived_at, ?2, NULL, ?3, ?4 FROM species_versions WHERE species_id = ?1 AND revision = 1",
            s.id.as_str(),
            species.after_json,
            "revision",
        ),
        (
            "form_versions",
            "UPDATE forms SET revision = 2 WHERE id = ?",
            "INSERT INTO form_versions(form_id, species_id, species_version, revision, slug, display_name, description, is_default, archived_at, audit_id, previous_revision, before_json, after_json,content_snapshot_version) SELECT form_id, species_id, species_version, 2, slug, display_name, description, is_default, archived_at, ?2, NULL, ?3, ?4,2 FROM form_versions WHERE form_id = ?1 AND revision = 1",
            s.default_form_id.as_str(),
            form.after_json,
            "revision",
        ),
        (
            "world_revisions",
            "UPDATE worlds SET revision = 2 WHERE id = ?",
            "INSERT INTO world_revisions(world_id, revision, audit_id, previous_revision, before_json, after_json,content_snapshot_version) VALUES (?1, 2, ?2, NULL, ?3, ?4,2)",
            w.id.as_str(),
            world.after_json,
            "revision",
        ),
        (
            "variant_revisions",
            "UPDATE variants SET revision = 2 WHERE id = ?",
            "INSERT INTO variant_revisions(world_id, variant_id, revision, audit_id, previous_revision, before_json, after_json,content_snapshot_version) SELECT world_id, variant_id, 2, ?2, NULL, ?3, ?4,2 FROM variant_revisions WHERE variant_id = ?1 AND revision = 1",
            v.id.as_str(),
            variant.after_json,
            "revision",
        ),
        (
            "world_access_revisions",
            "UPDATE worlds SET access_revision = 2 WHERE id = ?",
            "INSERT INTO world_access_revisions(world_id, revision, audit_id, previous_revision, before_json, after_json) VALUES (?1, 2, ?2, NULL, ?3, ?4)",
            w.id.as_str(),
            access.after_json,
            "access_revision",
        ),
    ];
    let audit_count = l.count("audit_events").await;
    for (table, update, insert, id, before, revision_key) in cases {
        let count = l.count(table).await;
        let mut after: serde_json::Value = serde_json::from_str(&before).unwrap();
        after["content"][revision_key] = 2.into();
        let mut tx = persistence::begin_write(&l.pool).await.unwrap();
        sqlx::query(update)
            .bind(id)
            .execute(&mut *tx)
            .await
            .unwrap();
        sqlx::query("INSERT INTO audit_events(id, actor_user_id, action) VALUES ('00000000000000000000000000000001', ?, 'edit')").bind(&l.admin.actor.user_id).execute(&mut *tx).await.unwrap();
        let sidecar = match table {
            "form_versions" => Some(
                "INSERT INTO form_content_snapshots(form_id,revision,typing_defined,before_json,after_json) VALUES (?1,2,0,?2,?3)",
            ),
            "world_revisions" => Some(
                "INSERT INTO world_content_snapshots(world_id,revision,before_json,after_json) VALUES (?1,2,?2,?3)",
            ),
            "variant_revisions" => Some(
                "INSERT INTO variant_content_snapshots(world_id,variant_id,revision,typing_defined,before_json,after_json) SELECT world_id,variant_id,2,0,?2,?3 FROM variant_revisions WHERE variant_id=?1 AND revision=1",
            ),
            _ => None,
        };
        if let Some(sql) = sidecar {
            sqlx::query(sql)
                .bind(id)
                .bind(&before)
                .bind(after.to_string())
                .execute(&mut *tx)
                .await
                .unwrap();
        }
        let projection = |s: &str| {
            let mut v: serde_json::Value = serde_json::from_str(s).unwrap();
            v["snapshot_version"] = 1.into();
            v["content"].as_object_mut().unwrap().remove("typing");
            v["content"].as_object_mut().unwrap().remove("type_ids");
            v.to_string()
        };
        let error = sqlx::query(insert)
            .bind(id)
            .bind("00000000000000000000000000000001")
            .bind(projection(&before))
            .bind(projection(&after.to_string()))
            .execute(&mut *tx)
            .await
            .unwrap_err();
        assert_eq!(
            error.as_database_error().unwrap().code().as_deref(),
            Some("275"),
            "{table}: must fail its CHECK, not an unrelated constraint"
        );
        tx.rollback().await.unwrap();
        assert_eq!(l.count(table).await, count);
        assert_eq!(l.count("audit_events").await, audit_count);
    }
    assert_eq!(l.service.species_detail(&s.id).await.unwrap(), s);
    assert_eq!(
        l.service
            .form_detail(&s.default_form_id)
            .await
            .unwrap()
            .revision,
        1
    );
    assert_eq!(l.service.world_detail(&w.id).await.unwrap(), w);
    assert_eq!(l.service.variant_detail(&vw.id, &v.id).await.unwrap(), v);
}

#[tokio::test]
async fn pin_versions_must_exist_on_the_selected_species_and_form() {
    let l = Lab::new().await;
    let s = l
        .service
        .create_species(&l.admin.token, fields("s"), None)
        .await
        .unwrap();
    let t = l
        .service
        .create_species(&l.admin.token, fields("t"), None)
        .await
        .unwrap();
    l.service
        .save_species(&l.admin.token, &t.id, edit(1, "t-new"))
        .await
        .unwrap();
    l.service
        .save_form(&l.admin.token, &t.default_form_id, 2, edit(1, "t-form-new"))
        .await
        .unwrap();
    assert_eq!(
        l.service.species_version(&t.id, 2).await.unwrap().revision,
        2
    );
    assert_eq!(
        l.service
            .form_version(&t.default_form_id, 2)
            .await
            .unwrap()
            .revision,
        2
    );
    let w = l
        .service
        .create_world(&l.owner.token, fields("w"), None)
        .await
        .unwrap();
    let v = l
        .service
        .create_variant(
            &l.owner.token,
            &w.id,
            1,
            Some(SourcePin {
                form_id: s.default_form_id.clone(),
                version: 1,
            }),
            fields("v"),
            None,
        )
        .await
        .unwrap();
    let count = l.count("audit_events").await;
    assert!(matches!(
        l.service
            .create_form(&l.admin.token, &s.id, 2, fields("invalid-form"), None)
            .await,
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        l.service
            .save_form(
                &l.admin.token,
                &s.default_form_id,
                2,
                edit(1, "invalid-form")
            )
            .await,
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        l.service
            .create_variant(
                &l.owner.token,
                &w.id,
                2,
                Some(SourcePin {
                    form_id: s.default_form_id.clone(),
                    version: 2
                }),
                fields("invalid-variant"),
                None
            )
            .await,
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        l.service
            .adopt_variant(&l.owner.token, &w.id, &v.id, 2, edit(1, "invalid-adoption"))
            .await,
        Err(Error::Invalid(_))
    ));
    assert_eq!(l.count("audit_events").await, count);
    for (sql, id, code) in [
        (
            "UPDATE forms SET species_version = 2 WHERE id = ?",
            s.default_form_id.as_str(),
            "787",
        ),
        (
            "UPDATE variants SET source_form_version = 2 WHERE id = ?",
            v.id.as_str(),
            "787",
        ),
        (
            "UPDATE variants SET source_form_id = NULL WHERE id = ?",
            v.id.as_str(),
            "275",
        ),
        (
            "UPDATE variants SET source_form_version = NULL WHERE id = ?",
            v.id.as_str(),
            "275",
        ),
    ] {
        let error = sqlx::query(sql)
            .bind(id)
            .execute(&l.pool)
            .await
            .unwrap_err();
        assert_eq!(
            error.as_database_error().unwrap().code().as_deref(),
            Some(code)
        );
    }
    let mut tx = persistence::begin_write(&l.pool).await.unwrap();
    sqlx::query("INSERT INTO audit_events(id, actor_user_id, action) VALUES ('00000000000000000000000000000001', ?, 'edit')").bind(&l.admin.actor.user_id).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO form_content_snapshots(form_id,revision,typing_defined,before_json,after_json) SELECT form_id,2,0,after_json,json_set(after_json,'$.content.revision',2,'$.content.species_version',2) FROM form_content_snapshots WHERE form_id=? AND revision=1").bind(&s.default_form_id).execute(&mut *tx).await.unwrap();
    let error = sqlx::query("INSERT INTO form_versions(form_id, species_id, species_version, revision, slug, display_name, description, is_default, archived_at, audit_id, previous_revision, before_json, after_json,content_snapshot_version) SELECT form_id, species_id, 2, 2, slug, display_name, description, is_default, archived_at, '00000000000000000000000000000001', 1, after_json, after_json,2 FROM form_versions WHERE form_id = ? AND revision = 1").bind(&s.default_form_id).execute(&mut *tx).await.unwrap_err();
    assert_eq!(
        error.as_database_error().unwrap().code().as_deref(),
        Some("787")
    );
    tx.rollback().await.unwrap();
    assert_eq!(l.count("audit_events").await, count);
    assert_eq!(l.service.variant_detail(&w.id, &v.id).await.unwrap(), v);
}

fn assert_snapshot_continuity(history: &[web_app::application::authoring::HistoryEntry]) {
    for pair in history.windows(2) {
        assert_eq!(pair[0].previous_revision, Some(pair[1].revision));
        assert_eq!(
            pair[0].before_json.as_deref(),
            Some(pair[1].after_json.as_str())
        );
    }
    assert!(history.last().unwrap().before_json.is_none());
}
fn snapshot_content(json: &str) -> serde_json::Value {
    serde_json::from_str::<serde_json::Value>(json).unwrap()["content"].clone()
}

#[tokio::test]
async fn snapshots_are_continuous_and_record_membership_and_adoption_pins() {
    let l = Lab::new().await;
    let s = l
        .service
        .create_species(&l.admin.token, fields("s"), None)
        .await
        .unwrap();
    l.service
        .save_species(&l.admin.token, &s.id, edit(1, "s-new"))
        .await
        .unwrap();
    l.service
        .save_form(&l.admin.token, &s.default_form_id, 2, edit(1, "form-new"))
        .await
        .unwrap();
    let w = l
        .service
        .create_world(&l.owner.token, fields("w"), None)
        .await
        .unwrap();
    let v = l
        .service
        .create_variant(
            &l.owner.token,
            &w.id,
            1,
            Some(SourcePin {
                form_id: s.default_form_id.clone(),
                version: 1,
            }),
            fields("local"),
            None,
        )
        .await
        .unwrap();
    let exclusive = l
        .service
        .create_variant(&l.owner.token, &w.id, 2, None, fields("exclusive"), None)
        .await
        .unwrap();
    l.service
        .grant_collaborator(&l.owner.token, &w.id, 1, &l.editor.actor.user_id, None)
        .await
        .unwrap();
    l.service
        .save_world(&l.owner.token, &w.id, edit(3, "world-new"))
        .await
        .unwrap();
    l.service
        .save_variant(&l.owner.token, &w.id, &v.id, edit(1, "local-edited"))
        .await
        .unwrap();
    l.service
        .adopt_variant(&l.owner.token, &w.id, &v.id, 2, edit(2, "local-adopted"))
        .await
        .unwrap();
    l.service
        .revoke_collaborator(&l.owner.token, &w.id, 2, &l.editor.actor.user_id, None)
        .await
        .unwrap();
    for (kind, id) in [
        (HistoryKind::Species, &s.id),
        (HistoryKind::Form, &s.default_form_id),
        (HistoryKind::World, &w.id),
        (HistoryKind::Variant, &v.id),
    ] {
        assert_snapshot_continuity(&l.service.history(kind, id, None).await.unwrap());
    }
    assert_snapshot_continuity(
        &l.service
            .access(&l.owner.token, &w.id)
            .await
            .unwrap()
            .history,
    );
    let world_history = l
        .service
        .history(HistoryKind::World, &w.id, None)
        .await
        .unwrap();
    let mut ids = vec![v.id.clone(), exclusive.id.clone()];
    ids.sort();
    assert_eq!(
        snapshot_content(&world_history[3].after_json)["variant_ids"],
        serde_json::json!([])
    );
    assert_eq!(
        snapshot_content(&world_history[2].after_json)["variant_ids"],
        serde_json::json!([v.id])
    );
    assert_eq!(
        snapshot_content(&world_history[1].after_json)["variant_ids"],
        serde_json::json!(ids)
    );
    assert_eq!(
        snapshot_content(&world_history[0].after_json)["variant_ids"],
        serde_json::json!(ids)
    );
    let adoption = l
        .service
        .history_entry(HistoryKind::Variant, &v.id, 3)
        .await
        .unwrap();
    assert_eq!(
        snapshot_content(adoption.before_json.as_ref().unwrap())["source"],
        serde_json::json!({"form_id":s.default_form_id,"version":1})
    );
    assert_eq!(
        snapshot_content(&adoption.after_json)["source"],
        serde_json::json!({"form_id":s.default_form_id,"version":2})
    );
    assert_eq!(
        snapshot_content(&adoption.after_json)["fields"]["slug"],
        "local-adopted"
    );
}

#[tokio::test]
async fn late_adoption_revision_and_audit_failures_roll_back_pin_fields_and_history() {
    for trigger in [
        "CREATE TRIGGER fault_adopt BEFORE INSERT ON variant_revisions WHEN NEW.revision = 2 BEGIN SELECT RAISE(ABORT, 'test fault'); END",
        "CREATE TRIGGER fault_adopt BEFORE INSERT ON audit_events WHEN NEW.action = 'adopt' BEGIN SELECT RAISE(ABORT, 'test fault'); END",
    ] {
        let l = Lab::new().await;
        let s = l
            .service
            .create_species(&l.admin.token, fields("s"), None)
            .await
            .unwrap();
        l.service
            .save_form(&l.admin.token, &s.default_form_id, 1, edit(1, "form-new"))
            .await
            .unwrap();
        let w = l
            .service
            .create_world(&l.owner.token, fields("w"), None)
            .await
            .unwrap();
        let v = l
            .service
            .create_variant(
                &l.owner.token,
                &w.id,
                1,
                Some(SourcePin {
                    form_id: s.default_form_id,
                    version: 1,
                }),
                fields("local"),
                None,
            )
            .await
            .unwrap();
        let world = l.service.world_detail(&w.id).await.unwrap();
        let initial = l
            .service
            .history_entry(HistoryKind::Variant, &v.id, 1)
            .await
            .unwrap();
        let audit_count = l.count("audit_events").await;
        let revision_count = l.count("variant_revisions").await;
        sqlx::query(trigger).execute(&l.pool).await.unwrap();
        assert_eq!(
            l.service
                .adopt_variant(
                    &l.owner.token,
                    &w.id,
                    &v.id,
                    2,
                    edit(1, "new-effective-values")
                )
                .await
                .unwrap_err(),
            Error::Unavailable
        );
        assert_eq!(l.service.variant_detail(&w.id, &v.id).await.unwrap(), v);
        assert_eq!(l.service.world_detail(&w.id).await.unwrap(), world);
        assert_eq!(l.count("audit_events").await, audit_count);
        assert_eq!(l.count("variant_revisions").await, revision_count);
        let history = l
            .service
            .history(HistoryKind::Variant, &v.id, None)
            .await
            .unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].after_json, initial.after_json);
        assert!(history[0].before_json.is_none());
    }
}

// Snapshot corruption is injected only into these temporary, test-owned databases.
async fn replace_history_snapshot(
    l: &Lab,
    kind: Option<HistoryKind>,
    id: &str,
    revision: i64,
    before: bool,
    json: &str,
) {
    let (table, key) = match kind {
        Some(HistoryKind::Species) => ("species_versions", "species_id"),
        Some(HistoryKind::Form) => ("form_content_snapshots", "form_id"),
        Some(HistoryKind::World) => ("world_content_snapshots", "world_id"),
        Some(HistoryKind::Variant) => ("variant_content_snapshots", "variant_id"),
        Some(HistoryKind::Type) => ("type_versions", "type_id"),
        Some(HistoryKind::WorldType) => ("world_type_versions", "type_id"),
        None => ("world_access_revisions", "world_id"),
    };
    let column = if before { "before_json" } else { "after_json" };
    let sql = format!("UPDATE {table} SET {column} = ? WHERE {key} = ? AND revision = ?");
    assert_eq!(
        sqlx::query(sqlx::AssertSqlSafe(sql))
            .bind(json)
            .bind(id)
            .bind(revision)
            .execute(&l.pool)
            .await
            .unwrap()
            .rows_affected(),
        1
    );
}
fn common_invalid_snapshot_contents(
    content: &serde_json::Value,
    id_key: &str,
    revision_key: &str,
) -> Vec<serde_json::Value> {
    let mut cases = vec![serde_json::json!({})];
    for id in ["not-a-canonical-id", "00000000000000000000000000000001"] {
        let mut bad = content.clone();
        bad[id_key] = id.into();
        cases.push(bad);
    }
    for revision in [-1, 0, 2] {
        let mut bad = content.clone();
        bad[revision_key] = revision.into();
        cases.push(bad);
    }
    cases
}

#[tokio::test]
async fn history_rejects_valid_envelopes_with_invalid_per_aggregate_content() {
    let l = Lab::new().await;
    let s = l
        .service
        .create_species(&l.admin.token, fields("s"), None)
        .await
        .unwrap();
    let w = l
        .service
        .create_world(&l.owner.token, fields("w"), None)
        .await
        .unwrap();
    let v = l
        .service
        .create_variant(
            &l.owner.token,
            &w.id,
            1,
            Some(SourcePin {
                form_id: s.default_form_id.clone(),
                version: 1,
            }),
            fields("v"),
            None,
        )
        .await
        .unwrap();
    for sql in [
        "DROP TRIGGER species_versions_update",
        "DROP TRIGGER form_versions_update",
        "DROP TRIGGER world_revisions_update",
        "DROP TRIGGER variant_revisions_update",
        "DROP TRIGGER access_revisions_update",
        "DROP TRIGGER form_content_update",
        "DROP TRIGGER world_content_update",
        "DROP TRIGGER variant_content_update",
    ] {
        sqlx::query(sql).execute(&l.pool).await.unwrap();
    }
    for (kind, id) in [
        (HistoryKind::Species, &s.id),
        (HistoryKind::Form, &s.default_form_id),
        (HistoryKind::World, &w.id),
        (HistoryKind::Variant, &v.id),
    ] {
        let baseline = l
            .service
            .history_entry(kind, id, 1)
            .await
            .unwrap()
            .after_json;
        let content = snapshot_content(&baseline);
        let mut cases = common_invalid_snapshot_contents(&content, "id", "revision");
        let mut bad = content.clone();
        bad["fields"]["slug"] = "UPPERCASE".into();
        cases.push(bad);
        let mut bad = content.clone();
        bad["fields"]["description"] = "Not\r\ncanonical".into();
        cases.push(bad);
        let mut bad = content.clone();
        bad["fields"]["display_name"] = "x".repeat(101).into();
        cases.push(bad);
        let mut bad = content.clone();
        bad.as_object_mut().unwrap().remove("archived_at");
        cases.push(bad);
        let mut bad = content.clone();
        bad["archived_at"] = "not a timestamp".into();
        cases.push(bad);
        match kind {
            HistoryKind::Type | HistoryKind::WorldType => {
                unreachable!("this test covers the pre-Type aggregates")
            }
            HistoryKind::Species => {
                for value in ["bad", "00000000000000000000000000000001"] {
                    let mut bad = content.clone();
                    bad["default_form_id"] = value.into();
                    cases.push(bad);
                }
            }
            HistoryKind::Form => {
                for value in ["bad", "00000000000000000000000000000001"] {
                    let mut bad = content.clone();
                    bad["species_id"] = value.into();
                    cases.push(bad);
                }
                let mut bad = content.clone();
                bad["species_version"] = 0.into();
                cases.push(bad);
                let mut bad = content.clone();
                bad["is_default"] = "true".into();
                cases.push(bad);
            }
            HistoryKind::World => {
                for field in [
                    "owner_user_id",
                    "access_revision",
                    "collaborator_ids",
                    "grants",
                ] {
                    let mut bad = content.clone();
                    bad[field] = serde_json::json!([]);
                    cases.push(bad);
                }
                for value in [
                    serde_json::json!(["bad"]),
                    serde_json::json!([v.id, v.id]),
                    serde_json::json!([
                        "ffffffffffffffffffffffffffffffff",
                        "00000000000000000000000000000001"
                    ]),
                ] {
                    let mut bad = content.clone();
                    bad["variant_ids"] = value;
                    cases.push(bad);
                }
            }
            HistoryKind::Variant => {
                for value in ["bad", "00000000000000000000000000000001"] {
                    let mut bad = content.clone();
                    bad["world_id"] = value.into();
                    cases.push(bad);
                }
                let mut bad = content.clone();
                bad.as_object_mut().unwrap().remove("source");
                cases.push(bad);
                for pin in [
                    serde_json::json!({}),
                    serde_json::json!({"form_id":s.default_form_id}),
                    serde_json::json!({"version":1}),
                    serde_json::json!({"form_id":null,"version":1}),
                    serde_json::json!({"form_id":s.default_form_id,"version":null}),
                    serde_json::json!({"form_id":"bad","version":1}),
                    serde_json::json!({"form_id":s.default_form_id,"version":0}),
                    serde_json::json!({"form_id":s.default_form_id,"version":1,"extra":true}),
                ] {
                    let mut bad = content.clone();
                    bad["source"] = pin;
                    cases.push(bad);
                }
            }
        }
        for content in cases {
            let version = if matches!(kind, HistoryKind::Species) {
                1
            } else {
                2
            };
            let bad = serde_json::json!({"snapshot_version":version,"content":content}).to_string();
            replace_history_snapshot(&l, Some(kind), id, 1, false, &bad).await;
            assert_eq!(
                l.service.history_entry(kind, id, 1).await.unwrap_err(),
                Error::Unavailable,
                "{kind:?}: {bad}"
            );
            assert_eq!(
                l.service.history(kind, id, None).await.unwrap_err(),
                Error::Unavailable
            );
        }
        replace_history_snapshot(&l, Some(kind), id, 1, false, &baseline).await;
    }
    let baseline = l
        .service
        .access(&l.owner.token, &w.id)
        .await
        .unwrap()
        .history
        .remove(0)
        .after_json;
    let content = snapshot_content(&baseline);
    let mut cases = common_invalid_snapshot_contents(&content, "world_id", "access_revision");
    let mut bad = content.clone();
    bad["owner_user_id"] = "bad".into();
    cases.push(bad);
    let mut bad = content.clone();
    bad["fields"] = serde_json::json!({});
    cases.push(bad);
    for ids in [
        serde_json::json!(["bad"]),
        serde_json::json!([l.editor.actor.user_id, l.editor.actor.user_id]),
        serde_json::json!([l.owner.actor.user_id]),
    ] {
        let mut bad = content.clone();
        bad["collaborator_ids"] = ids;
        cases.push(bad);
    }
    for content in cases {
        let bad = serde_json::json!({"snapshot_version":1,"content":content}).to_string();
        replace_history_snapshot(&l, None, &w.id, 1, false, &bad).await;
        assert_eq!(
            l.service.access(&l.owner.token, &w.id).await.unwrap_err(),
            Error::Unavailable
        );
    }
    replace_history_snapshot(&l, None, &w.id, 1, false, &baseline).await;

    l.service
        .save_species(&l.admin.token, &s.id, edit(1, "s-new"))
        .await
        .unwrap();
    l.service
        .save_form(&l.admin.token, &s.default_form_id, 1, edit(1, "form-new"))
        .await
        .unwrap();
    l.service
        .save_variant(&l.owner.token, &w.id, &v.id, edit(1, "v-new"))
        .await
        .unwrap();
    l.service
        .grant_collaborator(&l.owner.token, &w.id, 1, &l.editor.actor.user_id, None)
        .await
        .unwrap();
    for (kind, id) in [
        (HistoryKind::Species, &s.id),
        (HistoryKind::Form, &s.default_form_id),
        (HistoryKind::World, &w.id),
        (HistoryKind::Variant, &v.id),
    ] {
        let baseline = l
            .service
            .history_entry(kind, id, 2)
            .await
            .unwrap()
            .before_json
            .unwrap();
        for content in
            common_invalid_snapshot_contents(&snapshot_content(&baseline), "id", "revision")
        {
            let version = if matches!(kind, HistoryKind::Species) {
                1
            } else {
                2
            };
            let bad = serde_json::json!({"snapshot_version":version,"content":content}).to_string();
            replace_history_snapshot(&l, Some(kind), id, 2, true, &bad).await;
            assert_eq!(
                l.service.history_entry(kind, id, 2).await.unwrap_err(),
                Error::Unavailable,
                "{kind:?} before: {bad}"
            );
        }
        replace_history_snapshot(&l, Some(kind), id, 2, true, &baseline).await;
    }
    let baseline = l
        .service
        .access(&l.owner.token, &w.id)
        .await
        .unwrap()
        .history
        .remove(0)
        .before_json
        .unwrap();
    for content in common_invalid_snapshot_contents(
        &snapshot_content(&baseline),
        "world_id",
        "access_revision",
    ) {
        let bad = serde_json::json!({"snapshot_version":1,"content":content}).to_string();
        replace_history_snapshot(&l, None, &w.id, 2, true, &bad).await;
        assert_eq!(
            l.service.access(&l.owner.token, &w.id).await.unwrap_err(),
            Error::Unavailable
        );
    }
}
