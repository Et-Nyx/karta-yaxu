//! Native learning authoring. Services own scope, validation and transactional writes.
use super::authoring::{
    authoring_form_error, error_status, error_text, finish, invalid, library_permission, no_store,
    positive, submit_token, world_permission,
};
use super::*;
use crate::application::{authoring::Error, learning::*};
use axum::{extract::rejection::QueryRejection, http::Uri};
use game_data::{moves::Decimal, rulesets::RulesetPin};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

type Values = BTreeMap<String, String>;
type FormInput = Result<Form<Values>, FormRejection>;
type QueryInput = Result<Query<Values>, QueryRejection>;
const PAGE_SIZE: usize = 25;

pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/forms/{id}/learnset", get(page).post(submit))
        .route("/forms/{id}/learnset/edit", get(page).post(submit))
        .route("/forms/{id}/learnset/history", get(history))
        .route("/forms/{id}/learnset/history/{revision}", get(history))
        .route(
            "/worlds/{world_id}/variants/{variant_id}/learnset",
            get(page).post(submit),
        )
        .route(
            "/worlds/{world_id}/variants/{variant_id}/learnset/edit",
            get(page).post(submit),
        )
        .route(
            "/worlds/{world_id}/variants/{variant_id}/learnset/adopt",
            get(page).post(submit),
        )
        .route(
            "/worlds/{world_id}/variants/{variant_id}/learnset/history",
            get(history),
        )
        .route(
            "/worlds/{world_id}/variants/{variant_id}/learnset/history/{revision}",
            get(history),
        )
        .route(
            "/worlds/{world_id}/variants/{variant_id}/movesets",
            get(page),
        )
        .route(
            "/worlds/{world_id}/variants/{variant_id}/movesets/new",
            get(page).post(submit),
        )
        .route(
            "/worlds/{world_id}/variants/{variant_id}/movesets/{id}",
            get(page),
        )
        .route(
            "/worlds/{world_id}/variants/{variant_id}/movesets/{id}/edit",
            get(page).post(submit),
        )
        .route(
            "/worlds/{world_id}/variants/{variant_id}/movesets/{id}/history",
            get(history),
        )
        .route(
            "/worlds/{world_id}/variants/{variant_id}/movesets/{id}/history/{revision}",
            get(history),
        )
        .route(
            "/worlds/{world_id}/variants/{variant_id}/movesets/{id}/validation",
            get(page),
        )
        .layer(DefaultBodyLimit::max(32 * 1024))
}
fn val<'a>(v: &'a Values, key: &str) -> &'a str {
    v.get(key).map(String::as_str).unwrap_or("")
}
fn reason(v: &Values) -> Option<String> {
    v.get("reason").filter(|s| !s.is_empty()).cloned()
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
fn expected(v: &Values) -> Result<i64, Error> {
    val(v, "expected_revision")
        .parse::<i64>()
        .ok()
        .filter(|n| *n >= 0)
        .ok_or_else(|| {
            invalid(
                "expected_revision",
                "Use the saved aggregate revision, or zero for new data.",
            )
        })
}
fn exact(raw: &str, field: &str) -> Result<(String, i64), Error> {
    let (id, version) = raw
        .rsplit_once('@')
        .ok_or_else(|| invalid(field, "Use canonical identity@exact-version."))?;
    if !game_data::types::valid_id(id) {
        return Err(invalid(
            field,
            "Use a canonical identity and exact positive revision.",
        ));
    }
    Ok((id.into(), positive(version, field)?))
}
fn move_pin(raw: &str, field: &str) -> Result<MovePin, Error> {
    let (move_id, version) = exact(raw, field)?;
    Ok(MovePin { move_id, version })
}
fn learnset_pin(raw: &str, field: &str) -> Result<LearnsetPin, Error> {
    let (learnset_id, version) = exact(raw, field)?;
    Ok(LearnsetPin {
        learnset_id,
        version,
    })
}
fn pin_text(id: &str, version: i64) -> String {
    format!("{id}@{version}")
}

#[derive(Clone)]
struct Context {
    world: Option<String>,
    owner: String,
    preset: Option<String>,
    presets: bool,
    new: bool,
    edit: bool,
    adopt: bool,
}
impl Context {
    fn new(path: &Values, uri: &str) -> Self {
        let world = path.get("world_id").cloned();
        let presets = uri.contains("/movesets");
        Self {
            owner: if world.is_some() {
                val(path, "variant_id")
            } else {
                val(path, "id")
            }
            .into(),
            world,
            preset: presets.then(|| path.get("id").cloned()).flatten(),
            presets,
            new: uri.ends_with("/new"),
            edit: uri.ends_with("/edit"),
            adopt: uri.ends_with("/adopt"),
        }
    }
    fn world(&self) -> Option<&str> {
        self.world.as_deref()
    }
    fn local(&self) -> Result<&str, Error> {
        self.world().ok_or(Error::NotFound)
    }
    fn owner_url(&self) -> String {
        self.world().map_or_else(
            || format!("/forms/{}", self.owner),
            |w| format!("/worlds/{w}/variants/{}", self.owner),
        )
    }
    fn root(&self) -> String {
        format!(
            "{}/{}",
            self.owner_url(),
            if self.presets { "movesets" } else { "learnset" }
        )
    }
    fn target(&self) -> String {
        self.preset
            .as_ref()
            .map_or_else(|| self.root(), |id| format!("{}/{id}", self.root()))
    }
    fn action(&self) -> String {
        format!(
            "{}{}",
            self.target(),
            if self.new {
                "/new"
            } else if self.adopt {
                "/adopt"
            } else {
                "/edit"
            }
        )
    }
    async fn permission(&self, state: &AppState, headers: &HeaderMap) -> Result<(), Error> {
        if self.world.is_some() {
            world_permission(state, headers, self.world()).await
        } else {
            library_permission(state, headers).await
        }
    }
    async fn owner_name(&self, state: &AppState) -> Result<String, Error> {
        if let Some(w) = self.world() {
            Ok(state
                .authoring
                .variant_detail(w, &self.owner)
                .await?
                .fields
                .display_name)
        } else {
            Ok(state
                .authoring
                .form_detail(&self.owner)
                .await?
                .fields
                .display_name)
        }
    }
    async fn saved(&self, state: &AppState) -> Result<Saved, Error> {
        self.owner_name(state).await?;
        if self.presets {
            Ok(Saved::Preset(if let Some(id) = &self.preset {
                Some(
                    state
                        .learning
                        .preset(self.local()?, &self.owner, id)
                        .await?,
                )
            } else {
                None
            }))
        } else {
            Ok(Saved::Learnset(
                state.learning.learnset(self.world(), &self.owner).await?,
            ))
        }
    }
}
enum Saved {
    Learnset(Option<LearnsetRecord>),
    Preset(Option<MovesetRecord>),
}
impl Saved {
    fn revision(&self) -> i64 {
        match self {
            Self::Learnset(r) => r.as_ref().map_or(0, |r| r.revision),
            Self::Preset(r) => r.as_ref().map_or(0, |r| r.revision),
        }
    }
    fn learnset(&self) -> Option<&LearnsetRecord> {
        if let Self::Learnset(r) = self {
            r.as_ref()
        } else {
            None
        }
    }
    fn preset(&self) -> Option<&MovesetRecord> {
        if let Self::Preset(r) = self {
            r.as_ref()
        } else {
            None
        }
    }
}
struct Link {
    label: String,
    url: String,
}
struct Panel {
    title: String,
    lines: Vec<String>,
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
#[template(path = "learning_page.html")]
struct Page {
    title: String,
    action: String,
    cancel: String,
    csrf: String,
    controls: Vec<Control>,
    hidden: Values,
    panels: Vec<Panel>,
    links: Vec<Link>,
    record_buttons: Vec<Link>,
    errors: Values,
    error: String,
    notice: String,
    editable: bool,
    preview: bool,
    fingerprint: String,
    reviewed: String,
    draft: Values,
    baseline: String,
    previous: bool,
    next: bool,
}
impl Page {
    fn new(ctx: &Context, draft: Values, failure: Option<&Error>) -> Self {
        Self {
            title: if ctx.presets {
                "Moveset presets"
            } else {
                "Learnset"
            }
            .into(),
            action: ctx.action(),
            cancel: ctx.target(),
            csrf: String::new(),
            controls: vec![],
            hidden: Values::new(),
            panels: vec![],
            links: vec![],
            record_buttons: vec![],
            errors: match failure {
                Some(Error::Invalid(e)) => e.clone(),
                _ => Values::new(),
            },
            error: failure.map(error_text).unwrap_or("").into(),
            notice: String::new(),
            editable: false,
            preview: false,
            fingerprint: String::new(),
            reviewed: String::new(),
            draft,
            baseline: String::new(),
            previous: false,
            next: false,
        }
    }
    fn panel(&mut self, title: impl Into<String>, lines: Vec<String>) {
        self.panels.push(Panel {
            title: title.into(),
            lines,
        });
    }
    fn link(&mut self, label: impl Into<String>, url: impl Into<String>) {
        self.links.push(Link {
            label: label.into(),
            url: url.into(),
        });
    }
    fn control(
        &mut self,
        name: &str,
        label: &str,
        kind: &'static str,
        help: &str,
        mut choices: Vec<Choice>,
    ) {
        let value = val(&self.draft, name).to_owned();
        if kind == "select" && !choices.iter().any(|c| c.value == value) {
            choices.push(Choice {
                value: value.clone(),
                label: format!("Retained submitted choice: {value}"),
            });
        }
        self.controls.push(Control {
            name: name.into(),
            label: label.into(),
            value,
            kind,
            help: help.into(),
            error: val(&self.errors, name).into(),
            choices,
        });
    }
    fn text(&mut self, name: &str, label: &str, help: &str) {
        self.control(name, label, "text", help, vec![]);
    }
    fn select(&mut self, name: &str, label: &str, choices: &[(&str, &str)]) {
        self.control(
            name,
            label,
            "select",
            "",
            choices
                .iter()
                .map(|(v, l)| Choice {
                    value: (*v).into(),
                    label: (*l).into(),
                })
                .collect(),
        );
    }
}
fn threshold_text(n: &NumericValue) -> String {
    match n {
        NumericValue::Known(v) => format!("Known {v}"),
        NumericValue::Unknown => "Unknown".into(),
        NumericValue::NotApplicable => "Not applicable".into(),
    }
}
fn record_text(r: &LearningRecord, position: usize) -> String {
    format!(
        "{}. Acquisition {} · {} · Move {}@{} · Method: {} · Context: {} · Condition: {} · Threshold: {}",
        position,
        r.id,
        r.move_record.fields.display_name,
        r.move_record.id,
        r.move_record.revision,
        r.method.as_deref().unwrap_or("Unknown"),
        r.context.as_deref().unwrap_or("Unknown"),
        r.condition.as_deref().unwrap_or("Unknown"),
        threshold_text(&r.threshold)
    )
}
fn learnset_lines(row: Option<&LearnsetRecord>, page: usize) -> Vec<String> {
    let Some(row) = row else {
        return vec!["Unconfigured — no learnset has been saved.".into()];
    };
    let mut lines = vec![];
    if row.id == PROVISIONAL_ID {
        lines.push("New learnset proposal — identity will be assigned only on save.".into());
    } else {
        lines.push(format!(
            "Canonical learnset ID: {} · Revision {}",
            row.id, row.revision
        ));
    }
    lines.push(row.source.as_ref().map_or_else(
        || "No shared learnset source. Independent of the form-source pin.".into(),
        |p| {
            format!(
                "Independent shared learnset source: {}@{}",
                p.learnset_id, p.version
            )
        },
    ));
    match &row.records {
        None => lines.push("Unmodeled — acquisition records are Unknown.".into()),
        Some(records) if records.is_empty() => {
            lines.push("Defined-empty — explicitly no acquisition records.".into())
        }
        Some(records) => {
            lines.push(format!("Defined · {} acquisition records · Page {page} (25 per page). Multiple acquisitions of the same Move remain separate.", records.len()));
            lines.extend(
                records
                    .iter()
                    .enumerate()
                    .skip((page - 1) * PAGE_SIZE)
                    .take(PAGE_SIZE)
                    .map(|(i, r)| record_text(r, i + 1)),
            );
        }
    }
    if !row.mappings.is_empty() {
        lines.push(format!(
            "{} explicit Move mappings · Page {page} (25 per page).",
            row.mappings.len()
        ));
    }
    lines.extend(
        row.mappings
            .iter()
            .skip((page - 1) * PAGE_SIZE)
            .take(PAGE_SIZE)
            .map(|m| {
                format!(
                    "Mapping: {} → {}@{}",
                    m.shared_move_id, m.local_move.move_id, m.local_move.version
                )
            }),
    );
    lines
}
fn learnset_length(row: &LearnsetRecord) -> usize {
    row.records
        .as_ref()
        .map_or(0, Vec::len)
        .max(row.mappings.len())
}
fn preset_lines(row: &MovesetRecord, page: usize) -> Vec<String> {
    let mut lines = vec![format!(
        "{} · {}",
        row.fields.display_name, row.fields.description
    )];
    if row.id == PROVISIONAL_ID {
        lines.push("New preset proposal — identity will be assigned only on save.".into());
    } else {
        lines.push(format!(
            "Canonical preset ID: {} · Revision {}",
            row.id, row.revision
        ));
    }
    lines.push(format!(
        "Exact same-owner learnset: {}@{} · Exact same-world ruleset: {}@{} · Validator policy {}",
        row.learnset.learnset_id,
        row.learnset.version,
        row.ruleset.ruleset_id,
        row.ruleset.version,
        row.validator_version
    ));
    lines.push(format!(
        "{} ordered entries · Page {page}. Repeated selections are representable.",
        row.entries.len()
    ));
    lines.extend(
        row.entries
            .iter()
            .enumerate()
            .skip((page - 1) * PAGE_SIZE)
            .take(PAGE_SIZE)
            .map(|(i, id)| format!("Entry {}: acquisition {id}", i + 1)),
    );
    lines
}
fn selected_entries(row: &MovesetRecord, learnset: &LearnsetRecord, number: usize) -> Vec<String> {
    let records: BTreeMap<_, _> = learnset
        .records
        .iter()
        .flatten()
        .map(|r| (r.id.as_str(), r))
        .collect();
    row.entries.iter().enumerate().skip((number-1)*PAGE_SIZE).take(PAGE_SIZE).map(|(i,id)| {
        records.get(id.as_str()).map_or_else(||format!("Entry {}: acquisition {id} is absent from this exact context.",i+1),|r|format!("Entry {}: {} · Acquisition {} · Move {}@{} · Threshold: {} · Method: {} · Context: {} · Condition: {}",i+1,r.move_record.fields.display_name,r.id,r.move_record.id,r.move_record.revision,threshold_text(&r.threshold),r.method.as_deref().unwrap_or("Unknown"),r.context.as_deref().unwrap_or("Unknown"),r.condition.as_deref().unwrap_or("Unknown")))
    }).collect()
}
fn report(page: &mut Page, r: &ValidationReport) {
    page.panel(format!("Validation report · policy {} · not combat readiness", r.validator_version), vec!["Conditions and other acquisition prose are not executed. Well-formed Unknown or contradictory drafts may be saved.".into()]);
    page.panel(
        "Exact validation inputs",
        r.inputs
            .iter()
            .map(|i| {
                if i.id == PROVISIONAL_ID {
                    format!(
                        "{}: new proposal, revision {} (not a persisted identity)",
                        i.kind, i.version
                    )
                } else {
                    format!("{}: {}@{}", i.kind, i.id, i.version)
                }
            })
            .collect(),
    );
    for (title, messages) in [
        ("Contradictions", &r.contradictions),
        ("Incomplete inputs", &r.incomplete),
        ("Checks not evaluated", &r.not_evaluated),
    ] {
        page.panel(
            title,
            if messages.is_empty() {
                vec!["None reported.".into()]
            } else {
                messages.clone()
            },
        );
    }
}
async fn pinned_ruleset(
    page: &mut Page,
    state: &AppState,
    ctx: &Context,
    pin: &RulesetPin,
) -> Result<(), Error> {
    let row = state
        .moves
        .ruleset_version(ctx.local()?, &pin.ruleset_id, pin.version)
        .await?;
    page.panel(
        "Pinned ruleset configuration",
        vec![
            format!(
                "{} · {} · {}@{}",
                row.fields.display_name, row.fields.description, row.id, row.revision
            ),
            format!(
                "Maximum entries: {} · Repeat policy: {}",
                row.max_moves
                    .map_or_else(|| "No configured cap".into(), |n| n.to_string()),
                match row.repeat_policy {
                    game_data::rulesets::RepeatPolicy::Unspecified => "Unspecified",
                    game_data::rulesets::RepeatPolicy::Allow => "Allow",
                    game_data::rulesets::RepeatPolicy::Forbid =>
                        "Forbid canonical Move identity repeats",
                }
            ),
            format!(
                "Attribute expectations: {} · Inert interaction/environment notes: {}",
                row.expectations.as_ref().map_or_else(
                    || "Unmodeled".into(),
                    |e| format!("Defined · {} entries", e.len())
                ),
                row.notes
            ),
        ],
    );
    page.link(
        "Read this exact ruleset revision",
        format!("/worlds/{}/ruleset/history/{}", ctx.local()?, row.revision),
    );
    Ok(())
}
fn draft_digest(v: &Values) -> String {
    let fields: Values = v
        .iter()
        .filter(|(k, _)| {
            !matches!(
                k.as_str(),
                "csrf"
                    | "intent"
                    | "confirm"
                    | "review_fingerprint"
                    | "baseline"
                    | "reviewed_revision"
                    | "load_record"
                    | "load_text"
                    | "next_text_field"
            )
        })
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    hex::encode(Sha256::digest(
        serde_json::to_vec(&fields).expect("text fields serialize"),
    ))
}
fn fingerprint<T: serde::Serialize>(v: &Values, proposal: &T) -> Result<String, Error> {
    let bytes = serde_json::to_vec(&(draft_digest(v), proposal)).map_err(|_| Error::Unavailable)?;
    Ok(hex::encode(Sha256::digest(bytes)))
}
fn record_draft(v: &mut Values, r: &LearningRecord) {
    v.insert("record".into(), r.id.clone());
    v.insert(
        "move".into(),
        pin_text(&r.move_record.id, r.move_record.revision),
    );
    let text = match val(v, "text_field") {
        "context" => &r.context,
        "condition" => &r.condition,
        _ => &r.method,
    };
    v.insert("text".into(), text.clone().unwrap_or_default());
    let (state, number) = match &r.threshold {
        NumericValue::Known(n) => ("known", n.to_string()),
        NumericValue::Unknown => ("unknown", String::new()),
        NumericValue::NotApplicable => ("na", String::new()),
    };
    v.insert("threshold_state".into(), state.into());
    v.insert("threshold".into(), number);
}
async fn initial(
    state: &AppState,
    ctx: &Context,
    saved: &Saved,
    query: &Values,
) -> Result<Values, Error> {
    let mut v = Values::from([
        ("expected_revision".into(), saved.revision().to_string()),
        ("page".into(), page_number(query)?.to_string()),
        ("operation".into(), "keep".into()),
        ("threshold_state".into(), "unknown".into()),
        ("mapping_operation".into(), "keep".into()),
        ("policy".into(), "keep".into()),
        ("text_field".into(), text_field(query)?.into()),
        ("next_text_field".into(), text_field(query)?.into()),
    ]);
    if let Some(row) = saved.preset() {
        v.insert("validation_policy".into(), "saved".into());
        v.insert("display_name".into(), row.fields.display_name.clone());
        v.insert("description".into(), row.fields.description.clone());
        v.insert(
            "learnset".into(),
            pin_text(&row.learnset.learnset_id, row.learnset.version),
        );
        v.insert(
            "ruleset".into(),
            pin_text(&row.ruleset.ruleset_id, row.ruleset.version),
        );
    } else if ctx.presets && ctx.new {
        if let Some(l) = state.learning.learnset(ctx.world(), &ctx.owner).await? {
            v.insert("learnset".into(), pin_text(&l.id, l.revision));
        }
        if let Some(r) = state.moves.ruleset(ctx.local()?).await? {
            v.insert("ruleset".into(), pin_text(&r.id, r.revision));
        }
    }
    if let Some(row) = saved.learnset() {
        if let Some(p) = &row.source {
            v.insert("source".into(), pin_text(&p.learnset_id, p.version));
        }
        if !val(query, "record").is_empty() {
            let p = page_number(query)?;
            let r = row
                .records
                .iter()
                .flatten()
                .skip((p - 1) * PAGE_SIZE)
                .take(PAGE_SIZE)
                .find(|r| r.id == val(query, "record"))
                .ok_or_else(|| invalid("record", "Select a saved acquisition on this page."))?;
            record_draft(&mut v, r);
            v.insert("operation".into(), "edit".into());
        }
    }
    if ctx.adopt && !val(query, "source").is_empty() {
        v.insert("source".into(), val(query, "source").into());
    }
    Ok(v)
}
async fn move_choices(state: &AppState, ctx: &Context) -> Result<Vec<Choice>, Error> {
    Ok(state
        .moves
        .list(ctx.world())
        .await?
        .iter()
        .map(|m| Choice {
            value: pin_text(&m.id, m.revision),
            label: format!("{} · {}@{}", m.fields.display_name, m.id, m.revision),
        })
        .collect())
}
async fn build_page(
    state: &AppState,
    headers: &HeaderMap,
    ctx: &Context,
    saved: &Saved,
    draft: Values,
    failure: Option<&Error>,
) -> Result<Page, Error> {
    let number = page_number(&draft).unwrap_or(1);
    let mut p = Page::new(ctx, draft, failure);
    p.title = format!("{} — {}", p.title, ctx.owner_name(state).await?);
    p.link("Owner", ctx.owner_url());
    if ctx.presets {
        p.link("Variant learnset", format!("{}/learnset", ctx.owner_url()));
        p.link("All moveset presets", ctx.root());
    } else if ctx.world.is_some() {
        p.link("Moveset presets", format!("{}/movesets", ctx.owner_url()));
    }
    if !ctx.presets || ctx.preset.is_some() {
        p.link(
            if ctx.presets {
                "Preset history"
            } else {
                "Learnset history"
            },
            format!("{}/history", ctx.target()),
        );
    }
    let allowed = ctx.permission(state, headers).await.is_ok();
    p.editable = ctx.edit || ctx.adopt || ctx.new;
    if ctx.presets && ctx.preset.is_none() && !ctx.new {
        let rows = state.learning.presets(ctx.local()?, &ctx.owner).await?;
        p.panel(
            "Named presets",
            vec![format!(
                "{} presets. Each has its own exact contexts and history.",
                rows.len()
            )],
        );
        for r in rows.iter().skip((number - 1) * PAGE_SIZE).take(PAGE_SIZE) {
            p.link(&r.fields.display_name, format!("{}/{}", ctx.root(), r.id));
        }
        if allowed {
            p.link("Create moveset preset", format!("{}/new", ctx.root()));
        }
        if number > 1 {
            p.link(
                "Previous page",
                format!("{}?page={}", ctx.root(), number - 1),
            );
        }
        if rows.len() > number * PAGE_SIZE {
            p.link("Next page", format!("{}?page={}", ctx.root(), number + 1));
        }
        return Ok(p);
    }
    let count = if ctx.presets {
        if let Some(row) = saved.preset() {
            p.panel("Saved preset", preset_lines(row, number));
            report(
                &mut p,
                &state
                    .learning
                    .validation(ctx.local()?, &ctx.owner, &row.id, None)
                    .await?,
            );
            let l = state
                .learning
                .learnset_version(ctx.world(), &row.learnset.learnset_id, row.learnset.version)
                .await?;
            p.panel("Pinned learnset", learnset_lines(Some(&l), number));
            p.panel(
                "Selected acquisitions in order",
                selected_entries(row, &l, number),
            );
            pinned_ruleset(&mut p, state, ctx, &row.ruleset).await?;
            row.entries.len().max(learnset_length(&l))
        } else {
            0
        }
    } else {
        p.panel(
            "Current saved learnset",
            learnset_lines(saved.learnset(), number),
        );
        saved.learnset().map_or(0, learnset_length)
    };
    if !p.editable {
        if allowed {
            p.link(
                if ctx.presets {
                    "Edit moveset preset"
                } else {
                    "Edit learnset"
                },
                format!("{}/edit", ctx.target()),
            );
        }
        if !ctx.presets && ctx.world.is_some() && allowed {
            p.link(
                "Review learnset source adoption",
                format!("{}/adopt", ctx.target()),
            );
        }
        if number > 1 {
            p.link(
                "Previous page",
                format!("{}?page={}", ctx.target(), number - 1),
            );
        }
        if count > number * PAGE_SIZE {
            p.link("Next page", format!("{}?page={}", ctx.target(), number + 1));
        }
        return Ok(p);
    }
    p.hidden.insert(
        "expected_revision".into(),
        val(&p.draft, "expected_revision").into(),
    );
    p.hidden.insert("page".into(), number.to_string());
    if matches!(failure, Some(Error::Conflict)) {
        p.reviewed = saved.revision().to_string();
        p.notice = "Your draft is retained beside the current saved page. Explicitly review the current revision before resubmitting; targets are stable IDs, never positions inferred from a name.".into();
    }
    if ctx.adopt {
        let mut choices = vec![];
        for r in state.learning.shared_learnsets().await? {
            let form = state
                .authoring
                .form_detail(r.form_id.as_deref().ok_or(Error::Unavailable)?)
                .await?;
            choices.push(Choice {
                value: pin_text(&r.id, r.revision),
                label: format!("{} · {}@{}", form.fields.display_name, r.id, r.revision),
            });
        }
        p.panel("Mapping and replacement workflow",vec!["Choose the shared learnset explicitly. Keep-local is the default and does not change local acquisitions. Save one reviewed mapping at a time with keep-local to build a complete mapping; then explicitly preview replacement. Replacement copies every acquisition, including duplicate Moves, and preserves Unknown and Not applicable thresholds.".into()]);
        p.control("source","Shared learnset and exact version","text","Select a shared source independently of the variant's form pin. Older exact versions may be entered as identity@version.",choices);
        p.select(
            "policy",
            "Adoption policy",
            &[
                ("keep", "Keep local records (default)"),
                ("replace", "Replace all local records from source"),
            ],
        );
        p.select(
            "mapping_operation",
            "Move mapping operation",
            &[
                ("keep", "Keep saved mappings"),
                ("set", "Set one explicit mapping"),
                ("remove", "Remove one mapping"),
            ],
        );
        p.text(
            "shared_move",
            "Shared Move identity to map",
            "Canonical shared Move ID in the selected source. Mappings are explicit and injective.",
        );
        p.control(
            "local_move",
            "Local Move and exact version",
            "text",
            "The local Move must explicitly pin the matching shared identity.",
            move_choices(state, ctx).await?,
        );
    } else if ctx.presets {
        if let Some(row) = saved.preset() {
            p.control(
                "validation_policy",
                "Validation policy",
                "select",
                "Ordinary edits retain the saved policy. Revalidation is an explicit reviewed choice that creates a new attributed revision when the policy changes; old reports stay historical.",
                vec![
                    Choice { value: "saved".into(), label: format!("Keep saved policy {}", row.validator_version) },
                    Choice { value: "current".into(), label: format!("Revalidate using current policy {VALIDATOR_VERSION}") },
                ],
            );
        }
        p.text(
            "display_name",
            "Display name",
            "A separate named preset; no default is assumed.",
        );
        p.control(
            "description",
            "Description",
            "textarea",
            "Plain descriptive metadata.",
            vec![],
        );
        let current_learnset = state.learning.learnset(ctx.world(), &ctx.owner).await?;
        let current_rules = state.moves.ruleset(ctx.local()?).await?;
        p.panel("Current heads available for explicit reconciliation",vec![
            current_learnset.as_ref().map_or_else(||"Learnset: Unconfigured".into(),|l|format!("Learnset: {}@{}",l.id,l.revision)),
            current_rules.as_ref().map_or_else(||"Ruleset: Unconfigured".into(),|r|format!("Ruleset: {}@{}",r.id,r.revision)),
            "Current heads are choices only. Existing presets retain their exact pins and stable record selections until explicitly reviewed and saved.".into(),
        ]);
        p.control("learnset","Learnset and exact version","text","Exact same-owner identity@version. Changing context preserves selected record IDs; explicitly remove/add entries to reconcile.",current_learnset.iter().map(|l|Choice { value:pin_text(&l.id,l.revision),label:"Current same-owner learnset head (explicit choice)".into() }).collect());
        p.control(
            "ruleset",
            "Ruleset and exact version",
            "text",
            "Exact same-world identity@version. Saved reports never use current heads.",
            current_rules
                .iter()
                .map(|r| Choice {
                    value: pin_text(&r.id, r.revision),
                    label: "Current same-world ruleset head (explicit choice)".into(),
                })
                .collect(),
        );
        p.link(
            "Current world ruleset",
            format!("/worlds/{}/ruleset", ctx.local()?),
        );
        p.select(
            "operation",
            "Entry operation",
            &[
                ("keep", "Keep entries"),
                ("add", "Add one entry"),
                ("remove", "Remove one entry"),
                ("up", "Move one entry earlier"),
                ("down", "Move one entry later"),
            ],
        );
        p.text(
            "position",
            "Existing entry position",
            "One-based position on this displayed saved page; repeat entries remain distinct.",
        );
        let mut choices = vec![];
        if let Ok(pin) = learnset_pin(val(&p.draft, "learnset"), "learnset") {
            match state
                .learning
                .learnset_version(ctx.world(), &pin.learnset_id, pin.version)
                .await
            {
                Ok(l) if l.variant_id.as_deref() == Some(&ctx.owner) => {
                    choices = l
                        .records
                        .iter()
                        .flatten()
                        .skip((number - 1) * PAGE_SIZE)
                        .take(PAGE_SIZE)
                        .map(|r| Choice {
                            value: r.id.clone(),
                            label: format!("{} · {}", r.id, r.move_record.fields.display_name),
                        })
                        .collect();
                    p.next = learnset_length(&l) > number * PAGE_SIZE;
                    p.panel(
                        "Selected exact learnset context",
                        learnset_lines(Some(&l), number),
                    );
                }
                Ok(_) => {}
                Err(Error::NotFound | Error::Invalid(_)) => {}
                Err(e) => return Err(e),
            }
        }
        p.control("record","Acquisition record to add","text","Use a stable scoped acquisition ID from the exact learnset, never a Move ID or a display name.",choices);
    } else {
        p.hidden
            .insert("text_field".into(), val(&p.draft, "text_field").into());
        p.select(
            "next_text_field",
            "Text field to load",
            &[
                ("method", "Method"),
                ("context", "Context"),
                ("condition", "Condition"),
            ],
        );
        let field = text_field(&p.draft).unwrap_or("method");
        p.control("text",match field { "context"=>"Context (Unknown if blank)","condition"=>"Condition (Unknown if blank)",_=>"Method (Unknown if blank)" },"textarea","Only this loaded field is submitted (up to 2000 characters). Other saved texts stay read-only and are preserved from the verified revision. For a new acquisition, other texts start Unknown. Choose and load another field before editing.",vec![]);
        for key in ["method", "context", "condition"] {
            if let Some(raw) = p.draft.get(key) {
                p.panel(
                    format!("Retained unexpected field: {key}"),
                    vec![raw.clone()],
                );
            }
        }
        p.select(
            "operation",
            "Acquisition operation",
            &[
                ("keep", "Keep records"),
                ("defined", "Define an empty collection (unmodeled only)"),
                ("unmodeled", "Set collection to Unmodeled"),
                ("add", "Add one acquisition"),
                ("edit", "Edit one acquisition"),
                ("remove", "Remove one acquisition"),
                ("up", "Move one acquisition earlier"),
                ("down", "Move one acquisition later"),
            ],
        );
        let choices = saved
            .learnset()
            .into_iter()
            .flat_map(|r| r.records.iter().flatten())
            .skip((number - 1) * PAGE_SIZE)
            .take(PAGE_SIZE)
            .map(|r| Choice {
                value: r.id.clone(),
                label: format!("{} · {}", r.id, r.move_record.fields.display_name),
            })
            .collect();
        p.control("record","Existing acquisition ID","text","Choose a saved acquisition displayed on this page. Use its edit link to load its exact fields.",choices);
        p.control("move","Move and exact version","text","Canonical identity@version in this scope. Historical versions may be entered explicitly.",move_choices(state,ctx).await?);
        p.select(
            "threshold_state",
            "Threshold state",
            &[
                ("unknown", "Unknown"),
                ("known", "Known exact decimal"),
                ("na", "Not applicable"),
            ],
        );
        p.text(
            "threshold",
            "Threshold exact value",
            "Signed exact decimal, up to six decimal places. No level or unit assumption.",
        );
        for r in saved
            .learnset()
            .into_iter()
            .flat_map(|r| r.records.iter().flatten())
            .skip((number - 1) * PAGE_SIZE)
            .take(PAGE_SIZE)
        {
            p.record_buttons.push(Link {
                label: format!("Edit acquisition {}", r.id),
                url: r.id.clone(),
            });
        }
    }
    p.text(
        "reason",
        "Change reason (optional)",
        "Saved with this aggregate's independent history.",
    );
    p.previous = number > 1;
    p.next |= count > number * PAGE_SIZE;
    p.baseline = val(&p.draft, "baseline").into();
    if p.baseline.is_empty() {
        let mut displayed = p.hidden.clone();
        displayed.extend(p.controls.iter().map(|c| (c.name.clone(), c.value.clone())));
        p.baseline = draft_digest(&displayed);
    }
    Ok(p)
}
fn response(mut p: Page, headers: &HeaderMap, state: &AppState, status: StatusCode) -> Response {
    if !p.editable {
        return no_store(render_status(p, status));
    }
    let (csrf, set) = match csrf_for(headers) {
        Ok(pair) => pair,
        Err(_) => return authoring::read_error(Error::Unavailable),
    };
    p.csrf = csrf.clone();
    let mut r = no_store(render_status(p, status));
    if set {
        append_cookie(&mut r, csrf_cookie(&csrf, state.cookie_secure));
    }
    r
}
async fn page(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(path): Path<Values>,
    uri: Uri,
    query: QueryInput,
) -> Response {
    finish(
        async {
            let Query(query) = query.map_err(|_| invalid("page", "Check the page query."))?;
            let ctx = Context::new(&path, uri.path());
            if ctx.edit || ctx.adopt || ctx.new {
                ctx.permission(&state, &headers).await?;
            }
            let saved = ctx.saved(&state).await?;
            let draft = initial(&state, &ctx, &saved, &query).await?;
            let p = build_page(&state, &headers, &ctx, &saved, draft, None).await?;
            Ok(response(p, &headers, &state, StatusCode::OK))
        }
        .await,
    )
}

fn record_input(r: &LearningRecord) -> LearningRecordInput {
    LearningRecordInput {
        id: r.id.clone(),
        move_pin: r.move_record.pin(),
        method: r.method.clone(),
        context: r.context.clone(),
        condition: r.condition.clone(),
        threshold: r.threshold.clone(),
    }
}
fn text_field(v: &Values) -> Result<&str, Error> {
    match val(v, "text_field") {
        "" | "method" => Ok("method"),
        "context" => Ok("context"),
        "condition" => Ok("condition"),
        _ => Err(invalid(
            "text_field",
            "Choose and explicitly load Method, Context or Condition.",
        )),
    }
}
fn text_budget(v: &Values) -> Result<(), Error> {
    if ["method", "context", "condition"]
        .iter()
        .any(|k| v.contains_key(*k))
    {
        return Err(invalid(
            "text",
            "Submit only the loaded text field. Unexpected text values are retained read-only below; no changes were saved.",
        ));
    }
    text_field(v)?;
    if val(v, "text")
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .trim()
        .chars()
        .count()
        > 2000
    {
        return Err(invalid(
            "text",
            "Use at most 2000 characters in the loaded text field; your full draft is retained.",
        ));
    }
    game_data::authoring::validate_reason(reason(v).as_deref()).map_err(Error::Invalid)?;
    Ok(())
}
fn submitted_record(
    v: &Values,
    id: String,
    saved: Option<&LearningRecordInput>,
) -> Result<LearningRecordInput, Error> {
    let threshold = match val(v, "threshold_state") {
        "known" => NumericValue::Known(
            val(v, "threshold")
                .parse::<Decimal>()
                .map_err(|e| invalid("threshold", &e))?,
        ),
        "unknown" => NumericValue::Unknown,
        "na" => NumericValue::NotApplicable,
        _ => {
            return Err(invalid(
                "threshold_state",
                "Choose Known, Unknown or Not applicable.",
            ));
        }
    };
    let mut row = LearningRecordInput {
        id,
        move_pin: move_pin(val(v, "move"), "move")?,
        method: saved.and_then(|r| r.method.clone()),
        context: saved.and_then(|r| r.context.clone()),
        condition: saved.and_then(|r| r.condition.clone()),
        threshold,
    };
    let text = Some(val(v, "text").to_owned());
    match text_field(v)? {
        "context" => row.context = text,
        "condition" => row.condition = text,
        _ => row.method = text,
    };
    Ok(row)
}
fn learnset_edit(saved: &Saved, v: &Values) -> Result<LearnsetEdit, Error> {
    let mut records = saved
        .learnset()
        .and_then(|r| r.records.as_ref())
        .map(|rows| rows.iter().map(record_input).collect::<Vec<_>>());
    let operation = val(v, "operation");
    match operation {
        "keep" => {}
        "unmodeled" => records = None,
        "defined" => {
            if records.as_ref().is_some_and(|r| !r.is_empty()) {
                return Err(invalid(
                    "operation",
                    "Remove records individually before defining an empty collection.",
                ));
            }
            records = Some(vec![]);
        }
        "add" => {
            records
                .get_or_insert_with(Vec::new)
                .push(submitted_record(v, String::new(), None)?)
        }
        "edit" | "remove" | "up" | "down" => {
            let rows = records
                .as_mut()
                .ok_or_else(|| invalid("record", "There are no saved acquisition records."))?;
            let n = page_number(v)?;
            let i = rows
                .iter()
                .position(|r| r.id == val(v, "record"))
                .filter(|i| *i / PAGE_SIZE == n - 1)
                .ok_or_else(|| {
                    invalid(
                        "record",
                        "Select a saved acquisition displayed on this page.",
                    )
                })?;
            match operation {
                "edit" => rows[i] = submitted_record(v, rows[i].id.clone(), Some(&rows[i]))?,
                "remove" => {
                    rows.remove(i);
                }
                "up" if i > 0 => rows.swap(i, i - 1),
                "down" if i + 1 < rows.len() => rows.swap(i, i + 1),
                _ => {
                    return Err(invalid(
                        "record",
                        "The acquisition cannot move further in that direction.",
                    ));
                }
            }
        }
        _ => return Err(invalid("operation", "Choose one acquisition operation.")),
    }
    Ok(LearnsetEdit {
        expected_revision: expected(v)?,
        records,
        reason: reason(v),
    })
}
fn adoption_edit(saved: &Saved, v: &Values) -> Result<LearnsetAdoptEdit, Error> {
    let mut mappings = saved
        .learnset()
        .map_or_else(Vec::new, |r| r.mappings.clone());
    match val(v, "mapping_operation") {
        "keep" => {}
        "set" | "remove" => {
            let shared = val(v, "shared_move");
            if !game_data::types::valid_id(shared) {
                return Err(invalid(
                    "shared_move",
                    "Use a canonical shared Move identity.",
                ));
            }
            mappings.retain(|m| m.shared_move_id != shared);
            if val(v, "mapping_operation") == "set" {
                mappings.push(MoveMapping {
                    shared_move_id: shared.into(),
                    local_move: move_pin(val(v, "local_move"), "local_move")?,
                });
            }
        }
        _ => {
            return Err(invalid(
                "mapping_operation",
                "Choose one explicit mapping operation.",
            ));
        }
    }
    let apply_source = match val(v, "policy") {
        "keep" => false,
        "replace" => true,
        _ => {
            return Err(invalid(
                "policy",
                "Choose keep-local or explicit replacement.",
            ));
        }
    };
    Ok(LearnsetAdoptEdit {
        expected_revision: expected(v)?,
        source: learnset_pin(val(v, "source"), "source")?,
        apply_source,
        mappings,
        reason: reason(v),
    })
}
fn preset_edit(saved: &Saved, v: &Values) -> Result<MovesetEdit, Error> {
    let mut entries = saved.preset().map_or_else(Vec::new, |r| r.entries.clone());
    match val(v, "operation") {
        "keep" => {}
        "add" => {
            if !game_data::types::valid_id(val(v, "record")) {
                return Err(invalid(
                    "record",
                    "Use a stable canonical acquisition ID from the exact learnset.",
                ));
            }
            entries.push(val(v, "record").into());
        }
        "remove" | "up" | "down" => {
            let i = positive(val(v, "position"), "position")?
                .checked_sub(1)
                .and_then(|i| usize::try_from(i).ok())
                .filter(|i| {
                    *i < entries.len()
                        && *i / PAGE_SIZE == page_number(v).unwrap_or(0).saturating_sub(1)
                })
                .ok_or_else(|| {
                    invalid(
                        "position",
                        "Select an existing entry position displayed on this page.",
                    )
                })?;
            match val(v, "operation") {
                "remove" => {
                    entries.remove(i);
                }
                "up" if i > 0 => entries.swap(i, i - 1),
                "down" if i + 1 < entries.len() => entries.swap(i, i + 1),
                _ => {
                    return Err(invalid(
                        "position",
                        "This entry cannot move further in that direction.",
                    ));
                }
            }
        }
        _ => return Err(invalid("operation", "Choose one entry operation.")),
    }
    let (ruleset_id, version) = exact(val(v, "ruleset"), "ruleset")?;
    Ok(MovesetEdit {
        expected_revision: expected(v)?,
        fields: ContentFields {
            slug: String::new(),
            display_name: val(v, "display_name").into(),
            description: val(v, "description").into(),
        },
        learnset: learnset_pin(val(v, "learnset"), "learnset")?,
        ruleset: RulesetPin {
            ruleset_id,
            version,
        },
        entries,
        reason: reason(v),
    })
}

async fn submit(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(path): Path<Values>,
    uri: Uri,
    form: FormInput,
) -> Response {
    let mut v = match form {
        Ok(Form(v)) => v,
        Err(e) => return authoring_form_error(e),
    };
    let mut ctx = Context::new(&path, uri.path());
    if !ctx.adopt && !ctx.new {
        ctx.edit = true;
    }
    let result = async {
        let token = submit_token(&state,&headers,val(&v,"csrf"))?;
        ctx.permission(&state,&headers).await?;
        let saved = ctx.saved(&state).await?;
        if !val(&v,"reviewed_revision").is_empty() { v.insert("expected_revision".into(),val(&v,"reviewed_revision").into()); }
        // Check before parsing ANY submitted dependency/number. Services repeat under lock.
        if expected(&v)?!=saved.revision() { return Err(Error::Conflict); }
        page_number(&v)?;
        if !ctx.presets && !ctx.adopt { text_budget(&v)?; }
        if matches!(val(&v,"intent"),"next" | "previous") || !val(&v,"load_record").is_empty() || !val(&v,"load_text").is_empty() {
            if draft_digest(&v)!=val(&v,"baseline") { return Err(invalid("page","Save the current page before navigation, or use Discard draft and reload. Off-page drafts are never submitted.")); }
            let n=page_number(&v)?;
            if !val(&v,"load_text").is_empty() {
                let field=val(&v,"next_text_field");
                if !matches!(field,"method" | "context" | "condition") { return Err(invalid("next_text_field","Choose Method, Context or Condition.")); }
                let id=val(&v,"record");
                if !id.is_empty() && !saved.learnset().into_iter().flat_map(|r|r.records.iter().flatten()).skip((n-1)*PAGE_SIZE).take(PAGE_SIZE).any(|r|r.id==id) {
                    return Err(invalid("record","Choose an acquisition displayed on this saved page."));
                }
                return Ok(no_store(Redirect::to(&format!("{}?page={n}&text_field={field}{}",ctx.action(),if id.is_empty() { String::new() } else { format!("&record={id}") })).into_response()));
            }
            if !val(&v,"load_record").is_empty() {
                let id=val(&v,"load_record");
                if !saved.learnset().into_iter().flat_map(|r|r.records.iter().flatten()).skip((n-1)*PAGE_SIZE).take(PAGE_SIZE).any(|r|r.id==id) {
                    return Err(invalid("record","Choose an acquisition displayed on this saved page."));
                }
                return Ok(no_store(Redirect::to(&format!("{}?page={n}&record={id}&text_field={}",ctx.action(),text_field(&v)?)).into_response()));
            }
            let n=if val(&v,"intent")=="next" { n+1 } else { n.saturating_sub(1).max(1) };
            return Ok(no_store(Redirect::to(&format!("{}?page={n}",ctx.action())).into_response()));
        }
        let mut p=build_page(&state,&headers,&ctx,&saved,v.clone(),None).await?;
        let target = if ctx.adopt {
            let edit=adoption_edit(&saved,&v)?;
            let preview=state.learning.preview_adoption(ctx.local()?,&ctx.owner,&edit).await?;
            let n=page_number(&v)?;
            adoption_panels(&mut p,&preview,n,edit.apply_source);
            p.fingerprint=fingerprint(&v,&preview.proposed)?;
            if confirmed(&v,&p.fingerprint) {
                state.learning.adopt_learnset(&token,ctx.local()?,&ctx.owner,edit).await?;
                Some(ctx.target())
            } else { None }
        } else if ctx.presets {
            let edit=preset_edit(&saved,&v)?;
            let revalidate=match val(&v,"validation_policy") {
                "" | "saved" => false,
                "current" => true,
                _ => return Err(invalid("validation_policy","Keep the saved policy or explicitly revalidate using the current policy.")),
            };
            let preview=if let Some(id)=ctx.preset.as_deref().filter(|_|revalidate) {
                state.learning.preview_preset_revalidation(ctx.local()?,&ctx.owner,id,&edit).await?
            } else {
                state.learning.preview_preset(ctx.local()?,&ctx.owner,ctx.preset.as_deref(),&edit).await?
            };
            p.panel("Proposed preset",preset_lines(&preview.proposed,page_number(&v)?)); report(&mut p,&preview.report);
            let context=state.learning.learnset_version(ctx.world(),&preview.proposed.learnset.learnset_id,preview.proposed.learnset.version).await?;
            p.panel("Proposed selected acquisitions in order",selected_entries(&preview.proposed,&context,page_number(&v)?));
            pinned_ruleset(&mut p,&state,&ctx,&preview.proposed.ruleset).await?;
            p.fingerprint=fingerprint(&v,&preview.proposed)?;
            if confirmed(&v,&p.fingerprint) {
                let row=if let Some(id)=&ctx.preset {
                    if revalidate {state.learning.revalidate_preset(&token,ctx.local()?,&ctx.owner,id,edit).await?}
                    else {state.learning.save_preset(&token,ctx.local()?,&ctx.owner,id,edit).await?}
                } else { state.learning.create_preset(&token,ctx.local()?,&ctx.owner,edit).await? };
                Some(format!("{}/{}",ctx.root(),row.id))
            } else { None }
        } else {
            let edit=learnset_edit(&saved,&v)?;
            game_data::authoring::validate_reason(edit.reason.as_deref()).map_err(Error::Invalid)?;
            let dependency = if matches!(val(&v,"operation"),"add" | "edit") {
                let pin=move_pin(val(&v,"move"),"move")?;
                Some(state.moves.version(ctx.world(),&pin.move_id,pin.version).await.map_err(|e|match e {
                    Error::NotFound=>invalid("move","Choose an existing exact Move revision in this scope."),
                    other=>other,
                })?)
            } else { None };
            // Collection removal is an explicit reviewed operation too.
            p.fingerprint=fingerprint(&v,&(saved.revision(),saved.learnset(),dependency))?;
            p.panel("Proposed acquisition operation",vec![
                format!("Operation: {} · Target: {} · Exact Move: {} · Threshold state: {} · Raw value: {}",val(&v,"operation"),val(&v,"record"),val(&v,"move"),val(&v,"threshold_state"),val(&v,"threshold")),
                format!("Loaded text field: {} · Proposed text: {}. Other saved text fields are preserved from the exact starting revision.",text_field(&v)?,val(&v,"text")),
                if val(&v,"operation")=="unmodeled" {
                    "The collection becomes Unmodeled and all current acquisition records are removed from its new revision. History and presets on old exact pins are preserved.".into()
                } else {
                    "All other saved records are preserved. New acquisition identities are assigned only when saved; existing target IDs remain stable.".into()
                },
            ]);
            if val(&v,"operation")=="unmodeled" {
                p.panel("All acquisitions removed by Unmodeled",saved.learnset().into_iter().flat_map(|l|l.records.iter().flatten()).enumerate().map(|(i,r)|record_text(r,i+1)).collect());
            }
            if confirmed(&v,&p.fingerprint) {
                state.learning.save_learnset(&token,ctx.world(),&ctx.owner,edit).await?; Some(ctx.target())
            } else { None }
        };
        if let Some(target)=target { return Ok(no_store(Redirect::to(&target).into_response())); }
        p.preview=true;
        if val(&v,"intent")=="save" { p.notice="Nothing was saved. Review this fresh preview and check the confirmation again; the reviewed draft or proposal changed, or confirmation was missing.".into(); }
        Ok(response(p,&headers,&state,StatusCode::OK))
    }.await;
    match result {
        Ok(r) => r,
        Err(e) => match ctx.saved(&state).await {
            Ok(saved) => match build_page(&state, &headers, &ctx, &saved, v, Some(&e)).await {
                Ok(mut p) => {
                    // An incomplete replacement has no valid proposal. Show the
                    // service's keep-local comparison so missing mappings are
                    // visible, but never offer confirmation of this fallback.
                    if ctx.adopt
                        && matches!(e, Error::Invalid(_))
                        && let Ok(mut edit) = adoption_edit(&saved, &p.draft)
                    {
                        edit.apply_source = false;
                        if let Ok(preview) = state
                            .learning
                            .preview_adoption(ctx.local().unwrap_or(""), &ctx.owner, &edit)
                            .await
                        {
                            let number = page_number(&p.draft).unwrap_or(1);
                            adoption_panels(&mut p, &preview, number, false);
                            p.notice="Replacement was not valid. The comparison below keeps local records; resolve the reported mappings and preview replacement again. Nothing was saved.".into();
                        }
                    }
                    response(p, &headers, &state, error_status(&e))
                }
                Err(read) => authoring::read_error(read),
            },
            Err(read) => authoring::read_error(read),
        },
    }
}
fn confirmed(v: &Values, digest: &str) -> bool {
    val(v, "intent") == "save"
        && val(v, "confirm") == "yes"
        && val(v, "review_fingerprint") == digest
}

fn adoption_panels(p: &mut Page, preview: &LearnsetAdoptionPreview, number: usize, replace: bool) {
    p.panel(
        "Old shared source",
        learnset_lines(preview.old_source.as_ref(), number),
    );
    p.panel(
        "Current local learnset",
        learnset_lines(preview.entry.as_ref(), number),
    );
    p.panel(
        "New shared source",
        learnset_lines(Some(&preview.new_source), number),
    );
    p.panel(
        "Proposed learnset",
        learnset_lines(Some(&preview.proposed), number),
    );
    if let Some(form) = &preview.new_source.form_id {
        p.link(
            "Read all selected source acquisitions by page",
            format!(
                "/forms/{form}/learnset/history/{}",
                preview.new_source.revision
            ),
        );
    }
    p.panel("Explicit mapping effects",preview.proposed.mappings.iter().map(|m| {
        let count=preview.new_source.records.iter().flatten().filter(|r|r.move_record.id==m.shared_move_id).count();
        format!("Shared Move {} → local Move {}@{} · {count} separate source acquisitions. {}",m.shared_move_id,m.local_move.move_id,m.local_move.version,if replace { "Every acquisition is copied separately, preserving threshold state and descriptive text." } else { "Provenance only: local acquisition records stay unchanged." })
    }).collect());
    p.panel(
        "Unmapped source acquisitions",
        preview
            .new_source
            .records
            .iter()
            .flatten()
            .filter(|r| {
                !preview
                    .proposed
                    .mappings
                    .iter()
                    .any(|m| m.shared_move_id == r.move_record.id)
            })
            .enumerate()
            .map(|(i, r)| record_text(r, i + 1))
            .collect(),
    );
    p.panel(
        "Removed local acquisitions",
        if replace {
            preview
                .entry
                .as_ref()
                .into_iter()
                .flat_map(|r| r.records.iter().flatten())
                .filter(|r| !preview.proposed.records.iter().flatten().any(|n| n == *r))
                .enumerate()
                .map(|(i, r)| record_text(r, i + 1))
                .collect()
        } else {
            vec!["None — keep-local preserves every local acquisition.".into()]
        },
    );
}

async fn history(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(path): Path<Values>,
    uri: Uri,
    query: QueryInput,
) -> Response {
    finish(
        async {
            let Query(query) = query.map_err(|_| invalid("page", "Check the history query."))?;
            let ctx = Context::new(&path, uri.path());
            let saved = ctx.saved(&state).await?;
            let number = page_number(&query)?;
            let mut p = Page::new(&ctx, Values::new(), None);
            p.title = if ctx.presets {
                "Preset history"
            } else {
                "Learnset history"
            }
            .into();
            p.link("Current saved state", ctx.target());
            p.link("History index", format!("{}/history", ctx.target()));
            if !val(&path, "revision").is_empty() {
                let revision = positive(val(&path, "revision"), "revision")?;
                if let Some(row) = saved.preset() {
                    let r = state
                        .learning
                        .preset_version(ctx.local()?, &ctx.owner, &row.id, revision)
                        .await?;
                    p.panel(format!("Revision {revision}"), preset_lines(&r, number));
                    let l = state
                        .learning
                        .learnset_version(ctx.world(), &r.learnset.learnset_id, r.learnset.version)
                        .await?;
                    p.panel(
                        "Historical pinned learnset",
                        learnset_lines(Some(&l), number),
                    );
                    p.panel(
                        "Historical selected acquisitions in order",
                        selected_entries(&r, &l, number),
                    );
                    pinned_ruleset(&mut p, &state, &ctx, &r.ruleset).await?;
                    report(
                        &mut p,
                        &state
                            .learning
                            .validation(ctx.local()?, &ctx.owner, &row.id, Some(revision))
                            .await?,
                    );
                    history_page_links(
                        &mut p,
                        &ctx,
                        revision,
                        number,
                        r.entries.len().max(learnset_length(&l)),
                    );
                } else {
                    let row = saved.learnset().ok_or(Error::NotFound)?;
                    let r = state
                        .learning
                        .learnset_version(ctx.world(), &row.id, revision)
                        .await?;
                    p.panel(
                        format!("Revision {revision}"),
                        learnset_lines(Some(&r), number),
                    );
                    history_page_links(&mut p, &ctx, revision, number, learnset_length(&r));
                }
            } else {
                let before = if val(&query, "before").is_empty() {
                    None
                } else {
                    Some(positive(val(&query, "before"), "before")?)
                };
                let rows = if let Some(r) = saved.preset() {
                    state
                        .learning
                        .preset_history(ctx.local()?, &ctx.owner, &r.id, before)
                        .await?
                } else {
                    state
                        .learning
                        .learnset_history(ctx.world(), &ctx.owner, before)
                        .await?
                };
                for h in &rows {
                    p.link(
                        format!("Revision {}", h.revision),
                        format!("{}/history/{}", ctx.target(), h.revision),
                    );
                    p.panel(
                        format!("Revision {} · {}", h.revision, h.action),
                        vec![
                            format!(
                                "{} · {} · Reason: {}",
                                h.actor_name,
                                h.occurred_at,
                                h.reason.as_deref().unwrap_or("None")
                            ),
                            history_summary(&h.before_json, &h.after_json, ctx.presets)?,
                        ],
                    );
                }
                if let Some(last) = rows.last() {
                    p.link(
                        "Older history",
                        format!("{}/history?before={}", ctx.target(), last.revision),
                    );
                }
            }
            Ok(response(p, &headers, &state, StatusCode::OK))
        }
        .await,
    )
}
fn history_page_links(p: &mut Page, ctx: &Context, revision: i64, number: usize, count: usize) {
    if number > 1 {
        p.link(
            "Previous page",
            format!("{}/history/{revision}?page={}", ctx.target(), number - 1),
        );
    }
    if count > number * PAGE_SIZE {
        p.link(
            "Next page",
            format!("{}/history/{revision}?page={}", ctx.target(), number + 1),
        );
    }
}
fn history_summary(before: &Option<String>, after: &str, preset: bool) -> Result<String, Error> {
    // Snapshots have already been verified by the service. Only render their readable metadata.
    let a: serde_json::Value = serde_json::from_str(after).map_err(|_| Error::Unavailable)?;
    let b = before
        .as_ref()
        .map(|s| serde_json::from_str::<serde_json::Value>(s))
        .transpose()
        .map_err(|_| Error::Unavailable)?;
    let a = a.get("content").unwrap_or(&a);
    let b = b.as_ref().map(|b| b.get("content").unwrap_or(b));
    let key = if preset { "entries" } else { "records" };
    let state = |v: Option<&serde_json::Value>| match v.and_then(|v| v.get(key)) {
        Some(serde_json::Value::Array(v)) => format!("{} ordered {}", v.len(), key),
        Some(serde_json::Value::Null) => "Unmodeled".into(),
        _ => "Unconfigured".into(),
    };
    let changed = if let Some(b) = b {
        let labels = if preset {
            vec![
                ("fields", "name/description"),
                ("learnset", "exact learnset context"),
                ("ruleset", "exact ruleset context"),
                ("entries", "ordered acquisition selections"),
                ("validator_version", "validator policy"),
            ]
        } else {
            vec![
                (
                    "records",
                    "ordered acquisition data (IDs, Move pins, text or thresholds)",
                ),
                ("source", "independent shared source pin"),
                ("mappings", "explicit Move mappings"),
            ]
        };
        labels
            .into_iter()
            .filter(|(key, _)| a.get(*key) != b.get(*key))
            .map(|(_, label)| label)
            .collect::<Vec<_>>()
            .join("; ")
    } else {
        "Initial independent aggregate".into()
    };
    Ok(format!(
        "{} → {}. Changed: {}. Open the revision for exact pins and readable paginated state; record selections and provenance are independent of creature history.",
        state(b),
        state(Some(a)),
        changed,
    ))
}
