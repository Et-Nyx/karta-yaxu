//! ID-qualified, native card authoring. Application services own every write.
use super::authoring::{
    authoring_form_error, error_status, error_text, finish, invalid, library_permission, no_store,
    positive, submit_token, world_permission,
};
use super::*;
use crate::application::{
    authoring::{AddressKey, Error},
    cards::*,
};
use axum::{extract::rejection::QueryRejection, http::Uri};
use game_data::stats::NumericConstraints;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

type Values = BTreeMap<String, String>;
const SIZE: usize = 25;
mod pickers;

pub(super) fn routes() -> Router<AppState> {
    let mut r = Router::new();
    for root in ["/cards", "/worlds/{world_id}/cards"] {
        r = r
            .route(root, get(catalog))
            .route(&format!("{root}/new"), get(editor).post(submit))
            .route(&format!("{root}/{{namespace}}/{{id}}"), get(detail))
            .route(
                &format!("{root}/{{namespace}}/{{id}}/edit"),
                get(editor).post(submit),
            )
            .route(
                &format!("{root}/{{namespace}}/{{id}}/history"),
                get(history),
            )
            .route(
                &format!("{root}/{{namespace}}/{{id}}/history/{{revision}}"),
                get(detail),
            );
    }
    for root in ["/card-fields", "/worlds/{world_id}/card-fields"] {
        r = r
            .route(root, get(catalog))
            .route(&format!("{root}/new"), get(editor).post(submit))
            .route(&format!("{root}/{{id}}"), get(detail))
            .route(&format!("{root}/{{id}}/edit"), get(editor).post(submit))
            .route(&format!("{root}/{{id}}/history"), get(history))
            .route(&format!("{root}/{{id}}/history/{{revision}}"), get(detail));
    }
    r.layer(DefaultBodyLimit::max(32 * 1024))
}

fn value<'a>(d: &'a Values, k: &str) -> &'a str {
    d.get(k).map(String::as_str).unwrap_or("")
}
fn page(d: &Values) -> Result<usize, Error> {
    if value(d, "page").is_empty() {
        return Ok(1);
    }
    value(d, "page")
        .parse::<usize>()
        .ok()
        .filter(|n| *n > 0 && *n < usize::MAX / SIZE)
        .ok_or_else(|| invalid("page", "Use a positive page number."))
}
fn reason(d: &Values) -> Option<String> {
    d.get("reason").filter(|s| !s.is_empty()).cloned()
}
fn namespace(s: &str) -> Result<CardNamespace, Error> {
    match s {
        "form" => Ok(CardNamespace::Form),
        "variant" => Ok(CardNamespace::Variant),
        "type" => Ok(CardNamespace::Type),
        "move" => Ok(CardNamespace::Move),
        "standalone" => Ok(CardNamespace::Standalone),
        _ => Err(invalid(
            "namespace",
            "Choose form, variant, type, move or standalone.",
        )),
    }
}
#[derive(Clone)]
struct Context {
    world: Option<String>,
    field: bool,
    id: String,
    namespace: CardNamespace,
}
impl Context {
    fn new(p: &Values, uri: &Uri) -> Result<Self, Error> {
        let world = p.get("world_id").cloned();
        if world
            .as_ref()
            .is_some_and(|w| !game_data::types::valid_id(w))
        {
            return Err(Error::NotFound);
        }
        let id = value(p, "id").to_owned();
        if id.len() > 256 {
            return Err(Error::NotFound);
        }
        Ok(Self {
            world,
            field: uri.path().split('/').any(|s| s == "card-fields"),
            id,
            namespace: if p.contains_key("namespace") {
                namespace(value(p, "namespace"))?
            } else {
                CardNamespace::Standalone
            },
        })
    }
    fn root(&self) -> String {
        format!(
            "{}{}",
            self.world
                .as_ref()
                .map(|w| format!("/worlds/{w}"))
                .unwrap_or_default(),
            if self.field { "/card-fields" } else { "/cards" }
        )
    }
    fn url(&self) -> String {
        if self.id.is_empty() {
            format!("{}/new", self.root())
        } else if self.field {
            format!("{}/{}", self.root(), self.id)
        } else {
            format!("{}/{}/{}", self.root(), self.namespace.as_str(), self.id)
        }
    }
    fn identity(&self) -> CardIdentity {
        CardIdentity {
            namespace: self.namespace,
            world_id: self.world.clone(),
            id: self.id.clone(),
        }
    }
    async fn resolved(mut self, api: &Cards) -> Result<Self, Error> {
        if self.id.is_empty() {
            return Ok(self);
        }
        let key = if let Some(slug) = self.id.strip_prefix('~') {
            if slug != "new" && !game_data::types::valid_id(slug) {
                return Err(Error::NotFound);
            }
            AddressKey::Slug(slug.into())
        } else if game_data::types::valid_id(&self.id) {
            AddressKey::Id(self.id.clone())
        } else if self
            .id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        {
            AddressKey::Slug(self.id.clone())
        } else {
            return Err(Error::NotFound);
        };
        self.id = if self.field {
            api.resolve_field(self.world.as_deref(), key).await?.id
        } else {
            api.resolve_card(self.world.as_deref(), self.namespace, key)
                .await?
                .identity
                .id
        };
        Ok(self)
    }
    async fn permission(&self, s: &AppState, h: &HeaderMap) -> Result<(), Error> {
        if self.world.is_some() {
            world_permission(s, h, self.world.as_deref()).await
        } else {
            library_permission(s, h).await
        }
    }
}
fn service(s: &AppState) -> Cards {
    s.cards.clone()
}
fn field_pin(d: &Values, world: Option<String>) -> Result<FieldPin, Error> {
    let p = FieldPin {
        world_id: world,
        field_id: value(d, "field_id").into(),
        version: positive(value(d, "field_revision"), "field_revision")?,
    };
    p.validate().map_err(Error::Invalid)?;
    Ok(p)
}
fn parse_mappings(d: &Values, world: Option<String>) -> Result<Vec<FieldMapping>, Error> {
    let mut out = vec![];
    if !value(d, "shared_field_id").is_empty() {
        out.push(FieldMapping {
            shared_field_id: value(d, "shared_field_id").into(),
            local_field: field_pin(d, world.clone())?,
        });
    } else if !value(d, "field_id").is_empty() || !value(d, "field_revision").is_empty() {
        return Err(invalid(
            "shared_field_id",
            "Complete the initial mapping or clear all three initial mapping fields.",
        ));
    }
    for (index, line) in value(d, "mappings").lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let tokens = line.split_whitespace().collect::<Vec<_>>();
        if tokens.len() != 3
            || !game_data::types::valid_id(tokens[0])
            || !game_data::types::valid_id(tokens[1])
        {
            return Err(invalid(
                "mappings",
                &format!(
                    "Line {}: use exactly shared_field_id local_field_id revision, with immutable IDs.",
                    index + 1
                ),
            ));
        }
        let pin = FieldPin {
            world_id: world.clone(),
            field_id: tokens[1].into(),
            version: positive(tokens[2], "mappings")?,
        };
        pin.validate().map_err(Error::Invalid)?;
        out.push(FieldMapping {
            shared_field_id: tokens[0].into(),
            local_field: pin,
        });
    }
    Ok(out)
}
fn card_pin(d: &Values) -> Result<CardPin, Error> {
    let scope = value(d, "reference_scope");
    let p = CardPin {
        identity: CardIdentity {
            namespace: namespace(value(d, "reference_namespace"))?,
            world_id: if scope == "shared" {
                None
            } else {
                Some(scope.into())
            },
            id: value(d, "reference_id").into(),
        },
        version: positive(value(d, "reference_revision"), "reference_revision")?,
    };
    p.validate().map_err(Error::Invalid)?;
    Ok(p)
}
fn pin_text(p: &CardPin) -> String {
    format!(
        "{} / {} / {} @{}",
        p.identity.world_id.as_deref().unwrap_or("shared"),
        p.identity.namespace.as_str(),
        p.identity.id,
        p.version
    )
}
fn field_pin_text(p: &FieldPin) -> String {
    format!(
        "{} / {} @{}",
        p.world_id.as_deref().unwrap_or("shared"),
        p.field_id,
        p.version
    )
}
fn digest(v: &impl serde::Serialize) -> Result<String, Error> {
    Ok(hex::encode(Sha256::digest(
        serde_json::to_vec(v).map_err(|_| Error::Unavailable)?,
    )))
}

