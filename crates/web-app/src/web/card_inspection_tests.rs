//! Isolated public router contracts; no address-adapter or session dependency.
use super::*;
use crate::{
    application::{
        cards::{CardEdit, NewCard},
        fixtures::{Fixtures, SeedOptions},
    },
    auth::Subject,
    persistence,
};
use axum::body::{Body, to_bytes};
use game_data::{
    authoring::ContentFields,
    cards::{CardComponent, CardRecord},
    inspection::{CaptureGap, CaptureGapKind},
};
use sqlx::SqlitePool;
use tower::ServiceExt;

const WORLD: &str = "da05ae68560e4534ae229ce6139494ae";
struct TestSite {
    app: Router,
    state: AppState,
    pool: SqlitePool,
    token: String,
    owner: String,
    _dir: tempfile::TempDir,
}
impl TestSite {
    async fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let pool = persistence::connect_for_migration(&format!(
            "sqlite://{}/card-inspection.db",
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
                display_name: "private-card-inspection-owner".into(),
            })
            .await
            .unwrap();
        let (token, _) = accounts.start_session(&actor.user_id).await.unwrap();
        let state = AppState::new(accounts, None, "http://127.0.0.1:3000".into(), false);
        Self {
            app: routes().with_state(state.clone()),
            state,
            pool,
            token,
            owner: actor.user_id,
            _dir: dir,
        }
    }
    async fn request(&self, method: &str, url: &str) -> (StatusCode, HeaderMap, String) {
        let response = self
            .app
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(url)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let headers = response.headers().clone();
        let text = String::from_utf8(
            to_bytes(response.into_body(), 8 * 1024 * 1024)
                .await
                .unwrap()
                .to_vec(),
        )
        .unwrap();
        (status, headers, text)
    }
    async fn card(&self, name: &str) -> CardRecord {
        self.state
            .cards
            .create(
                &self.token,
                Some(WORLD),
                NewCard {
                    fields: ContentFields {
                        slug: String::new(),
                        display_name: name.into(),
                        description: "Public description".into(),
                    },
                    source: None,
                    component: CardComponent {
                        fields: Some(vec![]),
                        rules_text: Some("Inert <script>never executable</script>".into()),
                        labels: Some(vec!["Public label".into()]),
                    },
                    reason: None,
                },
            )
            .await
            .unwrap()
    }
    async fn audits(&self) -> i64 {
        sqlx::query_scalar("SELECT count(*) FROM audit_events")
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }
    fn private_free(&self, html: &str) {
        for s in [
            &self.owner,
            &self.token,
            "private-card-inspection-owner",
            "owner_user_id",
            "access_revision",
            "provider_subject",
            "collaborator_ids",
            "before_json",
            "after_json",
            "name=\"csrf\"",
            "method=\"post\"",
        ] {
            assert!(
                !html.contains(s),
                "Private data or mutation control in public HTML"
            );
        }
    }
}
fn url() -> String {
    format!("/worlds/{WORLD}/inspection/cards")
}
fn used(r: &CardRecord) -> String {
    format!(
        "{}/where-used?target=card&namespace=standalone&scope=local&id={}&version=all&view=direct",
        url(),
        r.identity.id
    )
}

#[tokio::test]
async fn public_get_head_are_private_free_no_cookie_read_only_and_escaped() {
    let site = TestSite::new().await;
    let r = site.card("Public <card>").await;
    let before = site.audits().await;
    for path in [url(), used(&r)] {
        let (status, headers, html) = site.request("GET", &path).await;
        assert_eq!(status, StatusCode::OK);
        assert!(
            headers[header::CACHE_CONTROL]
                .to_str()
                .unwrap()
                .contains("no-store")
        );
        assert!(!headers.contains_key(header::SET_COOKIE));
        site.private_free(&html);
        assert!(html.contains("Public &#60;card&#62;") || html.contains("Public &lt;card&gt;"));
        assert!(!html.contains("<script>never executable</script>"));
        let (status, headers, body) = site.request("HEAD", &path).await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.is_empty());
        assert!(!headers.contains_key(header::SET_COOKIE));
        assert_eq!(
            site.request("POST", &path).await.0,
            StatusCode::METHOD_NOT_ALLOWED
        );
    }
    assert_eq!(site.audits().await, before);
}

