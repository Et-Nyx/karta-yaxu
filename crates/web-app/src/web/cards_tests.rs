//! HTTP contracts for the isolated ID-bound Cards router.
use super::*;
use crate::{
    application::cards::*,
    auth::{Role, Subject},
    persistence,
};
use axum::body::{Body, to_bytes};
use std::collections::BTreeMap;
use tower::ServiceExt;

type Values = BTreeMap<String, String>;
struct Site {
    app: Router,
    api: Cards,
    authoring: Authoring,
    admin: String,
    editor: String,
    _dir: tempfile::TempDir,
}
async fn account(accounts: &Accounts, name: &str, role: Role) -> String {
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
async fn site() -> Site {
    let dir = tempfile::tempdir().unwrap();
    let pool = persistence::connect_for_migration(&format!(
        "sqlite://{}/cards-http.db",
        dir.path().display()
    ))
    .await
    .unwrap();
    persistence::migrate(&pool).await.unwrap();
    let accounts = Accounts::new(pool, 3600).unwrap();
    let admin = account(&accounts, "admin", Role::Administrator).await;
    let editor = account(&accounts, "editor", Role::Editor).await;
    let api = Cards::new(accounts.clone());
    let state = AppState::new(accounts, None, "http://127.0.0.1:3000".into(), false);
    Site {
        authoring: state.authoring.clone(),
        app: cards::routes().with_state(state),
        api,
        admin,
        editor,
        _dir: dir,
    }
}
fn fields(name: &str) -> game_data::authoring::ContentFields {
    game_data::authoring::ContentFields {
        slug: String::new(),
        display_name: name.into(),
        description: "Inert description".into(),
    }
}
fn encode(d: &Values) -> String {
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
fn draft(revision: i64, op: &str, extra: &[(&str, &str)]) -> Values {
    let mut d = Values::from([
        ("csrf".into(), "a".repeat(64)),
        ("expected_revision".into(), revision.to_string()),
        ("operation".into(), op.into()),
        ("intent".into(), "save".into()),
        ("reason".into(), "Keep raw review reason".into()),
    ]);
    for (k, v) in extra {
        d.insert((*k).into(), (*v).into());
    }
    d
}
fn input(html: &str, name: &str) -> String {
    html.split("<input ")
        .find_map(|tag| {
            if tag.contains(&format!("name=\"{name}\"")) {
                Some(
                    tag.split_once("value=\"")
                        .unwrap()
                        .1
                        .split('"')
                        .next()
                        .unwrap()
                        .to_owned(),
                )
            } else {
                None
            }
        })
        .unwrap_or_else(|| panic!("missing {name}: {html}"))
}
impl Site {
    async fn request(
        &self,
        method: &str,
        url: &str,
        token: &str,
        body: String,
    ) -> (StatusCode, HeaderMap, String) {
        let mut builder = axum::http::Request::builder().method(method).uri(url);
        if !token.is_empty() {
            builder = builder.header(
                header::COOKIE,
                format!("karta_session={token}; karta_csrf={}", "a".repeat(64)),
            );
        }
        if method == "POST" {
            builder = builder
                .header(header::ORIGIN, "http://127.0.0.1:3000")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded");
        }
        let response = self
            .app
            .clone()
            .oneshot(builder.body(Body::from(body)).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let headers = response.headers().clone();
        let text = String::from_utf8(
            to_bytes(response.into_body(), 1024 * 1024)
                .await
                .unwrap()
                .to_vec(),
        )
        .unwrap();
        (status, headers, text)
    }
    async fn post(&self, url: &str, token: &str, d: &Values) -> (StatusCode, HeaderMap, String) {
        self.request("POST", url, token, encode(d)).await
    }
    async fn card(&self, name: &str) -> CardRecord {
        self.api
            .create(
                &self.admin,
                None,
                NewCard {
                    fields: fields(name),
                    source: None,
                    component: CardComponent::default(),
                    reason: None,
                },
            )
            .await
            .unwrap()
    }
    async fn field(&self, name: &str, kind: FieldKind) -> FieldDefinition {
        self.api
            .create_field(
                &self.admin,
                None,
                NewField {
                    fields: fields(name),
                    source: None,
                    kind,
                    reason: None,
                },
            )
            .await
            .unwrap()
    }
}
fn url(card: &CardRecord) -> String {
    format!(
        "/cards/{}/{}",
        card.identity.namespace.as_str(),
        card.identity.id
    )
}

#[tokio::test]
async fn native_textarea_transport_preserves_paragraphs_as_lf_for_rules_and_field_values() {
    let s = site().await;
    let c = s.card("Paragraph card").await;
    let endpoint = format!("{}/edit", url(&c));
    let rules = draft(
        1,
        "rules_text",
        &[("state", "defined"), ("text", "First\r\nSecond\rThird")],
    );
    assert_eq!(
        s.post(&endpoint, &s.admin, &rules).await.0,
        StatusCode::SEE_OTHER
    );
    let f = s
        .field("Paragraph value", FieldKind::Text { max_length: 2000 })
        .await;
    let d = assignment(2, &f, "text", "🐉 paragraph\r\n\r\nNext paragraph");
    assert_eq!(
        s.post(&endpoint, &s.admin, &d).await.0,
        StatusCode::SEE_OTHER
    );
    let saved = s.api.detail(&c.identity).await.unwrap();
    assert_eq!(
        saved.component.rules_text.as_deref(),
        Some("First\nSecond\nThird")
    );
    assert_eq!(
        saved.component.fields.unwrap()[0].value,
        FieldValue::Text("🐉 paragraph\n\nNext paragraph".into())
    );
}
fn assignment(rev: i64, f: &FieldDefinition, state: &str, text: &str) -> Values {
    draft(
        rev,
        "assignment",
        &[
            ("mode", "set"),
            ("field_id", &f.id),
            ("field_revision", &f.revision.to_string()),
            ("value_state", state),
            ("text", text),
            ("boolean", "false"),
            ("reference_scope", "shared"),
            ("reference_namespace", "standalone"),
            ("reference_id", ""),
            ("reference_revision", ""),
        ],
    )
}

#[tokio::test]
async fn public_get_head_history_are_cookie_free_and_origins_are_qualified() {
    let site = site().await;
    let card = site.card("<script>Card</script>").await;
    for path in [
        "/cards".into(),
        url(&card),
        format!("{}/history", url(&card)),
        format!("{}/history/1", url(&card)),
    ] {
        let (status, h, html) = site.request("GET", &path, "", String::new()).await;
        assert_eq!(status, StatusCode::OK);
        assert!(!h.contains_key(header::SET_COOKIE));
        assert!(!html.contains("<script>Card</script>"));
        let (status, h, body) = site.request("HEAD", &path, "", String::new()).await;
        assert_eq!(status, StatusCode::OK);
        assert!(!h.contains_key(header::SET_COOKIE));
        assert!(body.is_empty());
    }
    assert_eq!(
        site.request(
            "GET",
            &format!("/cards/move/{}", card.identity.id),
            "",
            String::new()
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        site.request(
            "GET",
            &format!("{}/edit?operation=rules_text", url(&card)),
            "",
            String::new()
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn authorization_csrf_allowlists_and_raw_drafts_never_mutate() {
    let site = site().await;
    let card = site.card("Protected").await;
    let path = format!("{}/edit", url(&card));
    let d = draft(
        1,
        "rules_text",
        &[
            ("state", "defined"),
            ("text", "<script>Raw unsaved 🐉</script>"),
        ],
    );
    let denied = site.post(&path, &site.editor, &d).await;
    assert_eq!(denied.0, StatusCode::FORBIDDEN);
    assert!(denied.2.contains("Raw unsaved 🐉"));
    assert!(denied.2.contains("Keep raw review reason"));
    assert!(!denied.2.contains("<script>Raw"));
    let mut csrf = d.clone();
    csrf.insert("csrf".into(), "b".repeat(64));
    assert_eq!(
        site.post(&path, &site.admin, &csrf).await.0,
        StatusCode::FORBIDDEN
    );
    let mut injected = d.clone();
    injected.insert("actor_id".into(), "untrusted actor".into());
    let rejected = site.post(&path, &site.admin, &injected).await;
    assert_eq!(rejected.0, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(rejected.2.contains("untrusted actor"));
    let huge = draft(
        1,
        "rules_text",
        &[("state", "defined"), ("text", &"a".repeat(33 * 1024))],
    );
    assert_eq!(
        site.post(&path, &site.admin, &huge).await.0,
        StatusCode::PAYLOAD_TOO_LARGE
    );
    assert_eq!(site.api.detail(&card.identity).await.unwrap(), card);
}

#[tokio::test]
async fn revision_first_raw_parse_errors_and_explicit_conflict_review() {
    let site = site().await;
    let card = site.card("Revision").await;
    let f = site
        .field("Number", FieldKind::Number(Default::default()))
        .await;
    let path = format!("{}/edit", url(&card));
    let first = draft(
        1,
        "rules_text",
        &[("state", "defined"), ("text", "Saved text")],
    );
    assert_eq!(
        site.post(&path, &site.admin, &first).await.0,
        StatusCode::SEE_OTHER
    );
    let mut d = assignment(1, &f, "number", "1e100");
    let conflict = site.post(&path, &site.admin, &d).await;
    assert_eq!(conflict.0, StatusCode::CONFLICT);
    assert!(conflict.2.contains("1e100"));
    assert!(conflict.2.contains("Saved text"));
    d.insert("reviewed_revision".into(), "2".into());
    let invalid = site.post(&path, &site.admin, &d).await;
    assert_eq!(invalid.0, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(invalid.2.contains("1e100"));
    assert_eq!(input(&invalid.2, "expected_revision"), "2");
    d.remove("reviewed_revision");
    d.insert("expected_revision".into(), "2".into());
    d.insert("text".into(), "0".into());
    assert_eq!(
        site.post(&path, &site.admin, &d).await.0,
        StatusCode::SEE_OTHER
    );
    let saved = site.api.detail(&card.identity).await.unwrap();
    assert_eq!(saved.revision, 3);
    assert_eq!(saved.component.rules_text.as_deref(), Some("Saved text"));
    assert!(
        matches!(saved.component.fields.unwrap()[0].value,FieldValue::Number(n) if n.to_string()=="0")
    );
}

#[tokio::test]
async fn five_kinds_value_states_and_exact_card_reference_are_distinct() {
    let site = site().await;
    let card = site.card("Values").await;
    let other = site.card("Referenced").await;
    let kinds = [
        FieldKind::Number(Default::default()),
        FieldKind::Text { max_length: 2000 },
        FieldKind::Boolean,
        FieldKind::Choice {
            options: vec![ChoiceOption {
                id: "1".repeat(32),
                label: "First".into(),
            }],
        },
        FieldKind::CardReference,
    ];
    let mut revision = 1;
    for (i, kind) in kinds.into_iter().enumerate() {
        let f = site.field(&format!("Field {i}"), kind).await;
        let (state, text) = [
            ("number", "0"),
            ("text", ""),
            ("boolean", ""),
            ("choice", "11111111111111111111111111111111"),
            ("card_reference", ""),
        ][i];
        let mut d = assignment(revision, &f, state, text);
        if i == 4 {
            d.insert("reference_id".into(), other.identity.id.clone());
            d.insert("reference_revision".into(), "1".into());
        }
        assert_eq!(
            site.post(&format!("{}/edit", url(&card)), &site.admin, &d)
                .await
                .0,
            StatusCode::SEE_OTHER
        );
        revision += 1;
    }
    let saved = site.api.detail(&card.identity).await.unwrap();
    let entries = saved.component.fields.as_ref().unwrap();
    assert_eq!(entries.len(), 5);
    assert_eq!(entries[1].value, FieldValue::Text(String::new()));
    assert_eq!(entries[2].value, FieldValue::Boolean(false));
    assert_eq!(entries[4].value, FieldValue::CardReference(other.pin()));
    let f = &entries[1].definition;
    let not_loaded = assignment(revision, f, "text", "erase?");
    assert_eq!(
        site.post(&format!("{}/edit", url(&card)), &site.admin, &not_loaded)
            .await
            .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let loaded = site
        .request(
            "GET",
            &format!("{}/edit?operation=assignment&target={}", url(&card), f.id),
            &site.admin,
            String::new(),
        )
        .await;
    assert_eq!(loaded.0, StatusCode::OK);
    assert_eq!(input(&loaded.2, "loaded_field_id"), f.id);
    for state in ["missing", "unknown", "not_applicable"] {
        let d = assignment(revision, f, state, "");
        assert_eq!(
            site.post(&format!("{}/edit", url(&card)), &site.admin, &d)
                .await
                .0,
            StatusCode::SEE_OTHER
        );
        revision += 1;
    }
    assert_eq!(
        site.api
            .detail(&card.identity)
            .await
            .unwrap()
            .component
            .fields
            .unwrap()[1]
            .value,
        FieldValue::NotApplicable
    );
}

#[tokio::test]
async fn paged_lists_choice_operations_and_archived_new_pins() {
    let site = site().await;
    let card = site.card("Paged").await;
    let options = (0..27)
        .map(|i| ChoiceOption {
            id: format!("{i:032x}"),
            label: format!("Option {i}"),
        })
        .collect();
    let f = site.field("Choices", FieldKind::Choice { options }).await;
    let path = format!("/card-fields/{}/edit", f.id);
    let d = draft(
        1,
        "option",
        &[
            ("mode", "set"),
            ("option_id", &format!("{:032x}", 26)),
            ("label", "Off-page replacement"),
        ],
    );
    assert_eq!(
        site.post(&path, &site.admin, &d).await.0,
        StatusCode::SEE_OTHER
    );
    let saved = site.api.field(None, &f.id).await.unwrap();
    let FieldKind::Choice { options } = &saved.kind else {
        panic!()
    };
    assert_eq!(options.len(), 27);
    assert_eq!(options[0].label, "Option 0");
    assert_eq!(options[26].label, "Off-page replacement");
    let page2 = site
        .request(
            "GET",
            &format!("/card-fields/{}?page=2", f.id),
            "",
            String::new(),
        )
        .await;
    assert!(page2.2.contains("Off-page replacement"));
    assert!(!page2.2.contains("Option 0"));
    let mut archive = draft(2, "archive", &[("archived", "yes"), ("intent", "preview")]);
    let preview = site.post(&path, &site.admin, &archive).await;
    assert_eq!(preview.0, StatusCode::OK);
    assert_eq!(site.api.field(None, &f.id).await.unwrap().archived_at, None);
    archive.insert("intent".into(), "confirm".into());
    archive.insert("digest".into(), input(&preview.2, "digest"));
    assert_eq!(
        site.post(&path, &site.admin, &archive).await.0,
        StatusCode::SEE_OTHER
    );
    let d = assignment(1, &f, "choice", &format!("{:032x}", 0));
    assert_eq!(
        site.post(&format!("{}/edit", url(&card)), &site.admin, &d)
            .await
            .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    for i in 0..26 {
        site.field(&format!("Catalog {i:02}"), FieldKind::Boolean)
            .await;
    }
    let first = site.request("GET", "/card-fields", "", String::new()).await;
    assert_eq!(first.2.matches(">View field</a>").count(), 25);
    let second = site
        .request("GET", "/card-fields?page=2", "", String::new())
        .await;
    assert_eq!(second.2.matches(">View field</a>").count(), 2);
}

#[tokio::test]
async fn scoped_creation_field_adoption_and_copy_confirm_bind_exact_proposal() {
    let site = site().await;
    let create = draft(
        0,
        "create",
        &[
            ("display_name", "HTTP standalone"),
            ("text", "Native description"),
            ("source_id", ""),
            ("source_revision", ""),
        ],
    );
    let created = site.post("/cards/new", &site.admin, &create).await;
    assert_eq!(created.0, StatusCode::SEE_OTHER);
    assert!(
        created.1[header::LOCATION]
            .to_str()
            .unwrap()
            .starts_with("/cards/standalone/")
    );
    let mut field = create.clone();
    for (k, v) in [
        ("kind", "boolean"),
        ("minimum", ""),
        ("maximum", ""),
        ("whole_only", "no"),
        ("max_length", "2000"),
    ] {
        field.insert(k.into(), v.into());
    }
    let created = site.post("/card-fields/new", &site.admin, &field).await;
    assert_eq!(created.0, StatusCode::SEE_OTHER);
    let world = site
        .authoring
        .create_world(&site.editor, fields("Local world"), None)
        .await
        .unwrap();
    let shared = site.field("Shared flag", FieldKind::Boolean).await;
    let local = site
        .api
        .create_field(
            &site.editor,
            Some(&world.id),
            NewField {
                fields: fields("Local flag"),
                kind: FieldKind::Boolean,
                source: Some(shared.pin()),
                reason: None,
            },
        )
        .await
        .unwrap();
    let mut renamed = shared.fields.clone();
    renamed.display_name = "Updated shared flag".into();
    site.api
        .save_field(
            &site.admin,
            None,
            &shared.id,
            FieldEdit {
                expected_revision: 1,
                fields: renamed,
                kind: shared.kind.clone(),
                archived_at: None,
                reason: None,
            },
        )
        .await
        .unwrap();
    let field_path = format!("/worlds/{}/card-fields/{}/edit", world.id, local.id);
    let mut adoption = draft(
        1,
        "adopt",
        &[("source_revision", "2"), ("intent", "preview")],
    );
    let preview = site.post(&field_path, &site.editor, &adoption).await;
    assert_eq!(preview.0, StatusCode::OK);
    assert!(preview.2.contains("Old shared source"));
    assert!(preview.2.contains("Updated shared flag"));
    adoption.insert("intent".into(), "confirm".into());
    adoption.insert("digest".into(), input(&preview.2, "digest"));
    assert_eq!(
        site.post(&field_path, &site.admin, &adoption).await.0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        site.post(&field_path, &site.editor, &adoption).await.0,
        StatusCode::SEE_OTHER
    );
    let adopted = site.api.field(Some(&world.id), &local.id).await.unwrap();
    assert_eq!(adopted.source.as_ref().unwrap().version, 2);
    assert_eq!(adopted.fields.display_name, "Local flag");
    let source = site
        .api
        .create(
            &site.admin,
            None,
            NewCard {
                fields: fields("Source card"),
                source: None,
                component: CardComponent {
                    fields: Some(vec![CardField {
                        definition: shared.clone(),
                        value: FieldValue::Boolean(false),
                    }]),
                    ..Default::default()
                },
                reason: None,
            },
        )
        .await
        .unwrap();
    let target = site
        .api
        .create(
            &site.editor,
            Some(&world.id),
            NewCard {
                fields: fields("Local card"),
                source: Some(source.pin()),
                component: CardComponent::default(),
                reason: None,
            },
        )
        .await
        .unwrap();
    let path = format!(
        "/worlds/{}/cards/standalone/{}/edit",
        world.id, target.identity.id
    );
    let mut copy = draft(
        1,
        "copy",
        &[
            ("shared_field_id", &shared.id),
            ("field_id", &local.id),
            ("field_revision", "2"),
            ("intent", "preview"),
        ],
    );
    let preview = site.post(&path, &site.editor, &copy).await;
    assert_eq!(preview.0, StatusCode::OK);
    assert!(preview.2.contains("Boolean false"));
    assert!(preview.2.contains("Unmapped source assignments"));
    copy.insert("digest".into(), input(&preview.2, "digest"));
    copy.insert("intent".into(), "confirm".into());
    copy.insert("reason".into(), "Changed after review".into());
    assert_eq!(
        site.post(&path, &site.editor, &copy).await.0,
        StatusCode::CONFLICT
    );
    assert_eq!(site.api.detail(&target.identity).await.unwrap(), target);
    copy.insert("reason".into(), "Keep raw review reason".into());
    assert_eq!(
        site.post(&path, &site.editor, &copy).await.0,
        StatusCode::SEE_OTHER
    );
    assert_eq!(
        site.api
            .detail(&target.identity)
            .await
            .unwrap()
            .component
            .fields
            .unwrap()[0]
            .value,
        FieldValue::Boolean(false)
    );
}

#[tokio::test]
async fn one_operation_preserves_off_page_entries_and_retained_archived_pins() {
    let site = site().await;
    let mut assignments = vec![];
    for i in 0..27 {
        assignments.push(CardField {
            definition: site
                .field(&format!("Reusable {i}"), FieldKind::Boolean)
                .await,
            value: FieldValue::Unknown,
        });
    }
    let card = site
        .api
        .create(
            &site.admin,
            None,
            NewCard {
                fields: fields("Off-page preservation"),
                source: None,
                component: CardComponent {
                    fields: Some(assignments.clone()),
                    labels: Some((0..27).map(|i| format!("Label {i}")).collect()),
                    rules_text: Some("Saved independent rules".into()),
                },
                reason: None,
            },
        )
        .await
        .unwrap();
    let pin = assignments[26].definition.clone();
    site.api
        .save_field(
            &site.admin,
            None,
            &pin.id,
            FieldEdit {
                expected_revision: 1,
                fields: pin.fields.clone(),
                kind: pin.kind.clone(),
                archived_at: Some(1),
                reason: None,
            },
        )
        .await
        .unwrap();
    let d = assignment(1, &assignments[0].definition, "boolean", "");
    assert_eq!(
        site.post(&format!("{}/edit", url(&card)), &site.admin, &d)
            .await
            .0,
        StatusCode::SEE_OTHER
    );
    let saved = site.api.detail(&card.identity).await.unwrap();
    let entries = saved.component.fields.as_ref().unwrap();
    assert_eq!(&entries[1..], &assignments[1..]);
    assert_eq!(saved.component.labels, card.component.labels);
    assert_eq!(saved.component.rules_text, card.component.rules_text);
    let second = site
        .request("GET", &format!("{}?page=2", url(&card)), "", String::new())
        .await;
    assert_eq!(second.0, StatusCode::OK);
    assert!(second.2.contains("Reusable 26"));
    assert!(!second.2.contains("Reusable 0 —"));
    let label = draft(
        2,
        "label",
        &[
            ("mode", "set"),
            ("target", "Label 26"),
            ("label", "Changed last label"),
        ],
    );
    assert_eq!(
        site.post(&format!("{}/edit", url(&card)), &site.admin, &label)
            .await
            .0,
        StatusCode::SEE_OTHER
    );
    let saved = site.api.detail(&card.identity).await.unwrap();
    let labels = saved.component.labels.unwrap();
    assert_eq!(labels.len(), 27);
    assert_eq!(labels[0], "Label 0");
    assert_eq!(labels[26], "Changed last label");
}

#[tokio::test]
async fn native_form_common_component_uses_the_owning_revision_and_native_links() {
    let site = site().await;
    let species = site
        .authoring
        .create_species(&site.admin, fields("Native creature"), None)
        .await
        .unwrap();
    let identity = CardIdentity {
        namespace: CardNamespace::Form,
        world_id: None,
        id: species.default_form_id.clone(),
    };
    let base = format!("/cards/form/{}", identity.id);
    let get = site.request("GET", &base, "", String::new()).await;
    assert_eq!(get.0, StatusCode::OK);
    assert!(get.2.contains(&format!("/forms/{}/edit", identity.id)));
    assert!(get.2.contains("Native full history"));
    let d = draft(
        1,
        "rules_text",
        &[("state", "defined"), ("text", "Native inert rules")],
    );
    assert_eq!(
        site.post(&format!("{base}/edit"), &site.admin, &d).await.0,
        StatusCode::SEE_OTHER
    );
    let saved = site.api.detail(&identity).await.unwrap();
    assert_eq!(saved.revision, 2);
    assert_eq!(
        saved.component.rules_text.as_deref(),
        Some("Native inert rules")
    );
    let old = site
        .api
        .version(&CardPin {
            identity,
            version: 1,
        })
        .await
        .unwrap();
    assert_eq!(old.component.rules_text, None);
    let metadata = draft(2, "name", &[("display_name", "Wrong editor")]);
    assert_eq!(
        site.post(&format!("{base}/edit"), &site.admin, &metadata)
            .await
            .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
}

#[tokio::test]
async fn duplicate_fields_and_unknown_or_missing_intent_retain_drafts_without_writes() {
    let site = site().await;
    let card = site.card("Strict form").await;
    let path = format!("{}/edit", url(&card));
    let mut d = draft(
        1,
        "rules_text",
        &[("state", "defined"), ("text", "First raw draft")],
    );
    for intent in [Some("unexpected"), None] {
        if let Some(intent) = intent {
            d.insert("intent".into(), intent.into());
        } else {
            d.remove("intent");
        }
        let response = site.post(&path, &site.admin, &d).await;
        assert_eq!(response.0, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(response.2.contains("First raw draft"));
    }
    d.insert("intent".into(), "save".into());
    let body = format!("{}&text=Second%20raw%20draft", encode(&d));
    let response = site.request("POST", &path, &site.admin, body).await;
    assert_eq!(response.0, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(response.2.contains("First raw draft"));
    assert!(response.2.contains("Second raw draft"));
    assert!(response.2.contains("Duplicate form fields"));
    assert_eq!(site.api.detail(&card.identity).await.unwrap(), card);
}

#[tokio::test]
async fn standalone_archive_requires_review_and_binds_state_and_reason() {
    let site = site().await;
    let card = site.card("Archive review").await;
    let path = format!("{}/edit", url(&card));
    let mut d = draft(1, "archive", &[("archived", "yes"), ("intent", "preview")]);
    let preview = site.post(&path, &site.admin, &d).await;
    assert_eq!(preview.0, StatusCode::OK);
    assert!(preview.2.contains("Proposed archive state"));
    assert_eq!(site.api.detail(&card.identity).await.unwrap(), card);
    d.insert("intent".into(), "confirm".into());
    d.insert("digest".into(), input(&preview.2, "digest"));
    d.insert("archived".into(), "no".into());
    assert_eq!(
        site.post(&path, &site.admin, &d).await.0,
        StatusCode::CONFLICT
    );
    d.insert("archived".into(), "yes".into());
    d.insert("reason".into(), "Changed review reason".into());
    assert_eq!(
        site.post(&path, &site.admin, &d).await.0,
        StatusCode::CONFLICT
    );
    assert_eq!(site.api.detail(&card.identity).await.unwrap(), card);
    d.insert("reason".into(), "Keep raw review reason".into());
    assert_eq!(
        site.post(&path, &site.admin, &d).await.0,
        StatusCode::SEE_OTHER
    );
    assert!(
        site.api
            .detail(&card.identity)
            .await
            .unwrap()
            .archived_at
            .is_some()
    );
}

#[tokio::test]
async fn readable_slugs_resolve_but_actions_remain_id_bound_and_history_has_attribution() {
    let site = site().await;
    let card = site.card("Readable standalone").await;
    let f = site.field("Readable field", FieldKind::Boolean).await;
    for (slug, canonical) in [
        (
            format!("/cards/standalone/{}", card.fields.slug),
            url(&card),
        ),
        (
            format!("/card-fields/{}", f.fields.slug),
            format!("/card-fields/{}", f.id),
        ),
    ] {
        let response = site.request("GET", &slug, "", String::new()).await;
        assert_eq!(response.0, StatusCode::OK);
        assert!(!response.1.contains_key(header::SET_COOKIE));
        let response = site
            .request(
                "GET",
                &format!("{slug}/edit?operation=name"),
                &site.admin,
                String::new(),
            )
            .await;
        assert_eq!(response.0, StatusCode::OK);
        assert!(response.2.contains(&format!("action=\"{canonical}/edit\"")));
        let d = draft(1, "name", &[("display_name", "Illegal slug action")]);
        assert_eq!(
            site.post(&format!("{slug}/edit"), &site.admin, &d).await.0,
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
    for revision in 1..=26 {
        site.api
            .save_card(
                &site.admin,
                &card.identity,
                CardEdit {
                    expected_revision: revision,
                    component: CardComponent {
                        rules_text: Some(format!("History {revision}")),
                        ..Default::default()
                    },
                    reason: Some("Attributed history reason".into()),
                },
            )
            .await
            .unwrap();
    }
    let first = site
        .request("GET", &format!("{}/history", url(&card)), "", String::new())
        .await;
    assert_eq!(first.0, StatusCode::OK);
    assert_eq!(first.2.matches("\">Revision ").count(), 25);
    assert!(first.2.contains("By admin"));
    assert!(first.2.contains("Attributed history reason"));
    let second = site
        .request(
            "GET",
            &format!("{}/history?page=2", url(&card)),
            "",
            String::new(),
        )
        .await;
    assert_eq!(second.0, StatusCode::OK);
    assert_eq!(second.2.matches("\">Revision ").count(), 2);
}

#[tokio::test]
async fn arbitrary_mapping_set_is_preserved_reviewed_and_bound_without_a_one_or_25_cap() {
    let site = site().await;
    let world = site
        .authoring
        .create_world(&site.editor, fields("Many mappings"), None)
        .await
        .unwrap();
    let mut shared = vec![];
    let mut local = vec![];
    for i in 0..29 {
        let s = site
            .field(&format!("Shared mapping {i}"), FieldKind::Boolean)
            .await;
        if i < 27 {
            local.push(
                site.api
                    .create_field(
                        &site.editor,
                        Some(&world.id),
                        NewField {
                            fields: fields(&format!("Local mapping {i}")),
                            kind: FieldKind::Boolean,
                            source: Some(s.pin()),
                            reason: None,
                        },
                    )
                    .await
                    .unwrap(),
            );
        }
        shared.push(s);
    }
    let source = site
        .api
        .create(
            &site.admin,
            None,
            NewCard {
                fields: fields("Many source fields"),
                source: None,
                component: CardComponent {
                    fields: Some(
                        shared
                            .iter()
                            .map(|definition| CardField {
                                definition: definition.clone(),
                                value: FieldValue::Boolean(false),
                            })
                            .collect(),
                    ),
                    ..Default::default()
                },
                reason: None,
            },
        )
        .await
        .unwrap();
    let target = site
        .api
        .create(
            &site.editor,
            Some(&world.id),
            NewCard {
                fields: fields("Mapped target"),
                source: Some(source.pin()),
                component: CardComponent::default(),
                reason: None,
            },
        )
        .await
        .unwrap();
    let mappings = shared
        .iter()
        .zip(&local)
        .map(|(s, l)| format!("{} {} 1", s.id, l.id))
        .collect::<Vec<_>>()
        .join("\n");
    let path = format!(
        "/worlds/{}/cards/standalone/{}/edit",
        world.id, target.identity.id
    );
    let mut d = draft(
        1,
        "copy",
        &[
            ("mappings", &mappings),
            ("intent", "preview"),
            ("page", "2"),
        ],
    );
    assert!(encode(&d).len() < 32 * 1024);
    let preview = site.post(&path, &site.editor, &d).await;
    assert_eq!(preview.0, StatusCode::OK);
    assert!(preview.2.contains("defined, 27 entries"));
    assert!(preview.2.contains("Local mapping 26"));
    assert!(
        preview
            .2
            .contains("Unmapped source assignments (2 total; page 2")
    );
    assert!(preview.2.contains(&mappings));
    d.insert("intent".into(), "confirm".into());
    d.insert("digest".into(), input(&preview.2, "digest"));
    d.insert(
        "mappings".into(),
        mappings.lines().take(26).collect::<Vec<_>>().join("\n"),
    );
    assert_eq!(
        site.post(&path, &site.editor, &d).await.0,
        StatusCode::CONFLICT
    );
    assert_eq!(site.api.detail(&target.identity).await.unwrap(), target);
    d.insert("mappings".into(), format!("{} malformed", shared[0].id));
    let invalid = site.post(&path, &site.editor, &d).await;
    assert_eq!(invalid.0, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(invalid.2.contains("malformed"));
    d.insert("mappings".into(), mappings);
    assert_eq!(
        site.post(&path, &site.editor, &d).await.0,
        StatusCode::SEE_OTHER
    );
    assert_eq!(
        site.api
            .detail(&target.identity)
            .await
            .unwrap()
            .component
            .fields
            .unwrap()
            .len(),
        27
    );
}
