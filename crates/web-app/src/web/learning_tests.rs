//! Learning HTTP regressions and an explicitly guarded browser stress-data helper.
use super::*;
use crate::{
    application::{
        learning::*,
        moves::{NewMove, RulesetEdit},
    },
    auth::{Role, Subject},
    persistence,
};
use axum::body::{Body, to_bytes};
use game_data::{
    moves::MoveRecord,
    rulesets::{RepeatPolicy, RulesetRecord},
};
use std::collections::BTreeMap;
use tower::ServiceExt;

type Values = BTreeMap<String, String>;
fn fields(name: &str) -> ContentFields {
    ContentFields {
        slug: String::new(),
        display_name: name.into(),
        description: "Inert <script>learning</script> prose".into(),
    }
}
async fn user(accounts: &Accounts, name: &str, role: Role) -> String {
    accounts.invite("test", name, role, None).await.unwrap();
    let a = accounts
        .sign_in(Subject {
            provider: "test".into(),
            provider_subject: name.into(),
            display_name: name.into(),
        })
        .await
        .unwrap();
    accounts.start_session(&a.user_id).await.unwrap().0
}
async fn new_move(
    state: &AppState,
    token: &str,
    world: Option<&str>,
    name: &str,
    source: Option<MovePin>,
) -> MoveRecord {
    state
        .moves
        .create(
            token,
            world,
            NewMove {
                fields: fields(name),
                source,
                attributes: Some(vec![]),
                reason: None,
            },
        )
        .await
        .unwrap()
}
fn rules_edit(expected_revision: i64) -> RulesetEdit {
    RulesetEdit {
        expected_revision,
        fields: fields("Learning policy"),
        expectations: Some(vec![]),
        stat_roster: None,
        chart: None,
        max_moves: Some(1),
        repeat_policy: RepeatPolicy::Forbid,
        notes: String::new(),
        reason: None,
    }
}
fn record(pin: MovePin, n: usize) -> LearningRecordInput {
    LearningRecordInput {
        id: String::new(),
        move_pin: pin,
        method: None,
        context: Some(format!("Synthetic acquisition {n}")),
        condition: Some("When the designer decides; never execute.".into()),
        threshold: match n % 3 {
            0 => NumericValue::Known("-0.125".parse().unwrap()),
            1 => NumericValue::Unknown,
            _ => NumericValue::NotApplicable,
        },
    }
}
struct Site {
    state: AppState,
    app: Router,
    admin: String,
    owner: String,
    outsider: String,
    form: String,
    world: String,
    variant: String,
    shared: MoveRecord,
    local: MoveRecord,
    rules: RulesetRecord,
    _dir: tempfile::TempDir,
}
async fn site() -> Site {
    let dir = tempfile::tempdir().unwrap();
    let pool = persistence::connect_for_migration(&format!(
        "sqlite://{}/learning-http.db",
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
    let form = state
        .authoring
        .create_species(&admin, fields("Learning creature"), None)
        .await
        .unwrap()
        .default_form_id;
    let world = state
        .authoring
        .create_world(&owner, fields("Learning world"), None)
        .await
        .unwrap()
        .id;
    let variant = state
        .authoring
        .create_variant(&owner, &world, 1, None, fields("Learning variant"), None)
        .await
        .unwrap()
        .id;
    let shared = new_move(&state, &admin, None, "Shared learning move", None).await;
    let local = new_move(
        &state,
        &owner,
        Some(&world),
        "Local learning move",
        Some(shared.pin()),
    )
    .await;
    let rules = state
        .moves
        .save_ruleset(&owner, &world, rules_edit(0))
        .await
        .unwrap();
    Site {
        app: router(state.clone()),
        state,
        admin,
        owner,
        outsider,
        form,
        world,
        variant,
        shared,
        local,
        rules,
        _dir: dir,
    }
}
fn draft(items: &[(&str, &str)]) -> Values {
    let mut v = Values::from([
        ("csrf".into(), "a".repeat(64)),
        ("expected_revision".into(), "0".into()),
        ("page".into(), "1".into()),
        ("operation".into(), "keep".into()),
        ("threshold_state".into(), "unknown".into()),
        ("reason".into(), "Retain draft reason".into()),
    ]);
    v.extend(items.iter().map(|(k, v)| ((*k).into(), (*v).into())));
    v
}
fn encoded(v: &Values) -> String {
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
    v.iter()
        .map(|(k, v)| format!("{}={}", escape(k), escape(v)))
        .collect::<Vec<_>>()
        .join("&")
}
fn input(html: &str, name: &str) -> String {
    html.split("<input ")
        .find_map(|s| {
            if s.split('>').next()?.contains(&format!("name=\"{name}\"")) {
                Some(s.split_once("value=\"")?.1.split('"').next()?.into())
            } else {
                None
            }
        })
        .unwrap_or_else(|| panic!("Missing input {name}"))
}
fn native_fields(html: &str) -> Values {
    let attribute = |tag: &str, name: &str| {
        tag.split_once(&format!("{name}=\""))
            .and_then(|(_, v)| v.split('"').next())
            .unwrap_or("")
            .to_owned()
    };
    let mut v = Values::new();
    for tag in html
        .split("<input ")
        .skip(1)
        .filter_map(|s| s.split('>').next())
    {
        if attribute(tag, "type") == "checkbox" && !tag.contains(" checked") {
            continue;
        }
        let name = attribute(tag, "name");
        if !name.is_empty() {
            v.insert(name, attribute(tag, "value"));
        }
    }
    for item in html.split("<textarea ").skip(1) {
        let (tag, rest) = item.split_once('>').unwrap();
        v.insert(
            attribute(tag, "name"),
            rest.split("</textarea>").next().unwrap().to_owned(),
        );
    }
    for item in html.split("<select ").skip(1) {
        let (tag, rest) = item.split_once('>').unwrap();
        let options = rest
            .split("</select>")
            .next()
            .unwrap()
            .split("<option ")
            .skip(1)
            .collect::<Vec<_>>();
        let selected = options
            .iter()
            .find(|s| s.split('>').next().unwrap().contains(" selected"))
            .or_else(|| options.first())
            .unwrap();
        v.insert(attribute(tag, "name"), attribute(selected, "value"));
    }
    v
}
impl Site {
    fn form_url(&self) -> String {
        format!("/forms/{}/learnset/edit", self.form)
    }
    fn local_url(&self) -> String {
        format!("/worlds/{}/variants/{}/learnset", self.world, self.variant)
    }
    fn preset_url(&self) -> String {
        format!("/worlds/{}/variants/{}/movesets", self.world, self.variant)
    }
    async fn raw(
        &self,
        method: &str,
        url: &str,
        data: &str,
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
                    .body(Body::from(data.to_owned()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = r.status();
        let headers = r.headers().clone();
        let text = String::from_utf8(
            to_bytes(r.into_body(), 8 * 1024 * 1024)
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
        v: &Values,
        token: &str,
    ) -> (StatusCode, HeaderMap, String) {
        let mut url = url.to_owned();
        for _ in 0..5 {
            let r = self
                .raw(method, &url, &encoded(v), token, "http://127.0.0.1:3000")
                .await;
            if method != "GET" || r.0 != StatusCode::TEMPORARY_REDIRECT {
                return r;
            }
            url = r.1[header::LOCATION].to_str().unwrap().into();
        }
        panic!("redirect loop")
    }
    async fn confirmed(
        &self,
        url: &str,
        v: &Values,
        token: &str,
    ) -> (StatusCode, HeaderMap, String) {
        let mut v = v.clone();
        v.insert("intent".into(), "preview".into());
        let p = self.req("POST", url, &v, token).await;
        assert_eq!(p.0, StatusCode::OK, "{}", p.2);
        v.insert(
            "review_fingerprint".into(),
            input(&p.2, "review_fingerprint"),
        );
        v.insert("confirm".into(), "yes".into());
        v.insert("intent".into(), "save".into());
        self.req("POST", url, &v, token).await
    }
    async fn seed_shared(&self, n: usize) -> LearnsetRecord {
        self.state
            .learning
            .save_learnset(
                &self.admin,
                None,
                &self.form,
                LearnsetEdit {
                    expected_revision: 0,
                    records: Some((0..n).map(|n| record(self.shared.pin(), n)).collect()),
                    reason: None,
                },
            )
            .await
            .unwrap()
    }
    async fn seed_local(&self, n: usize) -> LearnsetRecord {
        self.state
            .learning
            .save_learnset(
                &self.owner,
                Some(&self.world),
                &self.variant,
                LearnsetEdit {
                    expected_revision: 0,
                    records: Some((0..n).map(|n| record(self.local.pin(), n)).collect()),
                    reason: None,
                },
            )
            .await
            .unwrap()
    }
    async fn audits(&self) -> i64 {
        sqlx::query_scalar("SELECT count(*) FROM audit_events")
            .fetch_one(self.state.accounts.pool())
            .await
            .unwrap()
    }
}

#[tokio::test]
async fn independent_states_duplicates_exact_numbers_and_raw_errors() {
    let s = site().await;
    let url = s.form_url();
    let page = s
        .req("GET", &url.replace("/edit", ""), &Values::new(), "")
        .await;
    assert_eq!(page.0, StatusCode::OK);
    assert!(page.2.contains("Unconfigured"));
    assert_eq!(
        s.confirmed(&url, &draft(&[]), &s.admin).await.0,
        StatusCode::SEE_OTHER
    );
    let current = s
        .state
        .learning
        .learnset(None, &s.form)
        .await
        .unwrap()
        .unwrap();
    assert!(current.records.is_none());
    let count = s.audits().await;
    assert_eq!(
        s.confirmed(&url, &draft(&[("expected_revision", "1")]), &s.admin)
            .await
            .0,
        StatusCode::SEE_OTHER
    );
    assert_eq!(
        count,
        s.audits().await,
        "a confirmed no-op must not add history"
    );
    assert_eq!(
        s.confirmed(
            &url,
            &draft(&[("expected_revision", "1"), ("operation", "defined")]),
            &s.admin
        )
        .await
        .0,
        StatusCode::SEE_OTHER
    );
    let detail = s
        .req("GET", &url.replace("/edit", ""), &Values::new(), "")
        .await;
    assert!(detail.2.contains("Defined-empty"));
    let pin = format!("{}@1", s.shared.id);
    let bad = draft(&[
        ("expected_revision", "2"),
        ("operation", "add"),
        ("move", &pin),
        ("threshold_state", "known"),
        ("threshold", "1e2"),
        ("text", "<b>raw text</b>"),
    ]);
    let r = s.req("POST", &url, &bad, &s.admin).await;
    assert_eq!(r.0, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(r.2.contains("value=\"1e2\""));
    assert!(r.2.contains("Retain draft reason"));
    assert!(r.2.contains("raw text"));
    assert!(!r.2.contains("<b>raw text</b>"));
    for (revision, threshold) in [("2", "unknown"), ("3", "na"), ("4", "known")] {
        let v = draft(&[
            ("expected_revision", revision),
            ("operation", "add"),
            ("move", &pin),
            ("threshold_state", threshold),
            ("threshold", "-0.125"),
        ]);
        assert_eq!(
            s.confirmed(&url, &v, &s.admin).await.0,
            StatusCode::SEE_OTHER
        );
    }
    let records = s
        .state
        .learning
        .learnset(None, &s.form)
        .await
        .unwrap()
        .unwrap()
        .records
        .unwrap();
    assert_eq!(records.len(), 3);
    assert_ne!(records[0].id, records[1].id);
    assert_eq!(records[0].threshold, NumericValue::Unknown);
    assert_eq!(records[1].threshold, NumericValue::NotApplicable);
    assert_eq!(
        records[2].threshold,
        NumericValue::Known("-0.125".parse().unwrap())
    );
    assert!(records.iter().all(|r| r.move_record.id == s.shared.id));
}

#[tokio::test]
async fn stale_precedes_parsing_review_is_explicit_and_revocation_retains_draft() {
    let s = site().await;
    let saved = s.seed_shared(2).await;
    let url = s.form_url();
    let mut v = draft(&[
        ("operation", "edit"),
        ("record", &saved.records.as_ref().unwrap()[0].id),
        ("move", "broken@pin"),
        ("threshold_state", "known"),
        ("threshold", "1e2"),
    ]);
    let r = s.req("POST", &url, &v, &s.admin).await;
    assert_eq!(r.0, StatusCode::CONFLICT);
    assert!(r.2.contains("broken@pin"));
    assert!(r.2.contains("Preview reviewed changes against revision 1"));
    assert!(r.2.contains("Synthetic acquisition 1"));
    v.insert("reviewed_revision".into(), "1".into());
    assert_eq!(
        s.req("POST", &url, &v, &s.admin).await.0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let local = s.seed_local(1).await;
    let mut edit = draft(&[
        ("expected_revision", "1"),
        ("operation", "edit"),
        ("record", &local.records.unwrap()[0].id),
        ("move", "retain-malformed@pin"),
    ]);
    let outsider = s
        .state
        .accounts
        .authenticate(&s.outsider)
        .await
        .unwrap()
        .unwrap();
    let access = s
        .state
        .authoring
        .access(&s.owner, &s.world)
        .await
        .unwrap()
        .world
        .access_revision;
    s.state
        .authoring
        .grant_collaborator(&s.owner, &s.world, access, &outsider.user_id, None)
        .await
        .unwrap();
    assert_eq!(
        s.req(
            "GET",
            &format!("{}/edit", s.local_url()),
            &Values::new(),
            &s.outsider
        )
        .await
        .0,
        StatusCode::OK
    );
    s.state
        .authoring
        .revoke_collaborator(&s.owner, &s.world, access + 1, &outsider.user_id, None)
        .await
        .unwrap();
    let r = s
        .req(
            "POST",
            &format!("{}/edit", s.local_url()),
            &edit,
            &s.outsider,
        )
        .await;
    assert_eq!(r.0, StatusCode::FORBIDDEN);
    assert!(r.2.contains("retain-malformed@pin"));
    assert!(r.2.contains("Retain draft reason"));
    edit.insert("csrf".into(), "b".repeat(64));
    assert_eq!(
        s.req("POST", &format!("{}/edit", s.local_url()), &edit, &s.owner)
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        s.raw(
            "POST",
            &url,
            &encoded(&v),
            &s.admin,
            "https://elsewhere.invalid"
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        s.raw(
            "POST",
            &url,
            &"x".repeat(32769),
            &s.admin,
            "http://127.0.0.1:3000"
        )
        .await
        .0,
        StatusCode::PAYLOAD_TOO_LARGE
    );
}

#[tokio::test]
async fn adoption_mapping_preserves_every_record_and_changed_preview_never_writes() {
    let s = site().await;
    let source = s.seed_shared(3).await;
    let source_pin = format!("{}@1", source.id);
    let local_pin = format!("{}@1", s.local.id);
    let url = format!("{}/adopt", s.local_url());
    let mut v = draft(&[
        ("source", &source_pin),
        ("policy", "keep"),
        ("mapping_operation", "set"),
        ("shared_move", &s.shared.id),
        ("local_move", &local_pin),
    ]);
    let missing = draft(&[
        ("source", &source_pin),
        ("policy", "replace"),
        ("mapping_operation", "keep"),
    ]);
    let missing = s.req("POST", &url, &missing, &s.owner).await;
    assert_eq!(missing.0, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(missing.2.contains("Unmapped source acquisitions"));
    assert!(!missing.2.contains("name=\"review_fingerprint\""));
    let before = s.audits().await;
    let p = s.req("POST", &url, &v, &s.owner).await;
    assert_eq!(p.0, StatusCode::OK);
    for label in [
        "Old shared source",
        "Current local learnset",
        "New shared source",
        "Proposed learnset",
        "Unmapped source acquisitions",
        "Removed local acquisitions",
    ] {
        assert!(p.2.contains(label));
    }
    assert!(!p.2.contains(&format!("Canonical learnset ID: {}", PROVISIONAL_ID)));
    v.insert(
        "review_fingerprint".into(),
        input(&p.2, "review_fingerprint"),
    );
    v.insert("intent".into(), "save".into());
    v.insert("confirm".into(), "yes".into());
    v.insert("policy".into(), "replace".into());
    let fresh = s.req("POST", &url, &v, &s.owner).await;
    assert_eq!(fresh.0, StatusCode::OK);
    assert!(
        fresh
            .2
            .contains("Nothing was saved. Review this fresh preview")
    );
    assert_eq!(before, s.audits().await);
    v.insert(
        "review_fingerprint".into(),
        input(&fresh.2, "review_fingerprint"),
    );
    assert_eq!(
        s.req("POST", &url, &v, &s.owner).await.0,
        StatusCode::SEE_OTHER
    );
    let local = s
        .state
        .learning
        .learnset(Some(&s.world), &s.variant)
        .await
        .unwrap()
        .unwrap();
    let records = local.records.unwrap();
    assert_eq!(records.len(), 3);
    for (a, b) in source.records.unwrap().iter().zip(&records) {
        assert_eq!(a.id, b.id);
        assert_eq!(a.threshold, b.threshold);
        assert_eq!(a.condition, b.condition);
        assert_eq!(b.move_record.id, s.local.id);
    }
    // Same-looking local moves without matching explicit source identity cannot map.
    let wrong = new_move(&s.state, &s.owner, Some(&s.world), "Unrelated", None).await;
    let wrong_pin = format!("{}@1", wrong.id);
    let bad = draft(&[
        ("expected_revision", "1"),
        ("source", &source_pin),
        ("policy", "keep"),
        ("mapping_operation", "set"),
        ("shared_move", &s.shared.id),
        ("local_move", &wrong_pin),
    ]);
    assert_eq!(
        s.req("POST", &url, &bad, &s.owner).await.0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        s.state
            .learning
            .learnset(Some(&s.world), &s.variant)
            .await
            .unwrap()
            .unwrap()
            .revision,
        1
    );
}

#[tokio::test]
async fn legacy_policy_is_retained_until_a_bound_explicit_revalidation_is_confirmed() {
    let s = site().await;
    let set = s.seed_local(0).await;
    let mut rules = rules_edit(1);
    rules.max_moves = None;
    rules.repeat_policy = RepeatPolicy::Unspecified;
    let rules = s
        .state
        .moves
        .save_ruleset(&s.owner, &s.world, rules)
        .await
        .unwrap();
    let mut metadata = fields("Legacy policy");
    metadata.slug = "legacy-policy".into();
    let legacy = MovesetRecord {
        id: "77000000000000000000000000000001".into(),
        world_id: s.world.clone(),
        variant_id: s.variant.clone(),
        fields: metadata,
        revision: 1,
        archived_at: None,
        learnset: set.pin(),
        ruleset: rules.pin(),
        entries: vec![],
        validator_version: 1,
    };
    let mut tx = persistence::begin_write(s.state.accounts.pool())
        .await
        .unwrap();
    s.state
        .learning
        .seed_preset_in(&mut tx, &legacy)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let url = format!("{}/{}/edit", s.preset_url(), legacy.id);
    let get = s.req("GET", &url, &Values::new(), &s.owner).await;
    assert_eq!(get.0, StatusCode::OK);
    assert!(get.2.contains("Keep saved policy 1"));
    let mut d = draft(&[
        ("expected_revision", "1"),
        ("display_name", "Legacy policy"),
        ("description", "Retain original diagnostics"),
        ("learnset", &format!("{}@1", set.id)),
        ("ruleset", &format!("{}@2", rules.id)),
        ("validation_policy", "current"),
        ("intent", "preview"),
    ]);
    let before = s.audits().await;
    let preview = s.req("POST", &url, &d, &s.owner).await;
    assert_eq!(preview.0, StatusCode::OK);
    assert!(preview.2.contains("Validation report · policy 2"));
    let proposed_incomplete = preview
        .2
        .rsplit_once("<h2>Incomplete inputs</h2>")
        .unwrap()
        .1
        .split("</section>")
        .next()
        .unwrap();
    assert!(proposed_incomplete.contains("None reported"));
    assert!(!proposed_incomplete.contains("no moveset maximum"));
    assert_eq!(s.audits().await, before);
    d.insert(
        "review_fingerprint".into(),
        input(&preview.2, "review_fingerprint"),
    );
    d.insert("confirm".into(), "yes".into());
    d.insert("intent".into(), "save".into());
    d.insert("validation_policy".into(), "saved".into());
    let changed = s.req("POST", &url, &d, &s.owner).await;
    assert_eq!(changed.0, StatusCode::OK);
    assert!(
        changed
            .2
            .contains("Nothing was saved. Review this fresh preview")
    );
    assert_eq!(s.audits().await, before);
    assert_eq!(
        s.state
            .learning
            .preset(&s.world, &s.variant, &legacy.id)
            .await
            .unwrap(),
        legacy
    );
    d.insert("validation_policy".into(), "current".into());
    assert_eq!(
        s.confirmed(&url, &d, &s.owner).await.0,
        StatusCode::SEE_OTHER
    );
    let saved = s
        .state
        .learning
        .preset(&s.world, &s.variant, &legacy.id)
        .await
        .unwrap();
    assert_eq!((saved.revision, saved.validator_version), (2, 2));
    assert_eq!(saved.learnset, legacy.learnset);
    assert_eq!(saved.ruleset, legacy.ruleset);
    assert_eq!(s.audits().await, before + 1);
    let old = s
        .req(
            "GET",
            &format!("{}/{}/history/1", s.preset_url(), legacy.id),
            &Values::new(),
            "",
        )
        .await;
    assert_eq!(old.0, StatusCode::OK);
    assert!(old.2.contains("Validation report · policy 1"));
    assert!(old.2.contains("The ruleset has no moveset maximum."));
    let current = s
        .req(
            "GET",
            &format!("{}/{}", s.preset_url(), legacy.id),
            &Values::new(),
            "",
        )
        .await;
    assert_eq!(current.0, StatusCode::OK);
    assert!(current.2.contains("Validation report · policy 2"));
    assert!(!current.2.contains("The ruleset has no moveset maximum."));
}

#[tokio::test]
async fn preset_preview_multiple_names_repeats_reports_and_historical_pins() {
    let s = site().await;
    let l = s.seed_local(2).await;
    let records = l.records.as_ref().unwrap();
    let learnset = format!("{}@1", l.id);
    let ruleset = format!("{}@1", s.rules.id);
    let url = format!("{}/new", s.preset_url());
    let v = draft(&[
        ("display_name", "First preset"),
        ("description", "Inert condition"),
        ("learnset", &learnset),
        ("ruleset", &ruleset),
        ("operation", "add"),
        ("record", &records[0].id),
    ]);
    let count = s.audits().await;
    let p = s.req("POST", &url, &v, &s.owner).await;
    assert_eq!(p.0, StatusCode::OK);
    for text in [
        "Incomplete inputs",
        "Contradictions",
        "Checks not evaluated",
        "Exact validation inputs",
        "not combat readiness",
        "policy 2",
    ] {
        assert!(p.2.contains(text), "missing {text}");
    }
    assert_eq!(count, s.audits().await);
    assert!(!p.2.contains(&format!("Canonical preset ID: {}", PROVISIONAL_ID)));
    let mut changed = v.clone();
    changed.insert(
        "review_fingerprint".into(),
        input(&p.2, "review_fingerprint"),
    );
    changed.insert("intent".into(), "save".into());
    changed.insert("confirm".into(), "yes".into());
    changed.insert("display_name".into(), "Changed after preview".into());
    let fresh = s.req("POST", &url, &changed, &s.owner).await;
    assert_eq!(fresh.0, StatusCode::OK);
    assert!(
        fresh
            .2
            .contains("Nothing was saved. Review this fresh preview")
    );
    assert_eq!(count, s.audits().await);
    let create = s.confirmed(&url, &v, &s.owner).await;
    assert_eq!(create.0, StatusCode::SEE_OTHER);
    let preset = s
        .state
        .learning
        .presets(&s.world, &s.variant)
        .await
        .unwrap()
        .remove(0);
    let edit = format!("{}/{}/edit", s.preset_url(), preset.id);
    let mut repeat = v.clone();
    repeat.insert("expected_revision".into(), "1".into());
    let preview = s.req("POST", &edit, &repeat, &s.owner).await;
    assert_eq!(preview.0, StatusCode::OK);
    assert!(preview.2.contains("repeats are forbidden"));
    assert_eq!(
        s.confirmed(&edit, &repeat, &s.owner).await.0,
        StatusCode::SEE_OTHER
    );
    let mut stale = repeat.clone();
    stale.insert("learnset".into(), "malformed-source@pin".into());
    stale.insert("ruleset".into(), "malformed-ruleset@pin".into());
    let conflict = s.req("POST", &edit, &stale, &s.owner).await;
    assert_eq!(conflict.0, StatusCode::CONFLICT);
    assert!(conflict.2.contains("malformed-source@pin"));
    assert!(conflict.2.contains("malformed-ruleset@pin"));
    let old_report = s
        .state
        .learning
        .validation(&s.world, &s.variant, &preset.id, Some(2))
        .await
        .unwrap();
    s.state
        .learning
        .save_learnset(
            &s.owner,
            Some(&s.world),
            &s.variant,
            LearnsetEdit {
                expected_revision: 1,
                records: Some(vec![]),
                reason: None,
            },
        )
        .await
        .unwrap();
    let mut rules = rules_edit(1);
    rules.max_moves = Some(99);
    rules.repeat_policy = RepeatPolicy::Allow;
    s.state
        .moves
        .save_ruleset(&s.owner, &s.world, rules)
        .await
        .unwrap();
    assert_eq!(
        old_report,
        s.state
            .learning
            .validation(&s.world, &s.variant, &preset.id, Some(2))
            .await
            .unwrap()
    );
    let detail = s
        .req(
            "GET",
            &format!("{}/{}/history/2", s.preset_url(), preset.id),
            &Values::new(),
            "",
        )
        .await;
    assert_eq!(detail.0, StatusCode::OK);
    assert!(detail.2.contains("repeats are forbidden"));
    assert!(detail.2.contains(&learnset));
    let mut second = v.clone();
    second.insert("display_name".into(), "Second preset".into());
    assert_eq!(
        s.confirmed(&url, &second, &s.owner).await.0,
        StatusCode::SEE_OTHER
    );
    assert_eq!(
        s.state
            .learning
            .presets(&s.world, &s.variant)
            .await
            .unwrap()
            .len(),
        2
    );
    let other = s
        .state
        .authoring
        .create_variant(&s.owner, &s.world, 2, None, fields("Other variant"), None)
        .await
        .unwrap();
    let other_l = s
        .state
        .learning
        .save_learnset(
            &s.owner,
            Some(&s.world),
            &other.id,
            LearnsetEdit {
                expected_revision: 0,
                records: Some(vec![]),
                reason: None,
            },
        )
        .await
        .unwrap();
    second.insert("learnset".into(), format!("{}@1", other_l.id));
    assert_eq!(
        s.req("POST", &url, &second, &s.owner).await.0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
}

#[tokio::test]
async fn bounded_operations_reject_off_page_targets_and_dirty_navigation() {
    let s = site().await;
    let l = s.seed_shared(28).await;
    let records = l.records.as_ref().unwrap();
    let url = s.form_url();
    let page = s.req("GET", &url, &Values::new(), &s.admin).await;
    assert_eq!(page.0, StatusCode::OK);
    assert!(page.2.contains(&records[24].id));
    assert!(!page.2.contains(&records[25].id));
    let v = draft(&[
        ("expected_revision", "1"),
        ("operation", "remove"),
        ("record", &records[27].id),
    ]);
    assert_eq!(
        s.req("POST", &url, &v, &s.admin).await.0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let v = draft(&[
        ("expected_revision", "1"),
        ("page", "2"),
        ("operation", "remove"),
        ("record", &records[27].id),
    ]);
    assert_eq!(
        s.confirmed(&url, &v, &s.admin).await.0,
        StatusCode::SEE_OTHER
    );
    assert_eq!(
        s.state
            .learning
            .learnset(None, &s.form)
            .await
            .unwrap()
            .unwrap()
            .records
            .unwrap()
            .len(),
        27
    );
    let v = draft(&[
        ("expected_revision", "2"),
        ("intent", "next"),
        ("threshold", "1e2"),
        ("baseline", &input(&page.2, "baseline")),
    ]);
    let r = s.req("POST", &url, &v, &s.admin).await;
    assert_eq!(r.0, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(r.2.contains("Save the current page before navigation"));
    assert!(r.2.contains("1e2"));
    let history = s
        .req(
            "GET",
            &url.replace("/edit", "/history/1?page=2"),
            &Values::new(),
            "",
        )
        .await;
    assert_eq!(history.0, StatusCode::OK);
    assert!(history.2.contains(&records[27].id));
    assert!(!history.2.contains(&records[0].id));
}

#[tokio::test]
async fn corrupt_pinned_references_fail_closed_on_public_pages() {
    let s = site().await;
    let row = s.seed_shared(1).await;
    // Deliberate corruption only in this disposable test-owned database.
    sqlx::query("DROP TRIGGER learnset_versions_sealed_update")
        .execute(s.state.accounts.pool())
        .await
        .unwrap();
    sqlx::query("UPDATE learnset_versions SET after_json=json_set(after_json,'$.content.records[0].move_record.id','ffffffffffffffffffffffffffffffff') WHERE learnset_id=? AND revision=1")
        .bind(&row.id)
        .execute(s.state.accounts.pool())
        .await
        .unwrap();
    let r = s
        .req(
            "GET",
            &s.form_url().replace("/edit", ""),
            &Values::new(),
            "",
        )
        .await;
    assert_eq!(r.0, StatusCode::SERVICE_UNAVAILABLE);
    assert!(!r.2.contains("Unmodeled"));
}

#[tokio::test]
async fn field_scoped_unicode_transport_preserves_other_texts_and_guards_loading() {
    let s = site().await;
    let method = "法".repeat(1300);
    let context = "場".repeat(1300);
    let mut r = record(s.shared.pin(), 0);
    r.method = Some(method.clone());
    r.context = Some(context.clone());
    r.condition = Some("旧".repeat(1300));
    let l = s
        .state
        .learning
        .save_learnset(
            &s.admin,
            None,
            &s.form,
            LearnsetEdit {
                expected_revision: 0,
                records: Some(vec![r]),
                reason: None,
            },
        )
        .await
        .unwrap();
    let id = &l.records.as_ref().unwrap()[0].id;
    let url = s.form_url();
    let page = s
        .req(
            "GET",
            &format!("{url}?record={id}"),
            &Values::new(),
            &s.admin,
        )
        .await;
    assert_eq!(page.0, StatusCode::OK);
    assert_eq!(page.2.matches("<textarea ").count(), 1);
    assert!(page.2.contains(&context));
    let mut v = native_fields(&page.2);
    v.insert("next_text_field".into(), "condition".into());
    v.insert("load_text".into(), "yes".into());
    let load = s.req("POST", &url, &v, &s.admin).await;
    assert_eq!(load.0, StatusCode::SEE_OTHER, "{}", load.2);
    let page = s
        .req(
            "GET",
            load.1[header::LOCATION].to_str().unwrap(),
            &Values::new(),
            &s.admin,
        )
        .await;
    let mut v = native_fields(&page.2);
    let condition = "界".repeat(1300);
    v.insert("text".into(), condition.clone());
    v.insert("reason".into(), "Retain raw Unicode draft".into());
    v.insert("next_text_field".into(), "context".into());
    v.insert("load_text".into(), "yes".into());
    let dirty = s.req("POST", &url, &v, &s.admin).await;
    assert_eq!(dirty.0, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(dirty.2.contains(&condition));
    assert!(dirty.2.contains("Retain raw Unicode draft"));
    assert!(dirty.2.contains("Save the current page before navigation"));
    v.remove("load_text");
    v.insert("next_text_field".into(), "condition".into());
    v.insert("intent".into(), "preview".into());
    for (revision, text, reason) in [
        (1, condition, "Unicode condition".to_owned()),
        (2, "🦀".repeat(2000), "🦉".repeat(500)),
    ] {
        if revision == 2 {
            let page = s
                .req(
                    "GET",
                    &format!("{url}?record={id}&text_field=condition"),
                    &Values::new(),
                    &s.admin,
                )
                .await;
            v = native_fields(&page.2);
        }
        v.insert("text".into(), text.clone());
        v.insert("reason".into(), reason);
        v.insert("intent".into(), "preview".into());
        assert!(
            encoded(&v).len() < 32 * 1024,
            "legal native preview must fit"
        );
        let preview = s.req("POST", &url, &v, &s.admin).await;
        assert_eq!(preview.0, StatusCode::OK, "{}", preview.2);
        let mut confirm = native_fields(&preview.2);
        confirm.insert("intent".into(), "save".into());
        confirm.insert("confirm".into(), "yes".into());
        assert!(
            encoded(&confirm).len() < 32 * 1024,
            "legal native confirmation must fit"
        );
        assert_eq!(
            s.req("POST", &url, &confirm, &s.admin).await.0,
            StatusCode::SEE_OTHER
        );
        let saved = s
            .state
            .learning
            .learnset(None, &s.form)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(saved.revision, revision + 1);
        let r = &saved.records.unwrap()[0];
        assert_eq!(r.method.as_ref(), Some(&method));
        assert_eq!(r.context.as_ref(), Some(&context));
        assert_eq!(r.condition.as_ref(), Some(&text));
    }
    let page = s
        .req(
            "GET",
            &format!("{url}?record={id}&text_field=condition"),
            &Values::new(),
            &s.admin,
        )
        .await;
    let mut changed = native_fields(&page.2);
    changed.insert("intent".into(), "preview".into());
    let preview = s.req("POST", &url, &changed, &s.admin).await;
    assert_eq!(preview.0, StatusCode::OK);
    changed.insert(
        "review_fingerprint".into(),
        input(&preview.2, "review_fingerprint"),
    );
    changed.insert("text_field".into(), "method".into());
    changed.insert("intent".into(), "save".into());
    changed.insert("confirm".into(), "yes".into());
    let count = s.audits().await;
    let fresh = s.req("POST", &url, &changed, &s.admin).await;
    assert_eq!(fresh.0, StatusCode::OK);
    assert!(
        fresh
            .2
            .contains("Nothing was saved. Review this fresh preview")
    );
    assert_eq!(s.audits().await, count);
    let mut illegal = v.clone();
    illegal.insert("expected_revision".into(), "3".into());
    illegal.insert("context".into(), "Unexpected raw context".into());
    let rejected = s.req("POST", &url, &illegal, &s.admin).await;
    assert_eq!(rejected.0, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(rejected.2.contains("Unexpected raw context"));
    assert!(!rejected.2.contains("name=\"context\""));
    illegal.remove("context");
    illegal.insert("text".into(), "界".repeat(2001));
    illegal.insert("reason".into(), "Retain over-budget draft".into());
    let rejected = s.req("POST", &url, &illegal, &s.admin).await;
    assert_eq!(rejected.0, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(rejected.2.contains(&"界".repeat(2001)));
    assert!(rejected.2.contains("Retain over-budget draft"));
    let new = draft(&[
        ("expected_revision", "3"),
        ("operation", "add"),
        ("move", &format!("{}@1", s.shared.id)),
        ("text_field", "condition"),
        ("text", "New inert condition"),
    ]);
    assert_eq!(
        s.confirmed(&url, &new, &s.admin).await.0,
        StatusCode::SEE_OTHER
    );
    let rows = s
        .state
        .learning
        .learnset(None, &s.form)
        .await
        .unwrap()
        .unwrap()
        .records
        .unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1].method, None);
    assert_eq!(rows[1].context, None);
    assert_eq!(rows[1].condition.as_deref(), Some("New inert condition"));
}

#[tokio::test]
async fn mapping_only_learnset_history_has_a_second_readable_page() {
    let s = site().await;
    let mut records = vec![];
    let mut mappings = vec![];
    for n in 0..26 {
        let shared = new_move(
            &s.state,
            &s.admin,
            None,
            &format!("Mapping shared {n:02}"),
            None,
        )
        .await;
        let local = new_move(
            &s.state,
            &s.owner,
            Some(&s.world),
            &format!("Mapping local {n:02}"),
            Some(shared.pin()),
        )
        .await;
        records.push(record(shared.pin(), n));
        mappings.push(MoveMapping {
            shared_move_id: shared.id,
            local_move: local.pin(),
        });
    }
    let source = s
        .state
        .learning
        .save_learnset(
            &s.admin,
            None,
            &s.form,
            LearnsetEdit {
                expected_revision: 0,
                records: Some(records),
                reason: None,
            },
        )
        .await
        .unwrap();
    s.seed_local(0).await;
    let local = s
        .state
        .learning
        .adopt_learnset(
            &s.owner,
            &s.world,
            &s.variant,
            LearnsetAdoptEdit {
                expected_revision: 1,
                source: source.pin(),
                apply_source: false,
                mappings,
                reason: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(local.records, Some(vec![]));
    assert_eq!(local.mappings.len(), 26);
    let url = format!("{}/history/2", s.local_url());
    let page = s.req("GET", &url, &Values::new(), "").await;
    assert_eq!(page.0, StatusCode::OK);
    assert!(page.2.contains("Next page"));
    assert_eq!(page.2.matches("Mapping: ").count(), 25);
    let page = s
        .req("GET", &format!("{url}?page=2"), &Values::new(), "")
        .await;
    assert_eq!(page.0, StatusCode::OK);
    assert_eq!(page.2.matches("Mapping: ").count(), 1);
    assert!(page.2.contains(&local.mappings[25].shared_move_id));
    assert!(page.2.contains("Previous page"));
}

/// The browser suite needs >25 records to exercise bounded navigation. This uses
/// real services and test-owned files only; it is not an application command.
#[tokio::test]
#[ignore = "explicit browser-fixture setup; requires a guarded test-owned database"]
async fn seed_learning_browser() {
    let raw =
        std::env::var_os("KARTA_LEARNING_BROWSER_DATABASE").expect("test database path required");
    let path = std::path::PathBuf::from(raw).canonicalize().unwrap();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/browser-tests")
        .canonicalize()
        .unwrap();
    assert!(path.starts_with(&root));
    assert_eq!(path.file_name().unwrap(), "auth.db");
    assert!(
        path.parent()
            .unwrap()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("karta-browser-")
    );
    let pool = persistence::connect(&format!("sqlite://{}", path.display()))
        .await
        .unwrap();
    let accounts = Accounts::new(pool, 3600).unwrap();
    let actor = accounts
        .sign_in(Subject {
            provider: "placeholder".into(),
            provider_subject: "librarian".into(),
            display_name: "librarian".into(),
        })
        .await
        .unwrap();
    let token = accounts.start_session(&actor.user_id).await.unwrap().0;
    let state = AppState::new(accounts, None, "http://127.0.0.1:3000".into(), false);
    let form = state
        .authoring
        .create_species(&token, fields("Learning creature"), None)
        .await
        .unwrap()
        .default_form_id;
    let world = state
        .authoring
        .create_world(&token, fields("Learning world"), None)
        .await
        .unwrap();
    let variant = state
        .authoring
        .create_variant(&token, &world.id, 1, None, fields("Learning variant"), None)
        .await
        .unwrap();
    let shared = new_move(&state, &token, None, "Shared learning move", None).await;
    let local = new_move(
        &state,
        &token,
        Some(&world.id),
        "Local learning move",
        Some(shared.pin()),
    )
    .await;
    state
        .moves
        .save_ruleset(&token, &world.id, rules_edit(0))
        .await
        .unwrap();
    for (scope, owner, pin) in [
        (None, form, shared.pin()),
        (Some(world.id.as_str()), variant.id, local.pin()),
    ] {
        state
            .learning
            .save_learnset(
                &token,
                scope,
                &owner,
                LearnsetEdit {
                    expected_revision: 0,
                    records: Some((0..28).map(|n| record(pin.clone(), n)).collect()),
                    reason: None,
                },
            )
            .await
            .unwrap();
    }
    state.accounts.sign_out(&token).await.unwrap();
}
