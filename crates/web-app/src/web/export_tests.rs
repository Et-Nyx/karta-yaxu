//! Native export HTTP contracts, using temporary fixture databases and real services.
use super::*;
use crate::{
    application::{
        authoring::{AddressKey, Edit},
        cards::{CardEdit, Cards, NewCard, NewField},
        fixtures::{Fixtures, SeedOptions},
        moves::{AttributeValueInput, MoveEdit},
    },
    auth::{Role, Subject},
    persistence,
};
use axum::body::{Body, to_bytes};
use game_data::{
    authoring::ContentFields,
    card_fields::{CardField, FieldKind, FieldValue},
    cards::CardComponent,
    record_package::RecordPackage as CardPackage,
};
use sqlx::{AssertSqlSafe, SqlitePool};
use std::collections::BTreeMap;
use tower::ServiceExt;

const CLASSIC: &str = "da05ae68560e4534ae229ce6139494ae";
const VARIANT: &str = "5d2d9dba132840f3acbaa5ed1e2b0b49";
const ORIGIN: &str = "http://127.0.0.1:3000";
struct Site {
    app: Router,
    state: AppState,
    pool: SqlitePool,
    token: String,
    owner: String,
    _dir: tempfile::TempDir,
}
fn url(world: &str) -> String {
    format!("/worlds/{world}/export")
}
fn form(intent: &str, digest: &str, confirmation: &str) -> String {
    format!(
        "csrf={}&intent={intent}&reviewed_digest={digest}&confirm={confirmation}",
        "a".repeat(64)
    )
}
fn input(html: &str, name: &str) -> String {
    html.split("<input ")
        .find_map(|tag| {
            let attribute = |key: &str| tag.split_once(&format!("{key}=\""))?.1.split('"').next();
            if attribute("name") == Some(name) {
                attribute("value")
            } else {
                None
            }
        })
        .unwrap_or_else(|| panic!("missing input {name}: {html}"))
        .into()
}
impl Site {
    async fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let pool = persistence::connect_for_migration(&format!(
            "sqlite://{}/export-http.db",
            dir.path().display()
        ))
        .await
        .unwrap();
        persistence::migrate(&pool).await.unwrap();
        let accounts = Accounts::new(pool.clone(), 3600).unwrap();
        Fixtures::new(accounts.clone())
            .seed(SeedOptions {
                dev_auth: true,
                ..Default::default()
            })
            .await
            .unwrap();
        let actor = accounts
            .sign_in(Subject {
                provider: "placeholder".into(),
                provider_subject: "admin".into(),
                display_name: "admin".into(),
            })
            .await
            .unwrap();
        let token = accounts.start_session(&actor.user_id).await.unwrap().0;
        let state = AppState::new(accounts, None, ORIGIN.into(), false);
        Self {
            app: router(state.clone()),
            state,
            pool,
            token,
            owner: actor.user_id,
            _dir: dir,
        }
    }
    async fn user(&self, name: &str, role: Role) -> (String, String) {
        self.state
            .accounts
            .invite("test", name, role, None)
            .await
            .unwrap();
        let actor = self
            .state
            .accounts
            .sign_in(Subject {
                provider: "test".into(),
                provider_subject: name.into(),
                display_name: format!("private-export-http-{name}"),
            })
            .await
            .unwrap();
        let token = self
            .state
            .accounts
            .start_session(&actor.user_id)
            .await
            .unwrap()
            .0;
        (actor.user_id, token)
    }
    async fn request(&self, request: Request) -> (StatusCode, HeaderMap, String) {
        let response = self.app.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let headers = response.headers().clone();
        let text = String::from_utf8(
            to_bytes(response.into_body(), 32 * 1024 * 1024)
                .await
                .unwrap()
                .to_vec(),
        )
        .unwrap();
        (status, headers, text)
    }
    async fn raw(
        &self,
        method: &str,
        path: &str,
        token: &str,
        body: String,
        origin: Option<&str>,
    ) -> (StatusCode, HeaderMap, String) {
        let mut request = Request::builder()
            .method(method)
            .uri(path)
            .header(
                header::COOKIE,
                format!("karta_session={token}; karta_csrf={}", "a".repeat(64)),
            )
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded");
        if let Some(origin) = origin {
            request = request.header(header::ORIGIN, origin);
        }
        self.request(request.body(Body::from(body)).unwrap()).await
    }
    async fn post(
        &self,
        world: &str,
        token: &str,
        body: String,
    ) -> (StatusCode, HeaderMap, String) {
        self.raw("POST", &url(world), token, body, Some(ORIGIN))
            .await
    }
    async fn get(&self, method: &str, world: &str, token: &str) -> (StatusCode, HeaderMap, String) {
        let mut response = self
            .raw(method, &url(world), token, String::new(), None)
            .await;
        for _ in 0..4 {
            if response.0 != StatusCode::TEMPORARY_REDIRECT {
                return response;
            }
            response = self
                .raw(
                    method,
                    response.1[header::LOCATION].to_str().unwrap(),
                    token,
                    String::new(),
                    None,
                )
                .await;
        }
        panic!("export entry redirect loop")
    }
    async fn snapshot(&self) -> BTreeMap<String, Vec<String>> {
        let tables: Vec<String> = sqlx::query_scalar("SELECT name FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name").fetch_all(&self.pool).await.unwrap();
        let mut result = BTreeMap::new();
        for table in tables {
            let columns: Vec<String> =
                sqlx::query_scalar("SELECT name FROM pragma_table_info(?) ORDER BY cid")
                    .bind(&table)
                    .fetch_all(&self.pool)
                    .await
                    .unwrap();
            let projection = columns
                .iter()
                .map(|c| format!("quote(\"{c}\")"))
                .collect::<Vec<_>>()
                .join(",");
            let mut rows: Vec<String> = sqlx::query_scalar(AssertSqlSafe(format!(
                "SELECT json_array({projection}) FROM \"{table}\""
            )))
            .fetch_all(&self.pool)
            .await
            .unwrap();
            rows.sort();
            result.insert(table, rows);
        }
        result
    }
    fn private_free(&self, text: &str) {
        for private in [
            &self.owner,
            &self.token,
            "owner_user_id",
            "access_revision",
            "collaborator_ids",
            "provider_subject",
            "token_hash",
            "before_json",
            "after_json",
            "private-export-http-",
        ] {
            assert!(
                !text.contains(private),
                "private export data leaked: {private}"
            );
        }
    }
    async fn denied(&self, token: &str, status: StatusCode, digest: &str) {
        let before = self.snapshot().await;
        for method in ["GET", "HEAD"] {
            let response = self.get(method, "classiclab", token).await;
            assert_eq!(response.0, status);
            assert_eq!(response.1[header::CACHE_CONTROL], "no-store");
            assert!(!response.1.contains_key(header::CONTENT_DISPOSITION));
            self.private_free(&response.2);
        }
        for body in [form("preview", "", ""), form("download", digest, "yes")] {
            let response = self.post(CLASSIC, token, body).await;
            assert_eq!(response.0, status, "{}", response.2);
            assert_eq!(response.1[header::CACHE_CONTROL], "no-store");
            assert!(!response.1.contains_key(header::CONTENT_DISPOSITION));
            self.private_free(&response.2);
        }
        assert_eq!(self.snapshot().await, before);
    }
}

