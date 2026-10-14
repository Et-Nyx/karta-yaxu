mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use game_data::{BUNDLED_FIXTURES, Catalog, authoring::ContentFields};
use web_app::{
    application::authoring::{
        Authoring, Edit, FormRecord, SourcePin, SpeciesRecord, VariantRecord, WorldRecord,
    },
    auth::{Role, Subject},
};

struct Site {
    app: common::TestApp,
    service: Authoring,
    token: String,
    species: SpeciesRecord,
    other_species: SpeciesRecord,
    form: FormRecord,
    other_form: FormRecord,
    world: WorldRecord,
    other_world: WorldRecord,
    variant: VariantRecord,
    other_variant: VariantRecord,
}

fn fields(name: &str) -> ContentFields {
    ContentFields {
        slug: "ignored-browser-value".into(),
        display_name: name.into(),
        description: "Description for URL tests".into(),
    }
}

fn edit(row: &ContentFields, revision: i64, name: &str) -> Edit {
    Edit {
        expected_revision: revision,
        fields: ContentFields {
            display_name: name.into(),
            ..row.clone()
        },
        reason: None,
    }
}

async fn site() -> Site {
    let app = common::app().await;
    app.accounts
        .invite("url-test", "owner", Role::Administrator, None)
        .await
        .unwrap();
    let actor = app
        .accounts
        .sign_in(Subject {
            provider: "url-test".into(),
            provider_subject: "owner".into(),
            display_name: "URL test owner".into(),
        })
        .await
        .unwrap();
    let (token, _) = app.accounts.start_session(&actor.user_id).await.unwrap();
    let service = Authoring::new(app.accounts.clone());
    let species = service
        .create_species(&token, fields("First species"), None)
        .await
        .unwrap();
    let other_species = service
        .create_species(&token, fields("Second species"), None)
        .await
        .unwrap();
    let form = service
        .create_form(&token, &species.id, 1, fields("Shared form"), None)
        .await
        .unwrap();
    let other_form = service
        .create_form(&token, &other_species.id, 1, fields("Shared form"), None)
        .await
        .unwrap();
    let world = service
        .create_world(&token, fields("First world"), None)
        .await
        .unwrap();
    let other_world = service
        .create_world(&token, fields("Second world"), None)
        .await
        .unwrap();
    let variant = service
        .create_variant(
            &token,
            &world.id,
            1,
            Some(SourcePin {
                form_id: form.id.clone(),
                version: 1,
            }),
            fields("Local variant"),
            None,
        )
        .await
        .unwrap();
    let other_variant = service
        .create_variant(
            &token,
            &other_world.id,
            1,
            None,
            fields("Local variant"),
            None,
        )
        .await
        .unwrap();
    // A real newer source makes adoption GETs exercise their normal workflow.
    let mut newer = edit(&form.fields, 1, &form.fields.display_name);
    newer.fields.description = "New source description".into();
    service.save_form(&token, &form.id, 1, newer).await.unwrap();
    Site {
        app,
        service,
        token,
        species,
        other_species,
        form,
        other_form,
        world,
        other_world,
        variant,
        other_variant,
    }
}

impl Site {
    fn cookie(&self) -> String {
        format!(
            "karta_session={}; karta_csrf={}",
            self.token,
            "a".repeat(64)
        )
    }

    async fn request(&self, method: &str, path: &str) -> axum::response::Response {
        common::send(
            &self.app.router,
            Request::builder()
                .method(method)
                .uri(path)
                .header(header::COOKIE, self.cookie())
                .header(header::HOST, "untrusted.example")
                .body(Body::empty())
                .unwrap(),
        )
        .await
    }

    async fn redirect(&self, method: &str, old: &str, pretty: &str) {
        let response = self.request(method, old).await;
        assert_eq!(
            response.status(),
            StatusCode::TEMPORARY_REDIRECT,
            "{method} {old}"
        );
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        assert_eq!(common::location(&response).as_deref(), Some(pretty));
        assert!(pretty.starts_with('/') && !pretty.starts_with("//"));
        assert!(common::read_body(response).await.is_empty());
        let response = self.request(method, pretty).await;
        assert_eq!(response.status(), StatusCode::OK, "{method} {pretty}");
        assert!(common::location(&response).is_none());
        if method == "HEAD" {
            assert!(common::read_body(response).await.is_empty());
        }
    }

