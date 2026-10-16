//! Native HTTP workflows; all storage and servers belong to each test.
use super::*;
use crate::{
    application::moves::*,
    auth::{Role, Subject},
    persistence,
};
use axum::body::{Body, to_bytes};
use std::collections::BTreeMap;
use tower::ServiceExt;

type Values = BTreeMap<String, String>;
struct Site {
    state: AppState,
    app: Router,
    admin: String,
    owner: String,
    outsider: String,
    _dir: tempfile::TempDir,
}
fn fields(name: &str) -> ContentFields {
    ContentFields {
        slug: String::new(),
        display_name: name.into(),
        description: "Inert <script>move</script> text".into(),
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
        "sqlite://{}/moves-http.db",
        dir.path().display()
    ))
    .await
    .unwrap();
    persistence::migrate(&pool).await.unwrap();
    let accounts = Accounts::new(pool, 3600).unwrap();
    let admin = user(&accounts, "admin", Role::Administrator).await;
    let owner = user(&accounts, "owner", Role::Editor).await;
    let outsider = user(&accounts, "outsider", Role::Editor).await;
    let state = AppState::new(accounts, None, "http://127.0.0.1:3000".into(), false);
    // Exercise precisely the fixed ID-oriented inner contract owned by this module.
    let app = moves::routes().with_state(state.clone());
    Site {
        state,
        app,
        admin,
        owner,
        outsider,
        _dir: dir,
    }
}
fn attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    tag.split_once(&format!("{name}=\""))?.1.split('"').next()
}
fn input(html: &str, name: &str) -> String {
    html.split("<input ")
        .find_map(|tag| {
            if attr(tag, "name") == Some(name) {
                attr(tag, "value")
            } else {
                None
            }
        })
        .unwrap_or_else(|| panic!("missing {name}: {html}"))
        .into()
}
fn native_fields(html: &str) -> Values {
    let mut d = Values::new();
    for tag in html
        .split("<input ")
        .skip(1)
        .filter_map(|s| s.split_once('>').map(|v| v.0))
    {
        if attr(tag, "type") == Some("checkbox") && !tag.contains(" checked") {
            continue;
        }
        if let Some(name) = attr(tag, "name") {
            d.insert(name.into(), attr(tag, "value").unwrap_or("").into());
        }
    }
    for element in html.split("<textarea ").skip(1) {
        let (tag, text) = element.split_once('>').unwrap();
        if let Some(name) = attr(tag, "name") {
            d.insert(
                name.into(),
                text.split("</textarea>").next().unwrap().into(),
            );
        }
    }
    for element in html.split("<select ").skip(1) {
        let (tag, options) = element.split_once('>').unwrap();
        if let Some(name) = attr(tag, "name") {
            let options = options
                .split("</select>")
                .next()
                .unwrap()
                .split("<option ")
                .skip(1)
                .collect::<Vec<_>>();
            if let Some(option) = options
                .iter()
                .find(|s| s.split('>').next().unwrap().contains(" selected"))
                .or_else(|| options.first())
            {
                d.insert(name.into(), attr(option, "value").unwrap_or("").into());
            }
        }
    }
    d
}
fn ruleset_draft(revision: i64, name: &str) -> Values {
    let mut d = draft(revision, name);
    let text = d.remove("description").unwrap();
    d.insert("text_field".into(), "description".into());
    d.insert("next_text_field".into(), "description".into());
    d.insert("text".into(), text);
    d
}
fn panel<'a>(html: &'a str, label: &str) -> &'a str {
    html.split_once(&format!("<h2>{label}</h2>"))
        .unwrap_or_else(|| panic!("missing panel {label}"))
        .1
        .split("</section>")
        .next()
        .unwrap()
}
fn encoded(d: &Values) -> String {
    fn escape(s: &str) -> String {
        s.bytes()
            .map(|b| {
                if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
                    (b as char).to_string()
                } else {
                    format!("%{b:02X}")
                }
            })
            .collect()
    }
    d.iter()
        .map(|(k, v)| format!("{}={}", escape(k), escape(v)))
        .collect::<Vec<_>>()
        .join("&")
}
fn draft(revision: i64, name: &str) -> Values {
    Values::from([
        ("csrf".into(), "a".repeat(64)),
        ("expected_revision".into(), revision.to_string()),
        ("display_name".into(), name.into()),
        ("description".into(), "Native HTTP data".into()),
        ("reason".into(), "Retain this raw reason".into()),
        ("page".into(), "1".into()),
        ("intent".into(), "preview".into()),
        ("state".into(), "defined".into()),
        ("operation".into(), "keep".into()),
        ("value_state".into(), "unknown".into()),
        ("repeat_policy".into(), "unspecified".into()),
    ])
}
impl Site {
    async fn raw(
        &self,
        method: &str,
        url: &str,
        data: String,
        token: &str,
        origin: &str,
    ) -> (StatusCode, HeaderMap, String) {
        let r = self
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
                    .body(Body::from(data))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = r.status();
        let headers = r.headers().clone();
        let text = String::from_utf8(
            to_bytes(r.into_body(), 4 * 1024 * 1024)
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
        d: &Values,
        token: &str,
    ) -> (StatusCode, HeaderMap, String) {
        self.raw(method, url, encoded(d), token, "http://127.0.0.1:3000")
            .await
    }
    async fn confirmed(
        &self,
        url: &str,
        d: &Values,
        token: &str,
    ) -> (StatusCode, HeaderMap, String) {
        let mut d = d.clone();
        d.insert("intent".into(), "preview".into());
        assert!(
            encoded(&d).len() < 32768,
            "preview must fit the native 32 KiB transport"
        );
        let p = self.req("POST", url, &d, token).await;
        assert_eq!(p.0, StatusCode::OK, "{}", p.2);
        d.insert(
            "review_fingerprint".into(),
            input(&p.2, "review_fingerprint"),
        );
        d.insert("confirm".into(), "yes".into());
        d.insert("intent".into(), "save".into());
        assert!(
            encoded(&d).len() < 32768,
            "confirmation must fit the native 32 KiB transport"
        );
        self.req("POST", url, &d, token).await
    }
    async fn attribute(
        &self,
        world: Option<&str>,
        name: &str,
        source: Option<AttributePin>,
    ) -> MoveAttributeDefinition {
        self.state
            .moves
            .create_attribute(
                if world.is_some() {
                    &self.owner
                } else {
                    &self.admin
                },
                world,
                NewAttribute {
                    fields: fields(name),
                    unit: "units".into(),
                    constraints: NumericConstraints::default(),
                    source,
                    reason: None,
                },
            )
            .await
            .unwrap()
    }
    async fn movement(
        &self,
        world: Option<&str>,
        name: &str,
        a: &MoveAttributeDefinition,
        source: Option<MovePin>,
    ) -> MoveRecord {
        self.state
            .moves
            .create(
                if world.is_some() {
                    &self.owner
                } else {
                    &self.admin
                },
                world,
                NewMove {
                    fields: fields(name),
                    source,
                    attributes: Some(vec![AttributeValueInput {
                        attribute: a.pin(),
                        value: NumericValue::Unknown,
                    }]),
                    reason: None,
                },
            )
            .await
            .unwrap()
    }
    async fn world(&self) -> String {
        self.state
            .authoring
            .create_world(&self.owner, fields("Native World"), None)
            .await
            .unwrap()
            .id
    }
    async fn write_chart_revision(&self, row: &game_data::charts::ChartRecord) {
        let actor = self
            .state
            .accounts
            .authenticate(&self.owner)
            .await
            .unwrap()
            .unwrap();
        let mut tx = persistence::begin_write(self.state.accounts.pool())
            .await
            .unwrap();
        persistence::charts::put(&mut tx, row, Some(&actor.user_id), "edit", None)
            .await
            .unwrap();
        tx.commit().await.unwrap();
        assert_eq!(
            self.state
                .charts
                .detail(row.world_id.as_deref(), &row.id)
                .await
                .unwrap(),
            *row
        );
    }
    async fn audits(&self) -> i64 {
        sqlx::query_scalar("SELECT count(*) FROM audit_events")
            .fetch_one(self.state.accounts.pool())
            .await
            .unwrap()
    }
}

#[tokio::test]
async fn native_attribute_raw_errors_stale_first_digest_and_attributed_history() {
    let s = site().await;
    let mut d = draft(0, "Precision");
    d.insert("minimum".into(), "-1.2345678".into());
    d.insert("maximum".into(), "1e2".into());
    let invalid = s.req("POST", "/move-attributes/new", &d, &s.admin).await;
    assert_eq!(invalid.0, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(invalid.2.contains("-1.2345678"));
    assert!(invalid.2.contains("1e2"));
    assert!(invalid.2.contains("Retain this raw reason"));
    assert!(s.state.moves.attributes(None).await.unwrap().is_empty());
    d.insert("minimum".into(), "-2.5".into());
    d.insert("maximum".into(), "5".into());
    let p = s.req("POST", "/move-attributes/new", &d, &s.admin).await;
    assert!(p.2.contains("Provisional identity — not persisted"));
    let fingerprint = input(&p.2, "review_fingerprint");
    d.insert("review_fingerprint".into(), fingerprint);
    d.insert("intent".into(), "save".into());
    d.insert("confirm".into(), "yes".into());
    d.insert("unit".into(), "changed after preview".into());
    let fresh = s.req("POST", "/move-attributes/new", &d, &s.admin).await;
    assert_eq!(fresh.0, StatusCode::OK);
    assert!(fresh.2.contains("Review this fresh preview"));
    assert!(!fresh.2.contains("name=\"confirm\" value=\"yes\" checked"));
    assert!(s.state.moves.attributes(None).await.unwrap().is_empty());
    let saved = s.confirmed("/move-attributes/new", &d, &s.admin).await;
    assert_eq!(saved.0, StatusCode::SEE_OTHER, "{}", saved.2);
    let row = s.state.moves.attributes(None).await.unwrap().remove(0);
    assert_eq!(row.unit, "changed after preview");
    let history = s
        .state
        .moves
        .attribute_history(None, &row.id, None)
        .await
        .unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].actor_name, "admin");
    assert_eq!(history[0].reason.as_deref(), Some("Retain this raw reason"));
    let url = format!("/move-attributes/{}/edit", row.id);
    d.insert("expected_revision".into(), "0".into());
    d.insert("minimum".into(), "raw invalid".into());
    let stale = s.req("POST", &url, &d, &s.admin).await;
    assert_eq!(stale.0, StatusCode::CONFLICT);
    assert!(stale.2.contains("raw invalid"));
    assert_eq!(input(&stale.2, "expected_revision"), "0");
    // Explicit review rebases, but malformed retained data still fails validation.
    d.insert("reviewed_revision".into(), "1".into());
    let reviewed = s.req("POST", &url, &d, &s.admin).await;
    assert_eq!(reviewed.0, StatusCode::UNPROCESSABLE_ENTITY);
    let old = s
        .req(
            "GET",
            &format!("/move-attributes/{}/history/1", row.id),
            &Values::new(),
            "",
        )
        .await;
    assert_eq!(old.0, StatusCode::OK);
    assert!(old.2.contains("Minimum (inclusive): -2.5"));
}

