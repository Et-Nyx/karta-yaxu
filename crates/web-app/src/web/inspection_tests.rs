//! Public read-only inspection HTTP contracts against temporary databases.
use super::*;
use crate::{
    application::{
        authoring::Edit,
        fixtures::{Fixtures, SeedOptions},
    },
    auth::Subject,
    persistence,
};
use axum::body::{Body, to_bytes};
use game_data::authoring::ContentFields;
use sqlx::{AssertSqlSafe, SqlitePool};
use std::collections::BTreeMap;
use tower::ServiceExt;

const CLASSIC: &str = "da05ae68560e4534ae229ce6139494ae";
const VARIANT: &str = "5d2d9dba132840f3acbaa5ed1e2b0b49";
const PROBE: &str = "42c00000000040008000000000000001";
struct Site {
    app: Router,
    state: AppState,
    pool: SqlitePool,
    token: String,
    owner: String,
    _dir: tempfile::TempDir,
}
impl Site {
    async fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let pool = persistence::connect_for_migration(&format!(
            "sqlite://{}/inspection-http.db",
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
                display_name: "private-inspection-owner".into(),
            })
            .await
            .unwrap();
        let (token, _) = accounts.start_session(&actor.user_id).await.unwrap();
        let state = AppState::new(accounts, None, "http://127.0.0.1:3000".into(), false);
        Self {
            app: router(state.clone()),
            state,
            pool,
            token,
            owner: actor.user_id,
            _dir: dir,
        }
    }
    async fn req(&self, method: &str, url: &str) -> (StatusCode, HeaderMap, String) {
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
        let body = String::from_utf8(
            to_bytes(response.into_body(), 8 * 1024 * 1024)
                .await
                .unwrap()
                .to_vec(),
        )
        .unwrap();
        (status, headers, body)
    }
    async fn get(&self, url: &str) -> (StatusCode, HeaderMap, String) {
        let mut response = self.req("GET", url).await;
        for _ in 0..4 {
            if response.0 != StatusCode::TEMPORARY_REDIRECT {
                return response;
            }
            response = self
                .req("GET", response.1[header::LOCATION].to_str().unwrap())
                .await;
        }
        panic!("inspection redirect loop")
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
    fn private_free(&self, html: &str) {
        for private in [
            &self.owner,
            &self.token,
            "private-inspection-owner",
            "owner_user_id",
            "access_revision",
            "provider_subject",
            "collaborator_ids",
            "after_json",
            "before_json",
        ] {
            assert!(
                !html.contains(private),
                "Private account/session or storage data leaked into inspection HTML"
            );
        }
        assert!(!html.contains("method=\"post\""));
        assert!(!html.contains("name=\"csrf\""));
    }
}
fn fingerprint(html: &str) -> String {
    html.split("name=\"fingerprint\" value=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap()
        .into()
}
fn usage_url(version: &str, view: &str) -> String {
    format!(
        "/worlds/variantlab/inspection/where-used?kind=move&scope=local&id={PROBE}&version={version}&view={view}"
    )
}

#[tokio::test]
async fn large_validator_evidence_is_presented_in_bounded_previews_without_losing_inventory() {
    use crate::application::moves::{
        AttributeValueInput, ExpectationInput, NewAttribute, NewMove, RulesetEdit,
    };
    use game_data::{moves::NumericValue, rulesets::RepeatPolicy, stats::NumericConstraints};
    let site = Site::new().await;
    let mut attributes = Vec::new();
    let mut expectations = Vec::new();
    for i in 0..32 {
        let d = site
            .state
            .moves
            .create_attribute(
                &site.token,
                Some(CLASSIC),
                NewAttribute {
                    fields: ContentFields {
                        slug: String::new(),
                        display_name: format!("Inspection evidence {i:02}"),
                        description: "An exact numeric input".into(),
                    },
                    unit: String::new(),
                    constraints: NumericConstraints::default(),
                    source: None,
                    reason: None,
                },
            )
            .await
            .unwrap();
        attributes.push(AttributeValueInput {
            attribute: d.pin(),
            value: NumericValue::Unknown,
        });
        expectations.push(ExpectationInput {
            attribute: d.pin(),
            required: true,
            allow_not_applicable: false,
            constraints: NumericConstraints::default(),
        });
    }
    site.state
        .moves
        .create(
            &site.token,
            Some(CLASSIC),
            NewMove {
                fields: ContentFields {
                    slug: String::new(),
                    display_name: "Evidence-heavy Move".into(),
                    description: "Many unknown inputs remain independently diagnosed".into(),
                },
                source: None,
                attributes: Some(attributes),
                reason: None,
            },
        )
        .await
        .unwrap();
    site.state
        .moves
        .save_ruleset(
            &site.token,
            CLASSIC,
            RulesetEdit {
                expected_revision: 1,
                fields: ContentFields {
                    slug: String::new(),
                    display_name: "Evidence-heavy configuration".into(),
                    description: "Every declared numeric input is required".into(),
                },
                expectations: Some(expectations),
                stat_roster: None,
                chart: None,
                max_moves: None,
                repeat_policy: RepeatPolicy::Unspecified,
                notes: String::new(),
                reason: None,
            },
        )
        .await
        .unwrap();
    let before = site.snapshot().await;
    let response = site
        .get("/worlds/classiclab/inspection?category=incomplete&kind=move")
        .await;
    assert_eq!(response.0, StatusCode::OK);
    assert!(response.2.contains("Coverage: Complete"));
    assert!(response.2.contains("Showing 25 of"));
    assert!(response.2.contains("validation used all inputs"));
    for evidence in response.2.split("<h4>Exact evidence</h4>").skip(1) {
        let evidence = evidence.split("</article>").next().unwrap();
        assert!(evidence.matches("class=\"record-meta\"").count() <= 25);
    }
    let report = site
        .state
        .inspection
        .inspect(
            crate::application::authoring::AddressKey::Id(CLASSIC.into()),
            false,
        )
        .await
        .unwrap();
    assert!(report.findings.iter().any(|f| f.evidence.len() > 25));
    assert_eq!(site.snapshot().await, before);
}

#[tokio::test]
async fn public_get_head_readable_redirect_filters_pagination_and_every_row_unchanged() {
    let site = Site::new().await;
    let before = site.snapshot().await;
    let legacy = format!("/worlds/{CLASSIC}/inspection?category=all&kind=all&page=1&compare=no");
    let redirect = site.req("GET", &legacy).await;
    assert_eq!(redirect.0, StatusCode::TEMPORARY_REDIRECT);
    assert_eq!(
        redirect.1[header::LOCATION],
        "/worlds/classiclab/inspection?category=all&kind=all&page=1&compare=no"
    );
    let (status, headers, html) = site.get(&legacy).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers[header::CACHE_CONTROL], "no-store");
    assert!(!headers.contains_key(header::SET_COOKIE));
    for text in [
        "Coverage: Complete",
        "Inspection contract version 1",
        "Current root inventory",
        "Exact dependency inventory",
        "not combat or export readiness",
        "World anchor",
        "Next page",
    ] {
        assert!(html.contains(text), "missing {text}");
    }
    site.private_free(&html);
    let fp = fingerprint(&html);
    assert_eq!(fp.len(), 64);
    let head = site.req("HEAD", "/worlds/classiclab/inspection").await;
    assert_eq!(head.0, StatusCode::OK);
    assert!(head.2.is_empty());
    assert_eq!(head.1[header::CACHE_CONTROL], "no-store");
    let second = site
        .get(&format!(
            "/worlds/classiclab/inspection?page=2&fingerprint={fp}"
        ))
        .await;
    assert_eq!(second.0, StatusCode::OK);
    assert!(second.2.contains("Page 2"));
    assert_eq!(fingerprint(&second.2), fp);
    let filtered = site
        .get("/worlds/classiclab/inspection?category=contradiction&kind=preset")
        .await;
    assert_eq!(filtered.0, StatusCode::OK);
    assert!(filtered.2.contains("preset.selection_repeat"));
    assert!(filtered.2.contains("Typed site: Selected acquisition"));
    assert_eq!(site.snapshot().await, before);
}