    async fn post(&self, path: &str, body: &str) -> axum::response::Response {
        common::send(
            &self.app.router,
            common::post_form(
                path,
                Some("http://127.0.0.1:3000"),
                Some(&self.cookie()),
                body,
            ),
        )
        .await
    }
}

#[tokio::test]
async fn all_object_pages_suffixes_queries_and_creation_routes_use_readable_addresses() {
    let site = site().await;
    let pairs = [
        (
            format!("/creatures/{}", site.species.id),
            "/creatures/first-species".to_owned(),
        ),
        (
            format!("/forms/{}", site.form.id),
            "/creatures/first-species/forms/shared-form".to_owned(),
        ),
        (
            format!("/creatures/{}/forms/{}", site.species.id, site.form.id),
            "/creatures/first-species/forms/shared-form".to_owned(),
        ),
        (
            format!("/worlds/{}", site.world.id),
            "/worlds/first-world".to_owned(),
        ),
        (
            format!("/worlds/{}/variants/{}", site.world.id, site.variant.id),
            "/worlds/first-world/variants/local-variant".to_owned(),
        ),
    ];
    for (id, pretty) in &pairs {
        for suffix in ["", "/edit", "/history", "/history/1"] {
            let query = "?opaque=%2f%25%26&opaque=two+words";
            for method in ["GET", "HEAD"] {
                site.redirect(
                    method,
                    &format!("{id}{suffix}{query}"),
                    &format!("{pretty}{suffix}{query}"),
                )
                .await;
            }
        }
    }
    for (old, pretty) in [
        (
            format!("/worlds/{}/access?opaque=%2F", site.world.id),
            "/worlds/first-world/access?opaque=%2F".to_owned(),
        ),
        (
            format!(
                "/worlds/{}/variants/{}/adopt?target_version=2&opaque=%26",
                site.world.id, site.variant.id
            ),
            "/worlds/first-world/variants/local-variant/adopt?target_version=2&opaque=%26"
                .to_owned(),
        ),
        (
            format!("/creatures/{}/forms/new?opaque=%25", site.species.id),
            "/creatures/first-species/forms/new?opaque=%25".to_owned(),
        ),
        (
            format!(
                "/worlds/{}/variants/new?form_id={}&form_version=1",
                site.world.id, site.form.id
            ),
            format!(
                "/worlds/first-world/variants/new?form_id={}&form_version=1",
                site.form.id
            ),
        ),
        (
            format!("/forms/{}/history?before=2&opaque=%2F", site.form.id),
            "/creatures/first-species/forms/shared-form/history?before=2&opaque=%2F".to_owned(),
        ),
    ] {
        for method in ["GET", "HEAD"] {
            site.redirect(method, &old, &pretty).await;
        }
    }
    for path in [
        "/creatures/new",
        "/worlds/new",
        "/creatures/first-species/forms/new",
        "/worlds/first-world/variants/new",
    ] {
        let response = site.request("GET", path).await;
        assert_eq!(response.status(), StatusCode::OK);
        let html = common::read_body(response).await;
        assert!(!html.contains("name=\"slug\""));
        assert!(html.contains("Automatically generated from display name"));
    }
    for (path, action) in [
        (
            "/creatures/first-species/edit",
            format!("/creatures/{}/edit", site.species.id),
        ),
        (
            "/creatures/first-species/forms/shared-form/edit",
            format!("/forms/{}/edit", site.form.id),
        ),
        (
            "/worlds/first-world/edit",
            format!("/worlds/{}/edit", site.world.id),
        ),
        (
            "/worlds/first-world/variants/local-variant/edit",
            format!(
                "/worlds/{}/variants/{}/edit",
                site.world.id, site.variant.id
            ),
        ),
    ] {
        let response = site.request("GET", path).await;
        let html = common::read_body(response).await;
        assert!(html.contains(&format!("action=\"{action}\"")));
        assert!(!html.contains("name=\"slug\""));
    }
}