#[tokio::test]
async fn query_allowlist_rejects_unknown_duplicate_and_invalid_values() {
    let site = TestSite::new().await;
    for query in [
        "cookie=please",
        "page=1&page=2",
        "page=0",
        "page=10001",
        "page=%2B1",
        "fingerprint=abc",
        "target=species",
        "namespace=item",
        "scope=foreign",
        "id=123",
        "version=0",
        "version=-1",
        "view=everything",
    ] {
        let (status, headers, html) = site.request("GET", &format!("{}?{query}", url())).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{query}");
        assert!(!headers.contains_key(header::SET_COOKIE));
        site.private_free(&html);
    }
    assert_eq!(
        site.request("GET", &format!("{}/where-used", url()))
            .await
            .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
}

#[tokio::test]
async fn fresh_capture_changes_fingerprint_and_resets_both_inventories_and_usage() {
    let site = TestSite::new().await;
    let r = site.card("Fresh card").await;
    let old = site
        .state
        .inspection
        .inspect(AddressKey::Id(WORLD.into()), false)
        .await
        .unwrap()
        .fingerprint
        .unwrap();
    site.state
        .cards
        .save_card(
            &site.token,
            &r.identity,
            CardEdit {
                expected_revision: 1,
                component: CardComponent {
                    rules_text: Some("New inert rules".into()),
                    ..r.component.clone()
                },
                reason: None,
            },
        )
        .await
        .unwrap();
    let before = site.audits().await;
    for path in [url(), used(&r)] {
        let (status, _, html) = site
            .request(
                "GET",
                &format!(
                    "{path}{}page=2&fingerprint={old}",
                    if path.contains('?') { "&" } else { "?" }
                ),
            )
            .await;
        assert_eq!(status, StatusCode::OK);
        assert!(html.contains("World changed"));
        assert!(html.contains("Page 1"));
        assert!(html.contains("Fresh card"));
        assert!(!html.contains(&format!("<code>{old}</code>")));
        site.private_free(&html);
    }
    assert_eq!(site.audits().await, before);
}

#[tokio::test]
async fn partial_refresh_resets_page_and_never_claims_unused_or_a_fingerprint() {
    let site = TestSite::new().await;
    let r = site.card("Partial card").await;
    let mut result = site
        .state
        .inspection
        .inspect(AddressKey::Id(WORLD.into()), false)
        .await
        .unwrap();
    let old = result.fingerprint.clone().unwrap();
    result.capture.gaps.push(CaptureGap {
        kind: CaptureGapKind::Budget,
        subject: None,
        message: "Test-owned incomplete capture".into(),
    });
    result.fingerprint = None;
    let p = parse(
        RawQuery {
            target: Some("card".into()),
            namespace: Some("standalone".into()),
            scope: Some("local".into()),
            id: Some(r.identity.id),
            page: Some("2".into()),
            fingerprint: Some(old),
            ..Default::default()
        },
        true,
    )
    .unwrap();
    let found = analysis::where_used(
        &result.capture,
        &UsageQuery {
            target: target(&p, WORLD),
            version: None,
            paths: false,
        },
    );
    let html = build(&result, &p, Some(&found)).render().unwrap();
    assert!(html.contains("Coverage is now Partial"));
    assert!(html.contains("Page 1"));
    assert!(html.contains("No observed uses in the inspected portion"));
    assert!(!html.contains("Unused in this world capture"));
    assert!(!html.contains("Capture fingerprint:"));
    site.private_free(&html);
}

#[tokio::test]
async fn component_previews_are_bounded_and_not_duplicated_in_hidden_controls() {
    use game_data::card_fields::{CardField, FieldDefinition, FieldKind, FieldValue};
    let site = TestSite::new().await;
    let r = site.card("Bounded card").await;
    let mut result = site
        .state
        .inspection
        .inspect(AddressKey::Id(WORLD.into()), false)
        .await
        .unwrap();
    let record = result.capture.card_data.cards.get_mut(&r.pin()).unwrap();
    record.component.fields = Some(
        (1..=26)
            .map(|n| CardField {
                definition: FieldDefinition {
                    id: format!("{n:032x}"),
                    world_id: Some(WORLD.into()),
                    source: None,
                    fields: ContentFields {
                        slug: format!("field-{n}"),
                        display_name: format!("Field {n}"),
                        description: String::new(),
                    },
                    revision: 1,
                    archived_at: None,
                    kind: FieldKind::Text { max_length: 2000 },
                },
                value: FieldValue::Text(format!("distinct-value-{n:02}")),
            })
            .collect(),
    );
    record.component.rules_text = Some(format!("{}TEXT_OUTSIDE_PREVIEW", "x".repeat(2000)));
    record.component.labels = Some((1..=26).map(|n| format!("distinct-label-{n:02}")).collect());
    let p = parse(
        RawQuery {
            target: Some("card".into()),
            id: Some(r.identity.id),
            ..Default::default()
        },
        false,
    )
    .unwrap();
    let html = build(&result, &p, None).render().unwrap();
    assert!(html.contains("distinct-value-25"));
    assert!(!html.contains("distinct-value-26"));
    assert!(html.contains("distinct-label-25"));
    assert!(!html.contains("distinct-label-26"));
    assert!(html.contains("Showing 25 of 26 assigned fields/values"));
    assert!(html.contains("preview truncated"));
    assert!(!html.contains("TEXT_OUTSIDE_PREVIEW"));
    assert!(!html.contains("type=\"hidden\""));
    site.private_free(&html);
}