#[tokio::test]
async fn exact_and_all_usage_preserve_acquisition_sites_repeated_slots_and_paths() {
    let site = Site::new().await;
    let before = site.snapshot().await;
    for version in ["all", "1"] {
        let (status, headers, html) = site.get(&usage_url(version, "direct")).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(headers[header::CACHE_CONTROL], "no-store");
        for text in [
            "acquisition slot 1",
            "acquisition slot 2",
            "preset selection slot 1",
            "preset selection slot 2",
            "preset selection slot 3",
            "Matching captured revisions",
            "Exact history",
            "/learnset/history/1",
            "/movesets/46a00000000040008000000000000004/history/1",
        ] {
            assert!(html.contains(text), "missing {text}");
        }
        site.private_free(&html);
    }
    let paths = site.get(&usage_url("all", "paths")).await;
    assert_eq!(paths.0, StatusCode::OK);
    assert!(
        paths
            .2
            .contains("one canonical shortest representative per root")
    );
    assert!(paths.2.contains("Observed world membership"));
    let missing = site.get(&usage_url("99", "direct")).await;
    assert!(missing.2.contains("not captured"));
    assert!(!missing.2.contains("Unused in this world capture:"));
    let head = site.req("HEAD", &usage_url("1", "direct")).await;
    assert_eq!(head.0, StatusCode::OK);
    assert!(head.2.is_empty());
    assert_eq!(site.snapshot().await, before);
}

