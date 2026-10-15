//! Bounded native chart forms. The application owns validation, policy and writes.
use super::authoring::{
    SourceChoice, authoring_form_error, error_status, error_text, finish, invalid,
    library_permission, no_store, positive, session, submit_token, world_permission,
};
use super::*;
use crate::application::{
    authoring::{Error, HistoryEntry},
    charts::{
        AdoptEdit, ChartCell, ChartMapping, ChartPin, ChartRecord, DefinitionEdit, Effectiveness,
        Factor, NeutralEdit, NewChart, RowEdit,
    },
};
use axum::{extract::rejection::QueryRejection, http::Uri};
use game_data::{authoring::ContentFields, types::TypePin};
use std::collections::{BTreeMap, BTreeSet};

type Values = BTreeMap<String, String>;
type FormInput = Result<Form<Values>, FormRejection>;
type QueryInput = Result<Query<Values>, QueryRejection>;

pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/charts", get(catalog))
        .route("/charts/new", get(shared_new).post(shared_create))
        .route("/charts/{id}", get(detail))
        .route("/charts/{id}/edit", get(editor).post(submit))
        .route("/charts/{id}/rows/{attacker}", get(editor).post(submit))
        .route("/charts/{id}/neutral", get(editor).post(submit))
        .route("/charts/{id}/history", get(history))
        .route("/charts/{id}/history/{revision}", get(snapshot))
        .route("/worlds/{world_id}/charts", get(world_landing))
        .route("/worlds/{world_id}/charts/new", get(editor).post(submit))
        .route("/worlds/{world_id}/charts/{id}", get(detail))
        .route(
            "/worlds/{world_id}/charts/{id}/edit",
            get(editor).post(submit),
        )
        .route(
            "/worlds/{world_id}/charts/{id}/rows/{attacker}",
            get(editor).post(submit),
        )
        .route(
            "/worlds/{world_id}/charts/{id}/neutral",
            get(editor).post(submit),
        )
        .route(
            "/worlds/{world_id}/charts/{id}/adopt",
            get(editor).post(submit),
        )
        .route("/worlds/{world_id}/charts/{id}/history", get(history))
        .route(
            "/worlds/{world_id}/charts/{id}/history/{revision}",
            get(snapshot),
        )
        .layer(DefaultBodyLimit::max(32 * 1024))
}

fn value<'a>(values: &'a Values, name: &str) -> &'a str {
    values.get(name).map(String::as_str).unwrap_or("")
}

fn base(world: Option<&str>) -> String {
    world.map_or_else(|| "/charts".into(), |w| format!("/worlds/{w}/charts"))
}

async fn pretty(state: &AppState, entry: &ChartRecord) -> Result<String, Error> {
    let root = if let Some(world) = &entry.world_id {
        let world = state.authoring.world_detail(world).await?;
        base(Some(&addresses::segment(&world.fields.slug)))
    } else {
        base(None)
    };
    Ok(format!("{root}/{}", addresses::segment(&entry.fields.slug)))
}

#[derive(Template)]
#[template(path = "charts.html")]
struct Catalog {
    charts: Vec<ChartRecord>,
    can_edit: bool,
}

async fn catalog(State(state): State<AppState>, headers: HeaderMap) -> Response {
    finish(
        async {
            let caps = state
                .authoring
                .capabilities(session(&headers).as_deref(), None)
                .await?;
            Ok(render(Catalog {
                charts: state.charts.charts().await?,
                can_edit: caps.can_edit_library,
            }))
        }
        .await,
    )
}

async fn world_landing(State(state): State<AppState>, Path(world): Path<String>) -> Response {
    no_store(finish(
        async {
            let parent = state.authoring.world_detail(&world).await?;
            let url = match state.charts.world_chart(&world).await? {
                Some(entry) => pretty(&state, &entry).await?,
                None => format!(
                    "{}/new",
                    base(Some(&addresses::segment(&parent.fields.slug)))
                ),
            };
            Ok(Redirect::to(&url).into_response())
        }
        .await,
    ))
}

struct Member {
    id: String,
    name: String,
    version: i64,
    url: String,
}
struct Pair {
    defender: String,
    text: String,
}
struct MatrixRow {
    name: String,
    edit_url: String,
    pairs: Vec<Pair>,
}
struct Matrix {
    provisional: bool,
    entry: ChartRecord,
    members: Vec<Member>,
    rows: Vec<MatrixRow>,
}

fn factor_text(entry: &ChartRecord, attacker: &str, defender: &str) -> String {
    match entry.lookup(attacker, defender) {
        Effectiveness::Unknown => "Unknown".into(),
        Effectiveness::OutsideRoster => "Not covered (outside roster)".into(),
        Effectiveness::Known(factor) => {
            if factor.numerator == 0 {
                format!("{factor} — No effect")
            } else if factor.numerator == factor.denominator {
                format!("{factor} — Neutral")
            } else {
                factor.to_string()
            }
        }
    }
}