#[tokio::test]
async fn native_permissions_origin_csrf_payload_and_revoked_draft() {
    let s = site().await;
    let world = s.world().await;
    let a = s.attribute(Some(&world), "Scoped", None).await;
    let url = format!("/worlds/{world}/move-attributes/{}/edit", a.id);
    let mut d = draft(1, "Scoped");
    d.insert("minimum".into(), "raw retained".into());
    let denied = s.req("POST", &url, &d, &s.admin).await;
    assert_eq!(denied.0, StatusCode::FORBIDDEN);
    assert!(denied.2.contains("raw retained"));
    assert_eq!(
        s.req("GET", &url, &Values::new(), &s.outsider).await.0,
        StatusCode::FORBIDDEN
    );
    let public = s
        .req(
            "GET",
            &format!("/worlds/{world}/move-attributes/{}", a.id),
            &Values::new(),
            "",
        )
        .await;
    assert_eq!(public.0, StatusCode::OK);
    assert!(!public.2.contains(">Edit</a>"));
    assert!(public.2.contains("&#60;script&#62;move&#60;/script&#62;"));
    assert!(!public.2.contains("<script>move</script>"));
    let actor = s
        .state
        .accounts
        .authenticate(&s.outsider)
        .await
        .unwrap()
        .unwrap();
    s.state
        .authoring
        .grant_collaborator(&s.owner, &world, 1, &actor.user_id, None)
        .await
        .unwrap();
    assert_eq!(
        s.req("GET", &url, &Values::new(), &s.outsider).await.0,
        StatusCode::OK
    );
    s.state
        .authoring
        .revoke_collaborator(&s.owner, &world, 2, &actor.user_id, None)
        .await
        .unwrap();
    let revoked = s.req("POST", &url, &d, &s.outsider).await;
    assert_eq!(revoked.0, StatusCode::FORBIDDEN);
    assert!(revoked.2.contains("raw retained"));
    assert!(revoked.2.contains("Retain this raw reason"));
    assert_eq!(
        s.raw(
            "POST",
            &url,
            encoded(&d),
            &s.owner,
            "https://outside.invalid"
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    d.insert("csrf".into(), "wrong".into());
    assert_eq!(
        s.req("POST", &url, &d, &s.owner).await.0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        s.raw(
            "POST",
            &url,
            "x".repeat(33 * 1024),
            &s.owner,
            "http://127.0.0.1:3000"
        )
        .await
        .0,
        StatusCode::PAYLOAD_TOO_LARGE
    );
    assert_eq!(
        s.state
            .moves
            .attribute(Some(&world), &a.id)
            .await
            .unwrap()
            .revision,
        1
    );
}

#[tokio::test]
async fn native_move_operations_preserve_exact_off_page_values_and_dirty_navigation() {
    let s = site().await;
    let a = s.attribute(None, "Optional numeric", None).await;
    let mut rows = vec![];
    for i in 0..27 {
        rows.push(s.attribute(None, &format!("Attribute {i:03}"), None).await);
    }
    let m = s
        .state
        .moves
        .create(
            &s.admin,
            None,
            NewMove {
                fields: fields("Many attributes"),
                source: None,
                attributes: Some(
                    rows.iter()
                        .map(|a| AttributeValueInput {
                            attribute: a.pin(),
                            value: NumericValue::Known("0".parse().unwrap()),
                        })
                        .collect(),
                ),
                reason: None,
            },
        )
        .await
        .unwrap();
    let url = format!("/moves/{}/edit", m.id);
    let mut d = draft(1, "Many attributes");
    d.insert("operation".into(), "add".into());
    d.insert(
        "exact_pin".into(),
        "ffffffffffffffffffffffffffffffff@1".into(),
    );
    let missing = s.req("POST", &url, &d, &s.admin).await;
    assert_eq!(missing.0, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(missing.2.contains("ffffffffffffffffffffffffffffffff@1"));
    assert!(missing.2.contains("Retain this raw reason"));
    d.remove("exact_pin");
    d.insert("operation".into(), "value".into());
    d.insert("position".into(), "27".into());
    d.insert("value_state".into(), "known".into());
    d.insert("value".into(), "-0.125".into());
    d.insert("value".into(), "1e2".into());
    let invalid = s.req("POST", &url, &d, &s.admin).await;
    assert_eq!(invalid.0, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(invalid.2.contains("1e2"));
    assert!(invalid.2.contains("Retain this raw reason"));
    d.insert("value".into(), "-0.125".into());
    let r = s.confirmed(&url, &d, &s.admin).await;
    assert_eq!(r.0, StatusCode::SEE_OTHER, "{}", r.2);
    let head = s.state.moves.detail(None, &m.id).await.unwrap();
    let attrs = head.attributes.unwrap();
    assert_eq!(attrs.len(), 27);
    assert_eq!(attrs[0].value, NumericValue::Known("0".parse().unwrap()));
    assert_eq!(
        attrs[26].value,
        NumericValue::Known("-0.125".parse().unwrap())
    );
    let page = s.req("GET", &url, &Values::new(), &s.admin).await;
    assert_eq!(page.0, StatusCode::OK);
    assert!(page.2.contains("name=\"value_state\""));
    for a in &rows {
        for prefix in ["value_", "pin_", "map_", "remove_"] {
            assert!(!page.2.contains(&format!("name=\"{prefix}{}\"", a.id)));
        }
    }
    let hidden = page
        .2
        .split("<input ")
        .skip(1)
        .filter_map(|s| s.split_once('>').map(|v| v.0))
        .filter(|tag| attr(tag, "type") == Some("hidden"))
        .filter_map(|tag| attr(tag, "name"))
        .collect::<Vec<_>>();
    assert_eq!(
        hidden,
        vec!["csrf", "expected_revision", "page", "page_baseline"]
    );
    d = draft(2, "Many attributes");
    d.insert("page_baseline".into(), input(&page.2, "page_baseline"));
    d.insert("intent".into(), "page:2".into());
    d.insert("reason".into(), "Dirty reason retained".into());
    let dirty = s.req("POST", &url, &d, &s.admin).await;
    assert_eq!(dirty.0, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(dirty.2.contains("Save the current page before navigation"));
    assert!(dirty.2.contains("Dirty reason retained"));
    d.insert("intent".into(), "discard".into());
    let clean = s.req("POST", &url, &d, &s.admin).await;
    assert_eq!(clean.0, StatusCode::OK);
    let mut second = draft(2, "Many attributes");
    second.insert("reason".into(), String::new());
    second.insert("page_baseline".into(), input(&clean.2, "page_baseline"));
    second.insert("intent".into(), "page:2".into());
    let nav = s.req("POST", &url, &second, &s.admin).await;
    assert_eq!(nav.0, StatusCode::OK, "{}", nav.2);
    assert_eq!(input(&nav.2, "page"), "2");
    let hist = s
        .req(
            "GET",
            &format!("/moves/{}/history/2?page=2", m.id),
            &Values::new(),
            "",
        )
        .await;
    assert_eq!(hist.0, StatusCode::OK);
    assert!(hist.2.contains("Known -0.125"));
    assert!(hist.2.contains("Attribute 026"));
    // Exact pin changes preserve the current numeric state, never reset to zero.
    let mut change = draft(2, "Many attributes");
    change.insert("operation".into(), "change".into());
    change.insert("position".into(), "27".into());
    change.insert("exact_pin".into(), format!("{}@1", a.id));
    assert_eq!(
        s.confirmed(&url, &change, &s.admin).await.0,
        StatusCode::SEE_OTHER
    );
    let head = s.state.moves.detail(None, &m.id).await.unwrap();
    assert_eq!(
        head.attributes.unwrap()[26].value,
        NumericValue::Known("-0.125".parse().unwrap())
    );
}

#[tokio::test]
async fn native_move_adoption_keeps_by_default_explicit_mappings_and_same_version_copy() {
    let s = site().await;
    let w = s.world().await;
    let shared = s.attribute(None, "Shared numeric", None).await;
    let other = s.attribute(None, "Unmapped numeric", None).await;
    let local = s
        .attribute(Some(&w), "Local numeric", Some(shared.pin()))
        .await;
    let removed = s.attribute(Some(&w), "Removed numeric", None).await;
    let source = s
        .state
        .moves
        .create(
            &s.admin,
            None,
            NewMove {
                fields: fields("Shared move"),
                source: None,
                attributes: Some(vec![
                    AttributeValueInput {
                        attribute: shared.pin(),
                        value: NumericValue::NotApplicable,
                    },
                    AttributeValueInput {
                        attribute: other.pin(),
                        value: NumericValue::Known("0".parse().unwrap()),
                    },
                ]),
                reason: None,
            },
        )
        .await
        .unwrap();
    let target = s
        .movement(Some(&w), "Local move", &removed, Some(source.pin()))
        .await;
    let url = format!("/worlds/{w}/moves/{}/adopt", target.id);
    let mut d = draft(1, "Local move");
    d.insert("description".into(), target.fields.description.clone());
    d.insert("target_version".into(), "1".into());
    assert_eq!(
        s.confirmed(&url, &d, &s.owner).await.0,
        StatusCode::SEE_OTHER
    );
    assert_eq!(
        s.state.moves.detail(Some(&w), &target.id).await.unwrap(),
        target
    );
    d.insert("apply_source".into(), "yes".into());
    d.insert("mapping_operation".into(), "set".into());
    d.insert("shared_attribute_id".into(), shared.id.clone());
    d.insert("exact_pin".into(), format!("{}@1", local.id));
    let p = s.req("POST", &url, &d, &s.owner).await;
    assert_eq!(p.0, StatusCode::OK, "{}", p.2);
    assert!(p.2.contains("All removed local assignments"));
    assert!(p.2.contains("Removed numeric"));
    assert!(p.2.contains("All unmapped source assignments"));
    assert!(p.2.contains("Unmapped numeric"));
    assert_eq!(
        s.confirmed(&url, &d, &s.owner).await.0,
        StatusCode::SEE_OTHER
    );
    let head = s.state.moves.detail(Some(&w), &target.id).await.unwrap();
    let values = head.attributes.unwrap();
    assert_eq!(values.len(), 1);
    assert_eq!(values[0].definition.id, local.id);
    assert_eq!(values[0].value, NumericValue::NotApplicable);
    assert_eq!(head.source.unwrap().version, 1);
}

#[tokio::test]
async fn native_ruleset_unconfigured_exact_expectations_preview_binding_and_reports() {
    let s = site().await;
    let w = s.world().await;
    let a = s.attribute(Some(&w), "Local numeric", None).await;
    let m = s.movement(Some(&w), "Incomplete", &a, None).await;
    let url = format!("/worlds/{w}/ruleset");
    let unconfigured = s.req("GET", &url, &Values::new(), "").await;
    assert_eq!(unconfigured.0, StatusCode::OK);
    assert!(unconfigured.2.contains("Unconfigured"));
    let mut d = ruleset_draft(0, "Explicit rules");
    let roster = s
        .state
        .stats
        .save_roster(&s.owner, &w, 0, vec![], None)
        .await
        .unwrap();
    let chart = s
        .state
        .charts
        .create(
            &s.owner,
            Some(&w),
            crate::application::charts::NewChart {
                fields: fields("Exact context"),
                roster: vec![],
                source: None,
                mapping: vec![],
                apply_source: false,
                reason: None,
            },
        )
        .await
        .unwrap();
    d.insert("stat_roster".into(), format!("{}@1", roster.id));
    d.insert("chart".into(), format!("{}@1", chart.id));
    d.insert("operation".into(), "add".into());
    d.insert("exact_pin".into(), format!("{}@1", a.id));
    d.insert("required".into(), "yes".into());
    d.insert("maximum".into(), "2.5".into());
    assert_eq!(
        s.confirmed(&format!("{url}/edit"), &d, &s.owner).await.0,
        StatusCode::SEE_OTHER
    );
    let rules = s.state.moves.ruleset(&w).await.unwrap().unwrap();
    assert_eq!(rules.max_moves, None);
    assert_eq!(rules.repeat_policy, RepeatPolicy::Unspecified);
    assert!(rules.expectations.as_ref().unwrap()[0].required);
    let report = s
        .req(
            "GET",
            &format!("/worlds/{w}/moves/{}", m.id),
            &Values::new(),
            "",
        )
        .await;
    assert!(report.2.contains("NOT combat readiness"));
    assert!(report.2.contains("required value is Unknown"));
    assert!(report.2.contains(&format!("ruleset {}@1", rules.id)));
    assert!(report.2.contains(&format!("stat_roster {}@1", roster.id)));
    assert!(report.2.contains(&format!("chart {}@1", chart.id)));
    let mut removal = ruleset_draft(1, "Explicit rules");
    removal.insert("operation".into(), "remove".into());
    removal.insert("position".into(), "1".into());
    let p = s
        .req("POST", &format!("{url}/edit"), &removal, &s.owner)
        .await;
    assert_eq!(p.0, StatusCode::OK);
    removal.insert(
        "review_fingerprint".into(),
        input(&p.2, "review_fingerprint"),
    );
    removal.insert("confirm".into(), "yes".into());
    removal.insert("intent".into(), "save".into());
    removal.insert("max_moves".into(), "7".into());
    let changed = s
        .req("POST", &format!("{url}/edit"), &removal, &s.owner)
        .await;
    assert_eq!(changed.0, StatusCode::OK);
    assert!(changed.2.contains("Review this fresh preview"));
    assert_eq!(
        s.state.moves.ruleset(&w).await.unwrap().unwrap().revision,
        1
    );
    assert_eq!(
        s.confirmed(&format!("{url}/edit"), &removal, &s.owner)
            .await
            .0,
        StatusCode::SEE_OTHER
    );
    let old = s
        .req("GET", &format!("{url}/history/1"), &Values::new(), "")
        .await;
    assert!(old.2.contains("Required Known"));
    assert!(old.2.contains("Unspecified — no default"));
    let historical = s
        .req(
            "GET",
            &format!("/worlds/{w}/moves/{}?ruleset_revision=1", m.id),
            &Values::new(),
            "",
        )
        .await;
    assert!(historical.2.contains("required value is Unknown"));
    let latest = s
        .req(
            "GET",
            &format!("/worlds/{w}/moves/{}", m.id),
            &Values::new(),
            "",
        )
        .await;
    assert!(!latest.2.contains("required value is Unknown"));
    assert!(latest.2.contains(&format!("ruleset {}@2", rules.id)));
}

#[tokio::test]
async fn native_ruleset_retained_archived_chart_allows_preview_confirmation_and_noops() {
    let s = site().await;
    let w = s.world().await;
    let chart = s
        .state
        .charts
        .create(
            &s.owner,
            Some(&w),
            crate::application::charts::NewChart {
                fields: fields("Retained chart"),
                roster: vec![],
                source: None,
                mapping: vec![],
                apply_source: false,
                reason: None,
            },
        )
        .await
        .unwrap();
    let url = format!("/worlds/{w}/ruleset/edit");
    let mut d = ruleset_draft(0, "Archived chart context");
    d.insert("chart".into(), format!("{}@1", chart.id));
    assert_eq!(
        s.confirmed(&url, &d, &s.owner).await.0,
        StatusCode::SEE_OTHER
    );
    let first = s.state.moves.ruleset(&w).await.unwrap().unwrap();
    let mut archived = chart.clone();
    archived.revision = 2;
    archived.archived_at = Some(1_800_000_000);
    s.write_chart_revision(&archived).await;
    assert_eq!(
        s.state
            .charts
            .version(Some(&w), &chart.id, 1)
            .await
            .unwrap(),
        chart
    );
    let history = s
        .state
        .charts
        .history(Some(&w), &chart.id, None)
        .await
        .unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].actor_name, "owner");
    assert_eq!(
        history[0].before_json.as_ref(),
        Some(&history[1].after_json)
    );

    d.insert("expected_revision".into(), "1".into());
    d.insert("max_moves".into(), "3".into());
    let preview = s.req("POST", &url, &d, &s.owner).await;
    assert_eq!(preview.0, StatusCode::OK, "{}", preview.2);
    assert!(panel(&preview.2, "Proposed state").contains("Maximum moves: 3"));
    assert_eq!(
        s.state.moves.ruleset(&w).await.unwrap(),
        Some(first.clone())
    );
    assert_eq!(
        s.confirmed(&url, &d, &s.owner).await.0,
        StatusCode::SEE_OTHER
    );
    let max = s.state.moves.ruleset(&w).await.unwrap().unwrap();
    assert_eq!(max.chart, first.chart);
    assert_eq!(max.max_moves, Some(3));

    d.insert("expected_revision".into(), max.revision.to_string());
    d.insert("text_field".into(), "notes".into());
    d.insert("next_text_field".into(), "notes".into());
    d.insert(
        "text".into(),
        "Notes retain the archived exact chart pin".into(),
    );
    assert_eq!(
        s.confirmed(&url, &d, &s.owner).await.0,
        StatusCode::SEE_OTHER
    );
    let notes = s.state.moves.ruleset(&w).await.unwrap().unwrap();
    assert_eq!(notes.chart, first.chart);
    assert_eq!(notes.max_moves, Some(3));
    assert_eq!(notes.notes, d["text"]);
    assert_eq!(notes.fields.description, first.fields.description);
    d.insert("expected_revision".into(), notes.revision.to_string());
    let audits = s.audits().await;
    assert_eq!(
        s.confirmed(&url, &d, &s.owner).await.0,
        StatusCode::SEE_OTHER
    );
    assert_eq!(
        s.state.moves.ruleset(&w).await.unwrap(),
        Some(notes.clone())
    );
    assert_eq!(s.audits().await, audits);

    d.insert("chart".into(), format!("{}@2", chart.id));
    for intent in ["preview", "save"] {
        d.insert("intent".into(), intent.into());
        d.insert("confirm".into(), "yes".into());
        d.insert("review_fingerprint".into(), "obsolete-review".into());
        let blocked = s.req("POST", &url, &d, &s.owner).await;
        assert_eq!(blocked.0, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(blocked.2.contains("Choose an unarchived chart revision."));
        assert_eq!(input(&blocked.2, "chart"), format!("{}@2", chart.id));
    }
    assert_eq!(
        s.state.moves.ruleset(&w).await.unwrap(),
        Some(notes.clone())
    );
    assert_eq!(s.audits().await, audits);

    // An unarchived head must not make its archived historical revision selectable.
    let mut unarchived = archived;
    unarchived.revision = 3;
    unarchived.archived_at = None;
    s.write_chart_revision(&unarchived).await;
    let audits = s.audits().await;
    let blocked = s.req("POST", &url, &d, &s.owner).await;
    assert_eq!(blocked.0, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(blocked.2.contains("Choose an unarchived chart revision."));
    assert_eq!(s.audits().await, audits);
    assert_eq!(s.state.moves.ruleset(&w).await.unwrap(), Some(notes));
    assert_eq!(
        s.state
            .moves
            .ruleset_version(&w, &first.id, 1)
            .await
            .unwrap(),
        first
    );
}

#[tokio::test]
async fn native_ruleset_current_configuration_diagnostics_find_impossible_required_domains() {
    use game_data::rulesets::CURRENT_VALIDATOR_VERSION;
    let policy = format!("Validator policy version {CURRENT_VALIDATOR_VERSION}");
    let saved_label = "Saved configuration diagnostics — current policy — NOT combat readiness";
    let proposed_label =
        "Proposed configuration diagnostics — current policy — NOT combat readiness";
    let historical_label = "Historical configuration diagnostics — evaluated now with current policy — NOT combat readiness";
    let contradiction = "required Known value has no possible value";
    for (whole_only, minimum, maximum) in [(false, "20", "30"), (true, "0.1", "0.9")] {
        let s = site().await;
        let w = s.world().await;
        let a = s
            .state
            .moves
            .create_attribute(
                &s.owner,
                Some(&w),
                NewAttribute {
                    fields: fields("Intrinsic zero to ten"),
                    unit: "units".into(),
                    constraints: NumericConstraints {
                        minimum: Some("0".parse().unwrap()),
                        maximum: Some("10".parse().unwrap()),
                        whole_only,
                    },
                    source: None,
                    reason: None,
                },
            )
            .await
            .unwrap();
        let root = format!("/worlds/{w}/ruleset");
        let url = format!("{root}/edit");
        let mut d = ruleset_draft(0, "Contradictory diagnostic draft");
        d.insert("operation".into(), "add".into());
        d.insert("exact_pin".into(), format!("{}@1", a.id));
        d.insert("required".into(), "yes".into());
        d.insert("minimum".into(), minimum.into());
        d.insert("maximum".into(), maximum.into());
        let audits = s.audits().await;
        let preview = s.req("POST", &url, &d, &s.owner).await;
        assert_eq!(preview.0, StatusCode::OK, "{}", preview.2);
        let report = panel(&preview.2, proposed_label);
        assert!(report.contains(&policy));
        assert!(report.contains("Ad-hoc configuration-only diagnostics"));
        assert!(report.contains(contradiction));
        assert!(report.contains(&format!("move_attribute {}@1", a.id)));
        assert!(!report.contains("required value is Unknown"));
        assert_eq!(s.audits().await, audits);
        assert!(s.state.moves.ruleset(&w).await.unwrap().is_none());
        assert!(s.state.moves.list(Some(&w)).await.unwrap().is_empty());
        // A contradiction is diagnostic: the draft still saves and can be read.
        assert_eq!(
            s.confirmed(&url, &d, &s.owner).await.0,
            StatusCode::SEE_OTHER
        );
        let rules = s.state.moves.ruleset(&w).await.unwrap().unwrap();
        let detail = s.req("GET", &root, &Values::new(), "").await;
        assert_eq!(detail.0, StatusCode::OK);
        assert!(panel(&detail.2, saved_label).contains(contradiction));
        assert!(panel(&detail.2, saved_label).contains(&policy));

        // Current Move reports use the same policy even for Unknown, absent and
        // unmodeled values; none can turn this known configuration issue incomplete.
        for attributes in [
            Some(vec![AttributeValueInput {
                attribute: a.pin(),
                value: NumericValue::Unknown,
            }]),
            Some(vec![]),
            None,
        ] {
            let m = s
                .state
                .moves
                .create(
                    &s.owner,
                    Some(&w),
                    NewMove {
                        fields: fields("Unknown or absent numeric"),
                        source: None,
                        attributes,
                        reason: None,
                    },
                )
                .await
                .unwrap();
            let page = s
                .req(
                    "GET",
                    &format!("/worlds/{w}/moves/{}", m.id),
                    &Values::new(),
                    "",
                )
                .await;
            assert_eq!(page.0, StatusCode::OK);
            let report = panel(&page.2, "Authoring validation — NOT combat readiness");
            assert!(report.contains(&policy));
            assert!(report.contains(contradiction));
        }

        d.insert("expected_revision".into(), rules.revision.to_string());
        d.insert("operation".into(), "value".into());
        d.insert("position".into(), "1".into());
        d.remove("required");
        let preview = s.req("POST", &url, &d, &s.owner).await;
        assert_eq!(preview.0, StatusCode::OK, "{}", preview.2);
        assert!(panel(&preview.2, saved_label).contains(contradiction));
        assert!(!panel(&preview.2, proposed_label).contains(contradiction));
        for label in [saved_label, proposed_label] {
            assert_eq!(preview.2.matches(&format!("<h2>{label}</h2>")).count(), 1);
        }
        assert_eq!(
            s.confirmed(&url, &d, &s.owner).await.0,
            StatusCode::SEE_OTHER
        );
        let historical = s
            .req("GET", &format!("{root}/history/1"), &Values::new(), "")
            .await;
        assert_eq!(historical.0, StatusCode::OK);
        let report = panel(&historical.2, historical_label);
        assert!(report.contains("Ad-hoc configuration-only diagnostics"));
        assert!(report.contains(&policy));
        assert!(report.contains(contradiction));
        assert!(report.contains(&format!("ruleset {}@1", rules.id)));
    }
}

#[tokio::test]
async fn native_repeated_conflicts_require_explicit_current_revision_review() {
    let s = site().await;
    let a = s.attribute(None, "Concurrent", None).await;
    let url = format!("/move-attributes/{}/edit", a.id);
    let d = draft(1, "My retained name");
    s.state
        .moves
        .save_attribute(
            &s.admin,
            None,
            &a.id,
            AttributeEdit {
                expected_revision: 1,
                fields: fields("Winning change"),
                unit: String::new(),
                constraints: NumericConstraints::default(),
                reason: None,
            },
        )
        .await
        .unwrap();
    let stale = s.req("POST", &url, &d, &s.admin).await;
    assert_eq!(stale.0, StatusCode::CONFLICT);
    assert!(stale.2.contains("Winning change"));
    assert!(stale.2.contains("My retained name"));
    s.state
        .moves
        .save_attribute(
            &s.admin,
            None,
            &a.id,
            AttributeEdit {
                expected_revision: 2,
                fields: fields("Another winning change"),
                unit: String::new(),
                constraints: NumericConstraints::default(),
                reason: None,
            },
        )
        .await
        .unwrap();
    let mut reviewed = d.clone();
    reviewed.insert("reviewed_revision".into(), "2".into());
    let again = s.req("POST", &url, &reviewed, &s.admin).await;
    assert_eq!(again.0, StatusCode::CONFLICT);
    assert!(again.2.contains("Another winning change"));
    reviewed.insert("reviewed_revision".into(), "3".into());
    let p = s.req("POST", &url, &reviewed, &s.admin).await;
    assert_eq!(p.0, StatusCode::OK);
    assert_eq!(
        s.state.moves.attribute(None, &a.id).await.unwrap().revision,
        3
    );
    reviewed.remove("reviewed_revision");
    reviewed.insert("expected_revision".into(), "3".into());
    assert_eq!(
        s.confirmed(&url, &reviewed, &s.admin).await.0,
        StatusCode::SEE_OTHER
    );
    assert_eq!(
        s.state.moves.attribute(None, &a.id).await.unwrap().revision,
        4
    );
}

#[tokio::test]
async fn native_history_summarizes_off_page_attributes_mappings_and_expectations() {
    let s = site().await;
    let w = s.world().await;
    let mut shared = vec![];
    let mut local = vec![];
    for i in 0..27 {
        let a = s.attribute(None, &format!("Shared {i:03}"), None).await;
        local.push(
            s.attribute(Some(&w), &format!("Local {i:03}"), Some(a.pin()))
                .await,
        );
        shared.push(a);
    }
    let source = s
        .state
        .moves
        .create(
            &s.admin,
            None,
            NewMove {
                fields: fields("Off page source"),
                source: None,
                attributes: Some(
                    shared
                        .iter()
                        .map(|a| AttributeValueInput {
                            attribute: a.pin(),
                            value: NumericValue::Unknown,
                        })
                        .collect(),
                ),
                reason: None,
            },
        )
        .await
        .unwrap();
    let mut values = source
        .attributes
        .as_ref()
        .unwrap()
        .iter()
        .map(|a| AttributeValueInput {
            attribute: a.definition.pin(),
            value: a.value.clone(),
        })
        .collect::<Vec<_>>();
    values[26].value = NumericValue::Known("7.25".parse().unwrap());
    s.state
        .moves
        .save(
            &s.admin,
            None,
            &source.id,
            MoveEdit {
                expected_revision: 1,
                fields: source.fields.clone(),
                attributes: Some(values),
                reason: None,
            },
        )
        .await
        .unwrap();
    let url = format!("/moves/{}/history/2", source.id);
    let first = s.req("GET", &url, &Values::new(), "").await;
    assert_eq!(first.0, StatusCode::OK);
    assert!(
        panel(&first.2, "Complete revision change summary")
            .contains("Attribute assignments: 1 changed positions")
    );
    assert!(panel(&first.2, "Readable changes").contains("No differences on this page."));
    assert!(!first.2.contains("No content difference."));
    assert!(first.2.contains("Changed state page 2"));
    let later = s
        .req("GET", &format!("{url}?page=2"), &Values::new(), "")
        .await;
    assert!(panel(&later.2, "Readable changes").contains("Shared 026 = Known 7.25"));

    let target = s
        .state
        .moves
        .create(
            &s.owner,
            Some(&w),
            NewMove {
                fields: fields("Off page mapping"),
                source: Some(source.pin()),
                attributes: Some(
                    local
                        .iter()
                        .map(|a| AttributeValueInput {
                            attribute: a.pin(),
                            value: NumericValue::Unknown,
                        })
                        .collect(),
                ),
                reason: None,
            },
        )
        .await
        .unwrap();
    let mut mappings = shared
        .iter()
        .zip(&local)
        .map(|(a, b)| AttributeMapping {
            shared_attribute_id: a.id.clone(),
            local_attribute: b.pin(),
        })
        .collect::<Vec<_>>();
    s.state
        .moves
        .adopt(
            &s.owner,
            &w,
            &target.id,
            MoveAdoptEdit {
                expected_revision: 1,
                target_version: 1,
                fields: target.fields.clone(),
                apply_source: false,
                mappings: mappings.clone(),
                reason: None,
            },
        )
        .await
        .unwrap();
    let changed = s
        .state
        .moves
        .save_attribute(
            &s.owner,
            Some(&w),
            &local[26].id,
            AttributeEdit {
                expected_revision: 1,
                fields: local[26].fields.clone(),
                unit: "new descriptive unit".into(),
                constraints: NumericConstraints::default(),
                reason: None,
            },
        )
        .await
        .unwrap();
    mappings[26].local_attribute = changed.pin();
    s.state
        .moves
        .adopt(
            &s.owner,
            &w,
            &target.id,
            MoveAdoptEdit {
                expected_revision: 2,
                target_version: 1,
                fields: target.fields.clone(),
                apply_source: false,
                mappings,
                reason: None,
            },
        )
        .await
        .unwrap();
    let url = format!("/worlds/{w}/moves/{}/history/3", target.id);
    let first = s.req("GET", &url, &Values::new(), "").await;
    assert_eq!(first.0, StatusCode::OK);
    assert!(
        panel(&first.2, "Complete revision change summary")
            .contains("Attribute mappings: 1 changed positions")
    );
    assert!(panel(&first.2, "Readable changes").contains("No differences on this page."));
    assert!(first.2.contains("Changed state page 2"));
    let later = s
        .req("GET", &format!("{url}?page=2"), &Values::new(), "")
        .await;
    assert!(panel(&later.2, "Readable changes").contains("Mapping position 27"));
    assert!(panel(&later.2, "Readable changes").contains(&format!("{}@2", local[26].id)));

    let mut expectations = local
        .iter()
        .map(|a| ExpectationInput {
            attribute: a.pin(),
            required: false,
            allow_not_applicable: false,
            constraints: NumericConstraints::default(),
        })
        .collect::<Vec<_>>();
    s.state
        .moves
        .save_ruleset(
            &s.owner,
            &w,
            RulesetEdit {
                expected_revision: 0,
                fields: fields("Off page rules"),
                expectations: Some(expectations.clone()),
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
    expectations[26].allow_not_applicable = true;
    s.state
        .moves
        .save_ruleset(
            &s.owner,
            &w,
            RulesetEdit {
                expected_revision: 1,
                fields: fields("Off page rules"),
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
    let url = format!("/worlds/{w}/ruleset/history/2");
    let first = s.req("GET", &url, &Values::new(), "").await;
    assert_eq!(first.0, StatusCode::OK);
    assert!(
        panel(&first.2, "Complete revision change summary")
            .contains("Attribute expectations: 1 changed positions")
    );
    assert!(panel(&first.2, "Readable changes").contains("No differences on this page."));
    assert!(first.2.contains("Changed state page 2"));
    let later = s
        .req("GET", &format!("{url}?page=2"), &Values::new(), "")
        .await;
    assert!(panel(&later.2, "Readable changes").contains("Expectation position 27"));
    assert!(panel(&later.2, "Readable changes").contains("allow NotApplicable: true"));
}

#[tokio::test]
async fn native_ruleset_field_scoped_unicode_prose_fits_and_retains_dirty_or_revoked_text() {
    let s = site().await;
    let w = s.world().await;
    let mut definitions = vec![];
    for i in 0..27 {
        definitions.push(
            s.attribute(Some(&w), &format!("Long prose {i:03}"), None)
                .await,
        );
    }
    let name = "🦀".repeat(100);
    let before_description = "🪷".repeat(2000);
    let old_notes = format!("{}ab", "界".repeat(666));
    assert_eq!(old_notes.len(), 2000);
    let original = s
        .state
        .moves
        .save_ruleset(
            &s.owner,
            &w,
            RulesetEdit {
                expected_revision: 0,
                fields: ContentFields {
                    slug: String::new(),
                    display_name: name.clone(),
                    description: before_description,
                },
                expectations: Some(
                    definitions
                        .iter()
                        .map(|a| ExpectationInput {
                            attribute: a.pin(),
                            required: false,
                            allow_not_applicable: true,
                            constraints: NumericConstraints::default(),
                        })
                        .collect(),
                ),
                stat_roster: None,
                chart: None,
                max_moves: None,
                repeat_policy: RepeatPolicy::Forbid,
                notes: old_notes.clone(),
                reason: None,
            },
        )
        .await
        .unwrap();
    let url = format!("/worlds/{w}/ruleset/edit");
    let opened = s.req("GET", &url, &Values::new(), &s.owner).await;
    assert_eq!(opened.0, StatusCode::OK);
    let mut d = native_fields(&opened.2);
    assert_eq!(d.get("text_field").map(String::as_str), Some("description"));
    assert!(!d.contains_key("description"));
    assert!(!d.contains_key("notes"));
    let description = format!("−{}", "🦋".repeat(1999));
    let reason = format!("−{}", "🦊".repeat(499));
    assert_eq!(description.chars().count(), 2000);
    assert_eq!(reason.chars().count(), 500);
    d.insert("text".into(), description.clone());
    d.insert("reason".into(), reason.clone());
    let mut oversized_old_protocol = d.clone();
    oversized_old_protocol.insert("notes".into(), old_notes.clone());
    assert!(
        encoded(&oversized_old_protocol).len() > 32768,
        "this legal aggregate exceeded the former all-prose form budget"
    );
    assert_eq!(
        s.confirmed(&url, &d, &s.owner).await.0,
        StatusCode::SEE_OTHER
    );
    let saved = s.state.moves.ruleset(&w).await.unwrap().unwrap();
    assert_eq!(saved.revision, 2);
    assert_eq!(saved.fields.description, description);
    assert_eq!(saved.notes, old_notes);
    assert_eq!(saved.expectations, original.expectations);
    let audit = s.state.moves.ruleset_history(&w, None).await.unwrap();
    assert_eq!(audit[0].reason.as_deref(), Some(reason.as_str()));

    let page = s.req("GET", &url, &Values::new(), &s.owner).await;
    let mut dirty = native_fields(&page.2);
    dirty.insert("next_text_field".into(), "notes".into());
    dirty.insert("load_text".into(), "yes".into());
    dirty.insert("text".into(), "Unsaved signed Unicode −雨".into());
    dirty.insert("reason".into(), "Retain this full − draft reason".into());
    let rejected = s.req("POST", &url, &dirty, &s.owner).await;
    assert_eq!(rejected.0, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        native_fields(&rejected.2).get("text").map(String::as_str),
        Some("Unsaved signed Unicode −雨")
    );
    assert!(rejected.2.contains("Retain this full − draft reason"));
    assert_eq!(input(&rejected.2, "text_field"), "description");
    dirty.remove("load_text");
    dirty.insert("intent".into(), "discard".into());
    let discarded = s.req("POST", &url, &dirty, &s.owner).await;
    assert_eq!(discarded.0, StatusCode::OK);
    let mut load = native_fields(&discarded.2);
    load.insert("next_text_field".into(), "notes".into());
    load.insert("load_text".into(), "yes".into());
    assert!(encoded(&load).len() < 32768);
    let notes = s.req("POST", &url, &load, &s.owner).await;
    assert_eq!(notes.0, StatusCode::OK);
    assert_eq!(input(&notes.2, "text_field"), "notes");
    let mut edit = native_fields(&notes.2);
    assert_eq!(edit.get("text"), Some(&old_notes));
    assert!(!edit.contains_key("description"));
    assert!(!edit.contains_key("notes"));
    assert!(!notes.2.contains("type=\"hidden\" name=\"text\""));
    let next_notes = format!("{}−ab", "雨".repeat(665));
    assert_eq!(next_notes.len(), 2000);
    edit.insert("text".into(), next_notes.clone());
    edit.insert("reason".into(), reason.clone());
    assert_eq!(
        s.confirmed(&url, &edit, &s.owner).await.0,
        StatusCode::SEE_OTHER
    );
    let saved = s.state.moves.ruleset(&w).await.unwrap().unwrap();
    assert_eq!(saved.revision, 3);
    assert_eq!(saved.fields.description, description);
    assert_eq!(saved.notes, next_notes);
    assert_eq!(saved.fields.display_name, name);
    assert_eq!(saved.expectations, original.expectations);

    let loaded = s
        .req(
            "GET",
            &format!("{url}?text_field=notes"),
            &Values::new(),
            &s.owner,
        )
        .await;
    let mut invalid = native_fields(&loaded.2);
    invalid.insert("text".into(), format!("−{}", "a".repeat(1998))); // 2001 bytes, 1999 characters.
    invalid.insert("reason".into(), "Retain invalid − notes".into());
    invalid.insert("intent".into(), "preview".into());
    let invalid_response = s.req("POST", &url, &invalid, &s.owner).await;
    assert_eq!(invalid_response.0, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        native_fields(&invalid_response.2).get("text"),
        invalid.get("text")
    );
    assert!(invalid_response.2.contains("2000 UTF-8 bytes"));
    let mut unexpected = native_fields(&loaded.2);
    unexpected.insert(
        "description".into(),
        "Unexpected off-field signed − prose".into(),
    );
    let rejected = s.req("POST", &url, &unexpected, &s.owner).await;
    assert_eq!(rejected.0, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(rejected.2.contains("Unexpected off-field signed − prose"));
    assert!(!native_fields(&rejected.2).contains_key("description"));
    let actor = s
        .state
        .accounts
        .authenticate(&s.outsider)
        .await
        .unwrap()
        .unwrap();
    s.state
        .authoring
        .grant_collaborator(&s.owner, &w, 1, &actor.user_id, None)
        .await
        .unwrap();
    let theirs = s
        .req(
            "GET",
            &format!("{url}?text_field=notes"),
            &Values::new(),
            &s.outsider,
        )
        .await;
    let mut revoked = native_fields(&theirs.2);
    revoked.insert("text".into(), "Revoked raw signed −雨 notes".into());
    revoked.insert("reason".into(), "Retain revoked − reason".into());
    s.state
        .authoring
        .revoke_collaborator(&s.owner, &w, 2, &actor.user_id, None)
        .await
        .unwrap();
    let response = s.req("POST", &url, &revoked, &s.outsider).await;
    assert_eq!(response.0, StatusCode::FORBIDDEN);
    assert_eq!(native_fields(&response.2).get("text"), revoked.get("text"));
    assert!(response.2.contains("Retain revoked − reason"));
    assert_eq!(
        s.state.moves.ruleset(&w).await.unwrap().unwrap().revision,
        3
    );
}

#[tokio::test]
async fn native_move_and_attribute_worst_case_unicode_metadata_fit_the_form_budget() {
    let s = site().await;
    let name = "🦀".repeat(100);
    let description = "🪷".repeat(2000);
    let reason = "🦊".repeat(500);
    let form = s
        .req("GET", "/move-attributes/new", &Values::new(), &s.admin)
        .await;
    let mut d = native_fields(&form.2);
    d.insert("display_name".into(), name.clone());
    d.insert("description".into(), description.clone());
    d.insert("reason".into(), reason.clone());
    let unit = format!("{}a", "界".repeat(33));
    assert_eq!(unit.len(), 100);
    d.insert("unit".into(), unit.clone());
    d.insert("minimum".into(), "-9223372036854775808".into());
    d.insert("maximum".into(), "9223372036854775807".into());
    assert_eq!(
        s.confirmed("/move-attributes/new", &d, &s.admin).await.0,
        StatusCode::SEE_OTHER
    );
    let a = s.state.moves.attributes(None).await.unwrap().remove(0);
    assert_eq!(a.unit, unit);
    assert_eq!(a.fields.description, description);
    let form = s.req("GET", "/moves/new", &Values::new(), &s.admin).await;
    let mut d = native_fields(&form.2);
    d.insert("display_name".into(), name);
    d.insert("description".into(), description.clone());
    d.insert("reason".into(), reason);
    d.insert("state".into(), "defined".into());
    d.insert("operation".into(), "add".into());
    d.insert("attribute_pin".into(), format!("{}@1", a.id));
    d.insert("value_state".into(), "known".into());
    d.insert("value".into(), "-0.125".into());
    assert_eq!(
        s.confirmed("/moves/new", &d, &s.admin).await.0,
        StatusCode::SEE_OTHER
    );
    let m = s.state.moves.list(None).await.unwrap().remove(0);
    assert_eq!(m.fields.description, description);
    assert_eq!(
        m.attributes.unwrap()[0].value,
        NumericValue::Known("-0.125".parse().unwrap())
    );
}
