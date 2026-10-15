//! HTTP workflows for native type authoring, exact pins and scope isolation.
use super::*;
use crate::{
    application::authoring::{Edit, HistoryKind},
    auth::{Role, Subject},
    persistence,
};
use axum::body::{Body, to_bytes};
use game_data::{authoring::ContentFields, types::TypePin};
use tempfile::TempDir;
use tower::ServiceExt;

struct Site {
    state: AppState,
    app: Router,
    token: String,
    _directory: TempDir,
}

fn fields(name: &str) -> ContentFields {
    ContentFields {
        slug: String::new(),
        display_name: name.into(),
        description: "Type metadata <script>inert</script>".into(),
    }
}

async fn site() -> Site {
    let directory = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}/types-http.db", directory.path().display());
    let pool = persistence::connect_for_migration(&url).await.unwrap();
    persistence::migrate(&pool).await.unwrap();
    let accounts = Accounts::new(pool, 3600).unwrap();
    let token = user(&accounts, "owner", Role::Administrator).await;
    let state = AppState::new(accounts, None, "http://127.0.0.1:3000".into(), false);
    Site {
        app: router(state.clone()),
        state,
        token,
        _directory: directory,
    }
}

async fn user(accounts: &Accounts, name: &str, role: Role) -> String {
    accounts.invite("test", name, role, None).await.unwrap();
    let actor = accounts
        .sign_in(Subject {
            provider: "test".into(),
            provider_subject: name.into(),
            display_name: name.into(),
        })
        .await
        .unwrap();
    accounts.start_session(&actor.user_id).await.unwrap().0
}

fn body(extra: &str) -> String {
    format!(
        "csrf={}&display_name=Draft+type&description=Retained+description&reason=Keep+reason&{extra}",
        "a".repeat(64)
    )
}

impl Site {
    async fn raw(
        &self,
        method: &str,
        path: &str,
        data: &str,
        token: Option<&str>,
        origin: Option<&str>,
        csrf: &str,
    ) -> (StatusCode, HeaderMap, String) {
        let cookie = match token {
            Some(token) => format!("karta_session={token}; karta_csrf={csrf}"),
            None => format!("karta_csrf={csrf}"),
        };
        let mut request = axum::http::Request::builder()
            .method(method)
            .uri(path)
            .header(header::COOKIE, cookie)
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded");
        if let Some(origin) = origin {
            request = request.header(header::ORIGIN, origin);
        }
        let response = self
            .app
            .clone()
            .oneshot(request.body(Body::from(data.to_owned())).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let headers = response.headers().clone();
        let bytes = to_bytes(response.into_body(), 512 * 1024).await.unwrap();
        (status, headers, String::from_utf8(bytes.to_vec()).unwrap())
    }

    async fn request(
        &self,
        method: &str,
        path: &str,
        data: &str,
        token: Option<&str>,
    ) -> (StatusCode, HeaderMap, String) {
        let mut path = path.to_owned();
        for _ in 0..5 {
            let response = self
                .raw(
                    method,
                    &path,
                    data,
                    token,
                    Some("http://127.0.0.1:3000"),
                    &"a".repeat(64),
                )
                .await;
            if method != "GET" || response.0 != StatusCode::TEMPORARY_REDIRECT {
                return response;
            }
            path = response.1[header::LOCATION].to_str().unwrap().to_owned();
        }
        panic!("GET address redirect loop")
    }
}

#[tokio::test]
async fn type_forms_enforce_origin_csrf_body_bound_and_session_attribution() {
    let site = site().await;
    for (origin, csrf, data, status) in [
        (
            None,
            "a".repeat(64),
            body("actor_id=forged"),
            StatusCode::FORBIDDEN,
        ),
        (
            Some("https://elsewhere.example"),
            "a".repeat(64),
            body(""),
            StatusCode::FORBIDDEN,
        ),
        (
            Some("http://127.0.0.1:3000"),
            "b".repeat(64),
            body(""),
            StatusCode::FORBIDDEN,
        ),
        (
            Some("http://127.0.0.1:3000"),
            "a".repeat(64),
            body(&format!("padding={}", "x".repeat(32 * 1024))),
            StatusCode::PAYLOAD_TOO_LARGE,
        ),
    ] {
        let response = site
            .raw(
                "POST",
                "/types/new",
                &data,
                Some(&site.token),
                origin,
                &csrf,
            )
            .await;
        assert_eq!(response.0, status);
        assert_eq!(response.1[header::CACHE_CONTROL], "no-store");
        assert!(site.state.types.types().await.unwrap().is_empty());
    }
    let response = site
        .request(
            "POST",
            "/types/new",
            &body("actor_id=forged&owner_user_id=forged&slug=forged"),
            Some(&site.token),
        )
        .await;
    assert_eq!(response.0, StatusCode::SEE_OTHER);
    let entry = site.state.types.types().await.unwrap().remove(0);
    assert_eq!(entry.fields.slug, "draft-type");
    let history = site
        .state
        .authoring
        .history(HistoryKind::Type, &entry.id, None)
        .await
        .unwrap();
    assert_eq!(history[0].actor_name, "owner");
    let (status, headers, html) = site
        .request("GET", "/types/draft-type/edit", "", Some(&site.token))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers[header::CACHE_CONTROL], "no-store");
    assert!(html.contains(&format!("action=\"/types/{}/edit\"", entry.id)));
    assert!(!html.contains("name=\"slug\""));
    assert!(!html.contains("<script>inert</script>"));
}

