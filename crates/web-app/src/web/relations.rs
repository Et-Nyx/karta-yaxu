//! Native, operation-scoped ADR0018 authoring. Register `routes()` in the main router.
use super::authoring::{
    authoring_form_error, error_status, error_text, finish, invalid, library_permission, no_store,
    positive, submit_token, world_permission,
};
use super::*;
use crate::application::{
    authoring::{AddressKey, Error},
    cards::{ArchiveFilter, CardSearch, FieldSearch},
    relations::*,
};
use axum::{extract::rejection::QueryRejection, http::Uri};
use game_data::{
    authoring::ContentFields,
    card_fields::{FieldKind, FieldPin},
    cards::{CardIdentity, CardNamespace, CardPin},
};
use serde::{Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
type Values = BTreeMap<String, String>;
const SIZE: usize = 25;
mod editing;
mod selectors;

pub(super) fn routes() -> Router<AppState> {
    let mut r = Router::new();
    for prefix in ["", "/worlds/{world_id}"] {
        for kind in ["record-layouts", "record-collections"] {
            let root = format!("{prefix}/{kind}");
            r = r
                .route(&root, get(catalog))
                .route(
                    &format!("{root}/new"),
                    get(editing::editor).post(editing::submit),
                )
                .route(&format!("{root}/{{id}}"), get(detail))
                .route(
                    &format!("{root}/{{id}}/edit"),
                    get(editing::editor).post(editing::submit),
                )
                .route(&format!("{root}/{{id}}/history"), get(history))
                .route(&format!("{root}/{{id}}/history/{{revision}}"), get(detail));
        }
        let root = format!("{prefix}/cards/{{namespace}}/{{id}}/records");
        r = r
            .route(&root, get(detail))
            .route(
                &format!("{root}/edit"),
                get(editing::editor).post(editing::submit),
            )
            .route(&format!("{root}/history"), get(history))
            .route(&format!("{root}/history/{{revision}}"), get(detail));
    }
    r.layer(DefaultBodyLimit::max(32 * 1024))
}
fn val<'a>(d: &'a Values, k: &str) -> &'a str {
    d.get(k).map(String::as_str).unwrap_or("")
}
fn put(d: &mut Values, k: &str, v: impl ToString) {
    d.insert(k.into(), v.to_string());
}
fn page(d: &Values) -> Result<usize, Error> {
    if val(d, "page").is_empty() {
        return Ok(1);
    }
    val(d, "page")
        .parse()
        .ok()
        .filter(|n| (1..=10000).contains(n))
        .ok_or_else(|| invalid("page", "Choose a page from 1 to 10000."))
}
fn read_query(
    raw: Result<Query<Vec<(String, String)>>, QueryRejection>,
    allowed: &[&str],
) -> Result<Values, Error> {
    let Query(pairs) = raw.map_err(|_| invalid("query", "Use the native query controls."))?;
    let mut values = Values::new();
    for (k, v) in pairs {
        if !allowed.contains(&k.as_str()) || values.insert(k, v).is_some() {
            return Err(invalid("query", "Use each listed query field once."));
        }
    }
    Ok(values)
}
fn encode(s: &str) -> String {
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
fn packed(v: &impl Serialize) -> Result<String, Error> {
    serde_json::to_string(v).map_err(|_| Error::Unavailable)
}
fn unpack<T: DeserializeOwned>(d: &Values, k: &str) -> Result<T, Error> {
    serde_json::from_str(val(d, k))
        .map_err(|_| invalid(k, "Choose a named exact definition using the selectors."))
}
fn hash(v: &impl std::fmt::Debug) -> String {
    hex::encode(Sha256::digest(format!("record-web-review-v1:{v:?}")))
}
fn text_hash(v: &str) -> String {
    hex::encode(Sha256::digest(v.as_bytes()))
}
fn new_id() -> Result<String, Error> {
    crate::application::authoring::new_id()
}
fn reason(d: &Values) -> Option<String> {
    d.get("reason").filter(|s| !s.is_empty()).cloned()
}
fn prefix(w: Option<&str>) -> String {
    w.map(|w| format!("/worlds/{w}")).unwrap_or_default()
}
fn card_url(p: &CardPin) -> String {
    format!(
        "{}/cards/{}/{}/history/{}",
        prefix(p.identity.world_id.as_deref()),
        p.identity.namespace.as_str(),
        p.identity.id,
        p.version
    )
}
fn field_url(p: &FieldPin) -> String {
    format!(
        "{}/card-fields/{}/history/{}",
        prefix(p.world_id.as_deref()),
        p.field_id,
        p.version
    )
}
fn layout_url(p: &LayoutPin) -> String {
    format!(
        "{}/record-layouts/{}/history/{}",
        prefix(p.world_id.as_deref()),
        p.layout_id,
        p.version
    )
}
fn collection_url(p: &CollectionPin) -> String {
    format!(
        "{}/record-collections/{}/history/{}",
        prefix(p.world_id.as_deref()),
        p.collection_id,
        p.version
    )
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Layout,
    Collection,
    Card,
}
#[derive(Clone)]
struct Context {
    world: Option<String>,
    id: String,
    kind: Kind,
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
        let kind = if uri.path().split('/').any(|s| s == "record-layouts") {
            Kind::Layout
        } else if uri.path().split('/').any(|s| s == "record-collections") {
            Kind::Collection
        } else {
            Kind::Card
        };
        let namespace = match val(p, "namespace") {
            "form" => CardNamespace::Form,
            "variant" => CardNamespace::Variant,
            "type" => CardNamespace::Type,
            "move" => CardNamespace::Move,
            "standalone" | "" => CardNamespace::Standalone,
            _ => return Err(Error::NotFound),
        };
        let id = val(p, "id").to_owned();
        if id.len() > 256 {
            return Err(Error::NotFound);
        }
        Ok(Self {
            world,
            id,
            kind,
            namespace,
        })
    }
    async fn resolved(mut self, s: &AppState) -> Result<Self, Error> {
        if self.id.is_empty() {
            return Ok(self);
        }
        if self.kind == Kind::Card {
            let key = addresses::key(&self.id)?;
            self.id = s
                .cards
                .resolve_card(self.world.as_deref(), self.namespace, key)
                .await?
                .identity
                .id;
        } else if let AddressKey::Slug(slug) = addresses::key(&self.id)? {
            self.id = if self.kind == Kind::Layout {
                api(s)
                    .resolve_layout(self.world.as_deref(), &slug)
                    .await?
                    .id
            } else {
                api(s)
                    .resolve_collection(self.world.as_deref(), &slug)
                    .await?
                    .id
            };
        }
        Ok(self)
    }
    fn identity(&self) -> CardIdentity {
        CardIdentity {
            namespace: self.namespace,
            world_id: self.world.clone(),
            id: self.id.clone(),
        }
    }
    fn root(&self) -> String {
        format!(
            "{}/{}",
            prefix(self.world.as_deref()),
            match self.kind {
                Kind::Layout => "record-layouts",
                Kind::Collection => "record-collections",
                Kind::Card => "cards",
            }
        )
    }
    fn url(&self) -> String {
        if self.id.is_empty() {
            format!("{}/new", self.root())
        } else if self.kind == Kind::Card {
            format!(
                "{}/{}/{}/records",
                self.root(),
                self.namespace.as_str(),
                self.id
            )
        } else {
            format!("{}/{}", self.root(), self.id)
        }
    }
    fn action(&self) -> String {
        if self.id.is_empty() {
            self.url()
        } else {
            format!("{}/edit", self.url())
        }
    }
    async fn permission(&self, s: &AppState, h: &HeaderMap) -> Result<(), Error> {
        if self.world.is_some() {
            world_permission(s, h, self.world.as_deref()).await
        } else {
            library_permission(s, h).await
        }
    }
}
fn api(s: &AppState) -> Relations {
    Relations::new(s.accounts.clone())
}
struct Link {
    label: String,
    url: String,
}
fn link(label: impl Into<String>, url: impl Into<String>) -> Link {
    Link {
        label: label.into(),
        url: url.into(),
    }
}
struct Section {
    title: String,
    lines: Vec<String>,
    links: Vec<Link>,
}
fn section(title: impl Into<String>, lines: Vec<String>) -> Section {
    Section {
        title: title.into(),
        lines,
        links: vec![],
    }
}
#[derive(Template)]
#[template(path = "records.html")]
struct Page {
    title: String,
    sections: Vec<Section>,
    links: Vec<Link>,
    previous: String,
    next: String,
    query: String,
    filter: bool,
}
fn nav(c: &Context) -> Vec<Link> {
    [
        ("Record layouts", "record-layouts"),
        ("Record collections", "record-collections"),
        ("Cards", "cards"),
    ]
    .into_iter()
    .map(|(n, p)| link(n, format!("{}/{p}", prefix(c.world.as_deref()))))
    .collect()
}
fn pages(base: &str, n: usize, more: bool, query: &str) -> (String, String) {
    (
        if n > 1 {
            format!("{base}?query={}&page={}", encode(query), n - 1)
        } else {
            String::new()
        },
        if more {
            format!("{base}?query={}&page={}", encode(query), n + 1)
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
            let q = read_query(q, &["query", "page"])?;
            let n = page(&q)?;
            let query = val(&q, "query");
            if query.chars().count() > 100 {
                return Err(invalid("query", "Use at most 100 characters."));
            }
            let a = api(&s);
            let mut sections = vec![];
            if c.kind == Kind::Layout {
                for row in a.layouts(c.world.as_deref(), query, n).await? {
                    let mut sec = section(
                        row.fields.display_name.clone(),
                        vec![
                            row.fields.description,
                            format!(
                                "Revision {} · {} columns · {}",
                                row.revision,
                                row.columns.len(),
                                archive(row.archived_at)
                            ),
                        ],
                    );
                    sec.links
                        .push(link("View layout", format!("{}/{}", c.root(), row.id)));
                    sections.push(sec);
                }
            } else {
                for row in a.collections(c.world.as_deref(), query, n).await? {
                    let mut sec = section(
                        row.fields.display_name.clone(),
                        vec![
                            row.fields.description,
                            format!(
                                "Layout: {} · revision {} · {}",
                                row.layout.fields.display_name,
                                row.revision,
                                archive(row.archived_at)
                            ),
                        ],
                    );
                    sec.links
                        .push(link("View collection", format!("{}/{}", c.root(), row.id)));
                    sections.push(sec);
                }
            }
            let more = sections.len() == SIZE;
            let (previous, next) = pages(&c.root(), n, more, query);
            let mut links = nav(&c);
            links.push(link("Create definition", format!("{}/new", c.root())));
            Ok(render_status(
                Page {
                    title: if c.kind == Kind::Layout {
                        "Record layouts"
                    } else {
                        "Record collections"
                    }
                    .into(),
                    sections,
                    links,
                    previous,
                    next,
                    query: query.into(),
                    filter: true,
                },
                StatusCode::OK,
            ))
        }
        .await,
    )
}
fn archive(t: Option<i64>) -> &'static str {
    if t.is_some() { "Archived" } else { "Active" }
}
fn layout_sections(row: &LayoutRecord, n: usize) -> Vec<Section> {
    let mut sec=section("Layout",vec![row.fields.display_name.clone(),row.fields.description.clone(),format!("Exact revision {} · {}",row.revision,archive(row.archived_at)),"Columns are flat and optional. Reusing a scalar field does not merge column identities. Conditions and effects are inert prose.".into()]);
    if let Some(p) = &row.source {
        sec.links.push(link(
            format!("Exact shared source revision {}", p.version),
            layout_url(p),
        ));
    }
    let mut out = vec![sec];
    for col in row.columns.iter().skip((n - 1) * SIZE).take(SIZE) {
        let mut sec = section(col.label.clone(), vec![]);
        match &col.kind {
            ColumnKind::Field(f) => {
                sec.lines.push(format!(
                    "Scalar field: {} · exact revision {}",
                    f.fields.display_name, f.revision
                ));
                sec.links
                    .push(link("Exact scalar field", field_url(&f.pin())));
            }
            ColumnKind::Reference { allowed } => sec
                .lines
                .push(format!("Allowed public reference kinds: {allowed:?}")),
        }
        out.push(sec);
    }
    out
}
fn collection_sections(row: &CollectionDefinition, n: usize) -> Vec<Section> {
    let mut out = layout_sections(&row.layout, n);
    let mut sec = section(
        "Collection definition",
        vec![
            row.fields.display_name.clone(),
            row.fields.description.clone(),
            format!(
                "Exact revision {} · {}",
                row.revision,
                archive(row.archived_at)
            ),
        ],
    );
    sec.links.push(link(
        format!(
            "Layout {} · exact revision {}",
            row.layout.fields.display_name, row.layout.revision
        ),
        layout_url(&row.layout.pin()),
    ));
    if let Some(p) = &row.source {
        sec.links.push(link(
            format!("Exact shared source revision {}", p.version),
            collection_url(p),
        ));
    }
    out.insert(0, sec);
    out
}
async fn reference_section(s: &AppState, p: &DefinitionPin) -> Result<(String, Link), Error> {
    Ok(match p {
        DefinitionPin::Card(p) => {
            let row = s.cards.version(p).await?;
            (
                row.fields.display_name.clone(),
                link(
                    format!("{} · Card revision {}", row.fields.display_name, p.version),
                    card_url(p),
                ),
            )
        }
        DefinitionPin::Field(p) => {
            let row = s.cards.field_version(p).await?;
            (
                row.fields.display_name.clone(),
                link(
                    format!("{} · Field revision {}", row.fields.display_name, p.version),
                    field_url(p),
                ),
            )
        }
        DefinitionPin::Stat {
            world_id,
            stat_id,
            version,
        } => {
            let row = s
                .stats
                .version(world_id.as_deref(), stat_id, *version)
                .await?;
            (
                row.fields.display_name.clone(),
                link(
                    format!("{} · Stat revision {version}", row.fields.display_name),
                    format!(
                        "{}/stats/{stat_id}/history/{version}",
                        prefix(world_id.as_deref())
                    ),
                ),
            )
        }
    })
}
async fn card_sections(
    s: &AppState,
    row: &CardRelationRecord,
    n: usize,
) -> Result<Vec<Section>, Error> {
    card_sections_at(s, row, n, 1).await
}
async fn card_sections_at(
    s: &AppState,
    row: &CardRelationRecord,
    n: usize,
    column_page: usize,
) -> Result<Vec<Section>, Error> {
    let mut out = vec![section(
        "Record inventory",
        vec![
            match &row.relations.collections {
                None => "Unmodeled".into(),
                Some(c) => format!(
                    "Defined: {} collections (defined-empty is distinct)",
                    c.len()
                ),
            },
            format!("Owning card revision {}", row.card.version),
        ],
    )];
    // Paginate collections and row occurrences independently with the same page;
    // no operation ever replaces off-page saved values.
    for col in row
        .relations
        .collections
        .iter()
        .flatten()
        .skip((n - 1) * SIZE)
        .take(SIZE)
    {
        let mut sec = section(
            col.definition.fields.display_name.clone(),
            vec![match &col.value {
                CollectionValue::Missing => "Missing".into(),
                CollectionValue::Unknown => "Unknown".into(),
                CollectionValue::NotApplicable => "NotApplicable".into(),
                CollectionValue::Entries(es) => {
                    format!("Known entries: {} (empty is distinct)", es.len())
                }
            }],
        );
        sec.links.push(link(
            "Exact collection definition",
            collection_url(&col.definition.pin()),
        ));
        out.push(sec);
    }
    // Bound rendered rows across all collections, rather than 25 per collection.
    let entries = row
        .relations
        .collections
        .iter()
        .flatten()
        .filter_map(|c| {
            if let CollectionValue::Entries(es) = &c.value {
                Some((c, es))
            } else {
                None
            }
        })
        .flat_map(|(c, es)| es.iter().map(move |e| (c, e)))
        .skip((n - 1) * SIZE)
        .take(SIZE)
        .collect::<Vec<_>>();
    for (c, e) in entries {
        let mut sec = section(
            format!("{} · entry {}", c.definition.fields.display_name, e.id),
            vec![],
        );
        for column in c
            .definition
            .layout
            .columns
            .iter()
            .skip((column_page - 1) * SIZE)
            .take(SIZE)
        {
            let value = e.cells.iter().find(|v| v.column_id == column.id);
            let text = match value.map(|v| &v.value) {
                None => "Absent cell".into(),
                Some(RecordValue::Reference(p)) => {
                    let (name, l) = reference_section(s, p).await?;
                    sec.links.push(l);
                    format!("Known reference: {name}")
                }
                Some(RecordValue::Choice(id)) => {
                    let label = if let ColumnKind::Field(f) = &column.kind {
                        if let FieldKind::Choice { options } = &f.kind {
                            options
                                .iter()
                                .find(|o| o.id == *id)
                                .map(|o| o.label.as_str())
                                .unwrap_or("Unresolved exact option")
                        } else {
                            "Invalid choice"
                        }
                    } else {
                        "Invalid choice"
                    };
                    format!("Known choice: {label}")
                }
                Some(v) => value_text(v),
            };
            sec.lines.push(format!("{}: {text}", column.label));
        }
        if c.definition.layout.columns.len() > SIZE {
            sec.lines.push(format!(
                "{} columns in the exact layout; column page {column_page} shows at most 25. Use column-page links to inspect further values.",
                c.definition.layout.columns.len()
            ));
        }
        out.push(sec);
    }
    Ok(out)
}
fn value_text(v: &RecordValue) -> String {
    match v {
        RecordValue::Missing => "Missing".into(),
        RecordValue::Unknown => "Unknown".into(),
        RecordValue::NotApplicable => "NotApplicable".into(),
        RecordValue::Number(v) => format!("Number {v}"),
        RecordValue::Text(t) => format!(
            "Text ({}) {t}",
            if t.is_empty() { "empty" } else { "known" }
        ),
        RecordValue::Boolean(v) => format!("Boolean {v}"),
        RecordValue::Choice(v) => format!("Choice {v}"),
        RecordValue::Reference(_) => "Exact reference".into(),
    }
}
async fn detail(
    State(s): State<AppState>,
    Path(p): Path<Values>,
    uri: Uri,
    q: Result<Query<Vec<(String, String)>>, QueryRejection>,
) -> Response {
    finish(
        async {
            let c = Context::new(&p, &uri)?.resolved(&s).await?;
            let q = read_query(q, &["page", "column_page"])?;
            let n = page(&q)?;
            let mut columns_query = Values::new();
            put(
                &mut columns_query,
                "page",
                if val(&q, "column_page").is_empty() {
                    "1"
                } else {
                    val(&q, "column_page")
                },
            );
            let column_page = page(&columns_query)?;
            let a = api(&s);
            let rev = if p.contains_key("revision") {
                Some(positive(val(&p, "revision"), "revision")?)
            } else {
                None
            };
            let mut links = nav(&c);
            links.push(link("History", format!("{}/history", c.url())));
            let (title, sections, count, ops) = match c.kind {
                Kind::Layout => {
                    let row = if let Some(version) = rev {
                        a.layout_version(&LayoutPin {
                            world_id: c.world.clone(),
                            layout_id: c.id.clone(),
                            version,
                        })
                        .await?
                    } else {
                        a.layout(c.world.as_deref(), &c.id).await?
                    };
                    let count = row.columns.len();
                    (
                        row.fields.display_name.clone(),
                        layout_sections(&row, n),
                        count,
                        vec!["name", "description", "column", "archive", "adopt"],
                    )
                }
                Kind::Collection => {
                    let row = if let Some(version) = rev {
                        a.collection_version(&CollectionPin {
                            world_id: c.world.clone(),
                            collection_id: c.id.clone(),
                            version,
                        })
                        .await?
                    } else {
                        a.collection(c.world.as_deref(), &c.id).await?
                    };
                    let count = row.layout.columns.len();
                    (
                        row.fields.display_name.clone(),
                        collection_sections(&row, n),
                        count,
                        vec!["name", "description", "layout", "archive", "adopt"],
                    )
                }
                Kind::Card => {
                    let scalar = if let Some(version) = rev {
                        s.cards
                            .version(&CardPin {
                                identity: c.identity(),
                                version,
                            })
                            .await?
                    } else {
                        s.cards.detail(&c.identity()).await?
                    };
                    let row = a.card(&scalar.pin()).await?;
                    let count = row
                        .relations
                        .collections
                        .iter()
                        .flatten()
                        .map(|c| match &c.value {
                            CollectionValue::Entries(es) => es.len(),
                            _ => 0,
                        })
                        .sum::<usize>()
                        .max(row.relations.collections.as_ref().map_or(0, Vec::len));
                    (
                        format!("{} · records", scalar.fields.display_name),
                        card_sections_at(&s, &row, n, column_page).await?,
                        count,
                        vec![
                            "inventory",
                            "collection",
                            "row",
                            "cell",
                            "reconcile",
                            "copy",
                        ],
                    )
                }
            };
            if rev.is_none() {
                for op in ops {
                    links.push(link(
                        format!("Edit {op}"),
                        format!("{}/edit?operation={op}", c.url()),
                    ));
                }
            }
            let (previous, next) = pages(uri.path(), n, count > n * SIZE, "");
            if c.kind == Kind::Card {
                let scalar = if let Some(version) = rev {
                    s.cards
                        .version(&CardPin {
                            identity: c.identity(),
                            version,
                        })
                        .await?
                } else {
                    s.cards.detail(&c.identity()).await?
                };
                let row = a.card(&scalar.pin()).await?;
                let columns = row
                    .relations
                    .collections
                    .iter()
                    .flatten()
                    .map(|c| c.definition.layout.columns.len())
                    .max()
                    .unwrap_or_default();
                if column_page > 1 {
                    links.push(link(
                        "Previous column page",
                        format!("{}?page={n}&column_page={}", uri.path(), column_page - 1),
                    ));
                }
                if column_page * SIZE < columns {
                    links.push(link(
                        "Next column page",
                        format!("{}?page={n}&column_page={}", uri.path(), column_page + 1),
                    ));
                }
            }
            Ok(render_status(
                Page {
                    title,
                    sections,
                    links,
                    previous,
                    next,
                    query: String::new(),
                    filter: false,
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
    q: Result<Query<Vec<(String, String)>>, QueryRejection>,
) -> Response {
    finish(
        async {
            let c = Context::new(&p, &uri)?.resolved(&s).await?;
            let q = read_query(q, &["before"])?;
            let before = if val(&q, "before").is_empty() {
                None
            } else {
                Some(positive(val(&q, "before"), "before")?)
            };
            let a = api(&s);
            let entries = match c.kind {
                Kind::Layout => a.layout_history(c.world.as_deref(), &c.id, before).await?,
                Kind::Collection => {
                    a.collection_history(c.world.as_deref(), &c.id, before)
                        .await?
                }
                Kind::Card => a.card_history(&c.identity(), before).await?,
            };
            let next = if entries.len() == SIZE {
                format!(
                    "{}/history?before={}",
                    c.url(),
                    entries.last().ok_or(Error::Unavailable)?.revision
                )
            } else {
                String::new()
            };
            let sections = entries
                .into_iter()
                .map(|e| {
                    let mut sec = section(
                        format!("Revision {}", e.revision),
                        vec![
                            format!("By {} · {} · {} UTC", e.actor_name, e.action, e.occurred_at),
                            format!("Reason: {}", e.reason.as_deref().unwrap_or("Not provided")),
                        ],
                    );
                    sec.links.push(link(
                        format!("Revision {}", e.revision),
                        format!("{}/history/{}", c.url(), e.revision),
                    ));
                    sec
                })
                .collect();
            let mut links = nav(&c);
            links.push(link("Current saved record", c.url()));
            Ok(render_status(
                Page {
                    title: "Record history".into(),
                    sections,
                    links,
                    previous: String::new(),
                    next,
                    query: String::new(),
                    filter: false,
                },
                StatusCode::OK,
            ))
        }
        .await,
    )
}
