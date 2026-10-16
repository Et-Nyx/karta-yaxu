//! Public-router regressions for independent move/configuration/learning addresses.
//! Content setup uses public services; requests never start a listening server.
mod common;

use axum::{
    body::Body,
    http::{HeaderMap, Request, StatusCode, header},
};
use game_data::{
    authoring::ContentFields,
    learning::{LearnsetRecord, MovesetRecord},
    moves::{
        AttributePin, MoveAttributeDefinition, MovePin, MoveRecord, NumericConstraints,
        NumericValue,
    },
    rulesets::{RepeatPolicy, RulesetRecord},
};
use std::collections::BTreeMap;
use web_app::{
    application::{
        authoring::{
            Authoring, Edit, FormRecord, SourcePin, SpeciesRecord, VariantRecord, WorldRecord,
        },
        learning::{Learning, LearningRecordInput, LearnsetEdit, MovesetEdit},
        moves::{
            AttributeEdit, AttributeValueInput, ExpectationInput, MoveEdit, Moves, NewAttribute,
            NewMove, RulesetEdit,
        },
    },
    auth::{Role, Subject},
};

type Values = BTreeMap<String, String>;
const QUERY: &str = "?page=1&opaque=%2f%25%26&opaque=two+words";
const CSRF: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
#[derive(Clone, Copy, Debug)]
enum Object {
    SharedAttribute,
    LocalAttribute,
    SharedMove,
    LocalMove,
    Preset,
}
const OBJECTS: [Object; 5] = [
    Object::SharedAttribute,
    Object::LocalAttribute,
    Object::SharedMove,
    Object::LocalMove,
    Object::Preset,
];