#[tokio::test]
async fn query_allowlists_reject_invalid_foreign_and_client_budget_inputs_without_writes() {
    let site = Site::new().await;
    let before = site.snapshot().await;
    for query in [
        "category=pass",
        "category=unknown",
        "kind=move_attribute",
        "kind=nope",
        "page=0",
        "page=-1",
        "page=10001",
        "page=184467440737095516160",
        "compare=true",
        "fingerprint=bad",
        "max_nodes=999999",
        "category=all&category=incomplete",
        "scope=shared",
    ] {
        let response = site
            .get(&format!("/worlds/classiclab/inspection?{query}"))
            .await;
        assert_eq!(response.0, StatusCode::UNPROCESSABLE_ENTITY, "{query}");
        assert_eq!(response.1[header::CACHE_CONTROL], "no-store");
    }
    for query in [
        format!("kind=move&scope={CLASSIC}&id={PROBE}"),
        format!("kind=move&scope=local&id={PROBE}&version=0"),
        format!("kind=move&scope=local&id={PROBE}&view=every-path"),
        format!("kind=world&scope=shared&id={CLASSIC}"),
        format!("kind=move&scope=shared&id={}&version=1", "A".repeat(32)),
        format!("kind=move&scope=local&id={PROBE}&world_id={CLASSIC}"),
        format!("kind=move&scope=local&id={PROBE}&max_results=999999"),
    ] {
        assert_eq!(
            site.get(&format!("/worlds/variantlab/inspection/where-used?{query}"))
                .await
                .0,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{query}"
        );
    }
    for method in ["POST", "PUT", "PATCH", "DELETE"] {
        for suffix in ["inspection", "inspection/where-used"] {
            assert_eq!(
                site.req(method, &format!("/worlds/{CLASSIC}/{suffix}"))
                    .await
                    .0,
                StatusCode::METHOD_NOT_ALLOWED
            );
        }
    }
    assert_eq!(site.snapshot().await, before);
}

#[tokio::test]
async fn saved_policy_one_and_optional_current_comparison_do_not_revalidate_or_write() {
    let site = Site::new().await;
    let before = site.snapshot().await;
    let saved = site
        .get("/worlds/variantlab/inspection?kind=preset&compare=no")
        .await;
    assert_eq!(saved.0, StatusCode::OK);
    assert!(saved.2.contains("Saved validator policy 1 (retained)"));
    assert!(saved.2.contains("validator.preset.saved"));
    assert!(!saved.2.contains("validator.preset.comparison"));
    let compare = site
        .get("/worlds/variantlab/inspection?kind=preset&compare=yes")
        .await;
    assert_eq!(compare.0, StatusCode::OK);
    assert!(compare.2.contains("validator.preset.comparison"));
    assert!(compare.2.contains("Validator policy 2"));
    assert!(
        compare
            .2
            .contains("Ad-hoc current-policy comparison; no attributed save")
    );
    assert_eq!(fingerprint(&saved.2), fingerprint(&compare.2));
    assert_eq!(site.snapshot().await, before);
}

#[tokio::test]
async fn parent_rename_refreshes_fingerprint_resets_page_and_escapes_public_fields() {
    let site = Site::new().await;
    let first = site.get("/worlds/variantlab/inspection").await;
    let fp = fingerprint(&first.2);
    let world = site.state.authoring.world_detail(VARIANT).await.unwrap();
    site.state
        .authoring
        .save_world(
            &site.token,
            VARIANT,
            Edit {
                expected_revision: world.revision,
                fields: ContentFields {
                    slug: String::new(),
                    display_name: "Renamed <script>world</script>".into(),
                    description: "Public <img src=x onerror=alert(1)> description".into(),
                },
                reason: None,
            },
        )
        .await
        .unwrap();
    let before = site.snapshot().await;
    let refreshed = site
        .get(&format!(
            "/worlds/{VARIANT}/inspection?fingerprint={fp}&page=2"
        ))
        .await;
    assert_eq!(refreshed.0, StatusCode::OK);
    assert!(refreshed.2.contains("World changed"));
    assert!(refreshed.2.contains("Pagination reset to page 1"));
    assert!(refreshed.2.contains("Page 1"));
    assert!(
        refreshed
            .2
            .contains("Renamed &#60;script&#62;world&#60;/script&#62;")
            || refreshed
                .2
                .contains("Renamed &lt;script&gt;world&lt;/script&gt;")
    );
    assert!(!refreshed.2.contains("<script>world</script>"));
    assert!(!refreshed.2.contains("<img src=x"));
    assert_ne!(fingerprint(&refreshed.2), fp);
    site.private_free(&refreshed.2);
    assert_eq!(site.snapshot().await, before);
}

