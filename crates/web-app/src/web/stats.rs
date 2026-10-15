//! Native, bounded Stat workflows. Exact arithmetic and mutation policy live in the services.
use super::authoring::{
    authoring_form_error, error_status, error_text, finish, invalid, library_permission, no_store,
    positive, session, submit_token, world_permission,
};
use super::*;
use crate::application::stat_values::{FormStatsEdit, SourceValuesEdit, VariantStatsEdit};
use crate::application::{
    authoring::{Error, HistoryEntry},
    stats::{DefinitionEdit, NewStat},
};
use axum::{extract::rejection::QueryRejection, http::Uri};
use game_data::{authoring::ContentFields, stats::*};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

type Values = BTreeMap<String, String>;
type FormInput = Result<Form<Values>, FormRejection>;
type QueryInput = Result<Query<Values>, QueryRejection>;
const PAGE_SIZE: usize = 25;

pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/stats", get(catalog))
        .route("/stats/new", get(editor).post(submit))
        .route("/stats/{id}", get(editor))
        .route("/stats/{id}/edit", get(editor).post(submit))
        .route("/stats/{id}/history", get(history))
        .route("/stats/{id}/history/{revision}", get(history))
        .route("/worlds/{world_id}/stats", get(catalog))
        .route("/worlds/{world_id}/stats/new", get(editor).post(submit))
        .route("/worlds/{world_id}/stats/{id}", get(editor))
        .route(
            "/worlds/{world_id}/stats/{id}/edit",
            get(editor).post(submit),
        )
        .route(
            "/worlds/{world_id}/stats/{id}/adopt",
            get(editor).post(submit),
        )
        .route("/worlds/{world_id}/stats/{id}/history", get(history))
        .route(
            "/worlds/{world_id}/stats/{id}/history/{revision}",
            get(history),
        )
        .route("/worlds/{world_id}/stat-roster", get(editor))
        .route(
            "/worlds/{world_id}/stat-roster/edit",
            get(editor).post(submit),
        )
        .route("/worlds/{world_id}/stat-roster/history", get(history))
        .route(
            "/worlds/{world_id}/stat-roster/history/{revision}",
            get(history),
        )
        .route("/forms/{id}/stats", get(editor).post(submit))
        .route(
            "/forms/{id}/history/{revision}/stats",
            get(historical_values),
        )
        .route(
            "/worlds/{world_id}/variants/{id}/history/{revision}/stats",
            get(historical_values),
        )
        .route(
            "/worlds/{world_id}/variants/{id}/stats",
            get(editor).post(submit),
        )
        .route(
            "/worlds/{world_id}/variants/{id}/stats/reconcile",
            get(editor).post(submit),
        )
        .route(
            "/worlds/{world_id}/variants/{id}/stats/source",
            get(editor).post(submit),
        )
        .layer(DefaultBodyLimit::max(32 * 1024))
}
fn val<'a>(v: &'a Values, k: &str) -> &'a str {
    v.get(k).map(String::as_str).unwrap_or("")
}
fn reason(v: &Values) -> Option<String> {
    v.get("reason").filter(|s| !s.is_empty()).cloned()
}
fn fields(v: &Values) -> ContentFields {
    ContentFields {
        slug: String::new(),
        display_name: val(v, "display_name").into(),
        description: val(v, "description").into(),
    }
}
fn decimal(raw: &str, field: &str) -> Result<Option<Decimal>, Error> {
    if raw.is_empty() {
        Ok(None)
    } else {
        raw.parse()
            .map(Some)
            .map_err(|e: String| invalid(field, &e))
    }
}
fn constraints(v: &Values) -> Result<NumericConstraints, Error> {
    let minimum = decimal(val(v, "minimum"), "minimum");
    let maximum = decimal(val(v, "maximum"), "maximum");
    let mut errors = Values::new();
    for result in [&minimum, &maximum] {
        if let Err(Error::Invalid(e)) = result {
            errors.extend(e.clone());
        }
    }
    if !errors.is_empty() {
        return Err(Error::Invalid(errors));
    }
    let c = NumericConstraints {
        minimum: minimum?,
        maximum: maximum?,
        whole_only: val(v, "whole_only") == "yes",
    };
    c.validate().map_err(Error::Invalid)?;
    Ok(c)
}
fn pin(raw: &str, field: &str) -> Result<StatPin, Error> {
    let (id, version) = raw
        .rsplit_once('@')
        .ok_or_else(|| invalid(field, "Choose an exact Stat identity@version."))?;
    let p = StatPin {
        stat_id: id.into(),
        version: positive(version, field)?,
    };
    p.validate().map_err(Error::Invalid)?;
    Ok(p)
}
fn expected(v: &Values) -> Result<i64, Error> {
    val(v, "expected_revision")
        .parse::<i64>()
        .ok()
        .filter(|n| *n >= 0)
        .ok_or_else(|| {
            invalid(
                "expected_revision",
                "Use the saved revision, or zero for an unconfigured roster.",
            )
        })
}
fn page_number(v: &Values) -> Result<usize, Error> {
    if val(v, "page").is_empty() {
        return Ok(1);
    }
    val(v, "page")
        .parse::<usize>()
        .ok()
        .filter(|n| *n > 0 && *n <= usize::MAX / PAGE_SIZE)
        .ok_or_else(|| invalid("page", "Use a positive page number."))
}
fn root(world: Option<&str>) -> String {
    world.map_or_else(|| "/stats".into(), |w| format!("/worlds/{w}/stats"))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    New,
    Detail,
    Edit,
    Adopt,
    Roster,
    RosterEdit,
    Form,
    Variant,
    Reconcile,
    Source,
}
struct Context {
    world: Option<String>,
    id: String,
    mode: Mode,
}
impl Context {
    fn new(path: &Values, uri: &str) -> Self {
        let mode = if uri.contains("/stat-roster") {
            if uri.ends_with("/edit") {
                Mode::RosterEdit
            } else {
                Mode::Roster
            }
        } else if uri.starts_with("/forms/") {
            Mode::Form
        } else if uri.contains("/variants/") {
            if uri.ends_with("/reconcile") {
                Mode::Reconcile
            } else if uri.ends_with("/source") {
                Mode::Source
            } else {
                Mode::Variant
            }
        } else if uri.ends_with("/new") {
            Mode::New
        } else if uri.ends_with("/adopt") {
            Mode::Adopt
        } else if uri.ends_with("/edit") {
            Mode::Edit
        } else {
            Mode::Detail
        };
        Self {
            world: path.get("world_id").cloned(),
            id: val(path, "id").into(),
            mode,
        }
    }
    fn world(&self) -> Option<&str> {
        self.world.as_deref()
    }
    fn definition(&self) -> bool {
        matches!(
            self.mode,
            Mode::New | Mode::Detail | Mode::Edit | Mode::Adopt
        )
    }
    fn roster(&self) -> bool {
        matches!(self.mode, Mode::Roster | Mode::RosterEdit)
    }
    fn target(&self) -> String {
        if self.roster() {
            format!(
                "/worlds/{}/stat-roster",
                self.world.as_deref().unwrap_or("")
            )
        } else if self.mode == Mode::Form {
            format!("/forms/{}/stats", self.id)
        } else if !self.definition() {
            format!(
                "/worlds/{}/variants/{}/stats",
                self.world.as_deref().unwrap_or(""),
                self.id
            )
        } else if self.mode == Mode::New {
            root(self.world())
        } else {
            format!("{}/{}", root(self.world()), self.id)
        }
    }
    fn action(&self) -> String {
        format!(
            "{}{}",
            self.target(),
            match self.mode {
                Mode::New => "/new",
                Mode::Edit | Mode::RosterEdit => "/edit",
                Mode::Adopt => "/adopt",
                Mode::Reconcile => "/reconcile",
                Mode::Source => "/source",
                _ => "",
            }
        )
    }
    async fn permission(&self, state: &AppState, headers: &HeaderMap) -> Result<(), Error> {
        match self.world() {
            Some(w) => world_permission(state, headers, Some(w)).await,
            None => library_permission(state, headers).await,
        }
    }
    async fn revision(&self, state: &AppState) -> Result<i64, Error> {
        if self.mode == Mode::New {
            Ok(0)
        } else if self.roster() {
            Ok(state
                .stats
                .world_roster(self.world().ok_or(Error::NotFound)?)
                .await?
                .map_or(0, |r| r.revision))
        } else if self.definition() {
            Ok(state.stats.detail(self.world(), &self.id).await?.revision)
        } else if self.mode == Mode::Form {
            Ok(state.authoring.form_detail(&self.id).await?.revision)
        } else {
            Ok(state
                .authoring
                .variant_detail(self.world().ok_or(Error::NotFound)?, &self.id)
                .await?
                .revision)
        }
    }
    async fn roster_revision(&self, state: &AppState) -> Result<i64, Error> {
        match self.world() {
            Some(w) => Ok(state.stats.world_roster(w).await?.map_or(0, |r| r.revision)),
            None => Ok(0),
        }
    }
}