impl Matrix {
    fn new(entry: ChartRecord, edit_base: &str) -> Self {
        let members = entry
            .roster
            .iter()
            .map(|assignment| {
                let root = entry
                    .world_id
                    .as_deref()
                    .map_or_else(|| "/types".into(), |w| format!("/worlds/{w}/types"));
                Member {
                    id: assignment.pin.type_id.clone(),
                    name: assignment.fields.display_name.clone(),
                    version: assignment.pin.version,
                    url: format!(
                        "{root}/{}/history/{}",
                        assignment.pin.type_id, assignment.pin.version
                    ),
                }
            })
            .collect::<Vec<_>>();
        let rows = members
            .iter()
            .map(|attacker| MatrixRow {
                name: attacker.name.clone(),
                edit_url: format!("{edit_base}/rows/{}", attacker.id),
                pairs: members
                    .iter()
                    .map(|defender| Pair {
                        defender: defender.name.clone(),
                        text: factor_text(&entry, &attacker.id, &defender.id),
                    })
                    .collect(),
            })
            .collect();
        Self {
            provisional: false,
            entry,
            members,
            rows,
        }
    }

    fn creation_preview(entry: ChartRecord) -> Self {
        let mut matrix = Self::new(entry, "");
        matrix.provisional = true;
        matrix
    }
}

#[derive(Template)]
#[template(path = "chart_detail.html")]
struct Detail {
    matrix: Matrix,
    url: String,
    can_edit: bool,
    source_url: String,
    latest: Option<ChartRecord>,
    worlds: Vec<types::WorldUse>,
}