#[tokio::test]
async fn unconfigured_inventory_is_explicit_and_world_anchor_scope_is_not_shared_content() {
    let site = Site::new().await;
    let world = site
        .state
        .authoring
        .create_world(
            &site.token,
            ContentFields {
                slug: String::new(),
                display_name: "Empty inspection".into(),
                description: "No mandatory contexts invented".into(),
            },
            None,
        )
        .await
        .unwrap();
    let before = site.snapshot().await;
    let report = site.get("/worlds/empty-inspection/inspection").await;
    assert_eq!(report.0, StatusCode::OK);
    for text in [
        "Coverage: Complete",
        "chart: Unconfigured",
        "stat-roster: Unconfigured",
        "ruleset: Unconfigured",
        "World anchor",
    ] {
        assert!(report.2.contains(text), "{text}");
    }
    let usage = site
        .get(&format!(
            "/worlds/empty-inspection/inspection/where-used?kind=world&scope=shared&id={}",
            world.id
        ))
        .await;
    assert_eq!(usage.0, StatusCode::OK);
    assert!(usage.2.contains("World anchor"));
    assert_eq!(
        site.get("/worlds/missing-inspection/inspection").await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(site.snapshot().await, before);
}

#[tokio::test]
async fn corrupt_and_unsupported_children_show_partial_gaps_and_cannot_confirm_prior_fingerprint() {
    for unsupported in [false, true] {
        let site = Site::new().await;
        let first = site.get("/worlds/classiclab/inspection").await;
        let fp = fingerprint(&first.2);
        // Explicit failure injection into a temporary, test-owned database only.
        if unsupported {
            sqlx::query("DROP TRIGGER variant_stat_update")
                .execute(&site.pool)
                .await
                .unwrap();
            let mut connection = site.pool.acquire().await.unwrap();
            sqlx::query("PRAGMA ignore_check_constraints=ON")
                .execute(&mut *connection)
                .await
                .unwrap();
            sqlx::query("UPDATE variant_stat_snapshots SET after_json=json_set(after_json,'$.snapshot_version',99) WHERE variant_id='31f00000000040008000000000000001'").execute(&mut *connection).await.unwrap();
            sqlx::query("PRAGMA ignore_check_constraints=OFF")
                .execute(&mut *connection)
                .await
                .unwrap();
        } else {
            sqlx::query("UPDATE forms SET description='inconsistent projection' WHERE id='31e00000000040008000000000000001'").execute(&site.pool).await.unwrap();
        }
        let before = site.snapshot().await;
        let response = site
            .get(&format!(
                "/worlds/classiclab/inspection?category=not_evaluated&fingerprint={fp}&page=2"
            ))
            .await;
        assert_eq!(response.0, StatusCode::OK);
        for text in [
            "Coverage: Partial",
            "Integrity coverage gap",
            "previous fingerprint cannot be confirmed",
            "No complete fingerprint",
            "Page 1",
            "No incoming content uses observed in the inspected portion",
        ] {
            assert!(response.2.contains(text), "missing {text}");
        }
        assert!(!response.2.contains("name=\"fingerprint\""));
        assert!(!response.2.contains("usage.unused_in_capture"));
        site.private_free(&response.2);
        let used = site.get(&format!("/worlds/classiclab/inspection/where-used?kind=move&scope=local&id=42b00000000040008000000000000001&version=1&fingerprint={fp}")).await;
        assert_eq!(used.0, StatusCode::OK);
        assert!(used.2.contains("Usage coverage: Partial"));
        assert_eq!(site.snapshot().await, before);
    }
}

#[tokio::test]
async fn unusable_world_anchor_is_unavailable_without_private_data_or_writes() {
    let site = Site::new().await;
    sqlx::query("DROP TRIGGER world_content_update")
        .execute(&site.pool)
        .await
        .unwrap();
    sqlx::query("UPDATE world_content_snapshots SET after_json=json_set(after_json,'$.content.fields.description','bad anchor projection') WHERE world_id=? AND revision=2").bind(CLASSIC).execute(&site.pool).await.unwrap();
    let before = site.snapshot().await;
    for suffix in [
        "inspection".to_owned(),
        "inspection/where-used?kind=move&scope=local&id=42b00000000040008000000000000001".into(),
    ] {
        let response = site.get(&format!("/worlds/classiclab/{suffix}")).await;
        assert_eq!(response.0, StatusCode::SERVICE_UNAVAILABLE);
        site.private_free(&response.2);
        assert!(!response.2.contains("Coverage: Complete"));
    }
    assert_eq!(site.snapshot().await, before);
}
