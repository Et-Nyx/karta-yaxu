//! Native Stat HTTP contract regressions, using test-owned databases only.
use super::*;
use crate::{
    application::{
        authoring::{HistoryKind, SourcePin},
        stat_values::*,
        stats::*,
    },
    auth::{Role, Subject},
    persistence,
};
use axum::body::{Body, to_bytes};
use tower::ServiceExt;

struct Site {
    state: AppState,
    app: Router,
    admin: String,
    owner: String,
    _dir: tempfile::TempDir,
}
fn fields(name: &str) -> ContentFields {
    ContentFields {
        slug: String::new(),
        display_name: name.into(),
        description: "Inert <script>stat</script> text".into(),
    }
}
async fn user(a: &Accounts, name: &str, role: Role) -> String {
    a.invite("test", name, role, None).await.unwrap();
    let actor = a
        .sign_in(Subject {
            provider: "test".into(),
            provider_subject: name.into(),
            display_name: name.into(),
        })
        .await
        .unwrap();
    a.start_session(&actor.user_id).await.unwrap().0
}
async fn site() -> Site {
    let dir = tempfile::tempdir().unwrap();
    let pool = persistence::connect_for_migration(&format!(
        "sqlite://{}/stats-http.db",
        dir.path().display()
    ))
    .await
    .unwrap();
    persistence::migrate(&pool).await.unwrap();
    let accounts = Accounts::new(pool, 3600).unwrap();
    let admin = user(&accounts, "admin", Role::Administrator).await;
    let owner = user(&accounts, "owner", Role::Editor).await;
    let state = AppState::new(accounts, None, "http://127.0.0.1:3000".into(), false);
    Site {
        app: router(state.clone()),
        state,
        admin,
        owner,
        _dir: dir,
    }
}
fn body(data: &str) -> String {
    format!("csrf={}&reason=Retain+reason&{data}", "a".repeat(64))
}
fn attribute<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    tag.split_once(&format!("{name}=\""))?.1.split('"').next()
}
fn input(html: &str, name: &str) -> String {
    html.split("<input ")
        .find_map(|tag| {
            if attribute(tag, "name") == Some(name) {
                attribute(tag, "value")
            } else {
                None
            }
        })
        .unwrap_or_else(|| panic!("missing input {name}"))
        .into()
}
fn native_fields(html: &str) -> std::collections::BTreeMap<String, String> {
    let mut fields = std::collections::BTreeMap::new();
    for tag in html
        .split("<input ")
        .skip(1)
        .filter_map(|s| s.split_once('>').map(|v| v.0))
    {
        if attribute(tag, "type") == Some("checkbox") && !tag.contains(" checked") {
            continue;
        }
        if let Some(name) = attribute(tag, "name") {
            fields.insert(name.into(), attribute(tag, "value").unwrap_or("").into());
        }
    }
    for select in html.split("<select ").skip(1) {
        let (tag, options) = select.split_once('>').unwrap();
        let options = options.split("</select>").next().unwrap();
        if let Some(name) = attribute(tag, "name") {
            let tags = options.split("<option ").skip(1).collect::<Vec<_>>();
            let chosen = tags
                .iter()
                .find(|tag| tag.split('>').next().unwrap().contains(" selected"))
                .or_else(|| tags.first());
            if let Some(chosen) = chosen {
                fields.insert(name.into(), attribute(chosen, "value").unwrap_or("").into());
            }
        }
    }
    fields
}
fn encoded(fields: &std::collections::BTreeMap<String, String>) -> String {
    let escape = |s: &str| {
        s.bytes()
            .map(|b| {
                if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
                    (b as char).to_string()
                } else {
                    format!("%{b:02X}")
                }
            })
            .collect::<String>()
    };
    fields
        .iter()
        .map(|(k, v)| format!("{}={}", escape(k), escape(v)))
        .collect::<Vec<_>>()
        .join("&")
}
impl Site {
    async fn large_form(&self, count: usize) -> (String, Vec<StatDefinition>) {
        let species = self
            .state
            .authoring
            .create_species(&self.admin, fields("Large stat creature"), None)
            .await
            .unwrap();
        let mut definitions = vec![];
        for i in 0..count {
            definitions.push(self.stat(None, &format!("Stat {i:03}"), None).await);
        }
        self.state
            .stat_values
            .save_form(
                &self.admin,
                &species.default_form_id,
                FormStatsEdit {
                    expected_revision: 1,
                    values: Some(
                        definitions
                            .iter()
                            .map(|d| (d.pin(), Some("0".parse().unwrap())))
                            .collect(),
                    ),
                    reason: None,
                },
            )
            .await
            .unwrap();
        (species.default_form_id, definitions)
    }
    async fn confirmed(
        &self,
        url: &str,
        data: &str,
        token: &str,
    ) -> (StatusCode, HeaderMap, String) {
        let data = data
            .replace("&confirm=yes", "")
            .replace("&intent=preview", "");
        let preview = self
            .req("POST", url, &format!("{data}&intent=preview"), token)
            .await;
        assert_eq!(preview.0, StatusCode::OK, "{}", preview.2);
        let fingerprint = input(&preview.2, "review_fingerprint");
        self.req(
            "POST",
            url,
            &format!("{data}&review_fingerprint={fingerprint}&confirm=yes&intent=save"),
            token,
        )
        .await
    }
    async fn raw(
        &self,
        method: &str,
        url: &str,
        data: &str,
        token: &str,
        origin: &str,
    ) -> (StatusCode, HeaderMap, String) {
        let response = self
            .app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method(method)
                    .uri(url)
                    .header(
                        header::COOKIE,
                        format!("karta_csrf={}; karta_session={token}", "a".repeat(64)),
                    )
                    .header(header::ORIGIN, origin)
                    .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(Body::from(data.to_owned()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let headers = response.headers().clone();
        let text = String::from_utf8(
            to_bytes(response.into_body(), 4 * 1024 * 1024)
                .await
                .unwrap()
                .to_vec(),
        )
        .unwrap();
        (status, headers, text)
    }
    async fn req(
        &self,
        method: &str,
        url: &str,
        data: &str,
        token: &str,
    ) -> (StatusCode, HeaderMap, String) {
        let mut url = url.to_owned();
        for _ in 0..5 {
            let r = self
                .raw(method, &url, data, token, "http://127.0.0.1:3000")
                .await;
            if method != "GET"
                || !matches!(r.0, StatusCode::TEMPORARY_REDIRECT | StatusCode::SEE_OTHER)
            {
                return r;
            }
            url = r.1[header::LOCATION].to_str().unwrap().into();
        }
        panic!("redirect loop")
    }
    async fn stat(
        &self,
        world: Option<&str>,
        name: &str,
        source: Option<StatPin>,
    ) -> StatDefinition {
        self.state
            .stats
            .create(
                if world.is_some() {
                    &self.owner
                } else {
                    &self.admin
                },
                world,
                NewStat {
                    fields: fields(name),
                    constraints: NumericConstraints::default(),
                    source,
                    reason: None,
                },
            )
            .await
            .unwrap()
    }
    async fn count(&self) -> i64 {
        sqlx::query_scalar("SELECT count(*) FROM audit_events")
            .fetch_one(self.state.accounts.pool())
            .await
            .unwrap()
    }
}

#[tokio::test]
async fn page_scoped_conflict_review_cannot_overwrite_unseen_entries_and_stale_clean_navigation_never_writes()
 {
    let s = site().await;
    let (id, defs) = s.large_form(34).await;
    let url = format!("/forms/{id}/stats");
    let second = s.req("GET", &format!("{url}?page=2"), "", &s.admin).await;
    let mut fields = native_fields(&second.2);
    fields.insert("intent".into(), "page:1".into());
    let first = s.req("POST", &url, &encoded(&fields), &s.admin).await;
    assert_eq!(first.0, StatusCode::OK);
    assert!(!first.2.contains(&format!("name=\"value_{}\"", defs[33].id)));
    let mut values = s
        .state
        .stat_values
        .form(&id)
        .await
        .unwrap()
        .assignments
        .unwrap();
    values[33].value = Some("34.5".parse().unwrap());
    s.state
        .stat_values
        .save_form(
            &s.admin,
            &id,
            FormStatsEdit {
                expected_revision: 2,
                values: Some(
                    values
                        .into_iter()
                        .map(|v| (v.definition.pin(), v.value))
                        .collect(),
                ),
                reason: None,
            },
        )
        .await
        .unwrap();
    let count = s.count().await;
    let navigate = s.req("POST", &url, &encoded(&fields), &s.admin).await;
    assert_eq!(navigate.0, StatusCode::OK);
    assert_eq!(input(&navigate.2, "expected_revision"), "3");
    assert_eq!(count, s.count().await);
    let mut draft = native_fields(&first.2);
    draft.insert(format!("value_{}", defs[0].id), "-1.25".into());
    draft.insert("intent".into(), "save".into());
    let conflict = s.req("POST", &url, &encoded(&draft), &s.admin).await;
    assert_eq!(conflict.0, StatusCode::CONFLICT);
    assert!(
        !conflict
            .2
            .contains(&format!("name=\"value_{}\"", defs[33].id))
    );
    let mut reviewed = native_fields(&conflict.2);
    reviewed.insert("reviewed_revision".into(), "3@0".into());
    assert_eq!(
        s.req("POST", &url, &encoded(&reviewed), &s.admin).await.0,
        StatusCode::SEE_OTHER
    );
    let saved = s
        .state
        .stat_values
        .form(&id)
        .await
        .unwrap()
        .assignments
        .unwrap();
    assert_eq!(saved[0].value, Some("-1.25".parse().unwrap()));
    assert_eq!(saved[33].value, Some("34.5".parse().unwrap()));
    let page = s.req("GET", &url, "", &s.admin).await;
    let mut injection = native_fields(&page.2);
    injection.insert(format!("value_{}", defs[33].id), "999".into());
    injection.insert(format!("pin_{}", defs[33].id), format!("{}@1", defs[33].id));
    let rejected = s.req("POST", &url, &encoded(&injection), &s.admin).await;
    assert_eq!(rejected.0, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(rejected.2.contains("Off-page fields were rejected"));
    assert!(
        !rejected
            .2
            .contains(&format!("name=\"value_{}\"", defs[33].id))
    );
    let history = s
        .req("GET", &format!("/forms/{id}/history/3"), "", "")
        .await;
    assert_eq!(history.0, StatusCode::OK);
    // The changed identity is outside page one; the readable diff still names it.
    let changes = history
        .2
        .split("<h2>Field changes</h2>")
        .nth(1)
        .unwrap()
        .split("<h2>Base stats at this revision</h2>")
        .next()
        .unwrap();
    assert!(changes.contains("Stat 033 = 0"));
    assert!(changes.contains("Stat 033 = 34.5"));
    assert!(changes.contains(&defs[33].id));
    let historical = s
        .req(
            "GET",
            &format!("/forms/{id}/history/3/stats?page=2"),
            "",
            "",
        )
        .await;
    assert_eq!(historical.0, StatusCode::OK);
    assert!(historical.2.contains("Stat 033 = 34.5"));
    assert!(historical.2.contains("Stat 033 = 0"));
    assert!(historical.2.contains("exact historical"));
}

#[tokio::test]
async fn three_hundred_one_entries_have_bounded_clean_transport_and_dirty_navigation_retains_current_page()
 {
    let s = site().await;
    let (id, defs) = s.large_form(301).await;
    let url = format!("/forms/{id}/stats");
    let mut page = s.req("GET", &url, "", &s.admin).await;
    let count = s.count().await;
    for n in 1..=13 {
        assert_eq!(page.0, StatusCode::OK);
        let mut fields = native_fields(&page.2);
        assert_eq!(
            fields.keys().filter(|k| k.starts_with("value_")).count(),
            if n == 13 { 1 } else { 25 }
        );
        assert!(!page.2.split("<input ").skip(1).any(|tag| {
            attribute(tag, "type") == Some("hidden")
                && attribute(tag, "name")
                    .is_some_and(|name| name.starts_with("value_") || name.starts_with("pin_"))
        }));
        assert!(encoded(&fields).len() < 12000, "request grew on page {n}");
        if n < 13 {
            fields.insert("intent".into(), format!("page:{}", n + 1));
            page = s.req("POST", &url, &encoded(&fields), &s.admin).await;
        }
    }
    assert_eq!(count, s.count().await);
    let mut draft = native_fields(&page.2);
    draft.insert(format!("value_{}", defs[300].id), "not-a-decimal".into());
    draft.insert("reason".into(), "Retain raw page 13".into());
    draft.insert("intent".into(), "page:12".into());
    let dirty = s.req("POST", &url, &encoded(&draft), &s.admin).await;
    assert_eq!(dirty.0, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(input(&dirty.2, "page"), "13");
    assert!(dirty.2.contains("not-a-decimal"));
    assert!(dirty.2.contains("Retain raw page 13"));
    assert_eq!(count, s.count().await);
    draft = native_fields(&dirty.2);
    draft.insert("intent".into(), "save".into());
    let invalid = s.req("POST", &url, &encoded(&draft), &s.admin).await;
    assert_eq!(invalid.0, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(invalid.2.contains("aria-invalid=\"true\""));
    draft = native_fields(&invalid.2);
    draft.insert(format!("value_{}", defs[300].id), "-0.125".into());
    assert_eq!(
        s.req("POST", &url, &encoded(&draft), &s.admin).await.0,
        StatusCode::SEE_OTHER
    );
    assert_eq!(
        s.state
            .stat_values
            .form(&id)
            .await
            .unwrap()
            .assignments
            .unwrap()[300]
            .value,
        Some("-0.125".parse().unwrap())
    );
}

#[tokio::test]
async fn changed_roster_reconcile_source_and_definition_inputs_require_a_fresh_unchecked_preview() {
    let s = site().await;
    let w = s
        .state
        .authoring
        .create_world(&s.owner, fields("Bound previews"), None)
        .await
        .unwrap();
    let shared = s.stat(None, "Shared source", None).await;
    let a = s.stat(Some(&w.id), "A", Some(shared.pin())).await;
    let b = s.stat(Some(&w.id), "B", None).await;
    s.state
        .stats
        .save_roster(&s.owner, &w.id, 0, vec![a.pin(), b.pin()], None)
        .await
        .unwrap();
    let roster = format!("/worlds/{}/stat-roster/edit", w.id);
    let draft = body("expected_revision=1&operation=remove&position=1&intent=preview");
    let preview = s.req("POST", &roster, &draft, &s.owner).await;
    assert_eq!(preview.0, StatusCode::OK);
    let count = s.count().await;
    let changed = s
        .req(
            "POST",
            &roster,
            &format!(
                "{}&review_fingerprint={}&confirm=yes",
                draft
                    .replace("position=1", "position=2")
                    .replace("intent=preview", "intent=save"),
                input(&preview.2, "review_fingerprint")
            ),
            &s.owner,
        )
        .await;
    assert_eq!(changed.0, StatusCode::OK);
    assert!(
        changed
            .2
            .contains("Nothing was saved. Review this fresh preview")
    );
    assert!(!changed.2.contains("name=\"confirm\" value=\"yes\" checked"));
    assert_eq!(count, s.count().await);
    let species = s
        .state
        .authoring
        .create_species(&s.admin, fields("Preview form"), None)
        .await
        .unwrap();
    s.state
        .stat_values
        .save_form(
            &s.admin,
            &species.default_form_id,
            FormStatsEdit {
                expected_revision: 1,
                values: Some(vec![(shared.pin(), Some("7".parse().unwrap()))]),
                reason: None,
            },
        )
        .await
        .unwrap();
    let worldrev = s
        .state
        .authoring
        .world_detail(&w.id)
        .await
        .unwrap()
        .revision;
    let v = s
        .state
        .authoring
        .create_variant(
            &s.owner,
            &w.id,
            worldrev,
            Some(SourcePin {
                form_id: species.default_form_id.clone(),
                version: 2,
            }),
            fields("Preview variant"),
            None,
        )
        .await
        .unwrap();
    let base = format!("/worlds/{}/variants/{}/stats", w.id, v.id);
    let rec = format!("{base}/reconcile");
    let draft = body(&format!(
        "expected_revision=1&expected_roster_revision=1&value_{}=1&intent=preview",
        a.id
    ));
    let preview = s.req("POST", &rec, &draft, &s.owner).await;
    assert_eq!(preview.0, StatusCode::OK);
    let count = s.count().await;
    let changed = s
        .req(
            "POST",
            &rec,
            &format!(
                "{}&review_fingerprint={}&confirm=yes",
                draft
                    .replace("=1&intent", "=2&intent")
                    .replace("intent=preview", "intent=save"),
                input(&preview.2, "review_fingerprint")
            ),
            &s.owner,
        )
        .await;
    assert_eq!(changed.0, StatusCode::OK);
    assert_eq!(count, s.count().await);
    assert!(changed.2.contains("A = 2"));
    assert_eq!(
        s.confirmed(&rec, &draft, &s.owner).await.0,
        StatusCode::SEE_OTHER
    );
    let source = format!("{base}/source");
    let draft =
        body("expected_revision=2&expected_roster_revision=1&target_form_version=2&intent=preview");
    let preview = s.req("POST", &source, &draft, &s.owner).await;
    let count = s.count().await;
    let changed = s
        .req(
            "POST",
            &source,
            &format!(
                "{}&map_{}={}&review_fingerprint={}&confirm=yes",
                draft.replace("intent=preview", "intent=save"),
                a.id,
                shared.id,
                input(&preview.2, "review_fingerprint")
            ),
            &s.owner,
        )
        .await;
    assert_eq!(changed.0, StatusCode::OK);
    assert_eq!(count, s.count().await);
    assert!(changed.2.contains("A = 7"));
    s.state
        .stats
        .save_definition(
            &s.admin,
            None,
            &shared.id,
            DefinitionEdit {
                expected_revision: 1,
                fields: fields("New shared"),
                constraints: NumericConstraints::default(),
                reason: None,
            },
        )
        .await
        .unwrap();
    let adopt = format!("/worlds/{}/stats/{}/adopt", w.id, a.id);
    let draft = body(
        "expected_revision=1&target_version=2&display_name=A&description=Local&minimum=-5&intent=preview",
    );
    let preview = s.req("POST", &adopt, &draft, &s.owner).await;
    let count = s.count().await;
    let changed = s
        .req(
            "POST",
            &adopt,
            &format!(
                "{}&review_fingerprint={}&confirm=yes",
                draft
                    .replace("minimum=-5", "minimum=-6")
                    .replace("intent=preview", "intent=save"),
                input(&preview.2, "review_fingerprint")
            ),
            &s.owner,
        )
        .await;
    assert_eq!(changed.0, StatusCode::OK);
    assert_eq!(count, s.count().await);
    assert!(changed.2.contains("Minimum (inclusive): -6"));
}

// Used only by the keyboard suite to provision substantial data through the real
// application services. No HTTP shortcut, raw SQL content write or fixture coupling.
#[tokio::test]
#[ignore = "explicit browser-owned database provisioning"]
async fn seed_large_browser_stats() {
    let path = std::env::var("KARTA_STATS_BROWSER_DATABASE").expect("browser-owned path required");
    let path = std::path::Path::new(&path).canonicalize().unwrap();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/browser-tests")
        .canonicalize()
        .unwrap();
    assert!(path.starts_with(root));
    assert!(
        path.parent()
            .unwrap()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("karta-browser-")
    );
    assert_eq!(path.file_name().unwrap(), "auth.db");
    let pool = persistence::connect(&format!("sqlite://{}", path.display()))
        .await
        .unwrap();
    persistence::require_schema(&pool).await.unwrap();
    let accounts = Accounts::new(pool, 3600).unwrap();
    let actor = accounts
        .sign_in(Subject {
            provider: "placeholder".into(),
            provider_subject: "librarian".into(),
            display_name: "librarian".into(),
        })
        .await
        .unwrap();
    let (token, _) = accounts.start_session(&actor.user_id).await.unwrap();
    let author = crate::application::authoring::Authoring::new(accounts.clone());
    let stats = Stats::new(accounts.clone());
    let values = StatValues::new(accounts);
    let species = author
        .create_species(&token, fields("Large stat creature"), None)
        .await
        .unwrap();
    let mut defs = vec![];
    for i in 0..301 {
        defs.push(
            stats
                .create(
                    &token,
                    None,
                    NewStat {
                        fields: fields(&format!("Stat {i:03}")),
                        constraints: NumericConstraints::default(),
                        source: None,
                        reason: None,
                    },
                )
                .await
                .unwrap(),
        );
    }
    values
        .save_form(
            &token,
            &species.default_form_id,
            FormStatsEdit {
                expected_revision: 1,
                values: Some(
                    defs.iter()
                        .map(|d| (d.pin(), Some("0".parse().unwrap())))
                        .collect(),
                ),
                reason: None,
            },
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn definitions_enforce_security_raw_decimals_conflict_and_readable_get_head() {
    let s = site().await;
    let url = "/stats/new";
    let draft =
        body("display_name=New&description=Exact&minimum=-1.2345678&maximum=1e2&whole_only=yes");
    for (token, data, origin) in [
        (&s.admin, draft.clone(), "https://wrong.example"),
        (&s.admin, "csrf=bad".into(), "http://127.0.0.1:3000"),
        (&s.owner, draft.clone(), "http://127.0.0.1:3000"),
    ] {
        assert_eq!(
            s.raw("POST", url, &data, token, origin).await.0,
            StatusCode::FORBIDDEN
        );
    }
    assert_eq!(
        s.req("POST", url, &body(&"x".repeat(33000)), &s.admin)
            .await
            .0,
        StatusCode::PAYLOAD_TOO_LARGE
    );
    let count = s.count().await;
    let invalid = s.req("POST", url, &draft, &s.admin).await;
    assert_eq!(invalid.0, StatusCode::UNPROCESSABLE_ENTITY);
    for raw in ["-1.2345678", "1e2", "Retain reason"] {
        assert!(invalid.2.contains(raw));
    }
    assert_eq!(count, s.count().await);
    let created = s
        .req(
            "POST",
            url,
            &body("display_name=New&description=Exact&minimum=-2.500000&maximum=5&whole_only=yes"),
            &s.admin,
        )
        .await;
    assert_eq!(created.0, StatusCode::SEE_OTHER);
    let id = s.state.stats.definitions(None).await.unwrap()[0].id.clone();
    let legacy = s
        .raw(
            "HEAD",
            &format!("/stats/{id}?raw=%2B"),
            "",
            "",
            "http://127.0.0.1:3000",
        )
        .await;
    assert_eq!(legacy.0, StatusCode::TEMPORARY_REDIRECT);
    assert_eq!(legacy.1[header::LOCATION], "/stats/~new?raw=%2B");
    assert!(legacy.2.is_empty());
    let detail = s.req("GET", "/stats/~new", "", "").await;
    assert_eq!(detail.0, StatusCode::OK);
    assert!(detail.2.contains("-2.5"));
    assert!(!detail.2.contains("Edit stat definition"));
    let edit = format!("/stats/{id}/edit");
    assert_eq!(
        s.req(
            "POST",
            &edit,
            &body("expected_revision=1&display_name=Renamed&description=Winner"),
            &s.admin
        )
        .await
        .0,
        StatusCode::SEE_OTHER
    );
    let stale = s
        .req(
            "POST",
            &edit,
            &body(
                "expected_revision=1&display_name=Draft&description=Retained&minimum=not-decimal",
            ),
            &s.admin,
        )
        .await;
    assert_eq!(stale.0, StatusCode::CONFLICT);
    assert!(stale.2.contains("not-decimal"));
    assert!(stale.2.contains("Save reviewed changes"));
    assert!(stale.2.contains("Renamed"));
    assert_eq!(s.req("POST",&edit,&body("expected_revision=1&reviewed_revision=2%400&display_name=Draft&description=Retained&minimum=-1.25"),&s.admin).await.0,StatusCode::SEE_OTHER);
    let history = s
        .req("GET", &format!("/stats/{id}/history/1"), "", "")
        .await;
    assert_eq!(history.0, StatusCode::OK);
    assert!(history.2.contains("-2.5"));
    assert!(history.2.contains("exact") || history.2.contains("Exact"));
    assert_eq!(
        s.req("GET", &format!("/stats/{id}/history?before=0"), "", "")
            .await
            .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
}

#[tokio::test]
async fn roster_operations_require_confirmation_scope_and_keep_independent_history() {
    let s = site().await;
    let w = s
        .state
        .authoring
        .create_world(&s.owner, fields("Roster Lab"), None)
        .await
        .unwrap();
    let d = s.stat(Some(&w.id), "Balance", None).await;
    let target = format!("/worlds/{}/stat-roster/edit", w.id);
    assert_eq!(
        s.req("GET", &target, "", &s.admin).await.0,
        StatusCode::FORBIDDEN
    );
    assert!(
        s.req("GET", &format!("/worlds/{}/stat-roster", w.id), "", "")
            .await
            .2
            .contains("Unconfigured")
    );
    let draft = body("expected_revision=0&operation=initialize");
    assert_eq!(
        s.req("POST", &target, &draft, &s.owner).await.0,
        StatusCode::OK
    );
    assert!(s.state.stats.world_roster(&w.id).await.unwrap().is_none());
    let preview = s
        .req(
            "POST",
            &target,
            &format!("{draft}&intent=preview"),
            &s.owner,
        )
        .await;
    assert_eq!(preview.0, StatusCode::OK);
    assert!(preview.2.contains("Defined-empty"));
    assert_eq!(
        s.confirmed(&target, &draft, &s.owner).await.0,
        StatusCode::SEE_OTHER
    );
    let add = body(&format!(
        "expected_revision=1&operation=add&stat_pin={}%401&confirm=yes",
        d.id
    ));
    assert_eq!(
        s.confirmed(&target, &add, &s.owner).await.0,
        StatusCode::SEE_OTHER
    );
    assert_eq!(
        s.state
            .stats
            .world_roster(&w.id)
            .await
            .unwrap()
            .unwrap()
            .entries[0],
        d
    );
    let remove = body("expected_revision=2&operation=remove&position=1&intent=preview");
    let preview = s.req("POST", &target, &remove, &s.owner).await;
    assert_eq!(preview.0, StatusCode::OK);
    assert!(preview.2.contains("Removed entries"));
    assert!(preview.2.contains("Balance"));
    let history = s
        .req(
            "GET",
            &format!("/worlds/{}/stat-roster/history/1", w.id),
            "",
            "",
        )
        .await;
    assert_eq!(history.0, StatusCode::OK);
    assert!(history.2.contains("Defined-empty"));
    assert_eq!(
        s.state
            .authoring
            .world_detail(&w.id)
            .await
            .unwrap()
            .revision,
        w.revision
    );
}

#[tokio::test]
async fn form_values_merge_partial_pages_and_v3_history_is_exact_and_escaped() {
    let s = site().await;
    let species = s
        .state
        .authoring
        .create_species(&s.admin, fields("Decimal creature"), None)
        .await
        .unwrap();
    let id = &species.default_form_id;
    let a = s.stat(None, "Signed", None).await;
    let b = s.stat(None, "Zero", None).await;
    s.state
        .stat_values
        .save_form(
            &s.admin,
            id,
            FormStatsEdit {
                expected_revision: 1,
                values: Some(vec![
                    (a.pin(), Some("-1.25".parse().unwrap())),
                    (b.pin(), Some("0".parse().unwrap())),
                ]),
                reason: None,
            },
        )
        .await
        .unwrap();
    let url = format!("/forms/{id}/stats");
    let count = s.count().await;
    let bad = s
        .req(
            "POST",
            &url,
            &body(&format!(
                "expected_revision=2&state=defined&value_{}=1e2&pin_{}={}%401",
                a.id, a.id, a.id
            )),
            &s.admin,
        )
        .await;
    assert_eq!(bad.0, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(bad.2.contains("value=\"1e2\""));
    assert!(bad.2.contains("aria-invalid=\"true\""));
    assert_eq!(s.count().await, count);
    let save = s
        .req(
            "POST",
            &url,
            &body(&format!(
                "expected_revision=2&state=defined&value_{}=&pin_{}={}%401",
                a.id, a.id, a.id
            )),
            &s.admin,
        )
        .await;
    assert_eq!(save.0, StatusCode::SEE_OTHER);
    let v = s
        .state
        .stat_values
        .form(id)
        .await
        .unwrap()
        .assignments
        .unwrap();
    assert_eq!(v[0].value, None);
    assert_eq!(v[1].value, Some("0".parse().unwrap()));
    let history = s
        .req("GET", &format!("/forms/{id}/history/2"), "", "")
        .await;
    assert_eq!(history.0, StatusCode::OK);
    for text in [
        "Base stats at this revision",
        "-1.25",
        "Zero = 0",
        "&#60;script&#62;stat",
        "snapshot_version",
    ] {
        assert!(history.2.contains(text), "missing {text}");
    }
    assert!(!history.2.contains("<script>"));
    let h = s
        .state
        .authoring
        .history_entry(HistoryKind::Form, id, 3)
        .await
        .unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&h.after_json).unwrap()["snapshot_version"],
        3
    );
}

#[tokio::test]
async fn variant_reconcile_requires_corrected_constraints_and_source_preview_preserves_unmapped() {
    let s = site().await;
    let species = s
        .state
        .authoring
        .create_species(&s.admin, fields("Source creature"), None)
        .await
        .unwrap();
    let form = &species.default_form_id;
    let shared = s.stat(None, "Shared", None).await;
    s.state
        .stat_values
        .save_form(
            &s.admin,
            form,
            FormStatsEdit {
                expected_revision: 1,
                values: Some(vec![(shared.pin(), Some("-2.5".parse().unwrap()))]),
                reason: None,
            },
        )
        .await
        .unwrap();
    let w = s
        .state
        .authoring
        .create_world(&s.owner, fields("Values lab"), None)
        .await
        .unwrap();
    let local = s.stat(Some(&w.id), "Local", Some(shared.pin())).await;
    s.state
        .stats
        .save_roster(&s.owner, &w.id, 0, vec![local.pin()], None)
        .await
        .unwrap();
    let v = s
        .state
        .authoring
        .create_variant(
            &s.owner,
            &w.id,
            1,
            Some(SourcePin {
                form_id: form.clone(),
                version: 1,
            }),
            fields("Local creature"),
            None,
        )
        .await
        .unwrap();
    let url = format!("/worlds/{}/variants/{}/stats", w.id, v.id);
    let reconcile = format!("{url}/reconcile");
    let draft = body("expected_revision=1&expected_roster_revision=1&intent=preview");
    assert_eq!(
        s.req("POST", &reconcile, &draft, &s.owner).await.0,
        StatusCode::OK
    );
    assert_eq!(
        s.confirmed(
            &reconcile,
            &body("expected_revision=1&expected_roster_revision=1&confirm=yes"),
            &s.owner
        )
        .await
        .0,
        StatusCode::SEE_OTHER
    );
    assert_eq!(
        s.req(
            "POST",
            &url,
            &body(&format!(
                "expected_revision=2&expected_roster_revision=1&value_{}=-1.25",
                local.id
            )),
            &s.owner
        )
        .await
        .0,
        StatusCode::SEE_OTHER
    );
    let source = format!("{url}/source");
    let draft =
        body("expected_revision=3&expected_roster_revision=1&target_form_version=2&intent=preview");
    let preview = s.req("POST", &source, &draft, &s.owner).await;
    assert_eq!(preview.0, StatusCode::OK);
    for text in [
        "Old source base stats",
        "New source base stats",
        "Local base stats",
        "Proposed base stats",
        "-2.5",
        "-1.25",
    ] {
        assert!(preview.2.contains(text));
    }
    assert_eq!(
        s.confirmed(
            &source,
            &body(
                "expected_revision=3&expected_roster_revision=1&target_form_version=2&confirm=yes"
            ),
            &s.owner
        )
        .await
        .0,
        StatusCode::SEE_OTHER
    );
    assert_eq!(
        s.state
            .stat_values
            .variant(&w.id, &v.id)
            .await
            .unwrap()
            .assignments
            .unwrap()[0]
            .value,
        Some("-1.25".parse().unwrap())
    );
    let mapped = body(&format!(
        "expected_revision=4&expected_roster_revision=1&target_form_version=2&map_{}={}&confirm=yes",
        local.id, shared.id
    ));
    assert_eq!(
        s.confirmed(&source, &mapped, &s.owner).await.0,
        StatusCode::SEE_OTHER
    );
    assert_eq!(
        s.state
            .stat_values
            .variant(&w.id, &v.id)
            .await
            .unwrap()
            .assignments
            .unwrap()[0]
            .value,
        Some("-2.5".parse().unwrap())
    );
    let new = s
        .state
        .stats
        .save_definition(
            &s.owner,
            Some(&w.id),
            &local.id,
            DefinitionEdit {
                expected_revision: 1,
                fields: local.fields.clone(),
                constraints: NumericConstraints {
                    whole_only: true,
                    ..Default::default()
                },
                reason: None,
            },
        )
        .await
        .unwrap();
    s.state
        .stats
        .save_roster(&s.owner, &w.id, 1, vec![new.pin()], None)
        .await
        .unwrap();
    let stale = s
        .req(
            "POST",
            &url,
            &body("expected_revision=5&expected_roster_revision=1&value_bad=NaN"),
            &s.owner,
        )
        .await;
    assert_eq!(stale.0, StatusCode::CONFLICT);
    let invalid = s
        .req(
            "POST",
            &reconcile,
            &body(&format!(
                "expected_revision=5&expected_roster_revision=2&value_{}=-2.5&intent=preview",
                local.id
            )),
            &s.owner,
        )
        .await;
    assert_eq!(invalid.0, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(invalid.2.contains("value=\"-2.5\""));
    assert_eq!(
        s.confirmed(
            &reconcile,
            &body(&format!(
                "expected_revision=5&expected_roster_revision=2&value_{}=&confirm=yes",
                local.id
            )),
            &s.owner
        )
        .await
        .0,
        StatusCode::SEE_OTHER
    );
    assert_eq!(
        s.state
            .stat_values
            .variant(&w.id, &v.id)
            .await
            .unwrap()
            .assignments
            .unwrap()[0]
            .value,
        None
    );
    s.state
        .stat_values
        .save_variant(
            &s.owner,
            &w.id,
            &v.id,
            VariantStatsEdit {
                expected_revision: 6,
                expected_roster_revision: 2,
                values: vec![(local.id.clone(), Some("-3".parse().unwrap()))],
                reason: None,
            },
        )
        .await
        .unwrap();
    s.state
        .stats
        .save_roster(&s.owner, &w.id, 2, vec![], None)
        .await
        .unwrap();
    let later_page = s
        .req(
            "POST",
            &reconcile,
            &body("expected_revision=7&expected_roster_revision=3&page=2&intent=preview"),
            &s.owner,
        )
        .await;
    assert_eq!(later_page.0, StatusCode::OK);
    let removals = later_page
        .2
        .split("<h2>Removed values</h2>")
        .nth(1)
        .unwrap()
        .split("</section>")
        .next()
        .unwrap();
    assert!(removals.contains("Local = -3"));
    assert!(removals.contains(&local.id));
}

#[tokio::test]
async fn large_stat_lists_are_paginated_without_type_cap_and_save_only_displayed_values() {
    let s = site().await;
    let species = s
        .state
        .authoring
        .create_species(&s.admin, fields("Many stats"), None)
        .await
        .unwrap();
    let mut defs = vec![];
    for i in 0..34 {
        defs.push(s.stat(None, &format!("Stat {i:02}"), None).await);
    }
    let values = defs
        .iter()
        .map(|d| (d.pin(), Some("0".parse().unwrap())))
        .collect();
    s.state
        .stat_values
        .save_form(
            &s.admin,
            &species.default_form_id,
            FormStatsEdit {
                expected_revision: 1,
                values: Some(values),
                reason: None,
            },
        )
        .await
        .unwrap();
    let url = format!("/forms/{}/stats", species.default_form_id);
    let page = s.req("GET", &url, "", &s.admin).await;
    assert_eq!(page.0, StatusCode::OK);
    assert_eq!(page.2.matches("name=\"value_").count(), 25);
    assert!(page.2.contains("Next draft page"));
    let count = s.count().await;
    let draft=s.req("POST",&url,&body(&format!("expected_revision=2&state=defined&page=1&intent=page%3A2&value_{}=not-decimal&pin_{}={}%401",defs[0].id,defs[0].id,defs[0].id)),&s.admin).await;
    assert_eq!(draft.0, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(draft.2.contains("value=\"not-decimal\""));
    assert!(draft.2.contains("Retain reason"));
    assert_eq!(s.count().await, count);
    let page = s.req("GET", &format!("{url}?page=2"), "", &s.admin).await;
    assert_eq!(page.0, StatusCode::OK);
    assert_eq!(page.2.matches("name=\"value_").count(), 9);
    let last = &defs[33];
    assert_eq!(
        s.req(
            "POST",
            &url,
            &body(&format!(
                "expected_revision=2&state=defined&page=2&value_{}=-0.125&pin_{}={}%401",
                last.id, last.id, last.id
            )),
            &s.admin
        )
        .await
        .0,
        StatusCode::SEE_OTHER
    );
    let values = s
        .state
        .stat_values
        .form(&species.default_form_id)
        .await
        .unwrap()
        .assignments
        .unwrap();
    assert_eq!(values.len(), 34);
    assert_eq!(values[0].value, Some("0".parse().unwrap()));
    assert_eq!(values[33].value, Some("-0.125".parse().unwrap()));
}

#[tokio::test]
async fn definition_adoption_compares_exact_old_local_new_and_history_is_bounded() {
    let s = site().await;
    let shared = s.stat(None, "Source Stat", None).await;
    let w = s
        .state
        .authoring
        .create_world(&s.owner, fields("Adoption lab"), None)
        .await
        .unwrap();
    let local = s.stat(Some(&w.id), "Local Stat", Some(shared.pin())).await;
    for revision in 1..52 {
        s.state
            .stats
            .save_definition(
                &s.admin,
                None,
                &shared.id,
                DefinitionEdit {
                    expected_revision: revision,
                    fields: fields(&format!("Source Stat {revision}")),
                    constraints: NumericConstraints {
                        minimum: Some("-1.25".parse().unwrap()),
                        ..Default::default()
                    },
                    reason: None,
                },
            )
            .await
            .unwrap();
    }
    let url = format!("/worlds/{}/stats/{}/adopt", w.id, local.id);
    let draft = body(
        "expected_revision=1&target_version=52&display_name=Local+Stat&description=Local+draft&minimum=-5&intent=preview",
    );
    let count = s.count().await;
    let preview = s.req("POST", &url, &draft, &s.owner).await;
    assert_eq!(preview.0, StatusCode::OK);
    for text in [
        "Old shared source",
        "Current local definition",
        "New shared source",
        "Proposed definition",
        "-1.25",
        "Local draft",
    ] {
        assert!(preview.2.contains(text));
    }
    assert_eq!(count, s.count().await);
    assert_eq!(
        s.confirmed(&url, &draft, &s.owner).await.0,
        StatusCode::SEE_OTHER
    );
    let saved = s.state.stats.detail(Some(&w.id), &local.id).await.unwrap();
    assert_eq!(saved.source.unwrap().version, 52);
    assert_eq!(saved.constraints.minimum, Some("-5".parse().unwrap()));
    let history = s
        .req("GET", &format!("/stats/{}/history", shared.id), "", "")
        .await;
    assert_eq!(history.0, StatusCode::OK);
    assert_eq!(history.2.matches("<article class=\"panel\">").count(), 50);
    assert!(history.2.contains("Older revisions"));
    let history = s
        .req(
            "GET",
            &format!("/stats/{}/history?before=3", shared.id),
            "",
            "",
        )
        .await;
    assert_eq!(history.0, StatusCode::OK);
    assert_eq!(history.2.matches("<article class=\"panel\">").count(), 2);
}