#[tokio::test]
async fn nested_resolution_is_scoped_for_ids_slugs_and_mixed_coordinates() {
    let site = site().await;
    for (pretty, id) in [
        ("/creatures/first-species/forms/shared-form", &site.form.id),
        (
            "/creatures/second-species/forms/shared-form",
            &site.other_form.id,
        ),
        (
            "/worlds/first-world/variants/local-variant",
            &site.variant.id,
        ),
        (
            "/worlds/second-world/variants/local-variant",
            &site.other_variant.id,
        ),
    ] {
        let response = site.request("GET", pretty).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert!(common::read_body(response).await.contains(id));
    }
    for suffix in ["", "/edit", "/history", "/history/1"] {
        for path in [
            format!("/creatures/second-species/forms/{}{suffix}", site.form.id),
            format!(
                "/creatures/{}/forms/{}{suffix}",
                site.other_species.id, site.form.id
            ),
            format!("/worlds/second-world/variants/{}{suffix}", site.variant.id),
            format!(
                "/worlds/{}/variants/{}{suffix}",
                site.other_world.id, site.variant.id
            ),
        ] {
            let response = site.request("GET", &path).await;
            assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
            assert!(common::location(&response).is_none());
        }
    }
    site.redirect(
        "GET",
        &format!("/creatures/{}/forms/shared-form", site.species.id),
        "/creatures/first-species/forms/shared-form",
    )
    .await;
    site.redirect(
        "GET",
        &format!("/worlds/first-world/variants/{}", site.variant.id),
        "/worlds/first-world/variants/local-variant",
    )
    .await;
}

#[tokio::test]
async fn reserved_new_and_id_shaped_slugs_are_unambiguous_for_all_four_types() {
    let site = site().await;
    let new_species = site
        .service
        .create_species(&site.token, fields("New"), None)
        .await
        .unwrap();
    let new_world = site
        .service
        .create_world(&site.token, fields("New"), None)
        .await
        .unwrap();
    let new_form = site
        .service
        .create_form(&site.token, &site.species.id, 1, fields("New"), None)
        .await
        .unwrap();
    let new_variant = site
        .service
        .create_variant(&site.token, &site.world.id, 2, None, fields("New"), None)
        .await
        .unwrap();
    for (old, escaped, creation, id) in [
        (
            format!("/creatures/{}", new_species.id),
            "/creatures/~new",
            "/creatures/new",
            &new_species.id,
        ),
        (
            format!("/worlds/{}", new_world.id),
            "/worlds/~new",
            "/worlds/new",
            &new_world.id,
        ),
        (
            format!("/forms/{}", new_form.id),
            "/creatures/first-species/forms/~new",
            "/creatures/first-species/forms/new",
            &new_form.id,
        ),
        (
            format!("/worlds/{}/variants/{}", site.world.id, new_variant.id),
            "/worlds/first-world/variants/~new",
            "/worlds/first-world/variants/new",
            &new_variant.id,
        ),
    ] {
        site.redirect("GET", &old, escaped).await;
        let html = common::read_body(site.request("GET", escaped).await).await;
        assert!(html.contains(id));
        let response = site.request("GET", creation).await;
        assert_eq!(response.status(), StatusCode::OK);
        let html = common::read_body(response).await;
        assert!(html.contains("method=\"post\""));
        assert!(!html.contains(&format!("action=\"{old}/edit\"")));
    }
    let shadow_species = site
        .service
        .create_species(&site.token, fields(&site.species.id), None)
        .await
        .unwrap();
    let shadow_world = site
        .service
        .create_world(&site.token, fields(&site.world.id), None)
        .await
        .unwrap();
    let shadow_form = site
        .service
        .create_form(
            &site.token,
            &site.species.id,
            1,
            fields(&site.form.id),
            None,
        )
        .await
        .unwrap();
    let shadow_variant = site
        .service
        .create_variant(
            &site.token,
            &site.world.id,
            3,
            None,
            fields(&site.variant.id),
            None,
        )
        .await
        .unwrap();
    for (plain, original_pretty, escaped, shadow_id) in [
        (
            format!("/creatures/{}", site.species.id),
            "/creatures/first-species",
            format!("/creatures/~{}", site.species.id),
            shadow_species.id,
        ),
        (
            format!("/worlds/{}", site.world.id),
            "/worlds/first-world",
            format!("/worlds/~{}", site.world.id),
            shadow_world.id,
        ),
        (
            format!("/creatures/first-species/forms/{}", site.form.id),
            "/creatures/first-species/forms/shared-form",
            format!("/creatures/first-species/forms/~{}", site.form.id),
            shadow_form.id,
        ),
        (
            format!("/worlds/first-world/variants/{}", site.variant.id),
            "/worlds/first-world/variants/local-variant",
            format!("/worlds/first-world/variants/~{}", site.variant.id),
            shadow_variant.id,
        ),
    ] {
        site.redirect("GET", &plain, original_pretty).await;
        let response = site.request("GET", &escaped).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert!(common::read_body(response).await.contains(&shadow_id));
    }
    let absent = "ffffffffffffffffffffffffffffffff";
    site.service
        .create_species(&site.token, fields(absent), None)
        .await
        .unwrap();
    site.service
        .create_world(&site.token, fields(absent), None)
        .await
        .unwrap();
    site.service
        .create_form(&site.token, &site.species.id, 1, fields(absent), None)
        .await
        .unwrap();
    site.service
        .create_variant(&site.token, &site.world.id, 4, None, fields(absent), None)
        .await
        .unwrap();
    for prefix in [
        "/creatures/",
        "/worlds/",
        "/creatures/first-species/forms/",
        "/worlds/first-world/variants/",
    ] {
        let response = site.request("GET", &format!("{prefix}{absent}")).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert!(common::location(&response).is_none());
        assert_eq!(
            site.request("GET", &format!("{prefix}~{absent}"))
                .await
                .status(),
            StatusCode::OK
        );
    }
}