fn fields(name: &str) -> ContentFields {
    ContentFields {
        slug: "ignored-address".into(),
        display_name: name.into(),
        description: "Address regression content".into(),
    }
}
fn metadata_edit(f: &ContentFields, revision: i64, name: &str) -> Edit {
    Edit {
        expected_revision: revision,
        fields: ContentFields {
            display_name: name.into(),
            ..f.clone()
        },
        reason: None,
    }
}
async fn attribute(
    moves: &Moves,
    token: &str,
    world: Option<&str>,
    name: &str,
    source: Option<AttributePin>,
) -> MoveAttributeDefinition {
    moves
        .create_attribute(
            token,
            world,
            NewAttribute {
                fields: fields(name),
                unit: "points".into(),
                constraints: NumericConstraints::default(),
                source,
                reason: None,
            },
        )
        .await
        .unwrap()
}
async fn movement(
    moves: &Moves,
    token: &str,
    world: Option<&str>,
    name: &str,
    definition: &MoveAttributeDefinition,
    source: Option<MovePin>,
) -> MoveRecord {
    moves
        .create(
            token,
            world,
            NewMove {
                fields: fields(name),
                source,
                attributes: Some(vec![AttributeValueInput {
                    attribute: definition.pin(),
                    value: NumericValue::Unknown,
                }]),
                reason: None,
            },
        )
        .await
        .unwrap()
}
fn preset_edit(name: &str, set: &LearnsetRecord, rules: &RulesetRecord) -> MovesetEdit {
    MovesetEdit {
        expected_revision: 0,
        fields: fields(name),
        learnset: set.pin(),
        ruleset: rules.pin(),
        entries: vec![set.records.as_ref().unwrap()[0].id.clone()],
        reason: None,
    }
}
async fn learn(
    learning: &Learning,
    token: &str,
    world: Option<&str>,
    owner: &str,
    m: &MoveRecord,
) -> LearnsetRecord {
    learning
        .save_learnset(
            token,
            world,
            owner,
            LearnsetEdit {
                expected_revision: 0,
                records: Some(vec![LearningRecordInput {
                    id: String::new(),
                    move_pin: m.pin(),
                    method: None,
                    context: None,
                    condition: None,
                    threshold: NumericValue::Unknown,
                }]),
                reason: None,
            },
        )
        .await
        .unwrap()
}
struct Site {
    app: common::TestApp,
    author: Authoring,
    moves: Moves,
    learning: Learning,
    token: String,
    species: SpeciesRecord,
    other_species: SpeciesRecord,
    form: FormRecord,
    world: WorldRecord,
    other_world: WorldRecord,
    variant: VariantRecord,
    sibling: VariantRecord,
    foreign: VariantRecord,
    shared_attribute: MoveAttributeDefinition,
    local_attribute: MoveAttributeDefinition,
    shared_move: MoveRecord,
    local_move: MoveRecord,
    rules: RulesetRecord,
    shared_set: LearnsetRecord,
    local_set: LearnsetRecord,
    preset: MovesetRecord,
}
async fn site() -> Site {
    let app = common::app().await;
    app.accounts
        .invite("address-test", "owner", Role::Administrator, None)
        .await
        .unwrap();
    let actor = app
        .accounts
        .sign_in(Subject {
            provider: "address-test".into(),
            provider_subject: "owner".into(),
            display_name: "Address owner".into(),
        })
        .await
        .unwrap();
    let token = app.accounts.start_session(&actor.user_id).await.unwrap().0;
    let author = Authoring::new(app.accounts.clone());
    let moves = Moves::new(app.accounts.clone());
    let learning = Learning::new(app.accounts.clone());
    let species = author
        .create_species(&token, fields("Address species"), None)
        .await
        .unwrap();
    let other_species = author
        .create_species(&token, fields("Other address species"), None)
        .await
        .unwrap();
    let form = author
        .create_form(&token, &species.id, 1, fields("Address form"), None)
        .await
        .unwrap();
    let world = author
        .create_world(&token, fields("Address world"), None)
        .await
        .unwrap();
    let other_world = author
        .create_world(&token, fields("Other address world"), None)
        .await
        .unwrap();
    let variant = author
        .create_variant(
            &token,
            &world.id,
            1,
            Some(SourcePin {
                form_id: form.id.clone(),
                version: 1,
            }),
            fields("Address variant"),
            None,
        )
        .await
        .unwrap();
    let revision = author.world_detail(&world.id).await.unwrap().revision;
    let sibling = author
        .create_variant(
            &token,
            &world.id,
            revision,
            None,
            fields("Sibling variant"),
            None,
        )
        .await
        .unwrap();
    let foreign = author
        .create_variant(
            &token,
            &other_world.id,
            1,
            None,
            fields("Address variant"),
            None,
        )
        .await
        .unwrap();
    let shared_attribute = attribute(&moves, &token, None, "Address numeric", None).await;
    let local_attribute = attribute(
        &moves,
        &token,
        Some(&world.id),
        "Address numeric",
        Some(shared_attribute.pin()),
    )
    .await;
    let shared_move = movement(
        &moves,
        &token,
        None,
        "Address move",
        &shared_attribute,
        None,
    )
    .await;
    let local_move = movement(
        &moves,
        &token,
        Some(&world.id),
        "Address move",
        &local_attribute,
        Some(shared_move.pin()),
    )
    .await;
    let rules = moves
        .save_ruleset(
            &token,
            &world.id,
            RulesetEdit {
                expected_revision: 0,
                fields: fields("Address rules"),
                expectations: Some(vec![ExpectationInput {
                    attribute: local_attribute.pin(),
                    required: true,
                    allow_not_applicable: false,
                    constraints: NumericConstraints::default(),
                }]),
                stat_roster: None,
                chart: None,
                max_moves: Some(7),
                repeat_policy: RepeatPolicy::Allow,
                notes: "Inert address notes".into(),
                reason: None,
            },
        )
        .await
        .unwrap();
    let shared_set = learn(&learning, &token, None, &form.id, &shared_move).await;
    let local_set = learn(&learning, &token, Some(&world.id), &variant.id, &local_move).await;
    let preset = learning
        .create_preset(
            &token,
            &world.id,
            &variant.id,
            preset_edit("Address preset", &local_set, &rules),
        )
        .await
        .unwrap();
    Site {
        app,
        author,
        moves,
        learning,
        token,
        species,
        other_species,
        form,
        world,
        other_world,
        variant,
        sibling,
        foreign,
        shared_attribute,
        local_attribute,
        shared_move,
        local_move,
        rules,
        shared_set,
        local_set,
        preset,
    }
}
struct Reply {
    status: StatusCode,
    headers: HeaderMap,
    body: String,
}
impl Reply {
    fn location(&self) -> Option<&str> {
        self.headers
            .get(header::LOCATION)
            .map(|v| v.to_str().unwrap())
    }
    fn no_store(&self) {
        assert_eq!(self.headers.get(header::CACHE_CONTROL).unwrap(), "no-store");
    }
}
impl Site {
    async fn row_json(&self, kind: Object, id: &str) -> serde_json::Value {
        match kind {
            Object::SharedAttribute => {
                serde_json::to_value(self.moves.attribute(None, id).await.unwrap()).unwrap()
            }
            Object::LocalAttribute => serde_json::to_value(
                self.moves
                    .attribute(Some(&self.world.id), id)
                    .await
                    .unwrap(),
            )
            .unwrap(),
            Object::SharedMove => {
                serde_json::to_value(self.moves.detail(None, id).await.unwrap()).unwrap()
            }
            Object::LocalMove => {
                serde_json::to_value(self.moves.detail(Some(&self.world.id), id).await.unwrap())
                    .unwrap()
            }
            Object::Preset => serde_json::to_value(
                self.learning
                    .preset(&self.world.id, &self.variant.id, id)
                    .await
                    .unwrap(),
            )
            .unwrap(),
        }
    }
    fn object(&self, kind: Object) -> (String, String, &ContentFields) {
        let w = &self.world;
        match kind {
            Object::SharedAttribute => (
                format!("/move-attributes/{}", self.shared_attribute.id),
                format!("/move-attributes/{}", self.shared_attribute.fields.slug),
                &self.shared_attribute.fields,
            ),
            Object::LocalAttribute => (
                format!(
                    "/worlds/{}/move-attributes/{}",
                    w.id, self.local_attribute.id
                ),
                format!(
                    "/worlds/{}/move-attributes/{}",
                    w.fields.slug, self.local_attribute.fields.slug
                ),
                &self.local_attribute.fields,
            ),
            Object::SharedMove => (
                format!("/moves/{}", self.shared_move.id),
                format!("/moves/{}", self.shared_move.fields.slug),
                &self.shared_move.fields,
            ),
            Object::LocalMove => (
                format!("/worlds/{}/moves/{}", w.id, self.local_move.id),
                format!(
                    "/worlds/{}/moves/{}",
                    w.fields.slug, self.local_move.fields.slug
                ),
                &self.local_move.fields,
            ),
            Object::Preset => (
                format!(
                    "/worlds/{}/variants/{}/movesets/{}",
                    w.id, self.variant.id, self.preset.id
                ),
                format!(
                    "/worlds/{}/variants/{}/movesets/{}",
                    w.fields.slug, self.variant.fields.slug, self.preset.fields.slug
                ),
                &self.preset.fields,
            ),
        }
    }
    async fn request(&self, method: &str, uri: &str, body: Option<&Values>) -> Reply {
        let mut r = Request::builder()
            .method(method)
            .uri(uri)
            .header(
                header::COOKIE,
                format!("karta_session={}; karta_csrf={CSRF}", self.token),
            )
            .header(header::HOST, "untrusted.example");
        let body = if let Some(values) = body {
            r = r
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .header(header::ORIGIN, "http://127.0.0.1:3000");
            Body::from(
                values
                    .iter()
                    .map(|(k, v)| format!("{}={}", encode(k), encode(v)))
                    .collect::<Vec<_>>()
                    .join("&"),
            )
        } else {
            Body::empty()
        };
        let response = common::send(&self.app.router, r.body(body).unwrap()).await;
        let status = response.status();
        let headers = response.headers().clone();
        Reply {
            status,
            headers,
            body: common::read_body(response).await,
        }
    }
    async fn redirect(&self, from: &str, to: &str) {
        for method in ["GET", "HEAD"] {
            let r = self.request(method, from, None).await;
            assert_eq!(
                r.status,
                StatusCode::TEMPORARY_REDIRECT,
                "{method} {from}: {}",
                r.body
            );
            assert_eq!(r.location(), Some(to), "{method} {from}");
            r.no_store();
            assert!(r.body.is_empty());
            let r = self.request(method, to, None).await;
            assert_eq!(r.status, StatusCode::OK, "{method} {to}: {}", r.body);
            assert!(r.location().is_none());
            r.no_store();
            if method == "HEAD" {
                assert!(r.body.is_empty(), "HEAD {to}");
            }
        }
    }
    fn draft(&self, kind: Object) -> Values {
        let (_, _, f) = self.object(kind);
        let mut d = Values::from([
            ("csrf".into(), CSRF.into()),
            ("expected_revision".into(), "1".into()),
            ("page".into(), "1".into()),
            ("display_name".into(), f.display_name.clone()),
            ("description".into(), "Edited through canonical ID".into()),
            ("reason".into(), "Retained address draft".into()),
            ("intent".into(), "preview".into()),
        ]);
        match kind {
            Object::SharedAttribute | Object::LocalAttribute => {
                d.insert("unit".into(), "points".into());
            }
            Object::SharedMove | Object::LocalMove => {
                d.insert("state".into(), "defined".into());
                d.insert("operation".into(), "keep".into());
            }
            Object::Preset => {
                d.insert("operation".into(), "keep".into());
                d.insert("learnset".into(), format!("{}@1", self.local_set.id));
                d.insert("ruleset".into(), format!("{}@1", self.rules.id));
            }
        }
        d
    }
    async fn confirmed(&self, action: &str, mut draft: Values) -> Reply {
        let r = self.request("POST", action, Some(&draft)).await;
        assert_eq!(r.status, StatusCode::OK, "preview {action}: {}", r.body);
        assert!(r.location().is_none());
        r.no_store();
        draft.insert(
            "review_fingerprint".into(),
            input(&r.body, "review_fingerprint"),
        );
        draft.insert("intent".into(), "save".into());
        draft.insert("confirm".into(), "yes".into());
        self.request("POST", action, Some(&draft)).await
    }
}
fn encode(v: &str) -> String {
    v.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}