struct Link {
    label: String,
    url: String,
}
struct Section {
    title: String,
    lines: Vec<String>,
    links: Vec<Link>,
}
#[derive(Template)]
#[template(path = "cards.html")]
struct Page {
    title: String,
    context: String,
    links: Vec<Link>,
    sections: Vec<Section>,
    previous: String,
    next: String,
    filters: Vec<Control>,
}
fn scope(c: &Context) -> String {
    format!(
        "Scope: {}. Technical origin namespaces do not impose gameplay classes.",
        c.world.as_deref().unwrap_or("shared library")
    )
}
fn link(label: impl Into<String>, url: impl Into<String>) -> Link {
    Link {
        label: label.into(),
        url: url.into(),
    }
}
fn query_text(s: &str) -> String {
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
fn nav(c: &Context) -> Vec<Link> {
    let mut other = c.clone();
    other.field = !c.field;
    vec![
        link("Cards", if c.field { other.root() } else { c.root() }),
        link(
            "Reusable fields",
            if c.field { c.root() } else { other.root() },
        ),
    ]
}
fn pagination(base: &str, n: usize, more: bool) -> (String, String) {
    (
        if n > 1 {
            format!("{base}?page={}", n - 1)
        } else {
            String::new()
        },
        if more {
            format!("{base}?page={}", n + 1)
        } else {
            String::new()
        },
    )
}
async fn catalog(
    State(s): State<AppState>,
    Path(p): Path<Values>,
    uri: Uri,
    q: Result<Query<Vec<(String, String)>>, QueryRejection>,
) -> Response {
    finish(
        async {
            let c = Context::new(&p, &uri)?;
            let Query(pairs) = q.map_err(|_| invalid("query", "Use native page navigation."))?;
            let mut q = Values::new();
            for (key, value) in pairs {
                if q.insert(key, value).is_some() {
                    return Err(invalid("query", "Use each catalog filter once."));
                }
            }
            pickers::catalog_allow(&q, c.field)?;
            let n = page(&q)?;
            let api = service(&s);
            let mut sections = vec![];
            let has_more = if c.field {
                let results = api
                    .search_fields(c.world.as_deref(), &pickers::field_search(&q, "")?)
                    .await?;
                let more = results.has_more;
                for f in results.items {
                    let mut object = c.clone();
                    object.id = addresses::segment(&f.fields.slug);
                    sections.push(Section {
                        title: f.fields.display_name.clone(),
                        lines: vec![
                            format!("Exact revision {}", f.pin.version),
                            f.kind.as_str().into(),
                            archive_text(f.archived_at),
                        ],
                        links: vec![link("View field", object.url())],
                    });
                }
                more
            } else {
                let results = api
                    .search_cards(c.world.as_deref(), &pickers::card_search(&q, "")?)
                    .await?;
                let more = results.has_more;
                for card in results.items {
                    let mut object = c.clone();
                    object.id = if card.pin.identity.namespace == CardNamespace::Standalone {
                        addresses::segment(&card.fields.slug)
                    } else {
                        card.pin.identity.id.clone()
                    };
                    object.namespace = card.pin.identity.namespace;
                    sections.push(Section {
                        title: card.fields.display_name.clone(),
                        lines: vec![
                            card.fields.description.clone(),
                            format!(
                                "Labels: {}{}",
                                card.labels.join(", "),
                                if card.label_count > card.labels.len() {
                                    " (more labels)"
                                } else {
                                    ""
                                }
                            ),
                            format!(
                                "{} origin · exact revision {}",
                                card.pin.identity.namespace.as_str(),
                                card.pin.version
                            ),
                            archive_text(card.archived_at),
                        ],
                        links: vec![link("View card", object.url())],
                    });
                }
                more
            };
            let (previous, next) = pickers::catalog_pages(&c.root(), &q, n, has_more);
            let mut links = nav(&c);
            links.push(link(
                if c.field {
                    "Create reusable field"
                } else {
                    "Create standalone card"
                },
                format!("{}/new", c.root()),
            ));
            Ok(render_status(
                Page {
                    title: if c.field { "Reusable fields" } else { "Cards" }.into(),
                    context: scope(&c),
                    links,
                    sections,
                    previous,
                    next,
                    filters: pickers::catalog_controls(&q, c.field),
                },
                StatusCode::OK,
            ))
        }
        .await,
    )
}
fn archive_text(a: Option<i64>) -> String {
    if a.is_some() { "Archived" } else { "Active" }.into()
}
fn kind_text(k: &FieldKind) -> String {
    match k {
        FieldKind::Number(n) => format!(
            "Number; minimum {}; maximum {}; whole only {}",
            n.minimum
                .map(|v| v.to_string())
                .unwrap_or("unbounded".into()),
            n.maximum
                .map(|v| v.to_string())
                .unwrap_or("unbounded".into()),
            n.whole_only
        ),
        FieldKind::Text { max_length } => format!("Text; at most {max_length} characters"),
        FieldKind::Boolean => "Boolean".into(),
        FieldKind::Choice { options } => format!("Choice; {} stable options", options.len()),
        FieldKind::CardReference => "Exact qualified CardReference".into(),
    }
}
fn value_text(v: &FieldValue) -> String {
    match v {
        FieldValue::Missing => "Missing".into(),
        FieldValue::Unknown => "Unknown".into(),
        FieldValue::NotApplicable => "NotApplicable".into(),
        FieldValue::Number(n) => format!("Number {n}"),
        FieldValue::Text(t) => format!(
            "Text ({}) {t}",
            if t.is_empty() { "empty" } else { "known" }
        ),
        FieldValue::Boolean(b) => format!("Boolean {b}"),
        FieldValue::Choice(id) => format!("Choice stable option {id}"),
        FieldValue::CardReference(p) => format!("CardReference {}", pin_text(p)),
    }
}
fn card_sections(card: &CardRecord, n: usize) -> Vec<Section> {
    let mut lines = vec![
        card.fields.display_name.clone(),
        card.fields.description.clone(),
    ];
    if card.legacy_unmodeled {
        lines.push("Legacy revision: card component unmodeled.".into());
    }
    lines.push(match &card.component.rules_text {
        None => "Rules text: unmodeled".into(),
        Some(t) => format!("Rules text (inert): {t}"),
    });
    let mut out = vec![Section {
        title: "Card definition".into(),
        lines,
        links: vec![],
    }];
    out.push(Section {
        title: format!("Labels — {}", collection(&card.component.labels)),
        lines: card
            .component
            .labels
            .iter()
            .flatten()
            .skip((n - 1) * SIZE)
            .take(SIZE)
            .cloned()
            .collect(),
        links: vec![],
    });
    out.push(Section {
        title: format!("Assignments — {}", collection(&card.component.fields)),
        lines: card
            .component
            .fields
            .iter()
            .flatten()
            .skip((n - 1) * SIZE)
            .take(SIZE)
            .map(|f| {
                format!(
                    "{} — {} — {}",
                    f.definition.fields.display_name,
                    field_pin_text(&f.definition.pin()),
                    value_text(&f.value)
                )
            })
            .collect(),
        links: vec![],
    });
    let mut technical = vec![pin_text(&card.pin()), archive_text(card.archived_at)];
    if let Some(pin) = &card.source {
        technical.push(format!("Pinned source: {}", pin_text(pin)));
    }
    out.push(Section {
        title: "Technical context".into(),
        lines: technical,
        links: vec![],
    });
    out
}
async fn readable_card_sections(
    api: &Cards,
    card: &CardRecord,
    n: usize,
) -> Result<Vec<Section>, Error> {
    let mut sections = card_sections(card, n);
    let displayed = card
        .component
        .fields
        .iter()
        .flatten()
        .skip((n - 1) * SIZE)
        .take(SIZE)
        .collect::<Vec<_>>();
    // Resolve only displayed exact references plus the single explicit source; never heads.
    let mut names = BTreeMap::new();
    if let Some(pin) = &card.source {
        let source = api.version(pin).await?;
        sections[0]
            .lines
            .push(format!("Source: {}", source.fields.display_name));
        sections[0].links.push(link(
            format!(
                "Source {} · exact revision {}",
                source.fields.display_name, pin.version
            ),
            pickers::card_url(pin),
        ));
        names.insert(pin_text(pin), source.fields.display_name);
    }
    sections[2].lines.clear();
    for assignment in displayed {
        let display = match &assignment.value {
            FieldValue::Choice(id) => {
                let label = if let FieldKind::Choice { options } = &assignment.definition.kind {
                    options
                        .iter()
                        .find(|o| o.id == *id)
                        .map(|o| o.label.as_str())
                } else {
                    None
                };
                format!("Choice: {}", label.unwrap_or("Unresolved exact choice"))
            }
            FieldValue::CardReference(pin) => {
                let key = pin_text(pin);
                if !names.contains_key(&key) {
                    names.insert(key.clone(), api.version(pin).await?.fields.display_name);
                }
                let name = names.get(&key).ok_or(Error::Unavailable)?;
                sections[2].links.push(link(
                    format!("Referenced {name} · exact revision {}", pin.version),
                    pickers::card_url(pin),
                ));
                format!("CardReference: {name} · exact revision {}", pin.version)
            }
            other => value_text(other),
        };
        sections[2].lines.push(format!(
            "{} — {}",
            assignment.definition.fields.display_name, display
        ));
        let technical = sections.last_mut().ok_or(Error::Unavailable)?;
        technical.lines.push(format!(
            "Field {} · {} · {}",
            assignment.definition.fields.display_name,
            field_pin_text(&assignment.definition.pin()),
            value_text(&assignment.value)
        ));
        sections[2].links.push(link(
            format!(
                "Field {} · exact revision {}",
                assignment.definition.fields.display_name, assignment.definition.revision
            ),
            pickers::field_url(&assignment.definition.pin()),
        ));
    }
    Ok(sections)
}
fn collection<T>(v: &Option<Vec<T>>) -> String {
    match v {
        None => "unmodeled".into(),
        Some(v) => format!("defined, {} entries", v.len()),
    }
}
fn field_sections(f: &FieldDefinition, n: usize) -> Vec<Section> {
    let mut lines = vec![
        f.fields.display_name.clone(),
        field_pin_text(&f.pin()),
        archive_text(f.archived_at),
        f.fields.description.clone(),
        kind_text(&f.kind),
    ];
    if let Some(p) = &f.source {
        lines.push(format!("Pinned source: {}", field_pin_text(p)));
    }
    let mut out = vec![Section {
        title: "Reusable field definition".into(),
        lines,
        links: vec![],
    }];
    if let FieldKind::Choice { options } = &f.kind {
        out.push(Section {
            title: format!("Choice options ({} total)", options.len()),
            lines: options
                .iter()
                .skip((n - 1) * SIZE)
                .take(SIZE)
                .map(|o| format!("{} — {}", o.id, o.label))
                .collect(),
            links: vec![],
        });
    }
    out
}
fn native_url(c: &Context) -> String {
    let prefix = c
        .world
        .as_ref()
        .map(|w| format!("/worlds/{w}"))
        .unwrap_or_default();
    match c.namespace {
        CardNamespace::Form => format!("/forms/{}", c.id),
        CardNamespace::Variant => format!("{prefix}/variants/{}", c.id),
        CardNamespace::Type => format!("{prefix}/types/{}", c.id),
        CardNamespace::Move => format!("{prefix}/moves/{}", c.id),
        CardNamespace::Standalone => String::new(),
    }
}
async fn detail(
    State(s): State<AppState>,
    Path(p): Path<Values>,
    uri: Uri,
    q: Result<Query<Values>, QueryRejection>,
) -> Response {
    finish(
        async {
            let c = Context::new(&p, &uri)?.resolved(&service(&s)).await?;
            let Query(q) = q.map_err(|_| invalid("query", "Use page navigation."))?;
            let n = page(&q)?;
            let api = service(&s);
            let historical = p.contains_key("revision");
            let mut links = nav(&c);
            links.push(link("History", format!("{}/history", c.url())));
            let (title, sections, count) = if c.field {
                let f = if historical {
                    api.field_version(&FieldPin {
                        world_id: c.world.clone(),
                        field_id: c.id.clone(),
                        version: positive(value(&p, "revision"), "revision")?,
                    })
                    .await?
                } else {
                    api.field(c.world.as_deref(), &c.id).await?
                };
                if !historical {
                    for op in ["name", "description", "kind", "option", "archive", "adopt"] {
                        if op != "adopt" || f.source.is_some() {
                            links.push(link(
                                format!("Edit {op}"),
                                format!("{}/edit?operation={op}", c.url()),
                            ));
                        }
                    }
                }
                let count = if let FieldKind::Choice { options } = &f.kind {
                    options.len()
                } else {
                    0
                };
                let mut sections = field_sections(&f, n);
                if let Some(pin) = &f.source {
                    let source = api.field_version(pin).await?;
                    sections[0]
                        .lines
                        .push(format!("Source: {}", source.fields.display_name));
                    sections[0].links.push(link(
                        format!(
                            "Source {} · exact revision {}",
                            source.fields.display_name, pin.version
                        ),
                        pickers::field_url(pin),
                    ));
                }
                if !historical && let FieldKind::Choice { options } = &f.kind {
                    for o in options.iter().skip((n - 1) * SIZE).take(SIZE) {
                        sections[1].links.push(link(
                            format!("Load option {}", o.label),
                            format!("{}/edit?operation=option&target={}", c.url(), o.id),
                        ));
                    }
                }
                (f.fields.display_name.clone(), sections, count)
            } else {
                let card = if historical {
                    api.version(&CardPin {
                        identity: c.identity(),
                        version: positive(value(&p, "revision"), "revision")?,
                    })
                    .await?
                } else {
                    api.detail(&c.identity()).await?
                };
                let native = native_url(&c);
                if !native.is_empty() {
                    links.push(link(
                        "Specialized metadata editor",
                        format!("{native}/edit"),
                    ));
                    links.push(link("Native full history", format!("{native}/history")));
                }
                if !historical {
                    for op in ["rules_text", "label", "assignment", "copy"] {
                        if op != "copy" || card.source.is_some() {
                            links.push(link(
                                format!("Edit {op}"),
                                format!("{}/edit?operation={op}", c.url()),
                            ));
                        }
                    }
                    if c.namespace == CardNamespace::Standalone {
                        for op in ["name", "description", "archive", "adopt"] {
                            if op != "adopt" || card.source.is_some() {
                                links.push(link(
                                    format!("Edit {op}"),
                                    format!("{}/edit?operation={op}", c.url()),
                                ));
                            }
                        }
                    }
                }
                let count = card
                    .component
                    .labels
                    .as_ref()
                    .map_or(0, Vec::len)
                    .max(card.component.fields.as_ref().map_or(0, Vec::len));
                let mut sections = readable_card_sections(&api, &card, n).await?;
                if !historical {
                    for label in card
                        .component
                        .labels
                        .iter()
                        .flatten()
                        .skip((n - 1) * SIZE)
                        .take(SIZE)
                    {
                        sections[1].links.push(link(
                            format!("Load label {label}"),
                            format!(
                                "{}/edit?operation=label&target={}",
                                c.url(),
                                query_text(label)
                            ),
                        ));
                    }
                    for f in card
                        .component
                        .fields
                        .iter()
                        .flatten()
                        .skip((n - 1) * SIZE)
                        .take(SIZE)
                    {
                        sections[2].links.push(link(
                            format!("Load assignment {}", f.definition.fields.display_name),
                            format!(
                                "{}/edit?operation=assignment&target={}",
                                c.url(),
                                f.definition.id
                            ),
                        ));
                    }
                }
                (card.fields.display_name.clone(), sections, count)
            };
            let (previous, next) = pagination(uri.path(), n, count > n * SIZE);
            Ok(render_status(
                Page {
                    title,
                    context: scope(&c),
                    links,
                    sections,
                    previous,
                    next,
                    filters: vec![],
                },
                StatusCode::OK,
            ))
        }
        .await,
    )
}
async fn history(
    State(s): State<AppState>,
    Path(p): Path<Values>,
    uri: Uri,
    q: Result<Query<Values>, QueryRejection>,
) -> Response {
    finish(async {
        let c=Context::new(&p,&uri)?.resolved(&service(&s)).await?;let Query(q)=q.map_err(|_|invalid("query","Use page navigation."))?;let n=page(&q)?;let api=service(&s);
        let revision=if c.field {api.field(c.world.as_deref(),&c.id).await?.revision}else{api.detail(&c.identity()).await?.revision};
        let start=revision.saturating_sub(((n-1)*SIZE).try_into().unwrap_or(i64::MAX));
        let mut sections=vec![];
        let entries=if start<=0{vec![]}else if c.field{api.field_history(c.world.as_deref(),&c.id,start.checked_add(1)).await?}else{api.history(&c.identity(),start.checked_add(1)).await?};
        for entry in entries {
            let version=entry.revision;
            let (name,summary)=if c.field {let f=api.field_version(&FieldPin {world_id:c.world.clone(),field_id:c.id.clone(),version}).await?;(f.fields.display_name.clone(),kind_text(&f.kind))}else{let card=api.version(&CardPin {identity:c.identity(),version}).await?;(card.fields.display_name.clone(),if card.legacy_unmodeled{"Legacy card component unmodeled".into()}else{format!("Assignments {}; labels {}",collection(&card.component.fields),collection(&card.component.labels))})};
            sections.push(Section {title:format!("Revision {version}: {name}"),lines:vec![summary,format!("By {} · {} · {}",entry.actor_name,entry.action,entry.occurred_at),format!("Reason: {}",entry.reason.as_deref().unwrap_or("Not provided"))],links:vec![link(format!("Revision {version}"),format!("{}/history/{version}",c.url()))]});
        }
        let (previous,next)=pagination(uri.path(),n,start>SIZE as i64);
        let mut links=nav(&c);links.push(link("Current definition",c.url()));let native=native_url(&c);if !c.field&&!native.is_empty(){links.push(link("Native full history",format!("{native}/history")));}
        Ok(render_status(Page {title:"Card authoring history".into(),context:"Exact saved revisions with public author attribution, action, UTC time and change reason. At most 25 revisions per page.".into(),links,sections,previous,next,filters:vec![]},StatusCode::OK))
    }.await)
}

struct OptionView {
    value: String,
    label: String,
    selected: bool,
}
struct Control {
    name: String,
    label: String,
    value: String,
    multiline: bool,
    options: Vec<OptionView>,
    error: String,
    advanced: bool,
    help: String,
    inputmode: String,
}
#[derive(Template)]
#[template(path = "cards_editor.html")]
struct Editor {
    title: String,
    context: String,
    action: String,
    csrf: String,
    revision: String,
    operation: String,
    controls: Vec<Control>,
    error: String,
    errors: Vec<(String, String)>,
    raw: Vec<(String, String)>,
    raw_readonly: Vec<(String, String)>,
    sections: Vec<Section>,
    preview: bool,
    digest: String,
    current_revision: String,
    back: String,
    catalogs: Vec<Link>,
    picker: pickers::Picker,
}
fn control(d: &Values, k: &str, label: &str, multiline: bool) -> Control {
    Control {
        name: k.into(),
        label: label.into(),
        value: value(d, k).into(),
        multiline,
        options: vec![],
        error: String::new(),
        advanced: false,
        help: String::new(),
        inputmode: String::new(),
    }
}
fn select(d: &Values, k: &str, label: &str, options: &[(&str, &str)]) -> Control {
    let mut c = control(d, k, label, false);
    c.options = options
        .iter()
        .map(|(v, l)| OptionView {
            value: (*v).into(),
            label: (*l).into(),
            selected: value(d, k) == *v,
        })
        .collect();
    if !options.iter().any(|(v, _)| *v == value(d, k)) {
        c.options.push(OptionView {
            value: value(d, k).into(),
            label: format!("Unrecognized raw value: {}", value(d, k)),
            selected: true,
        });
    }
    c
}
fn put(d: &mut Values, k: &str, v: impl ToString) {
    d.insert(k.into(), v.to_string());
}
fn kind_defaults(d: &mut Values, k: &FieldKind) {
    put(
        d,
        "kind",
        match k {
            FieldKind::Number(_) => "number",
            FieldKind::Text { .. } => "text",
            FieldKind::Boolean => "boolean",
            FieldKind::Choice { .. } => "choice",
            FieldKind::CardReference => "card_reference",
        },
    );
    if let FieldKind::Number(n) = k {
        put(
            d,
            "minimum",
            n.minimum.map(|v| v.to_string()).unwrap_or_default(),
        );
        put(
            d,
            "maximum",
            n.maximum.map(|v| v.to_string()).unwrap_or_default(),
        );
        put(d, "whole_only", if n.whole_only { "yes" } else { "no" });
    }
    if let FieldKind::Text { max_length } = k {
        put(d, "max_length", max_length);
    }
}
fn controls(c: &Context, d: &Values) -> Vec<Control> {
    let mut out = vec![];
    let op = value(d, "operation");
    match op {
        "create" => {
            out.push(control(d, "display_name", "Display name", false));
            out.push(control(d, "text", "Description (inert plain text)", true));
            out.push(control(
                d,
                "source_id",
                "Optional shared source identity",
                false,
            ));
            out.push(control(
                d,
                "source_revision",
                "Optional shared source revision",
                false,
            ));
        }
        "name" => out.push(control(d, "display_name", "Display name", false)),
        "description" => out.push(control(
            d,
            "text",
            "Loaded description (inert plain text)",
            true,
        )),
        "rules_text" => {
            out.push(select(
                d,
                "state",
                "Rules text state",
                &[
                    ("defined", "Defined (empty text is distinct)"),
                    ("unmodeled", "Unmodeled"),
                ],
            ));
            out.push(control(
                d,
                "text",
                "Loaded rules text (inert plain text)",
                true,
            ));
        }
        "archive" => out.push(select(
            d,
            "archived",
            "Archive state",
            &[("no", "Active"), ("yes", "Archived")],
        )),
        "label" => {
            out.push(select(
                d,
                "mode",
                "Label operation",
                &[
                    ("set", "Add or replace one label"),
                    ("remove", "Remove one label"),
                    ("define", "Define empty only if unmodeled"),
                    ("unmodel", "Unmodel only if empty"),
                ],
            ));
            out.push(control(
                d,
                "target",
                "Existing label (exact, blank to add)",
                false,
            ));
            out.push(control(d, "label", "New label", false));
        }
        "option" => {
            out.push(select(
                d,
                "mode",
                "Choice option operation",
                &[
                    ("set", "Add or update one option"),
                    ("remove", "Remove one option"),
                ],
            ));
            out.push(control(
                d,
                "option_id",
                "Stable option identity (blank generates one)",
                false,
            ));
            out.push(control(d, "label", "Option label", false));
        }
        "assignment" => {
            out.push(select(
                d,
                "mode",
                "Assignment operation",
                &[
                    ("set", "Add or replace one assignment"),
                    ("remove", "Remove one assignment"),
                    ("define", "Define empty only if unmodeled"),
                    ("unmodel", "Unmodel only if empty"),
                ],
            ));
            out.push(control(d, "field_id", "Field identity (same scope)", false));
            out.push(control(d, "field_revision", "Exact field revision", false));
            out.push(select(
                d,
                "value_state",
                "Field value state",
                &[
                    ("missing", "Missing"),
                    ("unknown", "Unknown"),
                    ("not_applicable", "NotApplicable"),
                    ("number", "Number (zero is known)"),
                    ("text", "Text (empty is known)"),
                    ("boolean", "Boolean"),
                    ("choice", "Choice"),
                    ("card_reference", "CardReference"),
                ],
            ));
            out.push(control(
                d,
                "text",
                "Loaded field value (number, text or stable choice ID)",
                true,
            ));
            out.push(select(
                d,
                "boolean",
                "Known boolean",
                &[("false", "false"), ("true", "true")],
            ));
            out.push(control(
                d,
                "reference_scope",
                "Reference scope (shared or immutable world ID)",
                false,
            ));
            out.push(select(
                d,
                "reference_namespace",
                "Reference technical namespace",
                &[
                    ("form", "form"),
                    ("variant", "variant"),
                    ("type", "type"),
                    ("move", "move"),
                    ("standalone", "standalone"),
                ],
            ));
            out.push(control(d, "reference_id", "Reference card identity", false));
            out.push(control(
                d,
                "reference_revision",
                "Exact reference revision",
                false,
            ));
        }
        "copy" => {
            out.push(control(d,"mappings","Additional explicit mappings (one shared_field_id local_field_id revision per line)",true));
            out.push(control(
                d,
                "page",
                "Review page (25 entries per list)",
                false,
            ));
            out.push(control(
                d,
                "shared_field_id",
                "Mapped shared field identity (optional initial mapping)",
                false,
            ));
            out.push(control(d, "field_id", "Mapped local field identity", false));
            out.push(control(
                d,
                "field_revision",
                "Exact mapped local field revision",
                false,
            ));
        }
        "adopt" => {
            out.push(control(
                d,
                "page",
                "Review page (25 entries per list)",
                false,
            ));
            out.push(control(
                d,
                "source_revision",
                "Shared source revision to adopt (keep local)",
                false,
            ));
        }
        _ => {}
    }
    if c.field && (op == "create" || op == "kind") {
        out.push(select(
            d,
            "kind",
            "Field kind",
            &[
                ("number", "Number"),
                ("text", "Text"),
                ("boolean", "Boolean"),
                ("choice", "Choice"),
                ("card_reference", "CardReference"),
            ],
        ));
        out.push(control(
            d,
            "minimum",
            "Minimum (inclusive, blank is unbounded)",
            false,
        ));
        out.push(control(
            d,
            "maximum",
            "Maximum (inclusive, blank is unbounded)",
            false,
        ));
        out.push(select(
            d,
            "whole_only",
            "Whole numbers only",
            &[("no", "No"), ("yes", "Yes")],
        ));
        out.push(control(
            d,
            "max_length",
            "Maximum text characters (0–2000)",
            false,
        ));
    }
    out.push(control(d, "reason", "Change reason (optional)", false));
    out
}
async fn defaults(c: &Context, api: &Cards, op: &str, target: &str) -> Result<Values, Error> {
    let mut d = Values::new();
    for (k, v) in [
        ("operation", op),
        ("expected_revision", "0"),
        ("mode", "set"),
        ("state", "defined"),
        ("kind", "text"),
        ("max_length", "2000"),
        ("whole_only", "no"),
        ("archived", "no"),
        ("value_state", "unknown"),
        ("boolean", "false"),
        ("reference_scope", "shared"),
        ("reference_namespace", "standalone"),
        ("page", "1"),
    ] {
        put(&mut d, k, v);
    }
    if c.id.is_empty() {
        pickers::prune_defaults(c, &mut d);
        return Ok(d);
    }
    if c.field {
        let f = api.field(c.world.as_deref(), &c.id).await?;
        put(&mut d, "expected_revision", f.revision);
        put(&mut d, "display_name", &f.fields.display_name);
        put(
            &mut d,
            "archived",
            if f.archived_at.is_some() { "yes" } else { "no" },
        );
        kind_defaults(&mut d, &f.kind);
        if op == "description" {
            put(&mut d, "text", f.fields.description);
        }
        if let Some(p) = f.source {
            put(&mut d, "source_revision", p.version);
        }
        if op == "option"
            && !target.is_empty()
            && let FieldKind::Choice { options } = f.kind
        {
            let o = options
                .iter()
                .find(|o| o.id == target)
                .ok_or(Error::NotFound)?;
            put(&mut d, "option_id", &o.id);
            put(&mut d, "label", &o.label);
        }
    } else {
        let card = api.detail(&c.identity()).await?;
        put(&mut d, "expected_revision", card.revision);
        put(&mut d, "display_name", &card.fields.display_name);
        put(
            &mut d,
            "archived",
            if card.archived_at.is_some() {
                "yes"
            } else {
                "no"
            },
        );
        if op == "description" {
            put(&mut d, "text", &card.fields.description);
        }
        if op == "rules_text" {
            put(
                &mut d,
                "state",
                if card.component.rules_text.is_some() {
                    "defined"
                } else {
                    "unmodeled"
                },
            );
            put(
                &mut d,
                "text",
                card.component.rules_text.as_deref().unwrap_or(""),
            );
        }
        if let Some(p) = card.source {
            put(&mut d, "source_revision", p.version);
        }
        if op == "label" && !target.is_empty() {
            if !card.component.labels.iter().flatten().any(|l| l == target) {
                return Err(Error::NotFound);
            }
            put(&mut d, "target", target);
            put(&mut d, "label", target);
        }
        if op == "assignment" && !target.is_empty() {
            let f = card
                .component
                .fields
                .iter()
                .flatten()
                .find(|f| f.definition.id == target)
                .ok_or(Error::NotFound)?;
            pickers::load_values(&mut d, f, card.revision);
        }
    }
    pickers::prune_defaults(c, &mut d);
    Ok(d)
}
async fn editor(
    State(s): State<AppState>,
    Path(p): Path<Values>,
    uri: Uri,
    h: HeaderMap,
    q: Result<Query<Values>, QueryRejection>,
) -> Response {
    finish(
        async {
            let c = Context::new(&p, &uri)?.resolved(&service(&s)).await?;
            c.permission(&s, &h).await?;
            let Query(q) = q.map_err(|_| invalid("query", "Use the native editor links."))?;
            let op = if c.id.is_empty() {
                "create"
            } else {
                value(&q, "operation")
            };
            let d = defaults(&c, &service(&s), op, value(&q, "target")).await?;
            let (csrf, set) = csrf_for(&h).map_err(|_| Error::Unavailable)?;
            let mut sections = vec![];
            if !c.id.is_empty() {
                if c.field {
                    let f = service(&s).field(c.world.as_deref(), &c.id).await?;
                    sections = field_sections(&f, 1);
                } else {
                    let card = service(&s).detail(&c.identity()).await?;
                    sections = card_sections(&card, 1);
                }
                sections[0].title = "Current saved context".into();
            }
            let mut response = Box::pin(show_editor(
                &service(&s),
                &c,
                &d,
                csrf.clone(),
                None,
                sections,
                ReviewState::default(),
            ))
            .await;
            if set {
                append_cookie(&mut response, csrf_cookie(&csrf, s.cookie_secure));
            }
            Ok(response)
        }
        .await,
    )
}
#[derive(Default)]
struct ReviewState {
    digest: String,
    current_revision: String,
}
async fn show_editor(
    api: &Cards,
    c: &Context,
    d: &Values,
    csrf: String,
    error: Option<&Error>,
    sections: Vec<Section>,
    review: ReviewState,
) -> Response {
    let mut controls = controls(c, d);
    let (picker, picker_error) = match Box::pin(pickers::prepare(api, c, d, &mut controls)).await {
        Ok(picker) => (picker, None),
        Err(e) => (pickers::Picker::empty(), Some(e)),
    };
    let error = error.or(picker_error.as_ref());
    if let Some(Error::Invalid(errors)) = error {
        for control in &mut controls {
            if let Some(message) = errors.get(&control.name) {
                control.error = message.clone();
            }
        }
    }
    let names = controls.iter().map(|v| v.name.as_str()).collect::<Vec<_>>();
    let raw_values = d
        .iter()
        .filter(|(k, _)| {
            !names.contains(&k.as_str())
                && !matches!(
                    k.as_str(),
                    "csrf"
                        | "expected_revision"
                        | "operation"
                        | "digest"
                        | "intent"
                        | "reviewed_revision"
                )
        })
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect::<Vec<_>>();
    let (raw, raw_readonly) = raw_values
        .into_iter()
        .partition(|(key, _)| pickers::KEYS.contains(&key.as_str()) || key == "loaded_field_id");
    no_store(render_status(
        Editor {
            title: format!(
                "{} {}",
                if c.field { "Reusable field" } else { "Card" },
                value(d, "operation")
            ),
            context: scope(c),
            action: if c.id.is_empty() {
                c.url()
            } else {
                format!("{}/edit", c.url())
            },
            csrf,
            revision: value(d, "expected_revision").into(),
            operation: value(d, "operation").into(),
            controls,
            error: error.map(error_text).unwrap_or("").into(),
            errors: if let Some(Error::Invalid(e)) = error {
                e.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
            } else {
                vec![]
            },
            raw,
            raw_readonly,
            sections,
            preview: !review.digest.is_empty(),
            digest: review.digest,
            current_revision: review.current_revision,
            back: if c.id.is_empty() { c.root() } else { c.url() },
            catalogs: nav(c),
            picker,
        },
        error.map(error_status).unwrap_or(StatusCode::OK),
    ))
}
fn parse_kind(d: &Values, old: Option<&FieldKind>) -> Result<FieldKind, Error> {
    let decimal = |k| {
        if value(d, k).is_empty() {
            Ok(None)
        } else {
            value(d, k)
                .parse()
                .map(Some)
                .map_err(|e: String| invalid(k, &e))
        }
    };
    match value(d, "kind") {
        "number" => Ok(FieldKind::Number(NumericConstraints {
            minimum: decimal("minimum")?,
            maximum: decimal("maximum")?,
            whole_only: match value(d, "whole_only") {
                "yes" => true,
                "no" => false,
                _ => return Err(invalid("whole_only", "Choose Yes or No.")),
            },
        })),
        "text" => Ok(FieldKind::Text {
            max_length: value(d, "max_length")
                .parse::<u32>()
                .ok()
                .filter(|n| *n <= 2000)
                .ok_or_else(|| invalid("max_length", "Use 0–2000 characters."))?,
        }),
        "boolean" => Ok(FieldKind::Boolean),
        "card_reference" => Ok(FieldKind::CardReference),
        "choice" => Ok(FieldKind::Choice {
            options: if let Some(FieldKind::Choice { options }) = old {
                options.clone()
            } else {
                vec![]
            },
        }),
        _ => Err(invalid("kind", "Choose a supported field kind.")),
    }
}
fn archive(d: &Values, old: Option<i64>) -> Result<Option<i64>, Error> {
    match value(d, "archived") {
        "no" => Ok(None),
        "yes" => Ok(Some(match old {
            Some(t) => t,
            None => std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|_| Error::Unavailable)?
                .as_secs()
                .try_into()
                .map_err(|_| Error::Unavailable)?,
        })),
        _ => Err(invalid("archived", "Choose Active or Archived.")),
    }
}
// Native textareas submit CRLF regardless of the document's LF representation.
// Normalize transport line endings, retaining every other entered character.
fn textarea_text(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}
fn field_value(d: &Values) -> Result<FieldValue, Error> {
    Ok(match value(d, "value_state") {
        "missing" => FieldValue::Missing,
        "unknown" => FieldValue::Unknown,
        "not_applicable" => FieldValue::NotApplicable,
        "number" => FieldValue::Number(
            value(d, "text")
                .parse()
                .map_err(|e: String| invalid("text", &e))?,
        ),
        "text" => FieldValue::Text(textarea_text(value(d, "text"))),
        "boolean" => FieldValue::Boolean(match value(d, "boolean") {
            "false" => false,
            "true" => true,
            _ => return Err(invalid("boolean", "Choose true or false.")),
        }),
        "choice" => FieldValue::Choice(value(d, "text").into()),
        "card_reference" => FieldValue::CardReference(card_pin(d)?),
        _ => return Err(invalid("value_state", "Choose an explicit value state.")),
    })
}
fn stable_id() -> Result<String, Error> {
    let mut bytes = [0; 16];
    getrandom::fill(&mut bytes).map_err(|_| Error::Unavailable)?;
    Ok(hex::encode(bytes))
}
fn apply_one<T>(
    items: &mut Option<Vec<T>>,
    mode: &str,
    index: Option<usize>,
    new: Option<T>,
) -> Result<(), Error> {
    match mode {
        "define" if items.is_none() => *items = Some(vec![]),
        "unmodel" if items.as_ref().is_none_or(Vec::is_empty) => *items = None,
        "remove" => {
            let index = index.ok_or_else(|| invalid("target", "Choose an existing entry."))?;
            items
                .as_mut()
                .ok_or_else(|| invalid("target", "Collection is unmodeled."))?
                .remove(index);
        }
        "set" => {
            let new = new.ok_or_else(|| invalid("value", "Provide one value."))?;
            let entries = items.get_or_insert_default();
            if let Some(i) = index {
                entries[i] = new;
            } else {
                entries.push(new);
            }
        }
        _ => {
            return Err(invalid(
                "mode",
                "Choose a supported operation. Remove entries individually before unmodeling; defining never erases saved entries.",
            ));
        }
    }
    Ok(())
}
fn allow(d: &Values, c: &Context) -> Result<(), Error> {
    let mut keys = controls(c, d)
        .into_iter()
        .map(|v| v.name)
        .collect::<Vec<_>>();
    keys.extend(
        [
            "csrf",
            "operation",
            "expected_revision",
            "intent",
            "digest",
            "reviewed_revision",
        ]
        .map(str::to_owned),
    );
    if value(d, "operation") == "assignment" {
        keys.push("loaded_field_id".into());
        keys.push("loaded_field_marker".into());
    }
    keys.extend(pickers::KEYS.iter().map(|k| (*k).into()));
    if d.keys().any(|k| !keys.contains(k)) {
        return Err(invalid(
            "form",
            "Unexpected fields are retained read-only. Only the loaded operation may be submitted.",
        ));
    }
    Ok(())
}
async fn submit(
    State(s): State<AppState>,
    Path(p): Path<Values>,
    uri: Uri,
    h: HeaderMap,
    query: Result<Query<Values>, QueryRejection>,
    form: Result<Form<Vec<(String, String)>>, FormRejection>,
) -> Response {
    let Form(pairs) = match form {
        Ok(d) => d,
        Err(e) => return authoring_form_error(e),
    };
    let mut d = Values::new();
    let mut duplicates = vec![];
    for (key, v) in pairs {
        match d.entry(key) {
            std::collections::btree_map::Entry::Occupied(entry) => {
                duplicates.push((entry.key().clone(), v));
            }
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(v);
            }
        }
    }
    let c = match Context::new(&p, &uri) {
        Ok(c) => c,
        Err(e) => return finish(Err(e)),
    };
    let api = service(&s);
    let mut sections = vec![];
    let mut current_revision = String::new();
    let mut review = String::new();
    let result:Result<Option<String>,Error>=async {
        let token=submit_token(&s,&h,value(&d,"csrf"))?;c.permission(&s,&h).await?;
        if !c.id.is_empty()&&!game_data::types::valid_id(&c.id){return Err(invalid("id","Mutation actions must use the immutable canonical ID from the loaded form."));}
        let Query(query)=query.map_err(|_|invalid("query","Use the explicit review button."))?;
        if query.keys().any(|k|k!="reviewed_revision"){return Err(invalid("query","Unexpected submission query."));}
        if let Some(revision)=query.get("reviewed_revision"){put(&mut d,"reviewed_revision",revision);}
        let mut card=None;let mut field=None;
        let expected=if c.id.is_empty(){0}else{
            let revision=if c.field{let f=api.field(c.world.as_deref(),&c.id).await?;let r=f.revision;field=Some(f);r}else{let r=api.detail(&c.identity()).await?;let rev=r.revision;card=Some(r);rev};
            current_revision=revision.to_string();
            // Revision conflicts precede all field/value parsing. Explicit review is a new base.
            if !value(&d,"reviewed_revision").is_empty(){let reviewed=value(&d,"reviewed_revision").to_owned();put(&mut d,"expected_revision",reviewed);d.remove("reviewed_revision");}
         if value(&d,"expected_revision").parse::<i64>().ok()!=Some(revision){return Err(Error::Conflict);}revision
        };
        if !duplicates.is_empty(){return Err(invalid("form","Duplicate form fields are not accepted. All duplicate values are retained below; nothing was saved."));}
         allow(&d,&c)?;
         if pickers::is_transition(value(&d,"intent")) {
            Box::pin(pickers::transition(&api, &c, &mut d)).await?;
            return Ok(None);
        }
        allow(&d,&c)?;let op=value(&d,"operation");let n=if matches!(op,"copy"|"adopt"){page(&d)?}else{1};
        let intent=value(&d,"intent");
        if if matches!(op,"copy"|"adopt"|"archive"){!matches!(intent,"preview"|"confirm")}else{intent!="save"}{return Err(invalid("intent","Use Save operation, Preview changes or the reviewed confirmation button for this operation."));}
        game_data::authoring::validate_reason(reason(&d).as_deref()).map_err(Error::Invalid)?;
        if c.id.is_empty(){
            if op!="create"{return Err(invalid("operation","Use the create operation."));}
            let fields=game_data::authoring::ContentFields {slug:String::new(),display_name:value(&d,"display_name").into(),description:value(&d,"text").into()};
            let source=if value(&d,"source_id").is_empty(){None}else{Some(FieldPin {world_id:None,field_id:value(&d,"source_id").into(),version:positive(value(&d,"source_revision"),"source_revision")?})};
            let mut destination=c.clone();
            if c.field{destination.id=api.create_field(&token,c.world.as_deref(),NewField {fields,kind:parse_kind(&d,None)?,source,reason:reason(&d)}).await?.id;}
            else{destination.id=api.create(&token,c.world.as_deref(),NewCard {fields,source:source.map(|p|CardPin{identity:CardIdentity{namespace:CardNamespace::Standalone,world_id:None,id:p.field_id},version:p.version}),component:CardComponent::default(),reason:reason(&d)}).await?.identity.id;}
            return Ok(Some(destination.url()));
        }
        if let Some(f)=field {
            let mut edit=FieldEdit {expected_revision:expected,fields:f.fields.clone(),kind:f.kind.clone(),archived_at:f.archived_at,reason:reason(&d)};
            match op {
                "name"=>edit.fields.display_name=value(&d,"display_name").into(),"description"=>edit.fields.description=value(&d,"text").into(),"kind"=>edit.kind=parse_kind(&d,Some(&f.kind))?,"archive"=>{
                    edit.archived_at=archive(&d,f.archived_at)?;
                    let mut proposed=f.clone();proposed.archived_at=edit.archived_at;
                    if proposed!=f{proposed.revision=expected.checked_add(1).ok_or(Error::Unavailable)?;}
                    for (title,entry) in [("Current saved state",&f),("Proposed archive state",&proposed)]{let mut sec=field_sections(entry,1);sec[0].title=title.into();sections.extend(sec);}
                    // The exact archive timestamp is allocated on confirmation, not on preview.
                    review=digest(&(&f,value(&d,"archived"),reason(&d)))?;
                    if intent=="preview"{return Ok(None);}
                    if value(&d,"digest")!=review{return Err(Error::Conflict);}
                },
                "option"=>{let FieldKind::Choice{options}=&mut edit.kind else{return Err(invalid("operation","Options belong to Choice fields only."));};
                    let id=if value(&d,"option_id").is_empty(){stable_id()?}else{value(&d,"option_id").into()};let index=options.iter().position(|o|o.id==id);
                    match value(&d,"mode"){"set"=>{let option=ChoiceOption{id,label:value(&d,"label").into()};if let Some(i)=index{options[i]=option;}else{options.push(option);}},"remove"=>{options.remove(index.ok_or_else(||invalid("option_id","Choose an existing stable option ID."))?);},_=>return Err(invalid("mode","Choose set or remove."))}
                },
                "adopt"=>{
                    let world=c.world.as_deref().ok_or_else(||invalid("source","Only local definitions adopt shared sources."))?;let target=positive(value(&d,"source_revision"),"source_revision")?;
                    let preview=api.preview_field_adoption(world,&c.id,target,&edit).await?;
                    for (title,entry) in [("Old shared source",&preview.old_source),("Current local definition",&preview.entry),("New shared source",&preview.new_source),("Proposed state",&preview.proposed)]{let mut sec=field_sections(entry,n);sec[0].title=title.into();sections.extend(sec);}
                    review=digest(&(&preview.entry,&preview.old_source,&preview.new_source,&preview.proposed,reason(&d)))?;
                    if value(&d,"intent")!="confirm"{return Ok(None);}
                    if value(&d,"digest")!=review{return Err(Error::Conflict);}
                    api.adopt_field(&token,world,&c.id,target,edit).await?;return Ok(Some(c.url()));
                },_=>return Err(invalid("operation","Choose a field editor operation.")),
            }
            // Service checks the authoritative revision and policy again under its write lock.
            api.save_field(&token,c.world.as_deref(),&c.id,edit).await?;
        }else if let Some(mut row)=card {
            match op {
                "name"|"description"|"archive"=>{let mut fields=row.fields.clone();if op=="name"{fields.display_name=value(&d,"display_name").into();}
                    if op=="description"{fields.description=value(&d,"text").into();}
                    if op=="archive"{
                        if c.namespace!=CardNamespace::Standalone{return Err(invalid("operation","Use the specialized editor to archive native definitions."));}
                        let mut proposed=row.clone();proposed.archived_at=archive(&d,row.archived_at)?;
                        if proposed!=row{proposed.revision=expected.checked_add(1).ok_or(Error::Unavailable)?;}
                        for (title,entry) in [("Current saved state",&row),("Proposed archive state",&proposed)]{let mut sec=card_sections(entry,1);sec[0].title=title.into();sections.extend(sec);}
                        review=digest(&(&row,value(&d,"archived"),reason(&d)))?;
                        if intent=="preview"{return Ok(None);}
                        if value(&d,"digest")!=review{return Err(Error::Conflict);}
                    }
                    api.save_metadata(&token,&c.identity(),CardMetadataEdit{expected_revision:expected,fields,archived_at:if op=="archive"{archive(&d,row.archived_at)?}else{row.archived_at},reason:reason(&d)}).await?;
                },
                "copy"=>{
                    let mappings=parse_mappings(&d,c.world.clone())?;
                    let preview=api.preview_source_fields(&c.identity(),expected,&mappings).await?;
                    sections=card_sections(&preview.proposed,n);sections[0].title="Proposed state — whole assignment collection replacement".into();
                    for (title,entries) in [("Removed local assignments",&preview.removed),("Unmapped source assignments",&preview.unmapped)]{sections.push(Section{title:format!("{title} ({} total; page {n}, at most 25 shown)",entries.len()),lines:entries.iter().skip((n-1)*SIZE).take(SIZE).map(|f|format!("{} — {} — {}",f.definition.fields.display_name,field_pin_text(&f.definition.pin()),value_text(&f.value))).collect(),links:vec![]});}
                    let exact_mappings=preview.mappings.iter().map(|m|(&m.shared_field_id,&m.local_field)).collect::<Vec<_>>();
                    review=digest(&(&preview.entry,&preview.source,&preview.proposed,&preview.removed,&preview.unmapped,exact_mappings,value(&d,"mappings"),value(&d,"shared_field_id"),value(&d,"field_id"),value(&d,"field_revision"),reason(&d)))?;
                    if value(&d,"intent")!="confirm"{return Ok(None);}
                    if value(&d,"digest")!=review{return Err(Error::Conflict);}api.copy_source_fields(&token,&preview,reason(&d).as_deref()).await?;
                },
                "adopt"=>{
                    if c.namespace!=CardNamespace::Standalone{return Err(invalid("operation","Use the specialized editor for native source adoption."));}
                    let pin=row.source.clone().ok_or_else(||invalid("source","No pinned shared source."))?;let target=positive(value(&d,"source_revision"),"source_revision")?;
                    if target<pin.version{return Err(invalid("source_revision","Choose the pinned or a newer revision."));}
                    let old=api.version(&pin).await?;let new=api.version(&CardPin{version:target,..pin.clone()}).await?;
                    if target!=pin.version&&(new.archived_at.is_some()||api.detail(&new.identity).await?.archived_at.is_some()){return Err(invalid("source_revision","Choose an active source revision and identity."));}
                    game_data::authoring::validate_reason(reason(&d).as_deref()).map_err(Error::Invalid)?;
                    let mut proposed=row.clone();proposed.source=Some(new.pin());
                    if proposed.source!=row.source{proposed.revision=expected.checked_add(1).ok_or(Error::Unavailable)?;}
                    for (title,entry) in [("Old shared source",&old),("Current local definition",&row),("New shared source",&new),("Proposed state",&proposed)]{let mut sec=card_sections(entry,n);sec[0].title=title.into();sections.extend(sec);}
                    review=digest(&(&row,&old,&new,&proposed,reason(&d)))?;
                    if value(&d,"intent")!="confirm"{return Ok(None);}
                    if value(&d,"digest")!=review{return Err(Error::Conflict);}api.adopt_card(&token,&c.identity(),expected,target,reason(&d).as_deref()).await?;
                },
                "rules_text"|"label"|"assignment"=>{
                    if op=="rules_text"{row.component.rules_text=match value(&d,"state"){"defined"=>Some(textarea_text(value(&d,"text"))),"unmodeled" if value(&d,"text").is_empty()=>None,_=>return Err(invalid("state","Clear loaded text before unmodeling, or choose Defined."))};}
                    if op=="label"{let index=row.component.labels.iter().flatten().position(|l|l==value(&d,"target"));if !value(&d,"target").is_empty()&&index.is_none(){return Err(invalid("target","Existing label not found."));}apply_one(&mut row.component.labels,value(&d,"mode"),index,Some(value(&d,"label").to_owned()))?;}
                    if op=="assignment"{let index=row.component.fields.iter().flatten().position(|f|f.definition.id==value(&d,"field_id"));
                        let new=if value(&d,"mode")=="set"{let definition=api.field_version(&field_pin(&d,c.world.clone())?).await.map_err(|e|if matches!(e,Error::NotFound){invalid("field_id","This exact same-scope field revision does not exist.")}else{e})?;
                            if index.is_some() && matches!(definition.kind,FieldKind::Text{..}) && matches!(value(&d,"value_state"),"text"|"known") && value(&d,"loaded_field_marker")!=pickers::loaded_marker(&definition.pin(),expected) {return Err(invalid("field_id","Load this saved assignment from card detail before editing its text. The loaded marker must match the exact field and starting card revision."));}
                            let parsed=pickers::typed_value(&d,&definition)?;
                            if let FieldValue::CardReference(pin)=&parsed {pickers::check_reference_scope(&c,pin)?;api.version(pin).await?;}
                            Some(CardField{definition,value:parsed})}else{None};
                        apply_one(&mut row.component.fields,value(&d,"mode"),index,new)?;
                    }
                    api.save_card(&token,&c.identity(),CardEdit{expected_revision:expected,component:row.component,reason:reason(&d)}).await?;
                },_=>return Err(invalid("operation","Choose a card component operation.")),
            }
        }
        Ok(Some(c.url()))
    }.await;
    match result {
        Ok(Some(url)) => no_store(Redirect::to(&url).into_response()),
        result => {
            let error = result.err();
            if !duplicates.is_empty() {
                sections.push(Section {
                    title: "Duplicate raw form values (not submitted again)".into(),
                    lines: duplicates
                        .iter()
                        .map(|(k, v)| format!("{k}: {v}"))
                        .collect(),
                    links: vec![],
                });
            }
            if matches!(error, Some(Error::Conflict)) && !c.id.is_empty() {
                if c.field {
                    if let Ok(f) = api.field(c.world.as_deref(), &c.id).await {
                        current_revision = f.revision.to_string();
                        let mut sec = field_sections(&f, 1);
                        sec[0].title = "Current saved state — review before resubmitting".into();
                        sections.extend(sec);
                    }
                } else if let Ok(card) = api.detail(&c.identity()).await {
                    current_revision = card.revision.to_string();
                    let mut sec = card_sections(&card, 1);
                    sec[0].title = "Current saved state — review before resubmitting".into();
                    sections.extend(sec);
                }
            }
            // CSRF cookies are issued on form GET only; failed POSTs keep the submitted draft.
            Box::pin(show_editor(
                &api,
                &c,
                &d,
                value(&d, "csrf").into(),
                error.as_ref(),
                sections,
                ReviewState {
                    digest: if error.is_none() {
                        review
                    } else {
                        String::new()
                    },
                    current_revision: if matches!(error, Some(Error::Conflict)) {
                        current_revision
                    } else {
                        String::new()
                    },
                },
            ))
            .await
        }
    }
}