#[tokio::test]
async fn authorized_get_head_are_entry_only_even_when_both_export_slots_are_held() {
    let site = Site::new().await;
    let before = site.snapshot().await;
    let first = site
        .state
        .export
        .preview(&site.token, AddressKey::Id(CLASSIC.into()))
        .await
        .unwrap();
    let second = site
        .state
        .export
        .preview(&site.token, AddressKey::Id(VARIANT.into()))
        .await
        .unwrap();
    let response = site
        .request(
            Request::builder()
                .uri("/worlds/classiclab/export")
                .header(header::COOKIE, format!("karta_session={}", site.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(response.0, StatusCode::OK);
    assert_eq!(response.1[header::CACHE_CONTROL], "no-store");
    assert!(
        response.1[header::SET_COOKIE]
            .to_str()
            .unwrap()
            .starts_with("karta_csrf=")
    );
    assert!(
        response
            .2
            .contains(&format!("action=\"/worlds/{CLASSIC}/export\""))
    );
    assert!(response.2.contains("Preview package"));
    assert!(!response.2.contains("name=\"reviewed_digest\""));
    assert!(!response.1.contains_key(header::CONTENT_DISPOSITION));
    site.private_free(&response.2);
    let head = site.get("HEAD", "classiclab", &site.token).await;
    assert_eq!(head.0, StatusCode::OK);
    assert_eq!(head.1[header::CACHE_CONTROL], "no-store");
    assert!(head.2.is_empty());
    assert!(!head.1.contains_key(header::CONTENT_DISPOSITION));
    let busy = site
        .post(CLASSIC, &site.token, form("preview", "", ""))
        .await;
    assert_eq!(busy.0, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(busy.1[header::CACHE_CONTROL], "no-store");
    assert!(!busy.1.contains_key(header::CONTENT_DISPOSITION));
    drop((first, second));
    assert_eq!(
        site.post(CLASSIC, &site.token, form("preview", "", ""))
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(site.snapshot().await, before);
}

#[tokio::test]
async fn full_fixture_preview_and_confirmed_attachment_roundtrip_offline_with_no_writes() {
    let site = Site::new().await;
    let before = site.snapshot().await;
    for world in [CLASSIC, VARIANT] {
        let preview = site.post(world, &site.token, form("preview", "", "")).await;
        assert_eq!(preview.0, StatusCode::OK, "{}", preview.2);
        assert_eq!(preview.1[header::CACHE_CONTROL], "no-store");
        assert!(!preview.1.contains_key(header::CONTENT_DISPOSITION));
        for text in [
            "Reviewed package",
            "Separate authoring diagnostics",
            "Contradiction",
            "Incomplete",
            "Confirm and download",
            "Saved preset validation policies are retained",
        ] {
            assert!(preview.2.contains(text), "missing preview text: {text}");
        }
        site.private_free(&preview.2);
        let digest = input(&preview.2, "reviewed_digest");
        assert_eq!(digest.len(), 64);
        let download = site
            .post(world, &site.token, form("download", &digest, "yes"))
            .await;
        assert_eq!(download.0, StatusCode::OK, "{}", download.2);
        assert_eq!(download.1[header::CACHE_CONTROL], "private, no-store");
        assert_eq!(download.1[header::CONTENT_TYPE], "application/json");
        assert_eq!(download.1[header::X_CONTENT_TYPE_OPTIONS], "nosniff");
        assert_eq!(
            download.1[header::CONTENT_DISPOSITION],
            format!(
                "attachment; filename=\"karta-{world}-{}.json\"",
                &digest[..12]
            )
        );
        site.private_free(&download.2);
        let package = CardPackage::from_bytes(download.2.as_bytes()).unwrap();
        assert_eq!(package.content.cards.typed.world.id, world);
        assert_eq!(package.manifest.content_sha256, digest);
        assert_eq!(package.to_bytes().unwrap(), download.2.as_bytes());
        let again = site
            .post(world, &site.token, form("download", &digest, "yes"))
            .await;
        assert_eq!(again.0, StatusCode::OK);
        assert_eq!(
            again.2, download.2,
            "unchanged requests produce byte-identical attachments"
        );
    }
    assert_eq!(site.snapshot().await, before);
}

#[tokio::test]
async fn standalone_card_attachment_preserves_false_and_custom_edit_invalidates_review_then_corruption_blocks_release()
 {
    let site = Site::new().await;
    let cards = Cards::new(site.state.accounts.clone());
    let fields = |name: &str| ContentFields {
        slug: String::new(),
        display_name: name.into(),
        description: "Inert HTTP card export".into(),
    };
    let definition = cards
        .create_field(
            &site.token,
            Some(CLASSIC),
            NewField {
                fields: fields("HTTP exported flag"),
                kind: FieldKind::Boolean,
                source: None,
                reason: None,
            },
        )
        .await
        .unwrap();
    let card = cards
        .create(
            &site.token,
            Some(CLASSIC),
            NewCard {
                fields: fields("HTTP exported standalone"),
                source: None,
                component: CardComponent {
                    fields: Some(vec![CardField {
                        definition: definition.clone(),
                        value: FieldValue::Boolean(false),
                    }]),
                    rules_text: Some("<script>inert</script>".into()),
                    labels: Some(vec![]),
                },
                reason: None,
            },
        )
        .await
        .unwrap();
    let world_revision = site
        .state
        .authoring
        .world_detail(CLASSIC)
        .await
        .unwrap()
        .revision;
    let before = site.snapshot().await;
    let preview = site
        .post(CLASSIC, &site.token, form("preview", "", ""))
        .await;
    assert_eq!(preview.0, StatusCode::OK, "{}", preview.2);
    let digest = input(&preview.2, "reviewed_digest");
    let download = site
        .post(CLASSIC, &site.token, form("download", &digest, "yes"))
        .await;
    assert_eq!(download.0, StatusCode::OK, "{}", download.2);
    assert_eq!(download.1[header::CACHE_CONTROL], "private, no-store");
    assert!(download.1.contains_key(header::CONTENT_DISPOSITION));
    let offline = CardPackage::from_bytes(download.2.as_bytes()).unwrap();
    assert_eq!(offline.manifest.schema_version, 3);
    assert_eq!(offline.manifest.content_sha256, digest);
    assert_eq!(offline.to_bytes().unwrap(), download.2.as_bytes());
    assert!(offline.content.cards.cards.contains(&card));
    assert!(offline.content.cards.card_roots.contains(&card.pin()));
    assert!(offline.content.cards.fields.contains(&definition));
    assert!(
        offline
            .content
            .cards
            .field_roots
            .contains(&definition.pin())
    );
    assert_eq!(
        offline.manifest.root_count,
        offline.content.cards.typed.roots.len()
            + offline.content.cards.card_roots.len()
            + offline.content.cards.field_roots.len()
            + offline.content.layout_roots.len()
            + offline.content.collection_roots.len()
    );
    assert_eq!(
        offline.manifest.record_count,
        offline.content.cards.typed.records.len()
            + offline.content.cards.cards.len()
            + offline.content.cards.fields.len()
            + offline.content.relations.len()
            + offline.content.layouts.len()
            + offline.content.collections.len()
    );
    site.private_free(&download.2);
    assert_eq!(site.snapshot().await, before);

    let mut component = card.component.clone();
    component.fields.as_mut().unwrap()[0].value = FieldValue::Boolean(true);
    let updated = cards
        .save_card(
            &site.token,
            &card.identity,
            CardEdit {
                expected_revision: 1,
                component,
                reason: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(
        site.state
            .authoring
            .world_detail(CLASSIC)
            .await
            .unwrap()
            .revision,
        world_revision
    );
    let before = site.snapshot().await;
    let stale = site
        .post(CLASSIC, &site.token, form("download", &digest, "yes"))
        .await;
    assert_eq!(stale.0, StatusCode::OK, "{}", stale.2);
    assert!(!stale.1.contains_key(header::CONTENT_DISPOSITION));
    assert!(stale.2.contains("Review the fresh preview"));
    let fresh = input(&stale.2, "reviewed_digest");
    assert_ne!(fresh, digest);
    let download = site
        .post(CLASSIC, &site.token, form("download", &fresh, "yes"))
        .await;
    assert_eq!(download.0, StatusCode::OK, "{}", download.2);
    assert!(
        CardPackage::from_bytes(download.2.as_bytes())
            .unwrap()
            .content
            .cards
            .cards
            .contains(&updated)
    );
    site.private_free(&download.2);
    assert_eq!(site.snapshot().await, before);

    // Deliberate temporary-database damage to the selected field component.
    sqlx::query("DROP TRIGGER card_component_fields_immutable_update")
        .execute(&site.pool)
        .await
        .unwrap();
    assert_eq!(sqlx::query("UPDATE card_component_fields SET boolean_value=0 WHERE namespace='standalone' AND scope=? AND card_id=? AND revision=2 AND state='boolean'")
        .bind(CLASSIC).bind(&card.identity.id).execute(&site.pool).await.unwrap().rows_affected(), 1);
    let before = site.snapshot().await;
    for body in [form("preview", "", ""), form("download", &fresh, "yes")] {
        let response = site.post(CLASSIC, &site.token, body).await;
        assert_eq!(
            response.0,
            StatusCode::SERVICE_UNAVAILABLE,
            "{}",
            response.2
        );
        assert_eq!(response.1[header::CACHE_CONTROL], "no-store");
        assert!(!response.1.contains_key(header::CONTENT_DISPOSITION));
        assert!(!response.2.contains("name=\"reviewed_digest\""));
        assert!(response.2.contains("No partial package was released"));
        site.private_free(&response.2);
    }
    assert_eq!(site.snapshot().await, before);
}

#[tokio::test]
async fn strict_forms_origin_csrf_body_limits_and_explicit_confirmation_prevent_accidental_downloads()
 {
    let site = Site::new().await;
    let before = site.snapshot().await;
    let preview = site
        .post(CLASSIC, &site.token, form("preview", "", ""))
        .await;
    assert_eq!(preview.0, StatusCode::OK, "{}", preview.2);
    let digest = input(&preview.2, "reviewed_digest");
    for suffix in [
        "&intent=download",
        "&confirm=yes",
        "&reviewed_digest=abcd",
        "&csrf=other",
        "&actor_id=admin",
        "&max_nodes=99999",
    ] {
        let response = site
            .post(
                CLASSIC,
                &site.token,
                format!("{}{suffix}", form("preview", "", "")),
            )
            .await;
        assert_eq!(
            response.0,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{suffix}: {}",
            response.2
        );
        assert_eq!(response.1[header::CACHE_CONTROL], "no-store");
        assert!(!response.1.contains_key(header::CONTENT_DISPOSITION));
    }
    for body in [
        format!("intent=download&reviewed_digest={digest}&confirm=yes"),
        format!(
            "csrf={}&intent=download&reviewed_digest={digest}&confirm=yes",
            "b".repeat(64)
        ),
    ] {
        assert_eq!(
            site.post(CLASSIC, &site.token, body).await.0,
            StatusCode::FORBIDDEN
        );
    }
    for origin in [Some("https://other.invalid"), None] {
        let response = site
            .raw(
                "POST",
                &url(CLASSIC),
                &site.token,
                form("download", &digest, "yes"),
                origin,
            )
            .await;
        assert_eq!(response.0, StatusCode::FORBIDDEN);
        assert!(!response.1.contains_key(header::CONTENT_DISPOSITION));
    }
    // A CSRF field without the matching cookie also fails even with a session.
    let response = site
        .request(
            Request::builder()
                .method("POST")
                .uri(url(CLASSIC))
                .header(header::COOKIE, format!("karta_session={}", site.token))
                .header(header::ORIGIN, ORIGIN)
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(form("download", &digest, "yes")))
                .unwrap(),
        )
        .await;
    assert_eq!(response.0, StatusCode::FORBIDDEN);
    for confirm in ["", "no"] {
        let response = site
            .post(CLASSIC, &site.token, form("download", &digest, confirm))
            .await;
        assert_eq!(response.0, StatusCode::OK);
        assert!(response.2.contains("Review the fresh preview"));
        assert_eq!(input(&response.2, "reviewed_digest"), digest);
        assert!(!response.1.contains_key(header::CONTENT_DISPOSITION));
    }
    for malformed in [
        String::new(),
        "A".repeat(64),
        "a".repeat(63),
        "g".repeat(64),
    ] {
        let response = site
            .post(CLASSIC, &site.token, form("download", &malformed, "yes"))
            .await;
        assert_eq!(response.0, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(!response.1.contains_key(header::CONTENT_DISPOSITION));
    }
    let oversized = site
        .post(
            CLASSIC,
            &site.token,
            format!("csrf={}&intent=preview", "a".repeat(4096)),
        )
        .await;
    assert_eq!(oversized.0, StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(oversized.1[header::CACHE_CONTROL], "no-store");
    assert!(!oversized.1.contains_key(header::CONTENT_DISPOSITION));
    assert_eq!(site.snapshot().await, before);
}

#[tokio::test]
async fn http_permissions_cover_anonymous_viewer_nonmember_admin_collaborator_revocation_and_disable()
 {
    let site = Site::new().await;
    let (_, viewer) = site.user("viewer", Role::Viewer).await;
    let (_, admin) = site.user("outside-admin", Role::Administrator).await;
    let (editor_id, editor) = site.user("collaborator", Role::Editor).await;
    let digest = site
        .state
        .export
        .preview(&site.token, AddressKey::Id(CLASSIC.into()))
        .await
        .unwrap()
        .package
        .manifest
        .content_sha256
        .clone();
    site.denied("", StatusCode::UNAUTHORIZED, &digest).await;
    site.denied(&viewer, StatusCode::FORBIDDEN, &digest).await;
    site.denied(&admin, StatusCode::FORBIDDEN, &digest).await;
    site.denied(&editor, StatusCode::FORBIDDEN, &digest).await;
    site.state
        .authoring
        .grant_collaborator(&site.token, CLASSIC, 1, &editor_id, None)
        .await
        .unwrap();
    let before = site.snapshot().await;
    assert_eq!(
        site.get("GET", "classiclab", &editor).await.0,
        StatusCode::OK
    );
    let download = site
        .post(CLASSIC, &editor, form("download", &digest, "yes"))
        .await;
    assert_eq!(download.0, StatusCode::OK);
    assert!(download.1.contains_key(header::CONTENT_DISPOSITION));
    assert_eq!(
        CardPackage::from_bytes(download.2.as_bytes())
            .unwrap()
            .manifest
            .content_sha256,
        digest
    );
    assert_eq!(site.snapshot().await, before);
    site.state
        .authoring
        .revoke_collaborator(&site.token, CLASSIC, 2, &editor_id, None)
        .await
        .unwrap();
    site.denied(&editor, StatusCode::FORBIDDEN, &digest).await;
    site.state
        .accounts
        .set_disabled(&site.owner, true)
        .await
        .unwrap();
    site.denied(&site.token, StatusCode::UNAUTHORIZED, &digest)
        .await;
}

#[tokio::test]
async fn stale_preview_after_independent_move_edit_or_world_rename_returns_html_until_reviewed_again()
 {
    let site = Site::new().await;
    let preview = site
        .post(CLASSIC, &site.token, form("preview", "", ""))
        .await;
    assert_eq!(preview.0, StatusCode::OK, "{}", preview.2);
    let mut digest = input(&preview.2, "reviewed_digest");
    for rename_world in [false, true] {
        if rename_world {
            let world = site.state.authoring.world_detail(CLASSIC).await.unwrap();
            site.state
                .authoring
                .save_world(
                    &site.token,
                    CLASSIC,
                    Edit {
                        expected_revision: world.revision,
                        fields: ContentFields {
                            slug: String::new(),
                            display_name: "Renamed <script>package</script>".into(),
                            description: "Public inert text".into(),
                        },
                        reason: None,
                    },
                )
                .await
                .unwrap();
        } else {
            let old = site
                .state
                .moves
                .detail(Some(CLASSIC), "42b00000000040008000000000000001")
                .await
                .unwrap();
            let mut fields = old.fields.clone();
            fields.display_name = "HTTP independent Move edit".into();
            site.state
                .moves
                .save(
                    &site.token,
                    Some(CLASSIC),
                    &old.id,
                    MoveEdit {
                        expected_revision: old.revision,
                        fields,
                        attributes: old.attributes.as_ref().map(|a| {
                            a.iter()
                                .map(|a| AttributeValueInput {
                                    attribute: a.definition.pin(),
                                    value: a.value.clone(),
                                })
                                .collect()
                        }),
                        reason: None,
                    },
                )
                .await
                .unwrap();
        }
        let before = site.snapshot().await;
        let stale = site
            .post(CLASSIC, &site.token, form("download", &digest, "yes"))
            .await;
        assert_eq!(stale.0, StatusCode::OK, "{}", stale.2);
        assert_eq!(stale.1[header::CACHE_CONTROL], "no-store");
        assert!(!stale.1.contains_key(header::CONTENT_DISPOSITION));
        assert!(stale.2.contains("Review the fresh preview"));
        let fresh = input(&stale.2, "reviewed_digest");
        assert_ne!(fresh, digest);
        if rename_world {
            assert!(!stale.2.contains("<script>package</script>"));
            assert!(
                stale
                    .2
                    .contains("Renamed &#60;script&#62;package&#60;/script&#62;")
                    || stale
                        .2
                        .contains("Renamed &lt;script&gt;package&lt;/script&gt;")
            );
        }
        let confirmed = site
            .post(CLASSIC, &site.token, form("download", &fresh, "yes"))
            .await;
        assert_eq!(confirmed.0, StatusCode::OK);
        assert_eq!(confirmed.1[header::CACHE_CONTROL], "private, no-store");
        assert_eq!(
            CardPackage::from_bytes(confirmed.2.as_bytes())
                .unwrap()
                .manifest
                .content_sha256,
            fresh
        );
        assert_eq!(site.snapshot().await, before);
        digest = fresh;
    }
}

#[tokio::test]
async fn corrupt_required_dependency_blocks_preview_and_previously_reviewed_download_without_writes()
 {
    let site = Site::new().await;
    let preview = site
        .post(CLASSIC, &site.token, form("preview", "", ""))
        .await;
    assert_eq!(preview.0, StatusCode::OK, "{}", preview.2);
    let digest = input(&preview.2, "reviewed_digest");
    // Dedicated failure injection into a required source's current projection.
    sqlx::query("UPDATE forms SET description='corrupt HTTP export source' WHERE id='31e00000000040008000000000000001'")
        .execute(&site.pool).await.unwrap();
    let before = site.snapshot().await;
    assert_eq!(
        site.get("GET", "classiclab", &site.token).await.0,
        StatusCode::OK
    );
    for body in [form("preview", "", ""), form("download", &digest, "yes")] {
        let response = site.post(CLASSIC, &site.token, body).await;
        assert_eq!(response.0, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(response.1[header::CACHE_CONTROL], "no-store");
        assert!(!response.1.contains_key(header::CONTENT_DISPOSITION));
        assert!(response.2.contains("No partial package was released"));
        assert!(!response.2.contains("name=\"reviewed_digest\""));
        site.private_free(&response.2);
    }
    assert_eq!(site.snapshot().await, before);
}