#[tokio::test]
async fn type_addresses_keep_id_posts_escape_reserved_slugs_and_preserve_unknown_shapes() {
    let site = site().await;
    for name in ["new", "0123456789abcdef0123456789abcdef"] {
        let entry = site
            .state
            .types
            .create_type(&site.token, fields(name), None)
            .await
            .unwrap();
        for suffix in ["", "/edit", "/history", "/history/1"] {
            let preferred = format!("/types/~{}{suffix}", entry.fields.slug);
            let legacy = format!("/types/{}{suffix}?slots=7", entry.id);
            let (status, headers, _) = site
                .raw("GET", &legacy, "", Some(&site.token), None, &"a".repeat(64))
                .await;
            assert_eq!(status, StatusCode::TEMPORARY_REDIRECT);
            assert_eq!(headers[header::LOCATION], format!("{preferred}?slots=7"));
            assert_eq!(
                site.request("GET", &preferred, "", Some(&site.token))
                    .await
                    .0,
                StatusCode::OK
            );
        }
        let (status, _, html) = site
            .raw(
                "HEAD",
                &format!("/types/~{}", entry.fields.slug),
                "",
                None,
                None,
                &"a".repeat(64),
            )
            .await;
        assert_eq!(status, StatusCode::OK);
        assert!(html.is_empty());
        let (status, headers, _) = site
            .request(
                "POST",
                &format!("/types/{}/edit", entry.id),
                &body("expected_revision=1"),
                Some(&site.token),
            )
            .await;
        assert_eq!(status, StatusCode::SEE_OTHER);
        assert_eq!(headers[header::LOCATION], format!("/types/{}", entry.id));
    }
    for (method, path, expected) in [
        ("GET", "/types/missing/bogus", StatusCode::NOT_FOUND),
        ("PUT", "/types/missing/edit", StatusCode::METHOD_NOT_ALLOWED),
        (
            "POST",
            "/types/missing/history",
            StatusCode::METHOD_NOT_ALLOWED,
        ),
    ] {
        assert_eq!(
            site.raw(method, path, "", None, None, &"a".repeat(64))
                .await
                .0,
            expected
        );
    }
    let (status, _, html) = site
        .raw("HEAD", "/types/not-found", "", None, None, &"a".repeat(64))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(html.is_empty());
}

