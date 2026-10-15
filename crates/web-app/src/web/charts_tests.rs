//! HTTP regressions use test-owned databases; no fixture or developer DB access.
use super::*;
use crate::{
    application::{authoring::Edit, charts::*},
    auth::{Role, Subject},
    persistence,
};
use axum::body::{Body, to_bytes};
use game_data::authoring::ContentFields;
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
        description: "Inert <script>chart</script>".into(),
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
async fn site() -> Site {
    let directory = tempfile::tempdir().unwrap();
    let pool = persistence::connect_for_migration(&format!(
        "sqlite://{}/charts-http.db",
        directory.path().display()
    ))
    .await
    .unwrap();
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
fn body(extra: &str) -> String {
    format!("csrf={}&reason=Keep+this+reason&{extra}", "a".repeat(64))
}
impl Site {
    async fn raw(
        &self,
        method: &str,
        path: &str,
        data: &str,
        token: Option<&str>,
        origin: Option<&str>,
    ) -> (StatusCode, HeaderMap, String) {
        let cookie = format!(
            "karta_csrf={}; karta_session={}",
            "a".repeat(64),
            token.unwrap_or("")
        );
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
        let html = String::from_utf8(
            to_bytes(response.into_body(), 2 * 1024 * 1024)
                .await
                .unwrap()
                .to_vec(),
        )
        .unwrap();
        (status, headers, html)
    }
    async fn request(
        &self,
        method: &str,
        path: &str,
        data: &str,
        token: Option<&str>,
    ) -> (StatusCode, HeaderMap, String) {
        let mut path = path.to_owned();
        for _ in 0..6 {
            let response = self
                .raw(method, &path, data, token, Some("http://127.0.0.1:3000"))
                .await;
            if method != "GET"
                || !matches!(
                    response.0,
                    StatusCode::TEMPORARY_REDIRECT | StatusCode::SEE_OTHER
                )
            {
                return response;
            }
            path = response.1[header::LOCATION].to_str().unwrap().into();
        }
        panic!("chart redirect loop")
    }
    async fn chart(&self, name: &str) -> ChartRecord {
        let ty = self
            .state
            .types
            .create_type(&self.token, fields("Mist"), None)
            .await
            .unwrap();
        self.state
            .charts
            .create(
                &self.token,
                None,
                NewChart {
                    fields: fields(name),
                    roster: vec![TypePin {
                        type_id: ty.id,
                        version: 1,
                    }],
                    source: None,
                    mapping: vec![],
                    apply_source: false,
                    reason: None,
                },
            )
            .await
            .unwrap()
    }
}

#[tokio::test]
async fn creation_preview_is_readonly_and_mutations_require_origin_csrf_role_and_body_bound() {
    let site = site().await;
    let draft = body("display_name=Draft+chart&description=Retained+description&slots=0");
    for (origin, data, expected) in [
        (None, draft.clone(), StatusCode::FORBIDDEN),
        (
            Some("https://elsewhere.example"),
            draft.clone(),
            StatusCode::FORBIDDEN,
        ),
        (
            Some("http://127.0.0.1:3000"),
            draft.replace(&"a".repeat(64), &"b".repeat(64)),
            StatusCode::FORBIDDEN,
        ),
        (
            Some("http://127.0.0.1:3000"),
            format!("{draft}&padding={}", "x".repeat(32 * 1024)),
            StatusCode::PAYLOAD_TOO_LARGE,
        ),
    ] {
        let (status, headers, _) = site
            .raw("POST", "/charts/new", &data, Some(&site.token), origin)
            .await;
        assert_eq!(status, expected);
        assert_eq!(headers[header::CACHE_CONTROL], "no-store");
    }
    let editor = user(&site.state.accounts, "editor", Role::Editor).await;
    let denied = site
        .request("POST", "/charts/new", &draft, Some(&editor))
        .await;
    assert_eq!(denied.0, StatusCode::FORBIDDEN);
    assert!(denied.2.contains("value=\"Draft chart\""));
    assert!(denied.2.contains("Keep this reason"));
    let preview = site
        .request("POST", "/charts/new", &draft, Some(&site.token))
        .await;
    assert_eq!(preview.0, StatusCode::OK);
    assert!(preview.2.contains("Proposed result"));
    assert!(preview.2.contains("Create reviewed chart"));
    assert!(
        preview
            .2
            .contains("Creation preview — no chart has been saved")
    );
    assert!(!preview.2.contains("Canonical chart ID"));
    assert!(site.state.charts.charts().await.unwrap().is_empty());
    let saved = site
        .request(
            "POST",
            "/charts/new",
            &format!("{draft}&intent=create&confirm=yes&actor_id=forged"),
            Some(&site.token),
        )
        .await;
    assert_eq!(saved.0, StatusCode::SEE_OTHER);
    let chart = site.state.charts.charts().await.unwrap().remove(0);
    assert_eq!(
        site.state
            .charts
            .history(None, &chart.id, None)
            .await
            .unwrap()[0]
            .actor_name,
        "owner"
    );
    assert_eq!(
        site.request("GET", "/charts", "", None).await.0,
        StatusCode::OK
    );
}

#[tokio::test]
async fn fraction_drafts_keep_both_malformed_fields_and_stale_revision_precedes_cell_validation() {
    let site = site().await;
    let chart = site.chart("Fraction lab").await;
    let ty = &chart.roster[0].pin.type_id;
    let path = format!("/charts/{}/rows/{ty}", chart.id);
    let bad = body(&format!(
        "expected_revision=1&numerator_{ty}=one%2Ftwo&denominator_{ty}=zero"
    ));
    let response = site.request("POST", &path, &bad, Some(&site.token)).await;
    assert_eq!(response.0, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        response.2.contains("value=\"one&#x2f;two\"") || response.2.contains("value=\"one/two\"")
    );
    assert!(response.2.contains("value=\"zero\""));
    assert!(response.2.contains(&format!("chart-numerator_{ty}-error")));
    assert!(
        response
            .2
            .contains(&format!("chart-denominator_{ty}-error"))
    );
    assert_eq!(
        site.state
            .charts
            .detail(None, &chart.id)
            .await
            .unwrap()
            .revision,
        1
    );
    let good = body(&format!(
        "expected_revision=1&numerator_{ty}=2&denominator_{ty}=4"
    ));
    assert_eq!(
        site.request("POST", &path, &good, Some(&site.token))
            .await
            .0,
        StatusCode::SEE_OTHER
    );
    let conflict = site.request("POST", &path, &bad, Some(&site.token)).await;
    assert_eq!(conflict.0, StatusCode::CONFLICT);
    assert!(conflict.2.contains("Current saved state"));
    assert!(conflict.2.contains("Save reviewed changes"));
    assert!(
        conflict
            .2
            .contains("name=\"reviewed_revision\" value=\"2\"")
    );
    assert!(conflict.2.contains("one"));
    let current = site.state.charts.detail(None, &chart.id).await.unwrap();
    assert_eq!(current.cells[0].factor, Factor::new(1, 2).unwrap());
    // Only explicit reviewed resubmission advances the base; malformed strings stay.
    let reviewed = site
        .request(
            "POST",
            &path,
            &format!("{bad}&reviewed_revision=2"),
            Some(&site.token),
        )
        .await;
    assert_eq!(reviewed.0, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        reviewed
            .2
            .contains("name=\"expected_revision\" value=\"2\"")
    );
}

#[tokio::test]
async fn blank_unknown_zero_neutral_noops_and_public_history_remain_distinct() {
    let site = site().await;
    let chart = site.chart("Known and unknown").await;
    let ty = &chart.roster[0].pin.type_id;
    let root = format!("/charts/{}", chart.id);
    let row = format!("{root}/rows/{ty}");
    let initial = site.request("GET", &root, "", None).await;
    assert_eq!(initial.0, StatusCode::OK);
    assert!(initial.2.contains("<th scope=\"row\">Mist</th>"));
    assert!(initial.2.contains("<th scope=\"col\">Mist</th>"));
    assert!(initial.2.contains("<td>Unknown</td>"));
    assert!(!initial.2.contains("<script>chart</script>"));
    for (revision, n, d) in [(1, "0", "99"), (2, "", "")] {
        let data = body(&format!(
            "expected_revision={revision}&numerator_{ty}={n}&denominator_{ty}={d}"
        ));
        assert_eq!(
            site.request("POST", &row, &data, Some(&site.token)).await.0,
            StatusCode::SEE_OTHER
        );
    }
    assert_eq!(
        site.state
            .charts
            .detail(None, &chart.id)
            .await
            .unwrap()
            .lookup(ty, ty),
        Effectiveness::Unknown
    );
    let neutral = format!("{root}/neutral");
    assert_eq!(
        site.request(
            "POST",
            &neutral,
            &body("expected_revision=3"),
            Some(&site.token)
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        site.request(
            "POST",
            &neutral,
            &body("expected_revision=3&confirm=yes"),
            Some(&site.token)
        )
        .await
        .0,
        StatusCode::SEE_OTHER
    );
    assert_eq!(
        site.request(
            "POST",
            &neutral,
            &body("expected_revision=4&confirm=yes"),
            Some(&site.token)
        )
        .await
        .0,
        StatusCode::SEE_OTHER
    );
    assert_eq!(
        site.state
            .charts
            .detail(None, &chart.id)
            .await
            .unwrap()
            .revision,
        4
    );
    let history = site
        .request("GET", &format!("{root}/history"), "", None)
        .await;
    assert_eq!(history.0, StatusCode::OK);
    assert!(history.2.contains("No effect"));
    assert!(history.2.contains("Neutral"));
    assert!(history.2.contains("Keep this reason"));
    let snapshot = site
        .request("GET", &format!("{root}/history/1"), "", None)
        .await;
    assert_eq!(snapshot.0, StatusCode::OK);
    assert!(snapshot.2.contains("<td>Unknown</td>"));
    assert!(snapshot.2.contains("Full inert revision snapshot"));
    assert_eq!(
        site.request("GET", &format!("{root}/history?before=bad"), "", None)
            .await
            .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        site.request(
            "GET",
            &format!("{root}/rows/outside-roster"),
            "",
            Some(&site.token)
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn chart_addresses_escape_reserved_slugs_keep_raw_queries_and_do_not_resolve_row_type_ids() {
    let site = site().await;
    for name in ["new", "0123456789abcdef0123456789abcdef"] {
        let chart = site.chart(name).await;
        let ty = &chart.roster[0].pin.type_id;
        for suffix in [
            String::new(),
            "/edit".into(),
            "/neutral".into(),
            "/history".into(),
            "/history/1".into(),
            format!("/rows/{ty}"),
        ] {
            let path = format!("/charts/{}{suffix}?q=%2B+%26&slots=7", chart.id);
            let (status, headers, _) = site.raw("GET", &path, "", Some(&site.token), None).await;
            assert_eq!(status, StatusCode::TEMPORARY_REDIRECT);
            assert_eq!(
                headers[header::LOCATION],
                format!("/charts/~{}{suffix}?q=%2B+%26&slots=7", chart.fields.slug)
            );
            let preferred = format!("/charts/~{}{suffix}", chart.fields.slug);
            assert_eq!(
                site.request("GET", &preferred, "", Some(&site.token))
                    .await
                    .0,
                StatusCode::OK
            );
            let head = site
                .raw("HEAD", &preferred, "", Some(&site.token), None)
                .await;
            assert_eq!(head.0, StatusCode::OK);
            assert!(head.2.is_empty());
        }
        let edit = site
            .request(
                "GET",
                &format!("/charts/~{}/edit", chart.fields.slug),
                "",
                Some(&site.token),
            )
            .await;
        assert!(
            edit.2
                .contains(&format!("action=\"/charts/{}/edit\"", chart.id))
        );
        assert!(edit.2.contains(&format!("value=\"{ty}@1\"")));
    }
    for (method, path, status) in [
        ("GET", "/charts/missing/bogus", StatusCode::NOT_FOUND),
        (
            "POST",
            "/charts/missing/history",
            StatusCode::METHOD_NOT_ALLOWED,
        ),
        (
            "PUT",
            "/charts/missing/edit",
            StatusCode::METHOD_NOT_ALLOWED,
        ),
        ("GET", "/charts/missing%2Fscope/edit", StatusCode::NOT_FOUND),
    ] {
        assert_eq!(site.raw(method, path, "", None, None).await.0, status);
    }
}

#[tokio::test]
async fn roster_exact_historical_pins_and_all_invalid_slots_survive_definition_errors() {
    let site = site().await;
    let chart = site.chart("Pinned metadata").await;
    let ty = &chart.roster[0].pin.type_id;
    site.state
        .types
        .save_type(
            &site.token,
            ty,
            Edit {
                expected_revision: 1,
                fields: fields("Renamed mist"),
                reason: None,
            },
        )
        .await
        .unwrap();
    let path = format!("/charts/{}/edit", chart.id);
    let get = site.request("GET", &path, "", Some(&site.token)).await;
    assert!(get.2.contains("Mist · retained version 1"));
    assert!(get.2.contains("Renamed mist · version 2"));
    let data = body(&format!(
        "expected_revision=1&display_name=Retained+name&description=Retained+description&slots=1&slot_1={ty}%40bad&slot_33=extra-malformed"
    ));
    let response = site.request("POST", &path, &data, Some(&site.token)).await;
    assert_eq!(response.0, StatusCode::UNPROCESSABLE_ENTITY);
    for text in [
        "Retained name",
        "Retained description",
        "extra-malformed",
        "Keep this reason",
    ] {
        assert!(response.2.contains(text));
    }
    assert!(response.2.contains(&format!("{ty}@bad")));
    assert_eq!(
        site.state
            .charts
            .detail(None, &chart.id)
            .await
            .unwrap()
            .revision,
        1
    );
}

#[tokio::test]
async fn world_landing_and_adoption_review_explicit_mapping_policy_and_conflict() {
    let site = site().await;
    let shared = site.chart("Shared source").await;
    let shared_type = &shared.roster[0].pin;
    let world = site
        .state
        .authoring
        .create_world(&site.token, fields("Chart world"), None)
        .await
        .unwrap();
    let landing = format!("/worlds/{}/charts", world.id);
    let new = site.request("GET", &landing, "", Some(&site.token)).await;
    assert_eq!(new.0, StatusCode::OK);
    assert!(new.2.contains("Create effectiveness chart"));
    let local_type = site
        .state
        .types
        .create_world_type(
            &site.token,
            &world.id,
            1,
            Some(shared_type.clone()),
            fields("Local mist"),
            None,
        )
        .await
        .unwrap();
    let local = site
        .state
        .charts
        .create(
            &site.token,
            Some(&world.id),
            NewChart {
                fields: fields("Local chart"),
                roster: vec![TypePin {
                    type_id: local_type.id.clone(),
                    version: 1,
                }],
                source: Some(ChartPin {
                    chart_id: shared.id.clone(),
                    version: 1,
                }),
                mapping: vec![],
                apply_source: false,
                reason: None,
            },
        )
        .await
        .unwrap();
    assert!(
        site.request("GET", &landing, "", None)
            .await
            .2
            .contains("Local chart")
    );
    let stranger = user(&site.state.accounts, "other-admin", Role::Administrator).await;
    let root = format!("/worlds/{}/charts/{}", world.id, local.id);
    let denied = site
        .request(
            "POST",
            &format!("{root}/neutral"),
            &body("expected_revision=1&confirm=yes"),
            Some(&stranger),
        )
        .await;
    assert_eq!(denied.0, StatusCode::FORBIDDEN);
    site.state
        .charts
        .fill_neutral(
            &site.token,
            None,
            &shared.id,
            NeutralEdit {
                expected_revision: 1,
                reason: None,
            },
        )
        .await
        .unwrap();
    let adopt = format!("{root}/adopt");
    let selection = format!(
        "expected_revision=1&target_version=2&policy=apply&map_{}={}&reason=Retain+adoption+reason",
        shared_type.type_id, local_type.id
    );
    let preview = site
        .request(
            "GET",
            &format!("{adopt}?preview=yes&{selection}"),
            "",
            Some(&site.token),
        )
        .await;
    assert_eq!(preview.0, StatusCode::OK);
    for label in [
        "Old shared source",
        "Current local effective chart",
        "New shared source",
        "Proposed result",
        "Adopt reviewed source version",
        "Retain adoption reason",
    ] {
        assert!(preview.2.contains(label), "missing {label}");
    }
    // CSRF must never be a successful control of the GET selection form.
    let get_form = preview
        .2
        .split("<form method=\"get\"")
        .nth(1)
        .unwrap()
        .split("</form>")
        .next()
        .unwrap();
    assert!(!get_form.contains("name=\"csrf\""));
    assert!(get_form.contains("value=\"apply\" selected"));
    assert_eq!(
        site.state
            .charts
            .detail(Some(&world.id), &local.id)
            .await
            .unwrap()
            .revision,
        1
    );
    // Concurrent local edit makes the reviewed adoption stale.
    site.state
        .charts
        .fill_neutral(
            &site.token,
            Some(&world.id),
            &local.id,
            NeutralEdit {
                expected_revision: 1,
                reason: None,
            },
        )
        .await
        .unwrap();
    let stale = site
        .request(
            "POST",
            &adopt,
            &body(&format!("{selection}&confirm=yes")),
            Some(&site.token),
        )
        .await;
    assert_eq!(stale.0, StatusCode::CONFLICT);
    assert!(stale.2.contains("Preview against reviewed revision"));
    assert!(stale.2.contains("Retain adoption reason"));
    assert!(!stale.2.contains("Adopt reviewed source version"));
    let reviewed = site
        .request(
            "GET",
            &format!("{adopt}?preview=yes&{selection}&reviewed_revision=2"),
            "",
            Some(&site.token),
        )
        .await;
    assert_eq!(reviewed.0, StatusCode::OK);
    assert!(
        reviewed
            .2
            .contains("name=\"expected_revision\" value=\"2\"")
    );
    let saved = site
        .request(
            "POST",
            &adopt,
            &body(&format!(
                "{}&confirm=yes",
                selection.replace("expected_revision=1", "expected_revision=2")
            )),
            Some(&site.token),
        )
        .await;
    assert_eq!(saved.0, StatusCode::SEE_OTHER);
    let effective = site
        .state
        .charts
        .detail(Some(&world.id), &local.id)
        .await
        .unwrap();
    assert_eq!(effective.source.unwrap().version, 2);
    assert_eq!(effective.mapping[0].local_type_id, local_type.id);
    assert_eq!(effective.cells[0].factor, Factor::new(1, 1).unwrap());
}

#[tokio::test]
async fn storage_failure_retains_chart_draft_and_reason() {
    let site = site().await;
    let chart = site.chart("Storage lab").await;
    let ty = &chart.roster[0].pin.type_id;
    site.state.accounts.pool().close().await;
    let response = site
        .request(
            "POST",
            &format!("/charts/{}/rows/{ty}", chart.id),
            &body(&format!(
                "expected_revision=1&numerator_{ty}=malformed-n&denominator_{ty}=malformed-d"
            )),
            Some(&site.token),
        )
        .await;
    assert_eq!(response.0, StatusCode::SERVICE_UNAVAILABLE);
    for text in [
        "malformed-n",
        "malformed-d",
        "Keep this reason",
        "storage is temporarily unavailable",
    ] {
        assert!(response.2.contains(text));
    }
    assert_eq!(response.1[header::CACHE_CONTROL], "no-store");
}