async fn detail(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(path): Path<Values>,
) -> Response {
    finish(
        async {
            let world = path.get("world_id").map(String::as_str);
            let entry = state.charts.detail(world, value(&path, "id")).await?;
            let url = pretty(&state, &entry).await?;
            let caps = state
                .authoring
                .capabilities(session(&headers).as_deref(), world)
                .await?;
            let (source_url, latest) = if let Some(pin) = &entry.source {
                (
                    format!("/charts/{}/history/{}", pin.chart_id, pin.version),
                    Some(state.charts.detail(None, &pin.chart_id).await?),
                )
            } else {
                (String::new(), None)
            };
            let worlds = if world.is_none() {
                types::world_uses(&state, &headers).await?
            } else {
                Vec::new()
            };
            let can_edit = if world.is_some() {
                caps.can_edit_world
            } else {
                caps.can_edit_library
            };
            Ok(render(Detail {
                matrix: Matrix::new(entry, &url),
                url,
                can_edit,
                source_url,
                latest,
                worlds,
            }))
        }
        .await,
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    New,
    Definition,
    Row,
    Neutral,
    Adopt,
}

struct Context {
    world: Option<String>,
    id: Option<String>,
    attacker: String,
    mode: Mode,
}
impl Context {
    fn from_path(path: Values, route: &str) -> Self {
        let mode = if route.ends_with("/new") {
            Mode::New
        } else if route.ends_with("/neutral") {
            Mode::Neutral
        } else if route.ends_with("/adopt") {
            Mode::Adopt
        } else if path.contains_key("attacker") {
            Mode::Row
        } else {
            Mode::Definition
        };
        Self {
            world: path.get("world_id").cloned(),
            id: path.get("id").cloned(),
            attacker: value(&path, "attacker").into(),
            mode,
        }
    }
    fn cancel(&self) -> String {
        match &self.id {
            Some(id) => format!("{}/{id}", base(self.world.as_deref())),
            None => base(self.world.as_deref()),
        }
    }
    fn action(&self) -> String {
        match self.mode {
            Mode::New => format!("{}/new", self.cancel()),
            Mode::Definition => format!("{}/edit", self.cancel()),
            Mode::Row => format!("{}/rows/{}", self.cancel(), self.attacker),
            Mode::Neutral => format!("{}/neutral", self.cancel()),
            Mode::Adopt => format!("{}/adopt", self.cancel()),
        }
    }
    async fn permission(&self, state: &AppState, headers: &HeaderMap) -> Result<(), Error> {
        match self.world.as_deref() {
            Some(world) => world_permission(state, headers, Some(world)).await,
            None => library_permission(state, headers).await,
        }
    }
    async fn current(&self, state: &AppState) -> Result<Option<ChartRecord>, Error> {
        match self.id.as_deref() {
            Some(id) => state
                .charts
                .detail(self.world.as_deref(), id)
                .await
                .map(Some),
            None => Ok(None),
        }
    }
}

fn fields(draft: &Values) -> ContentFields {
    ContentFields {
        slug: String::new(),
        display_name: value(draft, "display_name").into(),
        description: value(draft, "description").into(),
    }
}
fn reason(draft: &Values) -> Option<String> {
    draft.get("reason").filter(|s| !s.is_empty()).cloned()
}
fn revision(draft: &Values) -> Result<i64, Error> {
    positive(value(draft, "expected_revision"), "expected_revision")
}
fn pin(raw: &str, field: &str) -> Result<(String, i64), Error> {
    let (id, version) = raw
        .rsplit_once('@')
        .filter(|(id, _)| !id.is_empty())
        .ok_or_else(|| invalid(field, "Choose an identity and exact version."))?;
    Ok((id.into(), positive(version, field)?))
}
fn source(draft: &Values) -> Result<Option<ChartPin>, Error> {
    let raw = value(draft, "source_pin");
    if raw.is_empty() {
        return Ok(None);
    }
    let (chart_id, version) = pin(raw, "source_pin")?;
    Ok(Some(ChartPin { chart_id, version }))
}
fn slot_count(draft: &Values) -> Result<usize, Error> {
    value(draft, "slots")
        .parse::<usize>()
        .ok()
        .filter(|n| *n <= 32)
        .ok_or_else(|| {
            invalid(
                "slots",
                "Choose from 0 to 32 slots. This is a software bound, not a game rule.",
            )
        })
}
fn roster(draft: &Values) -> Result<Vec<TypePin>, Error> {
    let count = slot_count(draft)?;
    for key in draft.keys().filter(|key| key.starts_with("slot_")) {
        if !value(draft, key).is_empty() && !(1..=count).any(|i| key == &format!("slot_{i}")) {
            return Err(invalid(
                key,
                "This slot is outside the displayed roster. Reload the slot count before editing.",
            ));
        }
    }
    let mut result = Vec::new();
    let mut ids = BTreeSet::new();
    for i in 1..=count {
        let key = format!("slot_{i}");
        let raw = value(draft, &key);
        if raw.is_empty() {
            continue;
        }
        let (type_id, version) = pin(raw, &key)?;
        if !ids.insert(type_id.clone()) {
            return Err(invalid(
                &key,
                "Choose each canonical Type identity only once, even across versions.",
            ));
        }
        result.push(TypePin { type_id, version });
    }
    Ok(result)
}
fn mapping(draft: &Values) -> Vec<ChartMapping> {
    draft
        .iter()
        .filter_map(|(name, local)| {
            let source = name.strip_prefix("map_")?;
            if local.is_empty() {
                return None;
            }
            Some(ChartMapping {
                source_type_id: source.into(),
                local_type_id: local.clone(),
            })
        })
        .collect()
}
fn apply_source(draft: &Values) -> Result<bool, Error> {
    match value(draft, "policy") {
        "keep" => Ok(false),
        "apply" => Ok(true),
        _ => Err(invalid(
            "policy",
            "Explicitly choose Keep local cells or Apply source to mapped pairs.",
        )),
    }
}
fn new_command(draft: &Values, local: bool) -> Result<NewChart, Error> {
    Ok(NewChart {
        fields: fields(draft),
        roster: roster(draft)?,
        source: source(draft)?,
        mapping: mapping(draft),
        apply_source: if local { apply_source(draft)? } else { false },
        reason: reason(draft),
    })
}
fn adopt_command(draft: &Values) -> Result<AdoptEdit, Error> {
    Ok(AdoptEdit {
        expected_revision: revision(draft)?,
        target_version: positive(value(draft, "target_version"), "target_version")?,
        mapping: mapping(draft),
        apply_source: apply_source(draft)?,
        reason: reason(draft),
    })
}

fn row_cells(entry: &ChartRecord, attacker: &str, draft: &Values) -> Result<Vec<ChartCell>, Error> {
    if !entry.roster.iter().any(|a| a.pin.type_id == attacker) {
        return Err(Error::NotFound);
    }
    let mut errors = Values::new();
    let mut cells = Vec::new();
    for key in draft.keys() {
        if let Some(id) = key
            .strip_prefix("numerator_")
            .or_else(|| key.strip_prefix("denominator_"))
            && !entry.roster.iter().any(|a| a.pin.type_id == id)
            && !value(draft, key).is_empty()
        {
            errors.insert(
                key.clone(),
                "This defender is outside the current roster. Review the current saved chart and clear both obsolete fields."
                    .into(),
            );
        }
    }
    for defender in &entry.roster {
        let id = &defender.pin.type_id;
        let nkey = format!("numerator_{id}");
        let dkey = format!("denominator_{id}");
        let n = value(draft, &nkey);
        let d = value(draft, &dkey);
        if n.is_empty() && d.is_empty() {
            continue;
        }
        let parse = |raw: &str| raw.parse::<u32>().ok().filter(|n| *n <= 1_000_000);
        let numerator = parse(n);
        let denominator = parse(d).filter(|n| *n > 0);
        if numerator.is_none() {
            errors.insert(
                nkey.clone(),
                "Enter a whole numerator from 0 to 1000000; leave both fields blank for Unknown."
                    .into(),
            );
        }
        if denominator.is_none() {
            errors.insert(
                dkey.clone(),
                "Enter a whole denominator from 1 to 1000000; leave both fields blank for Unknown."
                    .into(),
            );
        }
        if let (Some(n), Some(d)) = (numerator, denominator) {
            let factor = Factor::new(n, d).map_err(|text| invalid(&nkey, &text))?;
            cells.push(ChartCell {
                attacker_type_id: attacker.into(),
                defender_type_id: id.clone(),
                factor,
            });
        }
    }
    if errors.is_empty() {
        Ok(cells)
    } else {
        Err(Error::Invalid(errors))
    }
}

struct Control {
    name: String,
    label: String,
    value: String,
    kind: &'static str,
    help: String,
    error: String,
    choices: Vec<SourceChoice>,
}
struct Comparison {
    label: &'static str,
    matrix: Matrix,
}

#[derive(Template)]
#[template(path = "chart_form.html")]
struct Editor {
    title: &'static str,
    context: String,
    action: String,
    cancel: String,
    csrf: String,
    draft: Values,
    controls: Vec<Control>,
    hidden: Values,
    error: String,
    errors: Values,
    comparisons: Vec<Comparison>,
    reviewed_revision: String,
    creation: bool,
    adoption: bool,
    definition: bool,
    neutral: bool,
    preview_ready: bool,
    reload_source: bool,
    sources: Vec<SourceChoice>,
}
impl Editor {
    fn control(
        &mut self,
        name: &str,
        label: impl Into<String>,
        kind: &'static str,
        help: &str,
        mut choices: Vec<SourceChoice>,
    ) {
        let raw = value(&self.draft, name).to_owned();
        if kind == "select" && !choices.iter().any(|choice| choice.value == raw) {
            choices.push(SourceChoice {
                value: raw.clone(),
                label: format!("Retained submitted value: {raw}"),
            });
        }
        self.controls.push(Control {
            name: name.into(),
            label: label.into(),
            value: raw,
            kind,
            help: help.into(),
            error: value(&self.errors, name).into(),
            choices,
        });
    }
}

async fn type_choices(
    state: &AppState,
    world: Option<&str>,
    draft: &Values,
    current: Option<&ChartRecord>,
) -> Result<Vec<SourceChoice>, Error> {
    let mut choices = vec![SourceChoice {
        value: String::new(),
        label: "Unused — skip this slot".into(),
    }];
    if let Some(world) = world {
        for ty in state.types.world_types(world).await? {
            choices.push(SourceChoice {
                value: format!("{}@{}", ty.id, ty.revision),
                label: format!(
                    "{} · version {} · {}",
                    ty.fields.display_name, ty.revision, ty.id
                ),
            });
        }
    } else {
        for ty in state.types.types().await? {
            choices.push(SourceChoice {
                value: format!("{}@{}", ty.id, ty.revision),
                label: format!(
                    "{} · version {} · {}",
                    ty.fields.display_name, ty.revision, ty.id
                ),
            });
        }
    }
    if let Some(current) = current {
        for assignment in &current.roster {
            let raw = format!("{}@{}", assignment.pin.type_id, assignment.pin.version);
            if !choices.iter().any(|c| c.value == raw) {
                choices.push(SourceChoice {
                    value: raw,
                    label: format!(
                        "{} · retained version {} · {}",
                        assignment.fields.display_name,
                        assignment.pin.version,
                        assignment.pin.type_id
                    ),
                });
            }
        }
    }
    for (key, raw) in draft.iter().filter(|(key, _)| key.starts_with("slot_")) {
        if raw.is_empty() || choices.iter().any(|c| &c.value == raw) {
            continue;
        }
        if let Ok((id, version)) = pin(raw, key) {
            let metadata = if let Some(world) = world {
                state
                    .types
                    .world_type_version(world, &id, version)
                    .await
                    .map(|t| t.fields)
            } else {
                state
                    .types
                    .type_version(&id, version)
                    .await
                    .map(|t| t.fields)
            };
            if let Ok(fields) = metadata {
                choices.push(SourceChoice {
                    value: raw.clone(),
                    label: format!(
                        "{} · historical version {version} · {id}",
                        fields.display_name
                    ),
                });
            }
        }
    }
    Ok(choices)
}

fn saved_draft(entry: &ChartRecord, ctx: &Context) -> Values {
    let mut draft = Values::from([("expected_revision".into(), entry.revision.to_string())]);
    if ctx.mode == Mode::Definition {
        draft.insert("display_name".into(), entry.fields.display_name.clone());
        draft.insert("description".into(), entry.fields.description.clone());
        draft.insert("slots".into(), entry.roster.len().max(3).to_string());
        for (i, assignment) in entry.roster.iter().enumerate() {
            draft.insert(
                format!("slot_{}", i + 1),
                format!("{}@{}", assignment.pin.type_id, assignment.pin.version),
            );
        }
    }
    if matches!(ctx.mode, Mode::Definition | Mode::Adopt) {
        if let Some(pin) = &entry.source {
            draft.insert(
                "source_pin".into(),
                format!("{}@{}", pin.chart_id, pin.version),
            );
        }
        for map in &entry.mapping {
            draft.insert(
                format!("map_{}", map.source_type_id),
                map.local_type_id.clone(),
            );
        }
    }
    if ctx.mode == Mode::Adopt {
        draft.insert("policy".into(), "keep".into());
    }
    if ctx.mode == Mode::Row {
        for cell in entry
            .cells
            .iter()
            .filter(|cell| cell.attacker_type_id == ctx.attacker)
        {
            draft.insert(
                format!("numerator_{}", cell.defender_type_id),
                cell.factor.numerator.to_string(),
            );
            draft.insert(
                format!("denominator_{}", cell.defender_type_id),
                cell.factor.denominator.to_string(),
            );
        }
    }
    draft
}

async fn make_editor(
    state: &AppState,
    ctx: &Context,
    draft: Values,
    current: Option<&ChartRecord>,
    failure: Option<&Error>,
) -> Editor {
    let mut page = Editor {
        title: match ctx.mode {
            Mode::New => "Create effectiveness chart",
            Mode::Definition => "Edit chart definition",
            Mode::Row => "Edit attacking row",
            Mode::Neutral => "Fill remaining Unknown cells with neutral",
            Mode::Adopt => "Review chart source adoption",
        },
        context: if ctx.mode == Mode::Row {
            let name = current
                .and_then(|c| c.roster.iter().find(|a| a.pin.type_id == ctx.attacker))
                .map(|a| a.fields.display_name.as_str())
                .unwrap_or("Type no longer in roster");
            format!(
                "Attacker: {name} · canonical Type ID: {}. This form replaces only this attacking row; omitted defenders become Unknown.",
                ctx.attacker
            )
        } else {
            String::new()
        },
        action: ctx.action(),
        cancel: ctx.cancel(),
        csrf: String::new(),
        draft,
        controls: Vec::new(),
        hidden: Values::new(),
        error: failure.map(error_text).unwrap_or("").into(),
        errors: match failure {
            Some(Error::Invalid(errors)) => errors.clone(),
            _ => Values::new(),
        },
        comparisons: Vec::new(),
        reviewed_revision: String::new(),
        creation: ctx.mode == Mode::New,
        adoption: ctx.mode == Mode::Adopt,
        definition: matches!(ctx.mode, Mode::New | Mode::Definition),
        neutral: ctx.mode == Mode::Neutral,
        preview_ready: false,
        reload_source: ctx.world.is_some() && ctx.mode == Mode::New,
        sources: Vec::new(),
    };
    if let Some(current) = current {
        page.comparisons.push(Comparison {
            label: "Current saved state",
            matrix: Matrix::new(current.clone(), ""),
        });
        if matches!(failure, Some(Error::Conflict)) {
            page.reviewed_revision = current.revision.to_string();
        }
    }
    if page.definition {
        page.control(
            "display_name",
            "Display name",
            "text",
            "Required: 1–100 characters. The readable slug is generated automatically from this name.",
            Vec::new(),
        );
        page.control(
            "description",
            "Description",
            "textarea",
            "Required: 1–2000 characters of plain text chart metadata.",
            Vec::new(),
        );
        let choices = type_choices(state, ctx.world.as_deref(), &page.draft, current)
            .await
            .unwrap_or_default();
        let count = slot_count(&page.draft).unwrap_or(32);
        for i in 1..=count {
            page.control(&format!("slot_{i}"), format!("Type slot {i}"), "select", "Exact metadata version. Each canonical identity may appear only once. Blank slots are skipped.", choices.iter().map(|c| SourceChoice { value: c.value.clone(), label: c.label.clone() }).collect());
        }
        page.control("slots", "Displayed slots", "numeric", "0–32 slots. Use the separate reload form before editing; changing this value alone does not add controls.", Vec::new());
    }
    if page.reload_source {
        page.sources.push(SourceChoice {
            value: String::new(),
            label: "World-exclusive — no source".into(),
        });
        if let Ok(charts) = state.charts.charts().await {
            page.sources
                .extend(charts.into_iter().map(|c| SourceChoice {
                    value: format!("{}@{}", c.id, c.revision),
                    label: format!(
                        "{} · version {} · {}",
                        c.fields.display_name, c.revision, c.id
                    ),
                }));
        }
        let raw = value(&page.draft, "source_pin");
        if !page.sources.iter().any(|c| c.value == raw) {
            page.sources.push(SourceChoice {
                value: raw.into(),
                label: format!("Retained exact source: {raw}"),
            });
        }
        // The selected source is immutable within this draft. Reloading is explicit.
        page.hidden.insert("source_pin".into(), raw.into());
    }
    let source_record = if ctx.mode == Mode::Adopt {
        match (
            current.and_then(|c| c.source.as_ref()),
            positive(value(&page.draft, "target_version"), "target_version"),
        ) {
            (Some(pin), Ok(version)) => state
                .charts
                .version(None, &pin.chart_id, version)
                .await
                .ok(),
            _ => None,
        }
    } else {
        match source(&page.draft) {
            Ok(Some(pin)) => state
                .charts
                .version(None, &pin.chart_id, pin.version)
                .await
                .ok(),
            _ => None,
        }
    };
    if ctx.world.is_some() && matches!(ctx.mode, Mode::New | Mode::Definition | Mode::Adopt) {
        if let Some(source) = &source_record {
            let local_choices = if ctx.mode == Mode::Adopt {
                current
                    .map(|c| {
                        c.roster
                            .iter()
                            .map(|a| SourceChoice {
                                value: a.pin.type_id.clone(),
                                label: format!("{} · {}", a.fields.display_name, a.pin.type_id),
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default()
            } else {
                choices_for_mapping(state, ctx.world.as_deref(), &page.draft).await
            };
            for member in &source.roster {
                let mut choices = vec![SourceChoice {
                    value: String::new(),
                    label: "Unmapped — stay local".into(),
                }];
                choices.extend(local_choices.iter().map(|c| SourceChoice {
                    value: c.value.clone(),
                    label: c.label.clone(),
                }));
                page.control(&format!("map_{}", member.pin.type_id), format!("Map source {} ({}) to local Type", member.fields.display_name, member.pin.type_id), "select", "Explicit mapping only. A local Type must have this canonical shared Type as its own source; metadata versions may differ. Each side must be unique.", choices);
            }
        }
        if ctx.mode != Mode::Definition {
            page.control("policy", "Source cell policy", "select", "Apply copies every mapped directed pair, including source Unknown which removes the local known value. Unmapped pairs stay local. No mappings are inferred.", vec![SourceChoice { value: "keep".into(), label: "Keep all local cells".into() }, SourceChoice { value: "apply".into(), label: "Apply source to mapped pairs".into() }]);
        }
    }
    if ctx.mode == Mode::Adopt {
        page.control("target_version", "New shared chart version", "numeric", "A newer exact version of the same source chart. Preview after changing this value or any mapping/policy.", Vec::new());
    }
    if ctx.mode == Mode::Row
        && let Some(entry) = current
    {
        for defender in &entry.roster {
            for (prefix, label) in [("numerator", "Numerator"), ("denominator", "Denominator")] {
                page.control(&format!("{prefix}_{}", defender.pin.type_id), format!("{label} against {} ({})", defender.fields.display_name, defender.pin.type_id), "numeric", "Both blank = Unknown. Numerator 0–1000000; denominator 1–1000000. 0/1 = No effect; 1/1 = Neutral.", Vec::new());
            }
        }
    }
    page.control(
        "reason",
        "Change reason (optional)",
        "text",
        "At most 500 characters, one line. Recorded in public history.",
        Vec::new(),
    );
    // Retain even malformed or removed slot/mapping/cell fields as editable strings.
    let extras = page
        .draft
        .keys()
        .filter(|key| {
            key.starts_with("slot_")
                || key.starts_with("map_")
                || key.starts_with("numerator_")
                || key.starts_with("denominator_")
        })
        .filter(|key| !page.controls.iter().any(|c| &c.name == *key))
        .cloned()
        .collect::<Vec<_>>();
    for key in extras {
        page.control(&key, format!("Retained submitted field: {key}"), "text", "This field is retained for review. Clear obsolete slot, mapping or cell fields before resubmitting; reload before expanding the roster.", Vec::new());
    }
    if let Some(raw) = page.draft.get("expected_revision") {
        page.hidden.insert("expected_revision".into(), raw.clone());
    }
    if ctx.mode == Mode::Definition
        && let Some(raw) = page.draft.get("source_pin")
    {
        page.hidden.insert("source_pin".into(), raw.clone());
    }
    page
}

async fn choices_for_mapping(
    state: &AppState,
    world: Option<&str>,
    draft: &Values,
) -> Vec<SourceChoice> {
    let mut choices = Vec::new();
    // Include the selected roster as well as available local identities: selecting
    // a slot and mapping it in the same native submission needs no JS/reload.
    if let Some(world) = world
        && let Ok(types) = state.types.world_types(world).await
    {
        choices.extend(types.into_iter().map(|t| SourceChoice {
            value: t.id.clone(),
            label: format!("{} · {}", t.fields.display_name, t.id),
        }));
    }
    if let Ok(pins) = roster(draft) {
        for pin in pins {
            if !choices.iter().any(|c| c.value == pin.type_id) {
                choices.push(SourceChoice {
                    value: pin.type_id.clone(),
                    label: format!("Retained local identity {}", pin.type_id),
                });
            }
        }
    }
    choices
}

fn editor_response(
    state: &AppState,
    headers: &HeaderMap,
    mut page: Editor,
    status: StatusCode,
) -> Response {
    // GET preview forms never contain CSRF/session tokens in their query strings.
    let (csrf, set) = match csrf_for(headers) {
        Ok(value) => value,
        Err(_) => {
            page.preview_ready = false;
            page.error = error_text(&Error::Unavailable).into();
            return no_store(render_status(page, StatusCode::SERVICE_UNAVAILABLE));
        }
    };
    page.csrf = csrf.clone();
    let mut response = no_store(render_status(page, status));
    if set {
        append_cookie(&mut response, csrf_cookie(&csrf, state.cookie_secure));
    }
    response
}

async fn shared_new(
    State(state): State<AppState>,
    headers: HeaderMap,
    query: QueryInput,
) -> Response {
    editor_get(
        state,
        headers,
        Context::from_path(Values::new(), "/charts/new"),
        query,
    )
    .await
}
async fn shared_create(
    State(state): State<AppState>,
    headers: HeaderMap,
    form: FormInput,
) -> Response {
    editor_submit(
        state,
        headers,
        Context::from_path(Values::new(), "/charts/new"),
        form,
    )
    .await
}
async fn editor(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(path): Path<Values>,
    uri: Uri,
    query: QueryInput,
) -> Response {
    editor_get(state, headers, Context::from_path(path, uri.path()), query).await
}
async fn submit(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(path): Path<Values>,
    uri: Uri,
    form: FormInput,
) -> Response {
    editor_submit(state, headers, Context::from_path(path, uri.path()), form).await
}

async fn editor_get(
    state: AppState,
    headers: HeaderMap,
    ctx: Context,
    query: QueryInput,
) -> Response {
    no_store(finish(
        async {
            ctx.permission(&state, &headers).await?;
            let Query(query) = query
                .map_err(|_| invalid("query", "Check the chart selection and preview fields."))?;
            let current = ctx.current(&state).await?;
            if ctx.mode == Mode::Row
                && !current
                    .as_ref()
                    .is_some_and(|c| c.roster.iter().any(|a| a.pin.type_id == ctx.attacker))
            {
                return Err(Error::NotFound);
            }
            let mut draft = current
                .as_ref()
                .map(|c| saved_draft(c, &ctx))
                .unwrap_or_else(|| {
                    Values::from([
                        ("slots".into(), "3".into()),
                        ("policy".into(), "keep".into()),
                    ])
                });
            // Definition reloads deliberately start from saved content; adoption GET
            // carries the entire draft so preview never drops mapping/policy/reason.
            if ctx.mode == Mode::Adopt {
                draft.extend(query.clone());
                if let Some(reviewed) = draft.remove("reviewed_revision").filter(|s| !s.is_empty())
                {
                    draft.insert("expected_revision".into(), reviewed);
                }
            } else {
                for key in ["slots", "source_pin"] {
                    if let Some(raw) = query.get(key) {
                        draft.insert(key.into(), raw.clone());
                    }
                }
            }
            let validation = if matches!(ctx.mode, Mode::New | Mode::Definition) {
                slot_count(&draft).err()
            } else {
                None
            };
            let mut status = validation
                .as_ref()
                .map(error_status)
                .unwrap_or(StatusCode::OK);
            let mut page =
                make_editor(&state, &ctx, draft, current.as_ref(), validation.as_ref()).await;
            if ctx.mode == Mode::Adopt && value(&query, "preview") == "yes" {
                match adoption_comparisons(&state, &ctx, &page.draft).await {
                    Ok(comparisons) => {
                        page.comparisons = comparisons;
                        page.preview_ready = true;
                    }
                    Err(error) => {
                        status = error_status(&error);
                        page =
                            make_editor(&state, &ctx, page.draft, current.as_ref(), Some(&error))
                                .await;
                    }
                }
            }
            Ok(editor_response(&state, &headers, page, status))
        }
        .await,
    ))
}

async fn adoption_comparisons(
    state: &AppState,
    ctx: &Context,
    draft: &Values,
) -> Result<Vec<Comparison>, Error> {
    let preview = state
        .charts
        .adoption_preview(
            ctx.world.as_deref().ok_or(Error::NotFound)?,
            ctx.id.as_deref().ok_or(Error::NotFound)?,
            &adopt_command(draft)?,
        )
        .await?;
    Ok(vec![
        Comparison {
            label: "Old shared source",
            matrix: Matrix::new(preview.old_source, ""),
        },
        Comparison {
            label: "Current local effective chart",
            matrix: Matrix::new(preview.entry, ""),
        },
        Comparison {
            label: "New shared source",
            matrix: Matrix::new(preview.new_source, ""),
        },
        Comparison {
            label: "Proposed result",
            matrix: Matrix::new(preview.proposed, ""),
        },
    ])
}

async fn editor_submit(
    state: AppState,
    headers: HeaderMap,
    ctx: Context,
    form: FormInput,
) -> Response {
    let mut draft = match form {
        Ok(Form(values)) => values,
        Err(error) => return authoring_form_error(error),
    };
    if let Some(reviewed) = draft.remove("reviewed_revision").filter(|s| !s.is_empty()) {
        draft.insert("expected_revision".into(), reviewed);
    }
    let result: Result<Option<ChartRecord>, Error> = async {
        let token = submit_token(&state, &headers, value(&draft, "csrf"))?;
        ctx.permission(&state, &headers).await?;
        // Check the aggregate base before parsing dependent roster/cell fields.
        let current = ctx.current(&state).await?;
        if let Some(current) = &current
            && revision(&draft)? != current.revision
        {
            return Err(Error::Conflict);
        }
        let world = ctx.world.as_deref();
        let id = ctx.id.as_deref().unwrap_or("");
        match ctx.mode {
            Mode::New => {
                let command = new_command(&draft, world.is_some())?;
                let proposed = state.charts.preview_create(world, &command).await?;
                if value(&draft, "intent") != "create" {
                    return Ok(Some(proposed));
                }
                if value(&draft, "confirm") != "yes" {
                    return Err(invalid(
                        "confirm",
                        "Review the proposed chart and confirm creation.",
                    ));
                }
                let entry = state.charts.create(&token, world, command).await?;
                Ok(Some(entry))
            }
            Mode::Definition => {
                state
                    .charts
                    .save_definition(
                        &token,
                        world,
                        id,
                        DefinitionEdit {
                            expected_revision: revision(&draft)?,
                            fields: fields(&draft),
                            roster: roster(&draft)?,
                            mapping: mapping(&draft),
                            reason: reason(&draft),
                        },
                    )
                    .await?;
                Ok(None)
            }
            Mode::Row => {
                let entry = current.ok_or(Error::NotFound)?;
                state
                    .charts
                    .save_row(
                        &token,
                        world,
                        id,
                        RowEdit {
                            expected_revision: revision(&draft)?,
                            attacker_type_id: ctx.attacker.clone(),
                            cells: row_cells(&entry, &ctx.attacker, &draft)?,
                            reason: reason(&draft),
                        },
                    )
                    .await?;
                Ok(None)
            }
            Mode::Neutral => {
                if value(&draft, "confirm") != "yes" {
                    return Err(invalid(
                        "confirm",
                        "Confirm filling every remaining Unknown directed pair with 1/1 Neutral.",
                    ));
                }
                state
                    .charts
                    .fill_neutral(
                        &token,
                        world,
                        id,
                        NeutralEdit {
                            expected_revision: revision(&draft)?,
                            reason: reason(&draft),
                        },
                    )
                    .await?;
                Ok(None)
            }
            Mode::Adopt => {
                if value(&draft, "confirm") != "yes" {
                    return Err(invalid(
                        "confirm",
                        "Preview and confirm the selected mappings, policy and proposed result.",
                    ));
                }
                state
                    .charts
                    .adopt(
                        &token,
                        world.ok_or(Error::NotFound)?,
                        id,
                        adopt_command(&draft)?,
                    )
                    .await?;
                Ok(None)
            }
        }
    }
    .await;
    // Tokens are never retained in visible draft diagnostics or GET preview data.
    draft.remove("csrf");
    draft.remove("confirm");
    match result {
        Ok(Some(entry)) if ctx.mode == Mode::New && value(&draft, "intent") != "create" => {
            let mut page = make_editor(&state, &ctx, draft, None, None).await;
            page.preview_ready = true;
            page.comparisons.push(Comparison {
                label: "Proposed result",
                matrix: Matrix::creation_preview(entry),
            });
            editor_response(&state, &headers, page, StatusCode::OK)
        }
        Ok(entry) => {
            let url = entry
                .map(|c| format!("{}/{}", base(ctx.world.as_deref()), c.id))
                .unwrap_or_else(|| ctx.cancel());
            no_store(Redirect::to(&url).into_response())
        }
        Err(error) => {
            let current = ctx.current(&state).await;
            let mut page = make_editor(
                &state,
                &ctx,
                draft,
                current.as_ref().ok().and_then(Option::as_ref),
                Some(&error),
            )
            .await;
            if matches!(error, Error::Conflict) && current.is_err() {
                page.error = format!(
                    "{} The newer saved chart could not be loaded. Keep this draft and reload before reviewing.",
                    error_text(&Error::Unavailable)
                );
                return editor_response(&state, &headers, page, StatusCode::SERVICE_UNAVAILABLE);
            }
            // Adoption errors retain the draft in a GET preview form; a fresh
            // preview against the explicitly reviewed revision precedes any POST.
            editor_response(&state, &headers, page, error_status(&error))
        }
    }
}

struct Change {
    label: String,
    before: String,
    after: String,
}
struct HistoryView {
    entry: HistoryEntry,
    changes: Vec<Change>,
}

fn roster_text(entry: &ChartRecord) -> String {
    entry
        .roster
        .iter()
        .map(|a| {
            format!(
                "{} · {}@{}",
                a.fields.display_name, a.pin.type_id, a.pin.version
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}
fn mapping_text(entry: &ChartRecord) -> String {
    entry
        .mapping
        .iter()
        .map(|m| format!("{} → {}", m.source_type_id, m.local_type_id))
        .collect::<Vec<_>>()
        .join("\n")
}
fn source_text(entry: &ChartRecord) -> String {
    entry
        .source
        .as_ref()
        .map(|p| format!("{}@{}", p.chart_id, p.version))
        .unwrap_or_else(|| "No shared source".into())
}
fn changes(old: Option<&ChartRecord>, new: &ChartRecord) -> Vec<Change> {
    let mut changes = Vec::new();
    let mut add = |label: &str, before: String, after: String| {
        if before != after {
            changes.push(Change {
                label: label.into(),
                before,
                after,
            });
        }
    };
    add(
        "Display name",
        old.map(|c| c.fields.display_name.clone())
            .unwrap_or_default(),
        new.fields.display_name.clone(),
    );
    add(
        "Description",
        old.map(|c| c.fields.description.clone())
            .unwrap_or_default(),
        new.fields.description.clone(),
    );
    add(
        "Slug",
        old.map(|c| c.fields.slug.clone()).unwrap_or_default(),
        new.fields.slug.clone(),
    );
    add(
        "Ordered exact roster",
        old.map(roster_text).unwrap_or_default(),
        roster_text(new),
    );
    add(
        "Shared source",
        old.map(source_text).unwrap_or_default(),
        source_text(new),
    );
    add(
        "Explicit mappings",
        old.map(mapping_text).unwrap_or_default(),
        mapping_text(new),
    );
    let mut identities = BTreeMap::new();
    for entry in old.into_iter().chain(std::iter::once(new)) {
        for a in &entry.roster {
            identities.insert(a.pin.type_id.clone(), a.fields.display_name.clone());
        }
    }
    for (attacker, name) in &identities {
        for (defender, defender_name) in &identities {
            add(
                &format!("{name} → {defender_name} ({attacker} → {defender})"),
                old.map(|c| factor_text(c, attacker, defender))
                    .unwrap_or_else(|| "Not covered (outside roster)".into()),
                factor_text(new, attacker, defender),
            );
        }
    }
    changes
}

#[derive(Template)]
#[template(path = "chart_history.html")]
struct History {
    url: String,
    entries: Vec<HistoryView>,
    next: Option<i64>,
}

async fn history(
    State(state): State<AppState>,
    Path(path): Path<Values>,
    query: QueryInput,
) -> Response {
    finish(
        async {
            let Query(query) =
                query.map_err(|_| invalid("before", "Use a positive revision cursor."))?;
            let before = if value(&query, "before").is_empty() {
                None
            } else {
                Some(positive(value(&query, "before"), "before")?)
            };
            let world = path.get("world_id").map(String::as_str);
            let id = value(&path, "id");
            let current = state.charts.detail(world, id).await?;
            let url = pretty(&state, &current).await?;
            let history = state.charts.history(world, id, before).await?;
            let next = if history.len() == 50 {
                history.last().map(|e| e.revision)
            } else {
                None
            };
            let mut entries = Vec::new();
            for entry in history {
                let new = state.charts.version(world, id, entry.revision).await?;
                let old = match entry.previous_revision {
                    Some(version) => Some(state.charts.version(world, id, version).await?),
                    None => None,
                };
                entries.push(HistoryView {
                    changes: changes(old.as_ref(), &new),
                    entry,
                });
            }
            Ok(render(History { url, entries, next }))
        }
        .await,
    )
}

#[derive(Template)]
#[template(path = "chart_snapshot.html")]
struct Snapshot {
    url: String,
    matrix: Matrix,
    view: HistoryView,
}

async fn snapshot(State(state): State<AppState>, Path(path): Path<Values>) -> Response {
    finish(
        async {
            let world = path.get("world_id").map(String::as_str);
            let id = value(&path, "id");
            let version = positive(value(&path, "revision"), "revision")?;
            let current = state.charts.detail(world, id).await?;
            let url = pretty(&state, &current).await?;
            let entry = state.charts.history_entry(world, id, version).await?;
            let new = state.charts.version(world, id, version).await?;
            let old = match entry.previous_revision {
                Some(version) => Some(state.charts.version(world, id, version).await?),
                None => None,
            };
            let view = HistoryView {
                changes: changes(old.as_ref(), &new),
                entry,
            };
            Ok(render(Snapshot {
                url,
                matrix: Matrix::new(new, ""),
                view,
            }))
        }
        .await,
    )
}