#[tokio::test]
async fn readonly_catalogs_hide_writes_and_admin_has_no_world_bypass() {
    let site = site().await;
    let shared = site
        .state
        .types
        .create_type(&site.token, fields("Fire"), None)
        .await
        .unwrap();
    let world = site
        .state
        .authoring
        .create_world(&site.token, fields("Lab"), None)
        .await
        .unwrap();
    let local = site
        .state
        .types
        .create_world_type(
            &site.token,
            &world.id,
            world.revision,
            Some(TypePin {
                type_id: shared.id.clone(),
                version: 1,
            }),
            fields("Local fire"),
            None,
        )
        .await
        .unwrap();
    let outsider = user(&site.state.accounts, "other-admin", Role::Administrator).await;
    let viewer = user(&site.state.accounts, "viewer", Role::Viewer).await;
    for token in [None, Some(viewer.as_str())] {
        for path in [
            "/types".into(),
            format!("/types/{}", shared.id),
            format!("/worlds/{}", world.id),
            format!("/worlds/{}/types/{}", world.id, local.id),
        ] {
            let (status, _, html) = site.request("GET", &path, "", token).await;
            assert_eq!(status, StatusCode::OK);
            assert!(!html.contains("Create shared type</a>"));
            assert!(!html.contains("Edit type</a>"));
            assert!(!html.contains("Add type</a>"));
            assert!(!html.contains("Add type version 1 to this world</a>"));
            assert!(!html.contains("owner_user_id"));
        }
    }
    for suffix in ["new", &format!("{}/edit", local.id)] {
        let path = format!("/worlds/{}/types/{suffix}", world.id);
        assert_eq!(
            site.request("GET", &path, "", Some(&outsider)).await.0,
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            site.request(
                "POST",
                &path,
                &body("expected_world_revision=2&expected_revision=1"),
                Some(&outsider)
            )
            .await
            .0,
            StatusCode::FORBIDDEN
        );
    }
    assert_eq!(
        site.request("POST", "/types/new", &body("actor_id=owner"), Some(&viewer))
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    let other_world = site
        .state
        .authoring
        .create_world(&site.token, fields("Other lab"), None)
        .await
        .unwrap();
    assert_eq!(
        site.request(
            "GET",
            &format!("/worlds/{}/types/{}", other_world.id, local.id),
            "",
            None
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn ordered_typing_skips_blanks_keeps_old_pins_and_requires_explicit_conflict_review() {
    let site = site().await;
    let species = site
        .state
        .authoring
        .create_species(&site.token, fields("Creature"), None)
        .await
        .unwrap();
    let fire = site
        .state
        .types
        .create_type(&site.token, fields("Fire"), None)
        .await
        .unwrap();
    let flying = site
        .state
        .types
        .create_type(&site.token, fields("Flying"), None)
        .await
        .unwrap();
    let path = format!("/forms/{}/typing", species.default_form_id);
    let extra = format!(
        "expected_revision=1&mode=defined&slots=3&slot_1={}@1&slot_2=&slot_3={}@1",
        flying.id, fire.id
    );
    assert_eq!(
        site.request("POST", &path, &body(&extra), Some(&site.token))
            .await
            .0,
        StatusCode::SEE_OTHER
    );
    let typing = site
        .state
        .types
        .form_typing(&species.default_form_id, None)
        .await
        .unwrap();
    let assignments = typing.assignments.unwrap();
    assert_eq!(
        assignments
            .iter()
            .map(|entry| entry.pin.type_id.as_str())
            .collect::<Vec<_>>(),
        [flying.id.as_str(), fire.id.as_str()]
    );
    site.state
        .types
        .save_type(
            &site.token,
            &fire.id,
            Edit {
                expected_revision: 1,
                fields: fields("Hot fire"),
                reason: None,
            },
        )
        .await
        .unwrap();
    let (status, headers, html) = site.request("GET", &path, "", Some(&site.token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers[header::CACHE_CONTROL], "no-store");
    assert!(html.contains("Fire · historical version 1"));
    assert!(html.contains("Hot fire · version 2"));
    assert!(html.contains(&format!("value=\"{}@1\" selected", fire.id)));
    let (status, _, html) = site
        .request("POST", &path, &body(&extra), Some(&site.token))
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert!(html.contains("Current saved state — revision 2"));
    assert!(html.contains("name=\"expected_revision\" value=\"1\""));
    assert!(html.contains("name=\"reviewed_revision\" value=\"2\""));
    assert!(html.contains(&format!("value=\"{}@1\" selected", flying.id)));
    let duplicate = format!(
        "expected_revision=2&mode=defined&slots=3&slot_1={}@1&slot_3={}@2",
        fire.id, fire.id
    );
    let (status, _, html) = site
        .request("POST", &path, &body(&duplicate), Some(&site.token))
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(html.contains("slot-3-error"));
    assert!(html.contains("already selected"));
    assert!(html.contains("Keep reason"));
    let reviewed = format!("{extra}&reviewed_revision=2").replace(
        &format!("slot_3={}@1", fire.id),
        &format!("slot_3={}@2", fire.id),
    );
    assert_eq!(
        site.request("POST", &path, &body(&reviewed), Some(&site.token))
            .await
            .0,
        StatusCode::SEE_OTHER
    );
    assert_eq!(
        site.state
            .types
            .form_typing(&species.default_form_id, None)
            .await
            .unwrap()
            .assignments
            .unwrap()[1]
            .pin
            .version,
        2
    );
    for (mode, label, revision) in [
        ("defined", "Defined — intentionally typeless", 3),
        ("unmodeled", "Typing not modeled", 4),
    ] {
        assert_eq!(
            site.request(
                "POST",
                &path,
                &body(&format!("expected_revision={revision}&mode={mode}&slots=3")),
                Some(&site.token)
            )
            .await
            .0,
            StatusCode::SEE_OTHER
        );
        let (status, _, html) = site
            .request(
                "GET",
                &format!("/forms/{}", species.default_form_id),
                "",
                None,
            )
            .await;
        assert_eq!(status, StatusCode::OK);
        assert!(html.contains(label));
        assert!(!html.contains("Edit typing</a>"));
    }
}

#[tokio::test]
async fn typing_supports_32_definitions_and_rejects_wrong_scope_and_oversize_work() {
    let site = site().await;
    let species = site
        .state
        .authoring
        .create_species(&site.token, fields("Many types"), None)
        .await
        .unwrap();
    let path = format!("/forms/{}/typing", species.default_form_id);
    let mut extra = "expected_revision=1&mode=defined&slots=32".to_owned();
    for index in 1..=32 {
        let entry = site
            .state
            .types
            .create_type(&site.token, fields(&format!("Type {index}")), None)
            .await
            .unwrap();
        extra.push_str(&format!("&slot_{index}={}@1", entry.id));
    }
    assert_eq!(
        site.request("POST", &path, &body(&extra), Some(&site.token))
            .await
            .0,
        StatusCode::SEE_OTHER
    );
    assert_eq!(
        site.state
            .types
            .form_typing(&species.default_form_id, None)
            .await
            .unwrap()
            .assignments
            .unwrap()
            .len(),
        32
    );
    let (status, _, html) = site
        .request("GET", &format!("{path}?slots=32"), "", Some(&site.token))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("name=\"slot_32\""));
    assert!(!html.contains("name=\"slot_33\""));
    assert_eq!(
        site.request("GET", &format!("{path}?slots=33"), "", Some(&site.token))
            .await
            .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let world = site
        .state
        .authoring
        .create_world(&site.token, fields("World"), None)
        .await
        .unwrap();
    let other = site
        .state
        .authoring
        .create_world(&site.token, fields("Other"), None)
        .await
        .unwrap();
    let local = site
        .state
        .types
        .create_world_type(&site.token, &other.id, 1, None, fields("Other local"), None)
        .await
        .unwrap();
    let variant = site
        .state
        .authoring
        .create_variant(&site.token, &world.id, 1, None, fields("Variant"), None)
        .await
        .unwrap();
    let variant_path = format!("/worlds/{}/variants/{}/typing", world.id, variant.id);
    let shared = site.state.types.types().await.unwrap().remove(0);
    for pin in [local.id, shared.id] {
        let (status, headers, html) = site
            .request(
                "POST",
                &variant_path,
                &body(&format!(
                    "expected_revision=1&mode=defined&slots=3&slot_2={pin}@1"
                )),
                Some(&site.token),
            )
            .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(headers[header::CACHE_CONTROL], "no-store");
        assert!(html.contains(&format!("value=\"{pin}@1\" selected")));
        assert_eq!(
            site.state
                .authoring
                .variant_detail(&world.id, &variant.id)
                .await
                .unwrap()
                .revision,
            1
        );
    }
}

#[tokio::test]
async fn metadata_conflicts_keep_fields_and_adoption_compares_named_pins_without_retargeting() {
    let site = site().await;
    let shared = site
        .state
        .types
        .create_type(&site.token, fields("Old fire"), None)
        .await
        .unwrap();
    let world = site
        .state
        .authoring
        .create_world(&site.token, fields("Lab"), None)
        .await
        .unwrap();
    let local = site
        .state
        .types
        .create_world_type(
            &site.token,
            &world.id,
            1,
            Some(TypePin {
                type_id: shared.id.clone(),
                version: 1,
            }),
            fields("Local fire"),
            None,
        )
        .await
        .unwrap();
    let variant = site
        .state
        .authoring
        .create_variant(&site.token, &world.id, 2, None, fields("Variant"), None)
        .await
        .unwrap();
    site.state
        .types
        .save_variant_typing(
            &site.token,
            &world.id,
            &variant.id,
            1,
            Some(vec![TypePin {
                type_id: local.id.clone(),
                version: 1,
            }]),
            None,
        )
        .await
        .unwrap();
    site.state
        .types
        .save_type(
            &site.token,
            &shared.id,
            Edit {
                expected_revision: 1,
                fields: fields("New fire"),
                reason: None,
            },
        )
        .await
        .unwrap();
    let (status, _, html) = site
        .request(
            "GET",
            &format!(
                "/worlds/{}/types/{}/adopt?target_version=2",
                world.id, local.id
            ),
            "",
            Some(&site.token),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    for label in [
        "Old shared source",
        "New shared source",
        "Old fire",
        "New fire",
        "Local fire",
        "Existing variant type pins stay frozen",
    ] {
        assert!(html.contains(label), "missing {label}");
    }
    assert!(!html.contains("<script>inert</script>"));
    let path = format!("/worlds/{}/types/{}/adopt", world.id, local.id);
    assert_eq!(
        site.request(
            "POST",
            &path,
            &body("expected_revision=1&target_version=2"),
            Some(&site.token)
        )
        .await
        .0,
        StatusCode::SEE_OTHER
    );
    assert_eq!(
        site.state
            .types
            .variant_typing(&world.id, &variant.id, None)
            .await
            .unwrap()
            .assignments
            .unwrap()[0]
            .pin
            .version,
        1
    );
    let (status, _, html) = site
        .request(
            "POST",
            &path,
            &body("expected_revision=1&target_version=2"),
            Some(&site.token),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert!(html.contains("name=\"reviewed_revision\" value=\"2\" disabled"));
    let (status, _, html) = site
        .request(
            "POST",
            &format!("/types/{}/edit", shared.id),
            &body("expected_revision=1"),
            Some(&site.token),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert!(html.contains("value=\"Draft type\""));
    assert!(html.contains("New fire"));
    assert!(html.contains("Save reviewed changes"));
    let (status, _, html) = site
        .request("GET", &format!("/worlds/{}/history", world.id), "", None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("World type membership"));
    assert!(html.contains(&format!("1 types: {}", local.id)));
    for private in ["owner_user_id", "collaborator_user_ids", "access_revision"] {
        assert!(!html.contains(private));
    }
    let world_revision = site
        .state
        .authoring
        .history_entry(HistoryKind::World, &world.id, 2)
        .await
        .unwrap();
    let snapshot: serde_json::Value = serde_json::from_str(&world_revision.after_json).unwrap();
    assert_eq!(
        snapshot.pointer("/content/type_ids").unwrap(),
        &serde_json::json!([local.id])
    );
    assert!(snapshot.pointer("/content/world_type_ids").is_none());

    let (status, _, html) = site
        .request(
            "GET",
            &format!("/worlds/{}/variants/{}/history/2", world.id, variant.id),
            "",
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let exact_link = format!("/worlds/{}/types/{}/history/1", world.id, local.id);
    assert!(html.contains(&format!("href=\"{exact_link}\"")));
    assert!(html.contains(&format!("Local fire · {} · exact version 1", local.id)));
    let (status, _, html) = site.request("GET", &exact_link, "", None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("Local fire"));
}

#[tokio::test]
async fn public_typing_history_renders_order_names_and_inert_full_snapshots() {
    let site = site().await;
    let species = site
        .state
        .authoring
        .create_species(&site.token, fields("Creature"), None)
        .await
        .unwrap();
    let entry = site
        .state
        .types
        .create_type(&site.token, fields("<script>Fire</script>"), None)
        .await
        .unwrap();
    let flying = site
        .state
        .types
        .create_type(&site.token, fields("Flying"), None)
        .await
        .unwrap();
    site.state
        .types
        .save_form_typing(
            &site.token,
            &species.default_form_id,
            1,
            Some(vec![
                TypePin {
                    type_id: flying.id.clone(),
                    version: 1,
                },
                TypePin {
                    type_id: entry.id.clone(),
                    version: 1,
                },
            ]),
            None,
        )
        .await
        .unwrap();
    let history_entry = site
        .state
        .authoring
        .history_entry(HistoryKind::Form, &species.default_form_id, 2)
        .await
        .unwrap();
    let snapshot: serde_json::Value = serde_json::from_str(&history_entry.after_json).unwrap();
    let assignments = snapshot
        .pointer("/content/typing/assignments")
        .unwrap()
        .as_array()
        .unwrap();
    assert_eq!(assignments.len(), 2);
    assert_eq!(
        assignments[0]["pin"],
        serde_json::json!({"type_id": flying.id, "version": 1})
    );
    assert_eq!(
        assignments[1]["fields"]["display_name"],
        "<script>Fire</script>"
    );
    site.state
        .types
        .save_type(
            &site.token,
            &entry.id,
            Edit {
                expected_revision: 1,
                fields: fields("Latest fire"),
                reason: None,
            },
        )
        .await
        .unwrap();
    for suffix in ["/history", "/history/2"] {
        let (status, _, html) = site
            .request(
                "GET",
                &format!("/forms/{}{suffix}", species.default_form_id),
                "",
                None,
            )
            .await;
        assert_eq!(status, StatusCode::OK);
        assert!(html.contains("Ordered typing"));
        assert!(html.contains("Typing not modeled"));
        let flying_link = format!("href=\"/types/{}/history/1\"", flying.id);
        let fire_link = format!("href=\"/types/{}/history/1\"", entry.id);
        assert!(html.find(&flying_link).unwrap() < html.find(&fire_link).unwrap());
        assert!(html.contains(&format!("Flying · {} · exact version 1", flying.id)));
        assert!(html.contains(&format!(
            "&#60;script&#62;Fire&#60;/script&#62; · {} · exact version 1",
            entry.id
        )));
        assert!(!html.contains("Latest fire"));
        assert!(html.contains("<ol>"));
        assert!(!html.contains("<script>Fire</script>"));
    }
    let (status, _, html) = site
        .request("GET", &format!("/types/{}/history/1", entry.id), "", None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("Immutable revision snapshots"));
    assert!(!html.contains("<script>Fire</script>"));
    assert!(html.contains("&#60;script&#62;Fire&#60;/script&#62;"));
    assert!(!html.contains("Latest fire"));
}

#[tokio::test]
async fn upgraded_v1_form_remains_unmodeled_and_v2_history_keeps_original_meaning() {
    use sqlx::SqlSafeStr;
    let directory = tempfile::tempdir().unwrap();
    let pool = persistence::connect_for_migration(&format!(
        "sqlite://{}/legacy-http.db",
        directory.path().display()
    ))
    .await
    .unwrap();
    let migrations = [
        (1, "auth", include_str!("../../migrations/0001_auth.sql")),
        (
            2,
            "world authoring",
            include_str!("../../migrations/0002_world_authoring.sql"),
        ),
    ]
    .into_iter()
    .map(|(version, description, sql)| {
        sqlx::migrate::Migration::new(
            version,
            description.into(),
            sqlx::migrate::MigrationType::Simple,
            sql.into_sql_str(),
            false,
        )
    })
    .collect();
    sqlx::migrate::Migrator {
        migrations: std::borrow::Cow::Owned(migrations),
        ..sqlx::migrate::Migrator::DEFAULT
    }
    .run(&pool)
    .await
    .unwrap();
    let accounts = Accounts::new(pool.clone(), 3600).unwrap();
    let token = user(&accounts, "owner", Role::Administrator).await;
    let actor = accounts.authenticate(&token).await.unwrap().unwrap();
    let species_id = "00000000000000000000000000000001";
    let form_id = "00000000000000000000000000000002";
    let species_audit = "00000000000000000000000000000003";
    let form_audit = "00000000000000000000000000000004";
    let content = ContentFields {
        slug: "legacy".into(),
        display_name: "Legacy".into(),
        description: "<script>legacy</script>".into(),
    };
    let species_json = serde_json::json!({"snapshot_version":1,"content":{"id":species_id,"fields":content,"revision":1,"default_form_id":form_id,"archived_at":null}}).to_string();
    let form_json = serde_json::json!({"snapshot_version":1,"content":{"id":form_id,"species_id":species_id,"species_version":1,"fields":content,"revision":1,"is_default":true,"archived_at":null}}).to_string();
    let mut tx = pool.begin().await.unwrap();
    for audit in [species_audit, form_audit] {
        sqlx::query("INSERT INTO audit_events(id, actor_user_id, action) VALUES (?, ?, 'create')")
            .bind(audit)
            .bind(&actor.user_id)
            .execute(&mut *tx)
            .await
            .unwrap();
    }
    sqlx::query("INSERT INTO species(id, slug, display_name, description, revision, default_form_id) VALUES (?, 'legacy', 'Legacy', ?, 1, ?)").bind(species_id).bind(&content.description).bind(form_id).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO species_versions(species_id, revision, slug, display_name, description, default_form_id, audit_id, after_json) VALUES (?, 1, 'legacy', 'Legacy', ?, ?, ?, ?)").bind(species_id).bind(&content.description).bind(form_id).bind(species_audit).bind(&species_json).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO forms(id, species_id, species_version, slug, display_name, description, revision, is_default) VALUES (?, ?, 1, 'legacy', 'Legacy', ?, 1, 1)").bind(form_id).bind(species_id).bind(&content.description).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO form_versions(form_id, species_id, species_version, revision, slug, display_name, description, is_default, audit_id, after_json) VALUES (?, ?, 1, 1, 'legacy', 'Legacy', ?, 1, ?, ?)").bind(form_id).bind(species_id).bind(&content.description).bind(form_audit).bind(&form_json).execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
    persistence::migrate(&pool).await.unwrap();
    let state = AppState::new(accounts, None, "http://127.0.0.1:3000".into(), false);
    let site = Site {
        app: router(state.clone()),
        state,
        token,
        _directory: directory,
    };
    for suffix in ["", "/history/1"] {
        let (status, _, html) = site
            .request("GET", &format!("/forms/{form_id}{suffix}"), "", None)
            .await;
        assert_eq!(status, StatusCode::OK);
        assert!(html.contains("Typing not modeled"));
        assert!(!html.contains("intentionally typeless"));
        assert!(!html.contains("<script>legacy</script>"));
    }
    let path = format!("/forms/{form_id}/typing");
    let mut revised_fields = content.clone();
    revised_fields
        .description
        .push_str("\nMetadata edit preserves unmodeled typing.");
    site.state
        .authoring
        .save_form(
            &site.token,
            form_id,
            1,
            Edit {
                expected_revision: 1,
                fields: revised_fields,
                reason: None,
            },
        )
        .await
        .unwrap();
    let (status, _, html) = site
        .request("GET", &format!("/forms/{form_id}/history/2"), "", None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("Typing not modeled"));
    assert!(
        !html.contains("Ordered typing"),
        "v1 absence and v2 null have the same meaning"
    );
    let unmodeled = site
        .state
        .authoring
        .history_entry(HistoryKind::Form, form_id, 2)
        .await
        .unwrap();
    let snapshot: serde_json::Value = serde_json::from_str(&unmodeled.after_json).unwrap();
    assert_eq!(
        snapshot.pointer("/content/typing").unwrap(),
        &serde_json::json!({"assignments": null})
    );
    assert_eq!(
        site.request(
            "POST",
            &path,
            &body("expected_revision=2&mode=defined&slots=3"),
            Some(&site.token)
        )
        .await
        .0,
        StatusCode::SEE_OTHER
    );
    let (status, _, html) = site
        .request("GET", &format!("/forms/{form_id}/history/3"), "", None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("Typing not modeled"));
    assert!(html.contains("Defined — intentionally typeless"));
    let defined = site
        .state
        .authoring
        .history_entry(HistoryKind::Form, form_id, 3)
        .await
        .unwrap();
    let snapshot: serde_json::Value = serde_json::from_str(&defined.after_json).unwrap();
    assert_eq!(
        snapshot.pointer("/content/typing").unwrap(),
        &serde_json::json!({"assignments": []})
    );
    let original = site
        .state
        .authoring
        .history_entry(HistoryKind::Form, form_id, 1)
        .await
        .unwrap();
    assert_eq!(original.after_json, form_json);
}

#[tokio::test]
async fn form_source_adoption_shows_old_new_and_local_typing_without_mapping() {
    use crate::application::authoring::SourcePin;
    let site = site().await;
    let species = site
        .state
        .authoring
        .create_species(&site.token, fields("Creature"), None)
        .await
        .unwrap();
    let fire = site
        .state
        .types
        .create_type(&site.token, fields("Shared fire"), None)
        .await
        .unwrap();
    let flying = site
        .state
        .types
        .create_type(&site.token, fields("Shared flying"), None)
        .await
        .unwrap();
    site.state
        .types
        .save_form_typing(
            &site.token,
            &species.default_form_id,
            1,
            Some(vec![TypePin {
                type_id: fire.id.clone(),
                version: 1,
            }]),
            None,
        )
        .await
        .unwrap();
    let world = site
        .state
        .authoring
        .create_world(&site.token, fields("Lab"), None)
        .await
        .unwrap();
    let local = site
        .state
        .types
        .create_world_type(
            &site.token,
            &world.id,
            1,
            Some(TypePin {
                type_id: fire.id,
                version: 1,
            }),
            fields("World flame"),
            None,
        )
        .await
        .unwrap();
    let variant = site
        .state
        .authoring
        .create_variant(
            &site.token,
            &world.id,
            2,
            Some(SourcePin {
                form_id: species.default_form_id.clone(),
                version: 2,
            }),
            fields("Local creature"),
            None,
        )
        .await
        .unwrap();
    site.state
        .types
        .save_variant_typing(
            &site.token,
            &world.id,
            &variant.id,
            1,
            Some(vec![TypePin {
                type_id: local.id.clone(),
                version: 1,
            }]),
            None,
        )
        .await
        .unwrap();
    site.state
        .types
        .save_form_typing(
            &site.token,
            &species.default_form_id,
            2,
            Some(vec![TypePin {
                type_id: flying.id,
                version: 1,
            }]),
            None,
        )
        .await
        .unwrap();
    let path = format!("/worlds/{}/variants/{}/adopt", world.id, variant.id);
    let (status, headers, html) = site
        .request(
            "GET",
            &format!("{path}?target_version=3"),
            "",
            Some(&site.token),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers[header::CACHE_CONTROL], "no-store");
    for text in [
        "Old shared source typing",
        "New shared source typing",
        "Local world typing (kept on adoption)",
        "Shared fire",
        "Shared flying",
        "World flame",
        "No name-based mapping",
    ] {
        assert!(html.contains(text), "missing {text}");
    }
    assert_eq!(
        site.request(
            "POST",
            &path,
            &body("expected_revision=2&target_version=3"),
            Some(&site.token)
        )
        .await
        .0,
        StatusCode::SEE_OTHER
    );
    let typing = site
        .state
        .types
        .variant_typing(&world.id, &variant.id, None)
        .await
        .unwrap();
    assert_eq!(typing.assignments.unwrap()[0].pin.type_id, local.id);
}

#[tokio::test]
async fn typing_security_failures_retain_the_draft_and_never_mutate() {
    let site = site().await;
    let species = site
        .state
        .authoring
        .create_species(&site.token, fields("Creature"), None)
        .await
        .unwrap();
    let entry = site
        .state
        .types
        .create_type(&site.token, fields("Fire"), None)
        .await
        .unwrap();
    let path = format!("/forms/{}/typing", species.default_form_id);
    let data = body(&format!(
        "mode=defined&slots=3&expected_revision=1&slot_2={}@1&actor_id=forged",
        entry.id
    ));
    for (origin, csrf, extra, expected) in [
        (None, "a".repeat(64), "".to_owned(), StatusCode::FORBIDDEN),
        (
            Some("http://127.0.0.1:3000"),
            "b".repeat(64),
            "".to_owned(),
            StatusCode::FORBIDDEN,
        ),
        (
            Some("http://127.0.0.1:3000"),
            "a".repeat(64),
            format!("&padding={}", "x".repeat(32 * 1024)),
            StatusCode::PAYLOAD_TOO_LARGE,
        ),
    ] {
        let (status, headers, html) = site
            .raw(
                "POST",
                &path,
                &format!("{data}{extra}"),
                Some(&site.token),
                origin,
                &csrf,
            )
            .await;
        assert_eq!(status, expected);
        assert_eq!(headers[header::CACHE_CONTROL], "no-store");
        if expected != StatusCode::PAYLOAD_TOO_LARGE {
            assert!(html.contains("Keep reason"));
            assert!(html.contains(&format!("value=\"{}@1\" selected", entry.id)));
        }
        assert_eq!(
            site.state
                .authoring
                .form_detail(&species.default_form_id)
                .await
                .unwrap()
                .revision,
            1
        );
    }
    assert_eq!(
        site.request("POST", &path, &data, None).await.0,
        StatusCode::UNAUTHORIZED
    );
    let (status, _, html) = site
        .request(
            "POST",
            &path,
            &data.replace("slots=3", "slots=33"),
            Some(&site.token),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(html.contains("Keep reason"));
    assert!(html.contains(&format!("value=\"{}@1\" selected", entry.id)));
    assert_eq!(
        site.request("POST", &path, &data, Some(&site.token))
            .await
            .0,
        StatusCode::SEE_OTHER
    );
    assert_eq!(
        site.state
            .authoring
            .history(HistoryKind::Form, &species.default_form_id, None)
            .await
            .unwrap()[0]
            .actor_name,
        "owner"
    );
}

#[tokio::test]
async fn eligible_collaborator_can_author_world_types_and_typing_after_owner_grant() {
    let site = site().await;
    let collaborator = user(&site.state.accounts, "designer", Role::Editor).await;
    let collaborator_id = site
        .state
        .accounts
        .authenticate(&collaborator)
        .await
        .unwrap()
        .unwrap()
        .user_id;
    let world = site
        .state
        .authoring
        .create_world(&site.token, fields("Lab"), None)
        .await
        .unwrap();
    let path = format!("/worlds/{}/types/new", world.id);
    assert_eq!(
        site.request("GET", &path, "", Some(&collaborator)).await.0,
        StatusCode::FORBIDDEN
    );
    site.state
        .authoring
        .grant_collaborator(&site.token, &world.id, 1, &collaborator_id, None)
        .await
        .unwrap();
    assert_eq!(
        site.request("GET", &path, "", Some(&collaborator)).await.0,
        StatusCode::OK
    );
    assert_eq!(
        site.request(
            "POST",
            &path,
            &body("expected_world_revision=1&source_pin="),
            Some(&collaborator)
        )
        .await
        .0,
        StatusCode::SEE_OTHER
    );
    let entry = site
        .state
        .types
        .world_types(&world.id)
        .await
        .unwrap()
        .remove(0);
    let variant = site
        .state
        .authoring
        .create_variant(&site.token, &world.id, 2, None, fields("Variant"), None)
        .await
        .unwrap();
    let path = format!("/worlds/{}/variants/{}/typing", world.id, variant.id);
    assert_eq!(
        site.request(
            "POST",
            &path,
            &body(&format!(
                "expected_revision=1&mode=defined&slots=3&slot_1={}@1",
                entry.id
            )),
            Some(&collaborator)
        )
        .await
        .0,
        StatusCode::SEE_OTHER
    );
    assert_eq!(
        site.state
            .authoring
            .history(HistoryKind::WorldType, &entry.id, None)
            .await
            .unwrap()[0]
            .actor_name,
        "designer"
    );
    site.state
        .authoring
        .revoke_collaborator(&site.token, &world.id, 2, &collaborator_id, None)
        .await
        .unwrap();
    assert_eq!(
        site.request("GET", &path, "", Some(&collaborator)).await.0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        site.request(
            "POST",
            &path,
            &body("expected_revision=2&mode=unmodeled&slots=3"),
            Some(&collaborator)
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
}