struct Link {
    label: String,
    url: String,
}
struct Panel {
    label: String,
    text: String,
}
struct Choice {
    value: String,
    label: String,
}
struct Control {
    name: String,
    label: String,
    value: String,
    kind: &'static str,
    help: String,
    error: String,
    choices: Vec<Choice>,
}
#[derive(Template)]
#[template(path = "stat_page.html")]
struct Page {
    title: String,
    action: String,
    cancel: String,
    csrf: String,
    controls: Vec<Control>,
    hidden: Values,
    panels: Vec<Panel>,
    links: Vec<Link>,
    error: String,
    errors: Values,
    editable: bool,
    preview: bool,
    ready: bool,
    reviewed: String,
    draft: Values,
    previous_page: String,
    next_page: String,
    baseline: String,
    review_fingerprint: String,
    notice: String,
}
impl Page {
    fn new(title: &str, ctx: &Context, draft: Values, failure: Option<&Error>) -> Self {
        Self {
            title: title.into(),
            action: ctx.action(),
            cancel: ctx.target(),
            csrf: String::new(),
            controls: vec![],
            hidden: Values::new(),
            panels: vec![],
            links: vec![],
            error: failure.map(error_text).unwrap_or("").into(),
            errors: match failure {
                Some(Error::Invalid(e)) => e.clone(),
                _ => Values::new(),
            },
            editable: false,
            preview: matches!(
                ctx.mode,
                Mode::Adopt | Mode::RosterEdit | Mode::Reconcile | Mode::Source
            ),
            ready: false,
            reviewed: String::new(),
            draft,
            previous_page: String::new(),
            next_page: String::new(),
            baseline: String::new(),
            review_fingerprint: String::new(),
            notice: String::new(),
        }
    }
    fn control(
        &mut self,
        name: &str,
        label: impl Into<String>,
        kind: &'static str,
        help: impl Into<String>,
        mut choices: Vec<Choice>,
    ) {
        let raw = val(&self.draft, name).to_owned();
        if kind == "select" && !choices.iter().any(|c| c.value == raw) {
            choices.push(Choice {
                value: raw.clone(),
                label: format!("Retained submitted choice: {raw}"),
            });
        }
        self.controls.push(Control {
            name: name.into(),
            label: label.into(),
            value: raw,
            kind,
            help: help.into(),
            error: val(&self.errors, name).into(),
            choices,
        });
    }
    fn text(&mut self, name: &str, label: impl Into<String>, help: impl Into<String>) {
        self.control(name, label, "text", help, vec![]);
    }
    fn select(&mut self, name: &str, label: &str, help: &str, choices: Vec<Choice>) {
        self.control(name, label, "select", help, choices);
    }
    fn check(&mut self, name: &str, label: impl Into<String>, help: &str) {
        self.control(name, label, "checkbox", help, vec![]);
    }
    fn panel(&mut self, label: impl Into<String>, text: impl Into<String>) {
        self.panels.push(Panel {
            label: label.into(),
            text: text.into(),
        });
    }
    fn link(&mut self, label: impl Into<String>, url: impl Into<String>) {
        self.links.push(Link {
            label: label.into(),
            url: url.into(),
        });
    }
}
fn choice(value: &str, label: &str) -> Choice {
    Choice {
        value: value.into(),
        label: label.into(),
    }
}
fn constraints_text(c: &NumericConstraints) -> String {
    format!(
        "Minimum (inclusive): {}; maximum (inclusive): {}; {}.",
        c.minimum
            .map(|v| v.to_string())
            .unwrap_or_else(|| "unbounded".into()),
        c.maximum
            .map(|v| v.to_string())
            .unwrap_or_else(|| "unbounded".into()),
        if c.whole_only {
            "whole numbers only"
        } else {
            "signed decimals allowed"
        }
    )
}
fn definition_text(d: &StatDefinition) -> String {
    format!(
        "{} · {}@{}\n{}\n{}\nSource: {}",
        d.fields.display_name,
        d.id,
        d.revision,
        d.fields.description,
        constraints_text(&d.constraints),
        d.source
            .as_ref()
            .map(|p| format!("{}@{}", p.stat_id, p.version))
            .unwrap_or_else(|| "world-exclusive / independent shared definition".into())
    )
}
pub(super) fn stats_text(s: &BaseStats) -> String {
    stats_page_text(s, 1)
}
// Compare all identities, not the first display page. Position and the entire
// exact definition (pin, metadata, constraints, source) participate in equality.
pub(super) fn stats_diff(old: &BaseStats, new: &BaseStats) -> (String, String) {
    let old_values = old.assignments.as_deref().unwrap_or_default();
    let new_values = new.assignments.as_deref().unwrap_or_default();
    let old_index = old_values
        .iter()
        .enumerate()
        .map(|(i, v)| (v.definition.id.as_str(), (i, v)))
        .collect::<BTreeMap<_, _>>();
    let new_index = new_values
        .iter()
        .enumerate()
        .map(|(i, v)| (v.definition.id.as_str(), (i, v)))
        .collect::<BTreeMap<_, _>>();
    let ids = old_index
        .keys()
        .chain(new_index.keys())
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    let changed = ids
        .into_iter()
        .filter(|id| old_index.get(id) != new_index.get(id))
        .collect::<Vec<_>>();
    let state = |s: &BaseStats| {
        format!(
            "State: {}; exact roster context: {}",
            match &s.assignments {
                None => "Unmodeled".into(),
                Some(v) if v.is_empty() => "Defined-empty".into(),
                Some(v) => format!("Defined ({} entries)", v.len()),
            },
            s.roster
                .as_ref()
                .map(|p| format!("{}@{}", p.roster_id, p.version))
                .unwrap_or_else(|| "None".into())
        )
    };
    let side = |s: &BaseStats, index: &BTreeMap<&str, (usize, &StatValue)>| {
        format!(
            "{}\n{} changed identities (showing at most 25; open readable historical stats for all entries).\n{}",
            state(s),
            changed.len(),
            changed
                .iter()
                .take(PAGE_SIZE)
                .map(|id| match index.get(id) {
                    None => format!("{id}: Not assigned"),
                    Some((i, v)) => format!(
                        "Position {}: {} = {}\n{}",
                        i + 1,
                        v.definition.fields.display_name,
                        v.value
                            .map(|v| v.to_string())
                            .unwrap_or_else(|| "Missing".into()),
                        definition_text(&v.definition)
                    ),
                })
                .collect::<Vec<_>>()
                .join("\n\n")
        )
    };
    (side(old, &old_index), side(new, &new_index))
}
fn stats_page_text(s: &BaseStats, page: usize) -> String {
    let context = s
        .roster
        .as_ref()
        .map(|p| format!("Exact roster context: {}@{}\n", p.roster_id, p.version))
        .unwrap_or_default();
    let body = match &s.assignments {
        None => "Unmodeled — base stats not modeled".into(),
        Some(v) if v.is_empty() => "Defined-empty — intentionally no stat entries".into(),
        Some(v) => format!(
            "{} entries; page {page}\n{}",
            v.len(),
            v.iter()
                .skip((page - 1) * PAGE_SIZE)
                .take(PAGE_SIZE)
                .enumerate()
                .map(|(i, v)| format!(
                    "{}. {} = {}\n{}",
                    (page - 1) * PAGE_SIZE + i + 1,
                    v.definition.fields.display_name,
                    v.value
                        .map(|n| n.to_string())
                        .unwrap_or_else(|| "Missing".into()),
                    definition_text(&v.definition)
                ))
                .collect::<Vec<_>>()
                .join("\n\n")
        ),
    };
    format!("{context}{body}")
}
fn roster_text(r: Option<&StatRoster>, page: usize) -> String {
    match r {
        None => "Unconfigured — no stat roster has been defined.".into(),
        Some(r) => format!(
            "Roster {} · revision {}\n{}",
            r.id,
            r.revision,
            if r.entries.is_empty() {
                "Defined-empty — intentionally no stat entries".into()
            } else {
                r.entries
                    .iter()
                    .skip((page - 1) * PAGE_SIZE)
                    .take(PAGE_SIZE)
                    .enumerate()
                    .map(|(i, d)| {
                        format!(
                            "Position {}: {}",
                            (page - 1) * PAGE_SIZE + i + 1,
                            definition_text(d)
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n\n")
            }
        ),
    }
}
async fn pretty_root(state: &AppState, world: Option<&str>) -> Result<String, Error> {
    Ok(match world {
        Some(w) => root(Some(&addresses::segment(
            &state.authoring.world_detail(w).await?.fields.slug,
        ))),
        None => root(None),
    })
}
async fn definition_url(state: &AppState, d: &StatDefinition) -> Result<String, Error> {
    Ok(format!(
        "{}/{}",
        pretty_root(state, d.world_id.as_deref()).await?,
        addresses::segment(&d.fields.slug)
    ))
}
fn pagination(page: &mut Page, url: &str, number: usize, len: usize) {
    if number > 1 {
        if page.editable {
            page.previous_page = format!("page:{}", number - 1);
        } else {
            page.link("Previous page", format!("{url}?page={}", number - 1));
        }
    }
    if number.saturating_mul(PAGE_SIZE) < len {
        if page.editable {
            page.next_page = format!("page:{}", number + 1);
        } else {
            page.link("Next page", format!("{url}?page={}", number + 1));
        }
    }
}
async fn catalog(
    State(state): State<AppState>,
    headers: HeaderMap,
    uri: Uri,
    query: QueryInput,
) -> Response {
    finish(async {
        let Query(q)=query.map_err(|_|invalid("page","Check the catalog page."))?;
        let n=page_number(&q)?;
        // Catalog routes have no ID extractor on the shared route.
        let world=uri.path().strip_prefix("/worlds/").and_then(|s|s.strip_suffix("/stats"));
        let ctx=Context{world:world.map(str::to_owned),id:String::new(),mode:Mode::New};
        let caps=state.authoring.capabilities(session(&headers).as_deref(),world).await?;
        let rows=state.stats.definitions(world).await?;
        let url=pretty_root(&state,world).await?;
        let mut p=Page::new(if world.is_some(){"World stats"}else{"Stats"},&ctx,Values::new(),None);
        p.cancel=if let Some(w)=world { format!("/worlds/{w}") } else { "/".into() };
        p.panel("Independent exact definitions","Creating a definition does not add it to a roster or change any existing creature pin. Catalog pages contain at most 25 definitions.");
        if if world.is_some(){caps.can_edit_world}else{caps.can_edit_library} { p.link(if world.is_some(){"Create world stat"}else{"Create shared stat"},format!("{url}/new")); }
        if let Some(w)=world { p.link("World stat roster",format!("/worlds/{w}/stat-roster")); }
        for d in rows.iter().skip((n-1)*PAGE_SIZE).take(PAGE_SIZE) { p.link(&d.fields.display_name,definition_url(&state,d).await?); p.panel(&d.fields.display_name,definition_text(d)); }
        pagination(&mut p,&url,n,rows.len()); Ok(render(p))
    }.await)
}

async fn definition_choices(
    state: &AppState,
    world: Option<&str>,
    number: usize,
) -> Result<Vec<Choice>, Error> {
    let rows = state.stats.definitions(world).await?;
    let mut choices = vec![choice("", "None — keep existing entries")];
    choices.extend(
        rows.into_iter()
            .skip((number - 1) * PAGE_SIZE)
            .take(PAGE_SIZE)
            .map(|d| Choice {
                value: format!("{}@{}", d.id, d.revision),
                label: format!(
                    "{} · exact version {} · {}",
                    d.fields.display_name, d.revision, d.id
                ),
            }),
    );
    Ok(choices)
}
fn definition_draft(d: &StatDefinition) -> Values {
    Values::from([
        ("expected_revision".into(), d.revision.to_string()),
        ("display_name".into(), d.fields.display_name.clone()),
        ("description".into(), d.fields.description.clone()),
        (
            "minimum".into(),
            d.constraints
                .minimum
                .map(|v| v.to_string())
                .unwrap_or_default(),
        ),
        (
            "maximum".into(),
            d.constraints
                .maximum
                .map(|v| v.to_string())
                .unwrap_or_default(),
        ),
        (
            "whole_only".into(),
            if d.constraints.whole_only { "yes" } else { "" }.into(),
        ),
    ])
}
async fn saved_draft(state: &AppState, ctx: &Context) -> Result<Values, Error> {
    let mut d = Values::from([
        (
            "expected_revision".into(),
            ctx.revision(state).await?.to_string(),
        ),
        ("page".into(), "1".into()),
        ("operation".into(), "initialize".into()),
    ]);
    if ctx.definition() && ctx.mode != Mode::New {
        let current = state.stats.detail(ctx.world(), &ctx.id).await?;
        d.extend(definition_draft(&current));
        if ctx.mode == Mode::Adopt
            && let Some(source) = &current.source
        {
            d.insert(
                "target_version".into(),
                state
                    .stats
                    .detail(None, &source.stat_id)
                    .await?
                    .revision
                    .to_string(),
            );
        }
    } else if !ctx.definition() && !ctx.roster() {
        let s = if ctx.mode == Mode::Form {
            state.stat_values.form(&ctx.id).await?
        } else {
            state
                .stat_values
                .variant(ctx.world().ok_or(Error::NotFound)?, &ctx.id)
                .await?
        };
        d.insert(
            "state".into(),
            if s.assignments.is_some() {
                "defined"
            } else {
                "unmodeled"
            }
            .into(),
        );
        if ctx.world().is_some() {
            d.insert(
                "expected_roster_revision".into(),
                ctx.roster_revision(state).await?.to_string(),
            );
        }
        if ctx.mode == Mode::Source {
            let variant = state
                .authoring
                .variant_detail(ctx.world().ok_or(Error::NotFound)?, &ctx.id)
                .await?;
            if let Some(source) = variant.source {
                d.insert(
                    "target_form_version".into(),
                    state
                        .authoring
                        .form_detail(&source.form_id)
                        .await?
                        .revision
                        .to_string(),
                );
            }
        }
    }
    Ok(d)
}
async fn make_page(
    state: &AppState,
    headers: &HeaderMap,
    ctx: &Context,
    draft: Values,
    failure: Option<&Error>,
) -> Result<Page, Error> {
    let title = match ctx.mode {
        Mode::New => "Create stat",
        Mode::Detail => "Stat definition",
        Mode::Edit => "Edit stat definition",
        Mode::Adopt => "Adopt stat definition source",
        Mode::Roster => "World stat roster",
        Mode::RosterEdit => "Edit world stat roster",
        Mode::Form => "Shared form base stats",
        Mode::Variant => "Variant base stats",
        Mode::Reconcile => "Reconcile roster context",
        Mode::Source => "Adopt source base values",
    };
    let mut p = Page::new(title, ctx, draft, failure);
    if let Some(w) = ctx.world() {
        let parent = state.authoring.world_detail(w).await?;
        p.link(
            "World",
            format!("/worlds/{}", addresses::segment(&parent.fields.slug)),
        );
    }
    let n = page_number(&p.draft)?;
    let caps = state
        .authoring
        .capabilities(session(headers).as_deref(), ctx.world())
        .await?;
    if ctx.mode != Mode::New {
        p.panel(
            "Current saved revision",
            format!(
                "Object revision: {}. Current world roster revision: {}.",
                ctx.revision(state).await?,
                ctx.roster_revision(state).await?
            ),
        );
    }
    p.editable = !matches!(ctx.mode, Mode::Detail | Mode::Roster)
        && (failure.is_some()
            || if ctx.world().is_some() {
                caps.can_edit_world
            } else {
                caps.can_edit_library
            });
    if ctx.definition() {
        if ctx.mode != Mode::New {
            let current = state.stats.detail(ctx.world(), &ctx.id).await?;
            p.panel("Current saved definition", definition_text(&current));
            let url = definition_url(state, &current).await?;
            p.cancel = url.clone();
            p.link("Stat history", format!("{url}/history"));
            if ctx.mode == Mode::Detail
                && if ctx.world().is_some() {
                    caps.can_edit_world
                } else {
                    caps.can_edit_library
                }
            {
                p.link("Edit stat definition", format!("{url}/edit"));
                if current.source.is_some() {
                    p.link("Review stat source adoption", format!("{url}/adopt"));
                }
            }
            if let Some(source) = current.source {
                p.link(
                    "Exact shared stat source",
                    format!("/stats/{}/history/{}", source.stat_id, source.version),
                );
            }
        } else {
            p.cancel = pretty_root(state, ctx.world()).await?;
        }
        if p.editable {
            p.text("display_name","Display name","Required, up to 100 characters. The scoped readable slug is generated from this name.");
            p.control(
                "description",
                "Description",
                "textarea",
                "Required plain text, up to 2000 characters.",
                vec![],
            );
            p.text("minimum","Minimum (inclusive)","Optional exact signed decimal. Blank means unbounded; at most six fractional digits, no exponents.");
            p.text(
                "maximum",
                "Maximum (inclusive)",
                "Optional exact signed decimal. Blank means unbounded.",
            );
            p.check(
                "whole_only",
                "Whole numbers only",
                "Applies only to consumers that explicitly select this exact definition version.",
            );
            if ctx.mode == Mode::New && ctx.world().is_some() {
                p.select("source_pin","Source shared stat and exact version","Optional source pin; metadata and constraints remain the explicitly entered local draft.",definition_choices(state,None,n).await?);
                p.text("source_exact","Other exact shared source pin","Optional identity@version overrides the catalog choice. Use public Stat history to find an older revision.");
                pagination(
                    &mut p,
                    &ctx.action(),
                    n,
                    state.stats.definitions(None).await?.len(),
                );
            }
            if ctx.mode == Mode::Adopt {
                p.text("target_version","New shared stat version","A newer exact revision of this same shared source. Local fields and constraints stay as entered.");
            }
        }
    } else if ctx.roster() {
        let world = ctx.world().ok_or(Error::NotFound)?;
        let r = state.stats.world_roster(world).await?;
        p.panel("Current saved roster", roster_text(r.as_ref(), n));
        let parent = state.authoring.world_detail(world).await?;
        let url = format!(
            "/worlds/{}/stat-roster",
            addresses::segment(&parent.fields.slug)
        );
        p.cancel = url.clone();
        p.link(
            "World stat definitions",
            pretty_root(state, Some(world)).await?,
        );
        if r.is_some() {
            p.link("Roster history", format!("{url}/history"));
        }
        if ctx.mode == Mode::Roster && caps.can_edit_world {
            p.link("Edit stat roster", format!("{url}/edit"));
        }
        if p.editable {
            p.select("operation","Roster operation","One bounded operation per review. Positions are one-based; there is no game stat-count limit.",vec![choice("initialize","Define empty roster / keep current roster"),choice("add","Add exact stat pin"),choice("remove","Remove entry at position"),choice("version","Change exact version at position"),choice("move","Move entry to position")]);
            p.select(
                "stat_pin",
                "Stat to add or select version",
                "Catalog choices on this page; exact historical pins can be entered below.",
                definition_choices(state, Some(world), n).await?,
            );
            p.text(
                "exact_pin",
                "Other exact stat pin",
                "Optional canonical identity@version overrides the catalog selection.",
            );
            p.text(
                "position",
                "Existing entry position",
                "One-based position to remove, change version or move.",
            );
            p.text(
                "destination",
                "New position",
                "For move, required. For add, blank appends to the ordered roster.",
            );
        }
        pagination(
            &mut p,
            &format!(
                "{url}{}",
                if ctx.mode == Mode::RosterEdit {
                    "/edit"
                } else {
                    ""
                }
            ),
            n,
            r.as_ref()
                .map_or(0, |r| r.entries.len())
                .max(state.stats.definitions(Some(world)).await?.len()),
        );
    } else {
        let old = if ctx.mode == Mode::Form {
            state.stat_values.form(&ctx.id).await?
        } else {
            state
                .stat_values
                .variant(ctx.world().ok_or(Error::NotFound)?, &ctx.id)
                .await?
        };
        p.panel("Current saved base stats", stats_page_text(&old, n));
        for value in old
            .assignments
            .iter()
            .flatten()
            .skip((n - 1) * PAGE_SIZE)
            .take(PAGE_SIZE)
        {
            let d = &value.definition;
            p.link(
                format!(
                    "Exact definition for {} at version {}",
                    d.fields.display_name, d.revision
                ),
                format!(
                    "{}/{}/history/{}",
                    root(d.world_id.as_deref()),
                    d.id,
                    d.revision
                ),
            );
        }
        let (mut entries, url) = if ctx.mode == Mode::Form {
            let (species, form) = state.authoring.resolve_legacy_form(&ctx.id).await?;
            let url = format!(
                "/creatures/{}/forms/{}",
                addresses::segment(&species.fields.slug),
                addresses::segment(&form.fields.slug)
            );
            p.link("Form history", format!("{url}/history"));
            p.link("Return to form", url.clone());
            (
                old.assignments
                    .as_deref()
                    .unwrap_or_default()
                    .iter()
                    .map(|v| v.definition.clone())
                    .collect::<Vec<_>>(),
                format!("{url}/stats"),
            )
        } else {
            let w = ctx.world().ok_or(Error::NotFound)?;
            let world = state.authoring.world_detail(w).await?;
            let variant = state.authoring.variant_detail(w, &ctx.id).await?;
            let url = format!(
                "/worlds/{}/variants/{}",
                addresses::segment(&world.fields.slug),
                addresses::segment(&variant.fields.slug)
            );
            p.link("Variant history", format!("{url}/history"));
            p.link("Return to variant", url.clone());
            let roster = state.stats.world_roster(w).await?;
            p.panel("Current world roster", roster_text(roster.as_ref(), n));
            p.link(
                "World stat roster",
                format!(
                    "/worlds/{}/stat-roster",
                    addresses::segment(&world.fields.slug)
                ),
            );
            if caps.can_edit_world {
                p.link(
                    "Review roster reconciliation",
                    format!("{url}/stats/reconcile"),
                );
                if variant.source.is_some() {
                    p.link("Review source base values", format!("{url}/stats/source"));
                }
            }
            if ctx.mode == Mode::Variant && old.roster != roster.as_ref().map(StatRoster::pin) {
                p.editable = false;
                p.panel("Explicit reconciliation required","These values are Unmodeled or retain an older exact roster context. Review roster reconciliation to model or update them. Nothing is rounded, clamped or silently discarded.");
            }
            (
                roster.map(|r| r.entries).unwrap_or_default(),
                format!("{url}/stats"),
            )
        };
        p.cancel = url.clone();
        let total_entries = entries
            .len()
            .max(old.assignments.as_ref().map_or(0, Vec::len))
            .max(if ctx.mode == Mode::Form {
                state.stats.definitions(None).await?.len()
            } else {
                0
            });
        let ids = displayed(&p.draft)?.unwrap_or_else(|| {
            entries
                .iter()
                .skip((n - 1) * PAGE_SIZE)
                .take(PAGE_SIZE)
                .map(|e| e.id.clone())
                .collect()
        });
        entries.retain(|d| ids.contains(&d.id));
        entries.sort_by_key(|d| ids.iter().position(|id| id == &d.id));
        p.draft.insert("displayed".into(), ids.join(","));
        p.hidden.insert("displayed".into(), ids.join(","));
        if failure.is_some() {
            p.panel(
                "Current saved values for this submitted page",
                ids.iter()
                    .map(|id| {
                        old.assignments
                            .as_ref()
                            .and_then(|values| values.iter().find(|v| &v.definition.id == id))
                            .map(|v| {
                                format!(
                                    "{} = {}\n{}",
                                    v.definition.fields.display_name,
                                    v.value
                                        .map(|v| v.to_string())
                                        .unwrap_or_else(|| "Missing".into()),
                                    definition_text(&v.definition)
                                )
                            })
                            .unwrap_or_else(|| {
                                format!("{id}: no saved assignment (removed or not modeled)")
                            })
                    })
                    .collect::<Vec<_>>()
                    .join("\n\n"),
            );
        }
        if p.editable {
            if ctx.mode == Mode::Form {
                p.select("state","Base stat state","Unmodeled differs from a defined-empty list. Existing entries outside this page are preserved when saving defined values.",vec![choice("unmodeled","Unmodeled"),choice("defined","Defined — optional/incomplete values")]);
                p.select("add_pin","Add shared stat and exact version","Adds a Missing assignment; existing entries are preserved. Catalog choices are paginated.",definition_choices(state,None,n).await?);
                p.text(
                    "add_exact",
                    "Other exact shared stat pin",
                    "Optional identity@version overrides the catalog choice.",
                );
            }
            if ctx.mode == Mode::Source {
                p.text("target_form_version","Source form version","Pinned or newer version of the same form. No mapping is selected by default; unmapped values stay local.");
                for d in &entries {
                    if let Some(source) = &d.source {
                        let name = format!("map_{}", d.id);
                        p.select(&name,&format!("Copy source into {} ({})",d.fields.display_name,d.id),"Explicit injective mapping. The local exact definition pins this shared Stat identity.",vec![choice("","Keep local"),choice(&source.stat_id,&format!("Copy shared Stat {}",source.stat_id))]);
                        p.check(&format!("clear_missing_{}",d.id),format!("Clear to Missing when source is Missing for {} ({})",d.fields.display_name,d.id),"Only this explicitly selected mapping may clear a local value. Unselected Missing sources keep local values.");
                    }
                }
            } else {
                for d in &entries {
                    let name = format!("value_{}", d.id);
                    p.draft.entry(name.clone()).or_insert_with(|| {
                        old.assignments
                            .as_ref()
                            .and_then(|v| v.iter().find(|v| v.definition.id == d.id))
                            .and_then(|v| v.value)
                            .map(|v| v.to_string())
                            .unwrap_or_default()
                    });
                    p.text(&name,format!("Base value for {} ({})",d.fields.display_name,d.id),format!("Blank = Missing; zero is explicit zero. Exact definition version {}. {}",d.revision,constraints_text(&d.constraints)));
                    if ctx.mode == Mode::Form {
                        let key = format!("pin_{}", d.id);
                        p.draft
                            .entry(key.clone())
                            .or_insert_with(|| format!("{}@{}", d.id, d.revision));
                        p.text(&key,format!("Exact pin for {} ({})",d.fields.display_name,d.id),"Canonical Stat identity@version. Changing a pin never rounds or clamps its value.");
                        p.check(
                            &format!("remove_{}", d.id),
                            format!("Remove {} ({})", d.fields.display_name, d.id),
                            "Explicitly removes this assignment from the shared form.",
                        );
                    }
                }
            }
        }
        let suffix = match ctx.mode {
            Mode::Reconcile => "/reconcile",
            Mode::Source => "/source",
            _ => "",
        };
        pagination(&mut p, &format!("{url}{suffix}"), n, total_entries);
    }
    if p.editable {
        p.text(
            "reason",
            "Change reason (optional)",
            "Up to 500 characters, one line. Recorded in public history.",
        );
        // Never carry off-page values/pins/mappings as hidden successful controls.
        // Obsolete or old-client fields remain visible diagnostics, then fall out
        // of the next submission instead of overwriting unseen saved content.
        let extras = p
            .draft
            .keys()
            .filter(|k| {
                ["value_", "pin_", "map_", "remove_", "clear_missing_"]
                    .iter()
                    .any(|prefix| k.starts_with(prefix))
            })
            .filter(|k| !p.controls.iter().any(|c| &c.name == *k))
            .cloned()
            .collect::<Vec<_>>();
        for key in extras {
            p.panel(
                format!("Retained off-page field (not submitted): {key}"),
                val(&p.draft, &key).to_owned(),
            );
        }
        for name in ["expected_revision", "expected_roster_revision", "page"] {
            if let Some(raw) = p.draft.get(name) {
                p.hidden.insert(name.into(), raw.clone());
            }
        }
        if matches!(failure, Some(Error::Conflict)) {
            p.reviewed = format!(
                "{}@{}",
                ctx.revision(state).await?,
                ctx.roster_revision(state).await?
            );
        }
        p.baseline = match p.draft.get("page_baseline") {
            Some(raw) => raw.clone(),
            None => page_baseline(ctx, &p.draft)?,
        };
    }
    Ok(p)
}
fn response(state: &AppState, headers: &HeaderMap, mut page: Page, status: StatusCode) -> Response {
    if !page.editable {
        return no_store(render_status(page, status));
    }
    let (csrf, set) = match csrf_for(headers) {
        Ok(v) => v,
        Err(_) => return authoring::read_error(Error::Unavailable),
    };
    page.csrf = csrf.clone();
    let mut r = no_store(render_status(page, status));
    if set {
        append_cookie(&mut r, csrf_cookie(&csrf, state.cookie_secure));
    }
    r
}
async fn editor(
    State(state): State<AppState>,
    headers: HeaderMap,
    path: Option<Path<Values>>,
    uri: Uri,
    query: QueryInput,
) -> Response {
    no_store(finish(
        async {
            let ctx = Context::new(&path.map(|Path(p)| p).unwrap_or_default(), uri.path());
            if matches!(
                ctx.mode,
                Mode::New
                    | Mode::Edit
                    | Mode::Adopt
                    | Mode::RosterEdit
                    | Mode::Reconcile
                    | Mode::Source
            ) {
                ctx.permission(&state, &headers).await?;
            }
            let Query(q) =
                query.map_err(|_| invalid("query", "Check the stat page and selection."))?;
            let mut draft = saved_draft(&state, &ctx).await?;
            for k in [
                "page",
                "source_pin",
                "target_version",
                "target_form_version",
            ] {
                if let Some(v) = q.get(k) {
                    draft.insert(k.into(), v.clone());
                }
            }
            let page = make_page(&state, &headers, &ctx, draft, None).await?;
            Ok(response(&state, &headers, page, StatusCode::OK))
        }
        .await,
    ))
}

fn definition_edit(d: &Values) -> Result<DefinitionEdit, Error> {
    Ok(DefinitionEdit {
        expected_revision: expected(d)?,
        fields: fields(d),
        constraints: constraints(d)?,
        reason: reason(d),
    })
}
fn chosen<'a>(d: &'a Values, exact: &str, select: &str) -> &'a str {
    if val(d, exact).is_empty() {
        val(d, select)
    } else {
        val(d, exact)
    }
}

fn row_field_id(key: &str) -> Option<&str> {
    ["value_", "pin_", "map_", "remove_", "clear_missing_"]
        .iter()
        .find_map(|prefix| key.strip_prefix(prefix))
}
fn displayed(d: &Values) -> Result<Option<Vec<String>>, Error> {
    let Some(raw) = d.get("displayed") else {
        return Ok(None);
    };
    let ids: Vec<String> = raw
        .split(',')
        .filter(|id| !id.is_empty())
        .map(str::to_owned)
        .collect();
    let unique = ids.iter().collect::<std::collections::BTreeSet<_>>();
    if ids.len() > PAGE_SIZE
        || unique.len() != ids.len()
        || ids.iter().any(|id| !game_data::types::valid_id(id))
    {
        return Err(invalid(
            "page",
            "Reload this bounded page before editing its entries.",
        ));
    }
    Ok(Some(ids))
}
async fn value_definitions(state: &AppState, ctx: &Context) -> Result<Vec<StatDefinition>, Error> {
    if ctx.mode == Mode::Form {
        Ok(state
            .stat_values
            .form(&ctx.id)
            .await?
            .assignments
            .unwrap_or_default()
            .into_iter()
            .map(|v| v.definition)
            .collect())
    } else {
        Ok(state
            .stats
            .world_roster(ctx.world().ok_or(Error::NotFound)?)
            .await?
            .map(|r| r.entries)
            .unwrap_or_default())
    }
}
async fn check_page_scope(state: &AppState, ctx: &Context, d: &Values) -> Result<(), Error> {
    if ctx.definition() || ctx.roster() {
        return Ok(());
    }
    let ids = match displayed(d)? {
        Some(ids) => ids,
        None => value_definitions(state, ctx)
            .await?
            .into_iter()
            .skip((page_number(d)? - 1) * PAGE_SIZE)
            .take(PAGE_SIZE)
            .map(|e| e.id)
            .collect(),
    };
    if d.keys()
        .filter_map(|k| row_field_id(k))
        .any(|id| !ids.iter().any(|shown| shown == id))
    {
        return Err(invalid(
            "page",
            "Off-page fields were rejected. Only the displayed entries can be saved. Their retained text is shown below but is not resubmitted; open that entry's page to review it.",
        ));
    }
    Ok(())
}
// Canonical raw control values, including absent unchecked boxes. This is a UX
// consistency digest, not authorization. Policy and expected revisions are still
// rechecked by the locked application mutations.
fn editable_draft(ctx: &Context, d: &Values) -> Result<Values, Error> {
    let names: &[&str] = match ctx.mode {
        Mode::New => &[
            "display_name",
            "description",
            "minimum",
            "maximum",
            "whole_only",
            "source_pin",
            "source_exact",
            "reason",
        ],
        Mode::Edit => &[
            "display_name",
            "description",
            "minimum",
            "maximum",
            "whole_only",
            "reason",
        ],
        Mode::Adopt => &[
            "display_name",
            "description",
            "minimum",
            "maximum",
            "whole_only",
            "target_version",
            "reason",
        ],
        Mode::RosterEdit => &[
            "operation",
            "stat_pin",
            "exact_pin",
            "position",
            "destination",
            "reason",
        ],
        Mode::Form => &["state", "add_pin", "add_exact", "reason"],
        Mode::Source => &["target_form_version", "reason"],
        _ => &["reason"],
    };
    let mut fields = names
        .iter()
        .map(|name| (name.to_string(), val(d, name).to_owned()))
        .collect::<Values>();
    for id in displayed(d)?.unwrap_or_default() {
        let prefixes: &[&str] = if ctx.mode == Mode::Form {
            &["value_", "pin_", "remove_"]
        } else if ctx.mode == Mode::Source {
            &["map_", "clear_missing_"]
        } else {
            &["value_"]
        };
        for prefix in prefixes {
            let key = format!("{prefix}{id}");
            fields.insert(key.clone(), val(d, &key).into());
        }
    }
    Ok(fields)
}
fn digest(value: &impl serde::Serialize) -> Result<String, Error> {
    Ok(hex::encode(Sha256::digest(
        serde_json::to_vec(value).map_err(|_| Error::Unavailable)?,
    )))
}
fn page_baseline(ctx: &Context, d: &Values) -> Result<String, Error> {
    digest(&editable_draft(ctx, d)?)
}
fn review_digest(ctx: &Context, d: &Values, proposal: &serde_json::Value) -> Result<String, Error> {
    digest(&(
        ctx.action(),
        val(d, "expected_revision"),
        val(d, "expected_roster_revision"),
        val(d, "page"),
        val(d, "displayed"),
        editable_draft(ctx, d)?,
        proposal,
    ))
}
async fn roster_proposal(state: &AppState, world: &str, d: &Values) -> Result<Vec<StatPin>, Error> {
    let mut entries = state
        .stats
        .world_roster(world)
        .await?
        .map(|r| r.entries.into_iter().map(|e| e.pin()).collect::<Vec<_>>())
        .unwrap_or_default();
    let index = || -> Result<usize, Error> {
        positive(val(d, "position"), "position")?
            .try_into()
            .ok()
            .and_then(|n: usize| n.checked_sub(1))
            .filter(|n| *n < entries.len())
            .ok_or_else(|| invalid("position", "Select an existing one-based roster position."))
    };
    match val(d, "operation") {
        "initialize" => {}
        "add" => {
            let p = pin(chosen(d, "exact_pin", "stat_pin"), "stat_pin")?;
            let at = if val(d, "destination").is_empty() {
                entries.len()
            } else {
                usize::try_from(positive(val(d, "destination"), "destination")?)
                    .ok()
                    .and_then(|n| n.checked_sub(1))
                    .filter(|n| *n <= entries.len())
                    .ok_or_else(|| {
                        invalid(
                            "destination",
                            "Choose a position within the roster or one after its end.",
                        )
                    })?
            };
            entries.insert(at, p);
        }
        "remove" => {
            entries.remove(index()?);
        }
        "version" => {
            let i = index()?;
            let p = pin(chosen(d, "exact_pin", "stat_pin"), "stat_pin")?;
            if entries[i].stat_id != p.stat_id {
                return Err(invalid(
                    "stat_pin",
                    "A version change must retain this Stat identity. Remove and add to change identity.",
                ));
            }
            entries[i] = p;
        }
        "move" => {
            let i = index()?;
            let at = usize::try_from(positive(val(d, "destination"), "destination")?)
                .ok()
                .and_then(|n| n.checked_sub(1))
                .filter(|n| *n < entries.len())
                .ok_or_else(|| {
                    invalid("destination", "Choose an existing one-based destination.")
                })?;
            let p = entries.remove(i);
            entries.insert(at, p);
        }
        _ => return Err(invalid("operation", "Choose a roster operation.")),
    }
    let mut ids = std::collections::BTreeSet::new();
    for p in &entries {
        if !ids.insert(&p.stat_id) {
            return Err(invalid("stat_pin", "Choose each Stat identity only once."));
        }
        state
            .stats
            .version(Some(world), &p.stat_id, p.version)
            .await?;
    }
    Ok(entries)
}
fn value_updates(d: &Values) -> Result<Vec<(String, Option<Decimal>)>, Error> {
    let mut errors = Values::new();
    let mut result = vec![];
    for (k, raw) in d {
        if let Some(id) = k.strip_prefix("value_") {
            match decimal(raw, k) {
                Ok(v) => result.push((id.into(), v)),
                Err(Error::Invalid(e)) => errors.extend(e),
                Err(e) => return Err(e),
            }
        }
    }
    if errors.is_empty() {
        Ok(result)
    } else {
        Err(Error::Invalid(errors))
    }
}
async fn form_edit(state: &AppState, id: &str, d: &Values) -> Result<FormStatsEdit, Error> {
    let values = match val(d, "state") {
        "unmodeled" => None,
        "defined" => {
            let old = state.stat_values.form(id).await?;
            let mut values = old
                .assignments
                .unwrap_or_default()
                .into_iter()
                .map(|v| (v.definition.pin(), v.value))
                .collect::<Vec<_>>();
            for (id, value) in value_updates(d)? {
                let Some(i) = values.iter().position(|(p, _)| p.stat_id == id) else {
                    if value.is_none() {
                        continue;
                    }
                    return Err(invalid(
                        &format!("value_{id}"),
                        "This entry is no longer assigned. Clear the obsolete value and review the saved form.",
                    ));
                };
                if val(d, &format!("remove_{id}")) == "yes" {
                    values.remove(i);
                    continue;
                }
                let p = pin(val(d, &format!("pin_{id}")), &format!("pin_{id}"))?;
                if p.stat_id != id {
                    return Err(invalid(
                        &format!("pin_{id}"),
                        "Keep the same identity; remove and add to choose another Stat.",
                    ));
                }
                values[i] = (p, value);
            }
            let add = chosen(d, "add_exact", "add_pin");
            if !add.is_empty() {
                values.push((pin(add, "add_pin")?, None));
            }
            Some(values)
        }
        _ => return Err(invalid("state", "Choose Unmodeled or Defined.")),
    };
    Ok(FormStatsEdit {
        expected_revision: expected(d)?,
        values,
        reason: reason(d),
    })
}
async fn variant_edit(
    state: &AppState,
    ctx: &Context,
    d: &Values,
) -> Result<VariantStatsEdit, Error> {
    let roster = state
        .stats
        .world_roster(ctx.world().ok_or(Error::NotFound)?)
        .await?;
    let mut values = value_updates(d)?;
    values.retain(|(id, value)| {
        value.is_some()
            || roster
                .as_ref()
                .is_some_and(|r| r.entries.iter().any(|e| &e.id == id))
    });
    Ok(VariantStatsEdit {
        expected_revision: expected(d)?,
        expected_roster_revision: positive(
            val(d, "expected_roster_revision"),
            "expected_roster_revision",
        )?,
        values,
        reason: reason(d),
    })
}
async fn validate_value_fields(state: &AppState, ctx: &Context, d: &Values) -> Result<(), Error> {
    let mut errors = Values::new();
    for (id, value) in value_updates(d)? {
        if ctx.mode == Mode::Form
            && (val(d, "state") == "unmodeled" || val(d, &format!("remove_{id}")) == "yes")
        {
            continue;
        }
        let definition = if ctx.mode == Mode::Form {
            let p = pin(val(d, &format!("pin_{id}")), &format!("pin_{id}"))?;
            state.stats.version(None, &p.stat_id, p.version).await?
        } else {
            let selected = state
                .stats
                .world_roster(ctx.world().ok_or(Error::NotFound)?)
                .await?
                .and_then(|r| r.entries.into_iter().find(|e| e.id == id));
            let Some(selected) = selected else {
                if value.is_none() {
                    continue;
                }
                return Err(invalid(
                    &format!("value_{id}"),
                    "This Stat is outside the current roster. Clear the obsolete draft and review reconciliation.",
                ));
            };
            selected
        };
        if value.is_some_and(|v| !definition.constraints.accepts(v)) {
            errors.insert(format!("value_{id}"),format!("Value does not satisfy exact pinned constraints. {} Correct it or leave blank for Missing.",constraints_text(&definition.constraints)));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(Error::Invalid(errors))
    }
}
fn mappings(d: &Values) -> Vec<StatMapping> {
    d.iter()
        .filter_map(|(key, shared)| {
            let local = key.strip_prefix("map_")?;
            if shared.is_empty() {
                return None;
            }
            Some(StatMapping {
                shared_stat_id: shared.clone(),
                local_stat_id: local.into(),
                clear_missing: val(d, &format!("clear_missing_{local}")) == "yes",
            })
        })
        .collect()
}
fn source_edit(d: &Values) -> Result<SourceValuesEdit, Error> {
    Ok(SourceValuesEdit {
        expected_revision: expected(d)?,
        expected_roster_revision: positive(
            val(d, "expected_roster_revision"),
            "expected_roster_revision",
        )?,
        target_form_version: positive(val(d, "target_form_version"), "target_form_version")?,
        mapping: mappings(d),
        reason: reason(d),
    })
}
async fn comparisons(
    state: &AppState,
    ctx: &Context,
    d: &Values,
) -> Result<(Vec<Panel>, serde_json::Value), Error> {
    let n = page_number(d)?;
    let mut p = vec![];
    let mut proposal = serde_json::Value::Null;
    let mut add = |label: &str, text: String| {
        p.push(Panel {
            label: label.into(),
            text,
        })
    };
    match ctx.mode {
        Mode::Adopt => {
            let v = state
                .stats
                .adoption_preview(
                    ctx.world().ok_or(Error::NotFound)?,
                    &ctx.id,
                    positive(val(d, "target_version"), "target_version")?,
                    &definition_edit(d)?,
                )
                .await?;
            proposal = serde_json::to_value(&v.proposed).map_err(|_| Error::Unavailable)?;
            for (label, v) in [
                ("Old shared source", v.old_source),
                ("Current local definition", v.entry),
                ("New shared source", v.new_source),
                ("Proposed definition", v.proposed),
            ] {
                add(label, definition_text(&v));
            }
        }
        Mode::RosterEdit => {
            let w = ctx.world().ok_or(Error::NotFound)?;
            let pins = roster_proposal(state, w, d).await?;
            let mut defs = vec![];
            for pin in pins {
                defs.push(
                    state
                        .stats
                        .version(Some(w), &pin.stat_id, pin.version)
                        .await?,
                );
            }
            let old = state.stats.world_roster(w).await?;
            add("Old roster", roster_text(old.as_ref(), n));
            let removed = old
                .as_ref()
                .map(|r| {
                    r.entries
                        .iter()
                        .filter(|v| !defs.iter().any(|d| d.id == v.id))
                        .map(definition_text)
                        .collect::<Vec<_>>()
                        .join("\n")
                })
                .unwrap_or_default();
            add(
                "Removed entries",
                if removed.is_empty() {
                    "None".into()
                } else {
                    removed
                },
            );
            let proposed = StatRoster {
                id: old.as_ref().map(|r| r.id.clone()).unwrap_or_default(),
                world_id: w.into(),
                revision: old.map_or(1, |r| r.revision + 1),
                entries: defs,
            };
            proposal = serde_json::to_value(&proposed).map_err(|_| Error::Unavailable)?;
            add("Proposed roster", roster_text(Some(&proposed), n));
        }
        Mode::Reconcile => {
            let edit = variant_edit(state, ctx, d).await?;
            let v = state
                .stat_values
                .preview_reconcile(
                    ctx.world().ok_or(Error::NotFound)?,
                    &ctx.id,
                    edit.expected_roster_revision,
                    &edit.values,
                )
                .await?;
            proposal = serde_json::to_value(&v.proposed).map_err(|_| Error::Unavailable)?;
            add("Old base stats", stats_page_text(&v.old, n));
            add("Proposed base stats", stats_page_text(&v.proposed, n));
            add(
                "Removed values",
                if v.removed.is_empty() {
                    "None".into()
                } else {
                    // Removals are not indexed by the *new* roster page. Show
                    // every affected identity/value here even if its old position
                    // is outside the currently edited page.
                    v.removed
                        .iter()
                        .map(|value| {
                            format!(
                                "{} = {} · {}@{}\n{}",
                                value.definition.fields.display_name,
                                value
                                    .value
                                    .map(|v| v.to_string())
                                    .unwrap_or_else(|| "Missing".into()),
                                value.definition.id,
                                value.definition.revision,
                                constraints_text(&value.definition.constraints),
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n\n")
                },
            );
        }
        Mode::Source => {
            let edit = source_edit(d)?;
            let v = state
                .stat_values
                .preview_source(
                    ctx.world().ok_or(Error::NotFound)?,
                    &ctx.id,
                    edit.target_form_version,
                    &edit.mapping,
                )
                .await?;
            proposal = serde_json::to_value(&v.proposed).map_err(|_| Error::Unavailable)?;
            for (label, s) in [
                ("Old source base stats", v.old_source),
                ("New source base stats", v.new_source),
                ("Local base stats", v.local),
                ("Proposed base stats", v.proposed),
            ] {
                add(label, stats_page_text(&s, n));
            }
        }
        _ => {}
    }
    Ok((p, proposal))
}
async fn submit(
    State(state): State<AppState>,
    headers: HeaderMap,
    path: Option<Path<Values>>,
    uri: Uri,
    form: FormInput,
) -> Response {
    let ctx = Context::new(&path.map(|Path(p)| p).unwrap_or_default(), uri.path());
    let mut d = match form {
        Ok(Form(d)) => d,
        Err(e) => return authoring_form_error(e),
    };
    if let Some(reviewed) = d.remove("reviewed_revision").filter(|s| !s.is_empty()) {
        if let Some((r, roster)) = reviewed.split_once('@') {
            d.insert("expected_revision".into(), r.into());
            if d.contains_key("expected_roster_revision") {
                d.insert("expected_roster_revision".into(), roster.into());
            }
        }
        if matches!(
            ctx.mode,
            Mode::Adopt | Mode::RosterEdit | Mode::Reconcile | Mode::Source
        ) {
            d.insert("intent".into(), "preview".into());
        }
    }
    let result:Result<Option<Vec<Panel>>,Error>=async{
        let token=submit_token(&state,&headers,val(&d,"csrf"))?;
        ctx.permission(&state,&headers).await?;
        // Navigation never writes or advances a stale draft's revision. A dirty
        // page remains visible until explicitly saved or explicitly discarded.
        if val(&d,"intent")=="discard" || val(&d,"intent").starts_with("page:") {
            let next=if val(&d,"intent")=="discard"{page_number(&d)?}else{
                if val(&d,"page_baseline").is_empty() || val(&d,"page_baseline")!=page_baseline(&ctx,&d)? {
                    return Err(invalid("page","This page has unsaved changes. Save the current page before navigation, or explicitly discard this page draft. Your raw values and reason are retained."));
                }
                val(&d,"intent").strip_prefix("page:").unwrap_or("").parse::<usize>().ok().filter(|n|*n>0 && *n<=usize::MAX/PAGE_SIZE).ok_or_else(||invalid("page","Choose a positive page number."))?
            };
            d=saved_draft(&state,&ctx).await?;d.insert("page".into(),next.to_string());
            return Ok(Some(vec![]));
        }
        if ctx.mode!=Mode::New && expected(&d)?!=ctx.revision(&state).await?{return Err(Error::Conflict);}
        if matches!(ctx.mode,Mode::Variant|Mode::Reconcile|Mode::Source) && val(&d,"expected_roster_revision").parse::<i64>().ok()!=Some(ctx.roster_revision(&state).await?){return Err(Error::Conflict);}
        if !ctx.definition() && !ctx.roster() && !d.contains_key("displayed"){
            let ids=value_definitions(&state,&ctx).await?.into_iter().skip((page_number(&d)?-1)*PAGE_SIZE).take(PAGE_SIZE).map(|e|e.id).collect::<Vec<_>>();d.insert("displayed".into(),ids.join(","));
        }
        check_page_scope(&state,&ctx,&d).await?;
        if matches!(ctx.mode,Mode::Form|Mode::Variant|Mode::Reconcile){validate_value_fields(&state,&ctx,&d).await?;}
        if matches!(ctx.mode,Mode::Adopt|Mode::RosterEdit|Mode::Reconcile|Mode::Source){
            let (panels,proposal)=comparisons(&state,&ctx,&d).await?;
            // Fill absent native controls before hashing so an omitted blank and
            // the browser's explicitly submitted blank have identical meaning.
            let page=make_page(&state,&headers,&ctx,d.clone(),None).await?;
            let mut canonical=page.draft;
            canonical.extend(d.clone());
            let fingerprint=review_digest(&ctx,&canonical,&proposal)?;
            if val(&d,"intent")=="preview" || val(&d,"review_fingerprint")!=fingerprint {
                if val(&d,"intent")!="preview"{d.insert("preview_changed".into(),"yes".into());}
                d.insert("review_fingerprint".into(),fingerprint);
                return Ok(Some(panels));
            }
            if val(&d,"confirm")!="yes"{return Err(invalid("confirm","Preview and explicitly confirm the proposed pins, removals, version changes and values."));}
        }
        match ctx.mode {
            Mode::New=>{
                let source=chosen(&d,"source_exact","source_pin");
                let row=state.stats.create(&token,ctx.world(),NewStat{fields:fields(&d),constraints:constraints(&d)?,source:if source.is_empty(){None}else{Some(pin(source,"source_pin")?)},reason:reason(&d)}).await?;
                // Keep the new canonical ID for the redirect; never derive it from a slug.
                d.insert("created_id".into(),row.id);
            },
            Mode::Edit=>{state.stats.save_definition(&token,ctx.world(),&ctx.id,definition_edit(&d)?).await?;},
            Mode::Adopt=>{state.stats.adopt(&token,ctx.world().ok_or(Error::NotFound)?,&ctx.id,positive(val(&d,"target_version"),"target_version")?,definition_edit(&d)?).await?;},
            Mode::RosterEdit=>{let w=ctx.world().ok_or(Error::NotFound)?;state.stats.save_roster(&token,w,expected(&d)?,roster_proposal(&state,w,&d).await?,reason(&d)).await?;},
            Mode::Form=>{state.stat_values.save_form(&token,&ctx.id,form_edit(&state,&ctx.id,&d).await?).await?;},
            Mode::Variant=>{state.stat_values.save_variant(&token,ctx.world().ok_or(Error::NotFound)?,&ctx.id,variant_edit(&state,&ctx,&d).await?).await?;},
            Mode::Reconcile=>{state.stat_values.reconcile(&token,ctx.world().ok_or(Error::NotFound)?,&ctx.id,variant_edit(&state,&ctx,&d).await?).await?;},
            Mode::Source=>{state.stat_values.apply_source(&token,ctx.world().ok_or(Error::NotFound)?,&ctx.id,source_edit(&d)?).await?;},
            _=>return Err(Error::NotFound),
        } Ok(None)
    }.await;
    d.remove("csrf");
    d.remove("confirm");
    match result {
        Ok(None) => no_store(
            Redirect::to(&if ctx.mode == Mode::New {
                format!("{}/{}", root(ctx.world()), val(&d, "created_id"))
            } else {
                ctx.target()
            })
            .into_response(),
        ),
        Ok(Some(panels)) => match make_page(&state, &headers, &ctx, d, None).await {
            Ok(mut p) => {
                p.ready = !panels.is_empty();
                if p.ready {
                    p.review_fingerprint = val(&p.draft, "review_fingerprint").into();
                }
                if val(&p.draft, "preview_changed") == "yes" {
                    p.notice="The reviewed inputs or proposal changed. Nothing was saved. Review this fresh preview and confirm again.".into();
                }
                p.panels.extend(panels);
                response(&state, &headers, p, StatusCode::OK)
            }
            Err(e) => authoring::read_error(e),
        },
        Err(e) => match make_page(&state, &headers, &ctx, d, Some(&e)).await {
            Ok(mut p) => {
                // Policy may have been revoked after drafting. Retain the draft visibly.
                if !p.editable && !matches!(ctx.mode, Mode::Detail | Mode::Roster) {
                    p.panel(
                        "Retained submitted draft",
                        p.draft
                            .iter()
                            .filter(|(k, _)| k.as_str() != "created_id")
                            .map(|(k, v)| format!("{k}: {v}"))
                            .collect::<Vec<_>>()
                            .join("\n"),
                    );
                }
                p.ready = false;
                response(&state, &headers, p, error_status(&e))
            }
            Err(read) => authoring::read_error(read),
        },
    }
}

#[derive(Template)]
#[template(path = "stat_history.html")]
struct History {
    title: String,
    url: String,
    entries: Vec<HistoryEntry>,
    next: Option<i64>,
    panels: Vec<Panel>,
    links: Vec<Link>,
}
async fn history(
    State(state): State<AppState>,
    Path(path): Path<Values>,
    uri: Uri,
    query: QueryInput,
) -> Response {
    finish(
        async {
            let world = path.get("world_id").map(String::as_str);
            let id = val(&path, "id");
            let roster = uri.path().contains("/stat-roster/");
            let Query(q) =
                query.map_err(|_| invalid("before", "Use a positive history cursor."))?;
            let revision = if val(&path, "revision").is_empty() {
                None
            } else {
                Some(positive(val(&path, "revision"), "revision")?)
            };
            let before = if val(&q, "before").is_empty() {
                None
            } else {
                Some(positive(val(&q, "before"), "before")?)
            };
            let (url, title, entries) = if roster {
                let w = world.ok_or(Error::NotFound)?;
                let parent = state.authoring.world_detail(w).await?;
                let url = format!(
                    "/worlds/{}/stat-roster",
                    addresses::segment(&parent.fields.slug)
                );
                let entries = if let Some(r) = revision {
                    state
                        .stats
                        .roster_history(w, r.checked_add(1))
                        .await?
                        .into_iter()
                        .filter(|e| e.revision == r)
                        .collect()
                } else {
                    state.stats.roster_history(w, before).await?
                };
                (url, "Roster history".into(), entries)
            } else {
                let current = state.stats.detail(world, id).await?;
                let entries = if let Some(r) = revision {
                    vec![state.stats.history_entry(world, id, r).await?]
                } else {
                    state.stats.history(world, id, before).await?
                };
                (
                    definition_url(&state, &current).await?,
                    "Stat history".into(),
                    entries,
                )
            };
            if revision.is_some() && entries.is_empty() {
                return Err(Error::NotFound);
            }
            let next = if revision.is_none() && entries.len() == 50 {
                entries.last().map(|e| e.revision)
            } else {
                None
            };
            let mut panels = vec![];
            let mut links = vec![];
            if let Some(r) = revision {
                let n = page_number(&q)?;
                let text = if roster {
                    let w = world.ok_or(Error::NotFound)?;
                    let current = state.stats.world_roster(w).await?.ok_or(Error::NotFound)?;
                    let state = state.stats.roster_version(w, &current.id, r).await?;
                    if n > 1 {
                        links.push(Link {
                            label: "Previous historical page".into(),
                            url: format!("{url}/history/{r}?page={}", n - 1),
                        });
                    }
                    if n * PAGE_SIZE < state.entries.len() {
                        links.push(Link {
                            label: "Next historical page".into(),
                            url: format!("{url}/history/{r}?page={}", n + 1),
                        });
                    }
                    roster_text(Some(&state), n)
                } else {
                    definition_text(&state.stats.version(world, id, r).await?)
                };
                panels.push(Panel {
                    label: format!("Exact state at revision {r}"),
                    text,
                });
            }
            Ok(render(History {
                title,
                url,
                entries,
                next,
                panels,
                links,
            }))
        }
        .await,
    )
}

async fn historical_values(
    State(state): State<AppState>,
    Path(path): Path<Values>,
    query: QueryInput,
) -> Response {
    finish(
        async {
            let Query(q) =
                query.map_err(|_| invalid("page", "Choose a positive historical page."))?;
            let n = page_number(&q)?;
            let id = val(&path, "id");
            let revision = positive(val(&path, "revision"), "revision")?;
            let (kind, url) = if let Some(w) = path.get("world_id") {
                let (world, variant) = state
                    .authoring
                    .resolve_variant(
                        crate::application::authoring::AddressKey::Id(w.clone()),
                        crate::application::authoring::AddressKey::Id(id.into()),
                    )
                    .await?;
                (
                    crate::application::authoring::HistoryKind::Variant,
                    format!(
                        "/worlds/{}/variants/{}",
                        addresses::segment(&world.fields.slug),
                        addresses::segment(&variant.fields.slug)
                    ),
                )
            } else {
                let (species, form) = state.authoring.resolve_legacy_form(id).await?;
                (
                    crate::application::authoring::HistoryKind::Form,
                    format!(
                        "/creatures/{}/forms/{}",
                        addresses::segment(&species.fields.slug),
                        addresses::segment(&form.fields.slug)
                    ),
                )
            };
            let entry = state.authoring.history_entry(kind, id, revision).await?;
            let after = serde_json::from_str(&entry.after_json).map_err(|_| Error::Unavailable)?;
            let before = entry
                .before_json
                .as_deref()
                .map(serde_json::from_str)
                .transpose()
                .map_err(|_| Error::Unavailable)?;
            let old = authoring::snapshot_stats(before.as_ref())?;
            let new = authoring::snapshot_stats(Some(&after))?;
            let ctx = Context {
                world: path.get("world_id").cloned(),
                id: id.into(),
                mode: if path.contains_key("world_id") {
                    Mode::Variant
                } else {
                    Mode::Form
                },
            };
            let mut p = Page::new(
                &format!("Historical base stats — revision {revision}"),
                &ctx,
                Values::new(),
                None,
            );
            p.cancel = format!("{url}/history/{revision}");
            p.panel(
                "Before — exact historical base stats",
                stats_page_text(&old, n),
            );
            p.panel(
                "After — exact historical base stats",
                stats_page_text(&new, n),
            );
            pagination(
                &mut p,
                &format!("{url}/history/{revision}/stats"),
                n,
                old.assignments
                    .as_ref()
                    .map_or(0, Vec::len)
                    .max(new.assignments.as_ref().map_or(0, Vec::len)),
            );
            Ok(render(p))
        }
        .await,
    )
}