#[tokio::test]
async fn encoded_segments_normalize_once_and_head_errors_have_no_body() {
    let site = site().await;
    for (encoded, pretty) in [
        ("/creatures/%66irst-species", "/creatures/first-species"),
        (
            "/creatures/first-species/forms/%73hared-form/edit",
            "/creatures/first-species/forms/shared-form/edit",
        ),
        (
            "/worlds/%66irst-world/history/%31",
            "/worlds/first-world/history/1",
        ),
        (
            "/worlds/first-world/variants/%6cocal-variant",
            "/worlds/first-world/variants/local-variant",
        ),
        ("/creatures/%6eew", "/creatures/new"),
        ("/%77orlds/new", "/worlds/new"),
    ] {
        for method in ["GET", "HEAD"] {
            site.redirect(
                method,
                &format!("{encoded}?x=%2F"),
                &format!("{pretty}?x=%2F"),
            )
            .await;
        }
    }
    for path in [
        "/creatures/first%2Fspecies",
        "/creatures/first%5cspecies",
        "/creatures/%00",
        "/creatures/%1f",
        "/creatures/%7f",
        "/creatures/%ff",
        "/creatures/%",
        "/creatures/%2",
        "/creatures/%gg",
        "/creatures/%2566irst-species",
        "/creatures/.",
        "/creatures/%2e%2e",
        "/creatures/~first-species",
        "/creatures/~~new",
        "/creatures/%7e%7enew",
        "/creatures/~",
        "/creatures/missing",
        "/worlds/missing",
        "/forms/missing",
        "/missing",
    ] {
        for method in ["GET", "HEAD"] {
            let response = site.request(method, path).await;
            assert_eq!(response.status(), StatusCode::NOT_FOUND, "{method} {path}");
            assert!(common::location(&response).is_none());
            if method == "HEAD" {
                assert!(common::read_body(response).await.is_empty());
            }
        }
    }
    let response = common::send(&site.app.router, common::get("/worlds/first-world/access")).await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let response = common::send(
        &site.app.router,
        Request::builder()
            .method("HEAD")
            .uri("/worlds/first-world/access")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert!(common::read_body(response).await.is_empty());
}

#[tokio::test]
async fn storage_failures_are_503_and_unknown_routes_and_other_methods_keep_inner_behavior() {
    let site = site().await;
    for method in ["POST", "PUT", "DELETE", "OPTIONS", "PATCH"] {
        for path in [
            format!("/creatures/{}", site.species.id),
            "/creatures/first-species".into(),
        ] {
            let response = site.request(method, &path).await;
            assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
            assert!(common::location(&response).is_none());
        }
    }
    site.app.pool.close().await;
    for path in [
        "/creatures/first-species",
        "/creatures/first-species/forms/shared-form",
        "/worlds/first-world",
        "/worlds/first-world/variants/local-variant",
        "/creatures/missing",
    ] {
        for method in ["GET", "HEAD"] {
            let response = site.request(method, path).await;
            assert_eq!(
                response.status(),
                StatusCode::SERVICE_UNAVAILABLE,
                "{method} {path}"
            );
            assert!(common::location(&response).is_none());
            if method == "HEAD" {
                assert!(common::read_body(response).await.is_empty());
            }
        }
    }
    for path in [
        "/creatures/first-species/unknown",
        "/creatures/first-species/history/",
        "/worlds/first-world/access/grant",
        "/creatures/first-species/forms/shared-form/adopt",
        "/worlds/first-world/variants/local-variant/history/1/extra",
    ] {
        let response = site.request("GET", path).await;
        let expected = if path.ends_with("/access/grant") {
            StatusCode::METHOD_NOT_ALLOWED
        } else {
            StatusCode::NOT_FOUND
        };
        assert_eq!(response.status(), expected, "{path}");
    }
    // POST passes through even when address resolution cannot access storage.
    assert_eq!(
        site.post("/creatures/first-species/edit", "")
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
}

#[tokio::test]
async fn parent_and_child_renames_use_current_rows_not_pinned_species_slugs() {
    let site = site().await;
    let species = site
        .service
        .save_species(
            &site.token,
            &site.species.id,
            edit(&site.species.fields, 1, "Renamed species"),
        )
        .await
        .unwrap();
    let form = site
        .service
        .save_form(
            &site.token,
            &site.form.id,
            1,
            edit(&site.form.fields, 2, "Renamed form"),
        )
        .await
        .unwrap();
    let world = site.service.world_detail(&site.world.id).await.unwrap();
    site.service
        .save_world(
            &site.token,
            &world.id,
            edit(&world.fields, world.revision, "Renamed world"),
        )
        .await
        .unwrap();
    site.service
        .save_variant(
            &site.token,
            &world.id,
            &site.variant.id,
            edit(&site.variant.fields, 1, "Renamed variant"),
        )
        .await
        .unwrap();
    assert_eq!(form.species_version, 1);
    assert_eq!(
        site.service
            .species_version(&species.id, 1)
            .await
            .unwrap()
            .fields
            .slug,
        "first-species"
    );
    for (old, pretty) in [
        (
            format!("/creatures/{}/history/1?x=%2f", species.id),
            "/creatures/renamed-species/history/1?x=%2f",
        ),
        (
            format!("/forms/{}/edit?x=%25", form.id),
            "/creatures/renamed-species/forms/renamed-form/edit?x=%25",
        ),
        (
            format!("/worlds/{}/history", world.id),
            "/worlds/renamed-world/history",
        ),
        (
            format!("/worlds/{}/variants/{}", world.id, site.variant.id),
            "/worlds/renamed-world/variants/renamed-variant",
        ),
    ] {
        site.redirect("GET", &old, pretty).await;
    }
    for path in [
        "/creatures/first-species",
        "/creatures/first-species/forms/renamed-form",
        "/creatures/renamed-species/forms/shared-form",
        "/worlds/first-world/variants/renamed-variant",
        "/worlds/renamed-world/variants/local-variant",
    ] {
        assert_eq!(
            site.request("GET", path).await.status(),
            StatusCode::NOT_FOUND,
            "{path}"
        );
    }
    let html = common::read_body(
        site.request("GET", "/creatures/renamed-species/forms/renamed-form")
            .await,
    )
    .await;
    assert!(html.contains("Species name at pin"));
    assert!(html.contains("First species"));
    assert!(html.contains("href=\"/creatures/renamed-species\""));
    assert!(!html.contains("href=\"/creatures/first-species"));
}

#[tokio::test]
async fn id_bound_stale_drafts_cannot_write_slug_replacements_and_posts_are_never_canonicalized() {
    let site = site().await;
    site.service
        .save_species(
            &site.token,
            &site.species.id,
            edit(&site.species.fields, 1, "Renamed species"),
        )
        .await
        .unwrap();
    let replacement_species = site
        .service
        .create_species(&site.token, fields("First species"), None)
        .await
        .unwrap();
    site.service
        .save_form(
            &site.token,
            &site.form.id,
            1,
            edit(&site.form.fields, 2, "Renamed form"),
        )
        .await
        .unwrap();
    let replacement_form = site
        .service
        .create_form(
            &site.token,
            &site.species.id,
            1,
            fields("Shared form"),
            None,
        )
        .await
        .unwrap();
    let current_world = site.service.world_detail(&site.world.id).await.unwrap();
    let world = site
        .service
        .save_world(
            &site.token,
            &site.world.id,
            edit(
                &current_world.fields,
                current_world.revision,
                "Renamed world",
            ),
        )
        .await
        .unwrap();
    let replacement_world = site
        .service
        .create_world(&site.token, fields("First world"), None)
        .await
        .unwrap();
    site.service
        .save_variant(
            &site.token,
            &site.world.id,
            &site.variant.id,
            edit(&site.variant.fields, 1, "Renamed variant"),
        )
        .await
        .unwrap();
    let replacement_variant = site
        .service
        .create_variant(
            &site.token,
            &site.world.id,
            world.revision,
            None,
            fields("Local variant"),
            None,
        )
        .await
        .unwrap();
    let cases = [
        (
            format!("/creatures/{}/edit", site.species.id),
            "/creatures/first-species/edit".to_owned(),
            1,
        ),
        (
            format!("/forms/{}/edit", site.form.id),
            "/creatures/renamed-species/forms/shared-form/edit".to_owned(),
            2,
        ),
        (
            format!("/worlds/{}/edit", site.world.id),
            "/worlds/first-world/edit".to_owned(),
            current_world.revision,
        ),
        (
            format!(
                "/worlds/{}/variants/{}/edit",
                site.world.id, site.variant.id
            ),
            "/worlds/renamed-world/variants/local-variant/edit".to_owned(),
            1,
        ),
    ];
    for (id_action, slug_action, revision) in cases {
        let body = format!(
            "csrf={}&expected_revision={revision}&species_version=1&slug=BOGUS&display_name=Retained+draft&description=Retained+description&reason=Retained+reason",
            "a".repeat(64)
        );
        let response = site.post(&id_action, &body).await;
        assert_eq!(response.status(), StatusCode::CONFLICT, "{id_action}");
        assert!(common::location(&response).is_none());
        let html = common::read_body(response).await;
        assert!(html.contains(&format!("action=\"{id_action}\"")));
        assert!(html.contains("value=\"Retained draft\""));
        assert!(html.contains("Retained description"));
        assert!(html.contains("Retained reason"));
        assert!(!html.contains("BOGUS"));
        let response = site.post(&slug_action, &body).await;
        assert!(matches!(
            response.status(),
            StatusCode::NOT_FOUND | StatusCode::UNPROCESSABLE_ENTITY
        ));
        assert!(common::location(&response).is_none());
    }
    assert_eq!(
        site.service
            .species_detail(&replacement_species.id)
            .await
            .unwrap(),
        replacement_species
    );
    assert_eq!(
        site.service
            .form_detail(&replacement_form.id)
            .await
            .unwrap(),
        replacement_form
    );
    assert_eq!(
        site.service
            .world_detail(&replacement_world.id)
            .await
            .unwrap(),
        replacement_world
    );
    assert_eq!(
        site.service
            .variant_detail(&site.world.id, &replacement_variant.id)
            .await
            .unwrap(),
        replacement_variant
    );
    // Reviewed ID submission changes the original and follows 303 -> 307 -> readable GET.
    let body = format!(
        "csrf={}&expected_revision=1&reviewed_revision=2&slug=BOGUS&display_name=Saved+reviewed+name&description=Reviewed+description",
        "a".repeat(64)
    );
    let response = site
        .post(&format!("/creatures/{}/edit", site.species.id), &body)
        .await;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let id_location = common::location(&response).unwrap();
    assert_eq!(id_location, format!("/creatures/{}", site.species.id));
    site.redirect("GET", &id_location, "/creatures/saved-reviewed-name")
        .await;
    assert_eq!(
        site.service
            .species_detail(&replacement_species.id)
            .await
            .unwrap(),
        replacement_species
    );
}

#[tokio::test]
async fn navigation_prefers_current_readable_coordinates() {
    let site = site().await;
    for (page, links) in [
        ("/creatures", vec!["/creatures/first-species"]),
        (
            "/creatures/first-species",
            vec![
                "/creatures/first-species/forms/shared-form",
                "/creatures/first-species/edit",
            ],
        ),
        ("/worlds", vec!["/worlds/first-world"]),
        (
            "/worlds/first-world",
            vec![
                "/worlds/first-world/variants/local-variant",
                "/worlds/first-world/variants/new",
            ],
        ),
        (
            "/creatures/first-species/forms/shared-form",
            vec![
                "/creatures/first-species",
                "/creatures/first-species/forms/shared-form/edit",
            ],
        ),
        (
            "/worlds/first-world/variants/local-variant",
            vec![
                "/worlds/first-world",
                "/worlds/first-world/variants/local-variant/adopt?target_version=2",
            ],
        ),
    ] {
        let html = common::read_body(site.request("GET", page).await).await;
        for link in links {
            assert!(
                html.contains(&format!("href=\"{link}\"")),
                "{page}: missing {link}"
            );
        }
    }
    // Fixture ids are durable permalinks, while catalog cards use stored fixture slugs.
    for creature in Catalog::from_json(BUNDLED_FIXTURES).unwrap().creatures() {
        site.redirect(
            "GET",
            &format!("/creatures/{}", creature.id()),
            &format!("/creatures/{}", creature.slug()),
        )
        .await;
    }
}