fn input(html: &str, name: &str) -> String {
    html.split(&format!("name=\"{name}\""))
        .nth(1)
        .unwrap_or_else(|| panic!("missing input {name}"))
        .split("value=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap()
        .to_owned()
}

#[tokio::test]
async fn all_five_object_resolvers_preserve_suffixes_raw_queries_and_id_bound_actions() {
    let s = site().await;
    for kind in OBJECTS {
        let (id, pretty, _) = s.object(kind);
        for suffix in ["", "/edit", "/history", "/history/1"] {
            s.redirect(
                &format!("{id}{suffix}{QUERY}"),
                &format!("{pretty}{suffix}{QUERY}"),
            )
            .await;
        }
        let r = s.request("GET", &format!("{pretty}/edit"), None).await;
        assert!(
            r.body.contains(&format!("action=\"{id}/edit\"")),
            "{kind:?}: expected ID action"
        );
    }
    for kind in [Object::LocalAttribute, Object::LocalMove] {
        let (id, pretty, _) = s.object(kind);
        s.redirect(
            &format!("{id}/adopt{QUERY}"),
            &format!("{pretty}/adopt{QUERY}"),
        )
        .await;
    }
    let (id, pretty, _) = s.object(Object::Preset);
    s.redirect(
        &format!("{id}/validation{QUERY}"),
        &format!("{pretty}/validation{QUERY}"),
    )
    .await;
}

#[tokio::test]
async fn singleton_ruleset_and_independent_learning_routes_resolve_full_hierarchies() {
    let s = site().await;
    let w = &s.world;
    let form_pretty = format!(
        "/creatures/{}/forms/{}/learnset",
        s.species.fields.slug, s.form.fields.slug
    );
    let variant_id = format!("/worlds/{}/variants/{}", w.id, s.variant.id);
    let variant_pretty = format!(
        "/worlds/{}/variants/{}",
        w.fields.slug, s.variant.fields.slug
    );
    for suffix in ["", "/edit", "/history", "/history/1"] {
        for from in [
            format!("/forms/{}/learnset", s.form.id),
            format!("/creatures/{}/forms/{}/learnset", s.species.id, s.form.id),
            format!(
                "/creatures/{}/forms/{}/learnset",
                s.species.fields.slug, s.form.id
            ),
        ] {
            s.redirect(
                &format!("{from}{suffix}{QUERY}"),
                &format!("{form_pretty}{suffix}{QUERY}"),
            )
            .await;
        }
        s.redirect(
            &format!("{variant_id}/learnset{suffix}{QUERY}"),
            &format!("{variant_pretty}/learnset{suffix}{QUERY}"),
        )
        .await;
        s.redirect(
            &format!(
                "/worlds/{}/variants/{}/learnset{suffix}{QUERY}",
                w.fields.slug, s.variant.id
            ),
            &format!("{variant_pretty}/learnset{suffix}{QUERY}"),
        )
        .await;
        s.redirect(
            &format!("/worlds/{}/ruleset{suffix}{QUERY}", w.id),
            &format!("/worlds/{}/ruleset{suffix}{QUERY}", w.fields.slug),
        )
        .await;
    }
    s.redirect(
        &format!("{variant_id}/learnset/adopt{QUERY}"),
        &format!("{variant_pretty}/learnset/adopt{QUERY}"),
    )
    .await;
    for suffix in [
        "/moves",
        "/moves/new",
        "/move-attributes",
        "/move-attributes/new",
    ] {
        s.redirect(
            &format!("/worlds/{}{suffix}{QUERY}", w.id),
            &format!("/worlds/{}{suffix}{QUERY}", w.fields.slug),
        )
        .await;
    }
    for suffix in ["/movesets", "/movesets/new"] {
        s.redirect(
            &format!("{variant_id}{suffix}{QUERY}"),
            &format!("{variant_pretty}{suffix}{QUERY}"),
        )
        .await;
    }
    for (pretty, action) in [
        (
            format!("{form_pretty}/edit"),
            format!("/forms/{}/learnset/edit", s.form.id),
        ),
        (
            format!("{variant_pretty}/learnset/edit"),
            format!("{variant_id}/learnset/edit"),
        ),
        (
            format!("/worlds/{}/ruleset/edit", w.fields.slug),
            format!("/worlds/{}/ruleset/edit", w.id),
        ),
    ] {
        let r = s.request("GET", &pretty, None).await;
        assert_eq!(r.status, StatusCode::OK);
        assert!(r.body.contains(&format!("action=\"{action}\"")), "{pretty}");
    }
}

#[tokio::test]
async fn wrong_parents_never_resolve_objects_from_another_scope_and_unknown_shapes_stay_404() {
    let s = site().await;
    let foreign_attribute = attribute(
        &s.moves,
        &s.token,
        Some(&s.other_world.id),
        "Address numeric",
        None,
    )
    .await;
    let foreign_move = movement(
        &s.moves,
        &s.token,
        Some(&s.other_world.id),
        "Address move",
        &foreign_attribute,
        None,
    )
    .await;
    let sibling_set = learn(
        &s.learning,
        &s.token,
        Some(&s.world.id),
        &s.sibling.id,
        &s.local_move,
    )
    .await;
    let sibling_preset = s
        .learning
        .create_preset(
            &s.token,
            &s.world.id,
            &s.sibling.id,
            preset_edit("Address preset", &sibling_set, &s.rules),
        )
        .await
        .unwrap();
    for path in [
        format!(
            "/worlds/{}/move-attributes/{}",
            s.other_world.fields.slug, s.local_attribute.id
        ),
        format!("/worlds/{}/moves/{}", s.other_world.id, s.local_move.id),
        format!("/worlds/{}/moves/{}", s.world.fields.slug, foreign_move.id),
        format!(
            "/worlds/{}/move-attributes/{}",
            s.world.id, s.shared_attribute.id
        ),
        format!("/moves/{}", s.local_move.id),
        format!("/move-attributes/{}", s.local_attribute.id),
        format!(
            "/worlds/{}/variants/{}/movesets/{}",
            s.world.fields.slug, s.sibling.fields.slug, s.preset.id
        ),
        format!(
            "/worlds/{}/variants/{}/movesets/{}",
            s.world.id, s.variant.id, sibling_preset.id
        ),
        format!(
            "/worlds/{}/variants/{}/movesets/{}",
            s.other_world.fields.slug, s.variant.id, s.preset.id
        ),
        format!(
            "/worlds/{}/variants/{}/learnset",
            s.world.fields.slug, s.foreign.id
        ),
        format!(
            "/creatures/{}/forms/{}/learnset",
            s.other_species.fields.slug, s.form.id
        ),
        "/moves/~ordinary".into(),
        "/worlds/address-world/moves/address-move%2fedit".into(),
        "/worlds/address-world/moves/address-move%252fedit".into(),
    ] {
        for method in ["GET", "HEAD"] {
            let r = s.request(method, &path, None).await;
            assert_eq!(
                r.status,
                StatusCode::NOT_FOUND,
                "{method} {path}: {}",
                r.body
            );
            assert!(r.location().is_none());
            r.no_store();
            if method == "HEAD" {
                assert!(r.body.is_empty());
            }
        }
    }
    let inner = s.request("GET", "/unrecognized-route", None).await;
    for path in [
        "/moves/address-move/unknown",
        "/move-attributes/address-numeric/history/1/extra",
        "/worlds/address-world/ruleset/unknown",
        "/worlds/address-world/ruleset/history/1/extra",
        "/creatures/address-species/forms/address-form/learnset/history/1/extra",
        "/worlds/address-world/variants/address-variant/learnset/unknown",
        "/worlds/address-world/variants/address-variant/movesets/address-preset/history/1/extra",
        "/worlds/address-world/variants/address-variant/movesets/address-preset/adopt",
    ] {
        let r = s.request("GET", path, None).await;
        assert_eq!(r.status, StatusCode::NOT_FOUND, "{path}: {}", r.body);
        assert!(r.location().is_none());
        assert_eq!(
            r.body, inner.body,
            "unknown shape must retain inner 404: {path}"
        );
        let head = s.request("HEAD", path, None).await;
        assert_eq!(head.status, StatusCode::NOT_FOUND);
        assert!(head.body.is_empty());
    }
}

#[tokio::test]
async fn new_and_id_shaped_slugs_are_escaped_for_all_five_object_kinds() {
    let s = site().await;
    for kind in OBJECTS {
        for name in [
            "new".to_owned(),
            match kind {
                Object::SharedAttribute => s.shared_attribute.id.clone(),
                Object::LocalAttribute => s.local_attribute.id.clone(),
                Object::SharedMove => s.shared_move.id.clone(),
                Object::LocalMove => s.local_move.id.clone(),
                Object::Preset => s.preset.id.clone(),
            },
        ] {
            let (id, root) = match kind {
                Object::SharedAttribute | Object::LocalAttribute => {
                    let world =
                        matches!(kind, Object::LocalAttribute).then_some(s.world.id.as_str());
                    let a = attribute(&s.moves, &s.token, world, &name, None).await;
                    (
                        a.id,
                        world.map_or_else(
                            || "/move-attributes".into(),
                            |_| format!("/worlds/{}/move-attributes", s.world.fields.slug),
                        ),
                    )
                }
                Object::SharedMove | Object::LocalMove => {
                    let local = matches!(kind, Object::LocalMove);
                    let m = movement(
                        &s.moves,
                        &s.token,
                        local.then_some(s.world.id.as_str()),
                        &name,
                        if local {
                            &s.local_attribute
                        } else {
                            &s.shared_attribute
                        },
                        None,
                    )
                    .await;
                    (
                        m.id,
                        if local {
                            format!("/worlds/{}/moves", s.world.fields.slug)
                        } else {
                            "/moves".into()
                        },
                    )
                }
                Object::Preset => {
                    let p = s
                        .learning
                        .create_preset(
                            &s.token,
                            &s.world.id,
                            &s.variant.id,
                            preset_edit(&name, &s.local_set, &s.rules),
                        )
                        .await
                        .unwrap();
                    (
                        p.id,
                        format!(
                            "/worlds/{}/variants/{}/movesets",
                            s.world.fields.slug, s.variant.fields.slug
                        ),
                    )
                }
            };
            let id_root = s.object(kind).0.rsplit_once('/').unwrap().0.to_owned();
            for suffix in ["", "/edit", "/history/1"] {
                s.redirect(
                    &format!("{id_root}/{id}{suffix}{QUERY}"),
                    &format!("{root}/~{name}{suffix}{QUERY}"),
                )
                .await;
            }
            if name == "new" {
                let creation = s.request("GET", &format!("{root}/new"), None).await;
                assert_eq!(creation.status, StatusCode::OK);
                assert!(creation.location().is_none());
                assert!(
                    !creation
                        .body
                        .contains(&format!("Canonical preset ID: {id}"))
                );
            } else {
                // The unescaped 32-hex segment remains the original canonical ID,
                // never the newly created object whose slug happens to equal it.
                let (original, pretty, _) = s.object(kind);
                s.redirect(&format!("{original}{QUERY}"), &format!("{pretty}{QUERY}"))
                    .await;
            }
        }
    }
}

#[tokio::test]
async fn encoded_segments_normalize_once_and_redirects_keep_raw_query_bytes() {
    let s = site().await;
    for (from, to) in [
        ("/%6doves/address%2dmove", "/moves/address-move"),
        (
            "/move%2dattributes/address%2dnumeric",
            "/move-attributes/address-numeric",
        ),
        (
            "/worlds/address%2dworld/moves/address%2dmove/history/1",
            "/worlds/address-world/moves/address-move/history/1",
        ),
        (
            "/worlds/address%2dworld/ruleset",
            "/worlds/address-world/ruleset",
        ),
        (
            "/creatures/address%2dspecies/forms/address%2dform/learnset",
            "/creatures/address-species/forms/address-form/learnset",
        ),
        (
            "/worlds/address%2dworld/variants/address%2dvariant/movesets/address%2dpreset",
            "/worlds/address-world/variants/address-variant/movesets/address-preset",
        ),
        ("/%6doves/%6eew", "/moves/new"),
    ] {
        s.redirect(&format!("{from}{QUERY}"), &format!("{to}{QUERY}"))
            .await;
    }
}

#[tokio::test]
async fn canonical_id_posts_save_then_303_to_id_and_307_to_readable_get() {
    let s = site().await;
    for kind in OBJECTS {
        let (id, pretty, _) = s.object(kind);
        let r = s.confirmed(&format!("{id}/edit"), s.draft(kind)).await;
        assert_eq!(r.status, StatusCode::SEE_OTHER, "{kind:?}: {}", r.body);
        assert_eq!(r.location(), Some(id.as_str()));
        r.no_store();
        s.redirect(&id, &pretty).await;
        let r = s.request("GET", &format!("{id}/history/2"), None).await;
        assert_eq!(r.status, StatusCode::TEMPORARY_REDIRECT);
        assert_eq!(r.location(), Some(format!("{pretty}/history/2").as_str()));
    }
    let action = format!("/worlds/{}/ruleset/edit", s.world.id);
    let mut d = Values::from([
        ("csrf".into(), CSRF.into()),
        ("expected_revision".into(), "1".into()),
        ("page".into(), "1".into()),
        ("display_name".into(), s.rules.fields.display_name.clone()),
        ("state".into(), "defined".into()),
        ("operation".into(), "keep".into()),
        ("max_moves".into(), "8".into()),
        ("repeat_policy".into(), "allow".into()),
        // Ruleset prose uses the bounded field-scoped protocol: only the loaded
        // field is submitted; the other saved text is composed server-side.
        ("text_field".into(), "description".into()),
        ("text".into(), s.rules.fields.description.clone()),
        ("intent".into(), "preview".into()),
    ]);
    let r = s.confirmed(&action, d.clone()).await;
    assert_eq!(r.status, StatusCode::SEE_OTHER);
    assert_eq!(
        r.location(),
        Some(format!("/worlds/{}/ruleset", s.world.id).as_str())
    );
    r.no_store();
    for (world, owner, set, id_root, pretty_root) in [
        (
            None,
            s.form.id.as_str(),
            &s.shared_set,
            format!("/forms/{}/learnset", s.form.id),
            format!(
                "/creatures/{}/forms/{}/learnset",
                s.species.fields.slug, s.form.fields.slug
            ),
        ),
        (
            Some(s.world.id.as_str()),
            s.variant.id.as_str(),
            &s.local_set,
            format!("/worlds/{}/variants/{}/learnset", s.world.id, s.variant.id),
            format!(
                "/worlds/{}/variants/{}/learnset",
                s.world.fields.slug, s.variant.fields.slug
            ),
        ),
    ] {
        let record = &set.records.as_ref().unwrap()[0];
        d = Values::from([
            ("csrf".into(), CSRF.into()),
            ("expected_revision".into(), "1".into()),
            ("page".into(), "1".into()),
            ("operation".into(), "edit".into()),
            ("record".into(), record.id.clone()),
            (
                "move".into(),
                format!("{}@{}", record.move_record.id, record.move_record.revision),
            ),
            ("threshold_state".into(), "known".into()),
            ("threshold".into(), "0".into()),
            ("text_field".into(), "method".into()),
            ("text".into(), "Canonical ID submission".into()),
            ("reason".into(), String::new()),
            ("intent".into(), "preview".into()),
        ]);
        let r = s.confirmed(&format!("{id_root}/edit"), d.clone()).await;
        assert_eq!(r.status, StatusCode::SEE_OTHER, "{}", r.body);
        assert_eq!(r.location(), Some(id_root.as_str()));
        r.no_store();
        s.redirect(&id_root, &pretty_root).await;
        assert_eq!(
            s.learning
                .learnset(world, owner)
                .await
                .unwrap()
                .unwrap()
                .revision,
            2
        );
    }
    // Presets still read their original independent learnset/ruleset revisions.
    let saved = s
        .learning
        .preset(&s.world.id, &s.variant.id, &s.preset.id)
        .await
        .unwrap();
    assert_eq!(saved.learnset, s.local_set.pin());
    assert_eq!(saved.ruleset, s.rules.pin());
}

#[tokio::test]
async fn stale_id_drafts_cannot_write_renamed_objects_or_reused_slug_replacements() {
    let s = site().await;
    for kind in OBJECTS {
        let (id_path, pretty, f) = s.object(kind);
        let original_id = id_path.rsplit_once('/').unwrap().1;
        let replacement_id = match kind {
            Object::SharedAttribute | Object::LocalAttribute => {
                let a = if matches!(kind, Object::SharedAttribute) {
                    &s.shared_attribute
                } else {
                    &s.local_attribute
                };
                s.moves
                    .save_attribute(
                        &s.token,
                        a.world_id.as_deref(),
                        &a.id,
                        AttributeEdit {
                            expected_revision: 1,
                            fields: fields("Renamed numeric"),
                            unit: a.unit.clone(),
                            constraints: a.constraints.clone(),
                            reason: None,
                        },
                    )
                    .await
                    .unwrap();
                attribute(
                    &s.moves,
                    &s.token,
                    a.world_id.as_deref(),
                    &f.display_name,
                    None,
                )
                .await
                .id
            }
            Object::SharedMove | Object::LocalMove => {
                let (m, a) = if matches!(kind, Object::SharedMove) {
                    (&s.shared_move, &s.shared_attribute)
                } else {
                    (&s.local_move, &s.local_attribute)
                };
                s.moves
                    .save(
                        &s.token,
                        m.world_id.as_deref(),
                        &m.id,
                        MoveEdit {
                            expected_revision: 1,
                            fields: fields("Renamed move"),
                            attributes: Some(vec![AttributeValueInput {
                                attribute: a.pin(),
                                value: NumericValue::Unknown,
                            }]),
                            reason: None,
                        },
                    )
                    .await
                    .unwrap();
                movement(
                    &s.moves,
                    &s.token,
                    m.world_id.as_deref(),
                    &f.display_name,
                    a,
                    None,
                )
                .await
                .id
            }
            Object::Preset => {
                let mut edit = preset_edit("Renamed preset", &s.local_set, &s.rules);
                edit.expected_revision = 1;
                s.learning
                    .save_preset(&s.token, &s.world.id, &s.variant.id, &s.preset.id, edit)
                    .await
                    .unwrap();
                s.learning
                    .create_preset(
                        &s.token,
                        &s.world.id,
                        &s.variant.id,
                        preset_edit(&f.display_name, &s.local_set, &s.rules),
                    )
                    .await
                    .unwrap()
                    .id
            }
        };
        let original = s.row_json(kind, original_id).await;
        let replacement = s.row_json(kind, &replacement_id).await;
        assert_eq!(original["revision"], 2);
        assert_eq!(replacement["revision"], 1);
        let mut stale = s.draft(kind);
        stale.insert("intent".into(), "save".into());
        stale.insert("confirm".into(), "yes".into());
        stale.insert("review_fingerprint".into(), "outdated-review".into());
        // An invalid dependency must not displace the revision conflict.
        stale.insert("exact_pin".into(), "not-a-pin".into());
        stale.insert("learnset".into(), "not-a-pin".into());
        let r = s
            .request("POST", &format!("{id_path}/edit"), Some(&stale))
            .await;
        assert_eq!(r.status, StatusCode::CONFLICT, "{kind:?}: {}", r.body);
        assert!(r.location().is_none());
        r.no_store();
        assert!(r.body.contains("Retained address draft"));
        assert!(r.body.contains(&format!("action=\"{id_path}/edit\"")));
        let r = s
            .request("POST", &format!("{pretty}/edit"), Some(&stale))
            .await;
        assert_eq!(
            r.status,
            StatusCode::NOT_FOUND,
            "slug POST {pretty}: {}",
            r.body
        );
        assert!(r.location().is_none());
        assert_eq!(s.row_json(kind, original_id).await, original);
        assert_eq!(s.row_json(kind, &replacement_id).await, replacement);
        // Reused readable GETs now identify the replacement, while the original
        // canonical ID still resolves exclusively to its own newly named address.
        let r = s.request("GET", &pretty, None).await;
        assert_eq!(r.status, StatusCode::OK);
        let current_slug = original["fields"]["slug"].as_str().unwrap();
        let pretty_root = pretty.rsplit_once('/').unwrap().0;
        s.redirect(&id_path, &format!("{pretty_root}/{current_slug}"))
            .await;
    }
}

#[tokio::test]
async fn parent_renames_update_readable_learning_addresses_without_retargeting_pins_or_histories() {
    let s = site().await;
    let shared_history = s
        .learning
        .learnset_history(None, &s.form.id, None)
        .await
        .unwrap();
    let local_history = s
        .learning
        .learnset_history(Some(&s.world.id), &s.variant.id, None)
        .await
        .unwrap();
    let preset_history = s
        .learning
        .preset_history(&s.world.id, &s.variant.id, &s.preset.id, None)
        .await
        .unwrap();
    let rules_history = s.moves.ruleset_history(&s.world.id, None).await.unwrap();
    s.author
        .save_species(
            &s.token,
            &s.species.id,
            metadata_edit(&s.species.fields, 1, "Renamed address species"),
        )
        .await
        .unwrap();
    let form = s
        .author
        .save_form(
            &s.token,
            &s.form.id,
            s.form.species_version,
            metadata_edit(&s.form.fields, 1, "Renamed address form"),
        )
        .await
        .unwrap();
    let world = s.author.world_detail(&s.world.id).await.unwrap();
    let world = s
        .author
        .save_world(
            &s.token,
            &world.id,
            metadata_edit(&world.fields, world.revision, "Renamed address world"),
        )
        .await
        .unwrap();
    let variant = s
        .author
        .save_variant(
            &s.token,
            &world.id,
            &s.variant.id,
            metadata_edit(&s.variant.fields, 1, "Renamed address variant"),
        )
        .await
        .unwrap();
    assert_eq!(form.species_version, 1);
    assert_eq!(variant.source, s.variant.source);
    assert_eq!(
        s.learning.learnset(None, &form.id).await.unwrap().unwrap(),
        s.shared_set
    );
    assert_eq!(
        s.learning
            .learnset(Some(&world.id), &variant.id)
            .await
            .unwrap()
            .unwrap(),
        s.local_set
    );
    assert_eq!(
        s.learning
            .preset(&world.id, &variant.id, &s.preset.id)
            .await
            .unwrap(),
        s.preset
    );
    assert_eq!(s.moves.ruleset(&world.id).await.unwrap().unwrap(), s.rules);
    for (before, after) in [
        (
            shared_history,
            s.learning
                .learnset_history(None, &form.id, None)
                .await
                .unwrap(),
        ),
        (
            local_history,
            s.learning
                .learnset_history(Some(&world.id), &variant.id, None)
                .await
                .unwrap(),
        ),
        (
            preset_history,
            s.learning
                .preset_history(&world.id, &variant.id, &s.preset.id, None)
                .await
                .unwrap(),
        ),
        (
            rules_history,
            s.moves.ruleset_history(&world.id, None).await.unwrap(),
        ),
    ] {
        assert_eq!(before.len(), after.len());
        assert_eq!(before[0].after_json, after[0].after_json);
    }
    for suffix in ["", "/edit", "/history", "/history/1"] {
        s.redirect(&format!("/forms/{}/learnset{suffix}{QUERY}",form.id),&format!("/creatures/renamed-address-species/forms/renamed-address-form/learnset{suffix}{QUERY}")).await;
        s.redirect(&format!("/worlds/{}/variants/{}/learnset{suffix}{QUERY}",world.id,variant.id),&format!("/worlds/renamed-address-world/variants/renamed-address-variant/learnset{suffix}{QUERY}")).await;
        s.redirect(&format!("/worlds/{}/variants/{}/movesets/{}{suffix}{QUERY}",world.id,variant.id,s.preset.id),&format!("/worlds/renamed-address-world/variants/renamed-address-variant/movesets/address-preset{suffix}{QUERY}")).await;
        s.redirect(
            &format!("/worlds/{}/ruleset{suffix}{QUERY}", world.id),
            &format!("/worlds/renamed-address-world/ruleset{suffix}{QUERY}"),
        )
        .await;
    }
    for kind in [Object::LocalMove, Object::LocalAttribute] {
        let (id, pretty, _) = s.object(kind);
        s.redirect(
            &format!("{id}/history/1{QUERY}"),
            &format!(
                "{}/history/1{QUERY}",
                pretty.replace("/worlds/address-world/", "/worlds/renamed-address-world/")
            ),
        )
        .await;
    }
    for old in [
        "/creatures/address-species/forms/address-form/learnset",
        "/worlds/address-world/ruleset",
        "/worlds/address-world/variants/address-variant/learnset",
        "/worlds/renamed-address-world/variants/address-variant/movesets/address-preset",
    ] {
        let r = s.request("GET", old, None).await;
        assert_eq!(r.status, StatusCode::NOT_FOUND, "{old}");
    }
}

#[tokio::test]
async fn learnset_and_ruleset_stale_posts_remain_bound_to_original_owner_ids_after_slug_reuse() {
    let s = site().await;
    for (world, owner, set) in [
        (None, s.form.id.as_str(), &s.shared_set),
        (
            Some(s.world.id.as_str()),
            s.variant.id.as_str(),
            &s.local_set,
        ),
    ] {
        let saved = s
            .learning
            .save_learnset(
                &s.token,
                world,
                owner,
                LearnsetEdit {
                    expected_revision: 1,
                    records: Some(
                        set.records
                            .as_ref()
                            .unwrap()
                            .iter()
                            .map(|r| LearningRecordInput {
                                id: r.id.clone(),
                                move_pin: r.move_record.pin(),
                                method: Some("Independently revised".into()),
                                context: r.context.clone(),
                                condition: r.condition.clone(),
                                threshold: r.threshold.clone(),
                            })
                            .collect(),
                    ),
                    reason: None,
                },
            )
            .await
            .unwrap();
        let (replacement_owner, id_root, pretty_root) = if world.is_none() {
            s.author
                .save_form(
                    &s.token,
                    owner,
                    1,
                    metadata_edit(&s.form.fields, 1, "Renamed form owner"),
                )
                .await
                .unwrap();
            let replacement = s
                .author
                .create_form(&s.token, &s.species.id, 1, s.form.fields.clone(), None)
                .await
                .unwrap();
            (
                replacement.id,
                format!("/forms/{owner}/learnset"),
                format!(
                    "/creatures/{}/forms/{}/learnset",
                    s.species.fields.slug, s.form.fields.slug
                ),
            )
        } else {
            s.author
                .save_variant(
                    &s.token,
                    &s.world.id,
                    owner,
                    metadata_edit(&s.variant.fields, 1, "Renamed variant owner"),
                )
                .await
                .unwrap();
            let current = s.author.world_detail(&s.world.id).await.unwrap();
            let replacement = s
                .author
                .create_variant(
                    &s.token,
                    &s.world.id,
                    current.revision,
                    None,
                    s.variant.fields.clone(),
                    None,
                )
                .await
                .unwrap();
            (
                replacement.id,
                format!("/worlds/{}/variants/{owner}/learnset", s.world.id),
                format!(
                    "/worlds/{}/variants/{}/learnset",
                    s.world.fields.slug, s.variant.fields.slug
                ),
            )
        };
        let m = if world.is_none() {
            &s.shared_move
        } else {
            &s.local_move
        };
        let replacement = learn(&s.learning, &s.token, world, &replacement_owner, m).await;
        let d = Values::from([
            ("csrf".into(), CSRF.into()),
            ("expected_revision".into(), "1".into()),
            ("page".into(), "1".into()),
            ("operation".into(), "unmodeled".into()),
            ("reason".into(), "Retained owner-bound draft".into()),
            ("intent".into(), "save".into()),
            ("confirm".into(), "yes".into()),
        ]);
        let r = s
            .request("POST", &format!("{id_root}/edit"), Some(&d))
            .await;
        assert_eq!(r.status, StatusCode::CONFLICT, "{id_root}: {}", r.body);
        assert!(r.location().is_none());
        r.no_store();
        assert!(r.body.contains("Retained owner-bound draft"));
        assert!(r.body.contains(&format!("action=\"{id_root}/edit\"")));
        let r = s
            .request("POST", &format!("{pretty_root}/edit"), Some(&d))
            .await;
        assert_eq!(r.status, StatusCode::NOT_FOUND);
        assert!(r.location().is_none());
        assert_eq!(
            s.learning.learnset(world, owner).await.unwrap().unwrap(),
            saved
        );
        assert_eq!(
            s.learning
                .learnset(world, &replacement_owner)
                .await
                .unwrap()
                .unwrap(),
            replacement
        );
    }
    let mut edit = RulesetEdit {
        expected_revision: 1,
        fields: s.rules.fields.clone(),
        expectations: Some(vec![]),
        stat_roster: None,
        chart: None,
        max_moves: Some(7),
        repeat_policy: RepeatPolicy::Allow,
        notes: "Independently revised".into(),
        reason: None,
    };
    let saved = s
        .moves
        .save_ruleset(&s.token, &s.world.id, edit.clone())
        .await
        .unwrap();
    let current = s.author.world_detail(&s.world.id).await.unwrap();
    s.author
        .save_world(
            &s.token,
            &s.world.id,
            metadata_edit(&current.fields, current.revision, "Renamed ruleset owner"),
        )
        .await
        .unwrap();
    let replacement = s
        .author
        .create_world(&s.token, s.world.fields.clone(), None)
        .await
        .unwrap();
    edit.expected_revision = 0;
    let replacement_rules = s
        .moves
        .save_ruleset(&s.token, &replacement.id, edit)
        .await
        .unwrap();
    let d = Values::from([
        ("csrf".into(), CSRF.into()),
        ("expected_revision".into(), "1".into()),
        ("reason".into(), "Retained singleton draft".into()),
        ("intent".into(), "save".into()),
        ("confirm".into(), "yes".into()),
        ("max_moves".into(), "not-a-number".into()),
    ]);
    let action = format!("/worlds/{}/ruleset/edit", s.world.id);
    let r = s.request("POST", &action, Some(&d)).await;
    assert_eq!(r.status, StatusCode::CONFLICT, "{}", r.body);
    assert!(r.location().is_none());
    r.no_store();
    let r = s
        .request(
            "POST",
            &format!("/worlds/{}/ruleset/edit", s.world.fields.slug),
            Some(&d),
        )
        .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    assert!(r.location().is_none());
    assert_eq!(s.moves.ruleset(&s.world.id).await.unwrap().unwrap(), saved);
    assert_eq!(
        s.moves.ruleset(&replacement.id).await.unwrap().unwrap(),
        replacement_rules
    );
}
