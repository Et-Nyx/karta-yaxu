//! Native move/attribute/configuration workflows. Services own policy and writes.
use super::authoring::{
    authoring_form_error, error_status, error_text, finish, invalid, library_permission, no_store,
    positive, session, submit_token, world_permission,
};
use super::*;
use crate::application::{
    authoring::{Error, HistoryEntry},
    moves::*,
};
use axum::{extract::rejection::QueryRejection, http::Uri};
use game_data::rulesets::{CURRENT_VALIDATOR_VERSION, validate_ruleset_with_policy};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

type Values = BTreeMap<String, String>;
type QueryInput = Result<Query<Values>, QueryRejection>;
type FormInput = Result<Form<Values>, FormRejection>;
const SIZE: usize = 25;
const PROVISIONAL: &str = "00000000000000000000000000000000";

pub(super) fn routes() -> Router<AppState> {
    let mut r = Router::new();
    for root in [
        "/move-attributes",
        "/worlds/{world_id}/move-attributes",
        "/moves",
        "/worlds/{world_id}/moves",
    ] {
        r = r
            .route(root, get(catalog))
            .route(&format!("{root}/new"), get(editor).post(submit))
            .route(&format!("{root}/{{id}}"), get(editor))
            .route(&format!("{root}/{{id}}/edit"), get(editor).post(submit))
            .route(&format!("{root}/{{id}}/history"), get(history))
            .route(&format!("{root}/{{id}}/history/{{revision}}"), get(history));
        if root.starts_with("/worlds/") {
            r = r.route(&format!("{root}/{{id}}/adopt"), get(editor).post(submit));
        }
    }
    r.route("/worlds/{world_id}/ruleset", get(editor))
        .route("/worlds/{world_id}/ruleset/edit", get(editor).post(submit))
        .route("/worlds/{world_id}/ruleset/history", get(history))
        .route(
            "/worlds/{world_id}/ruleset/history/{revision}",
            get(history),
        )
        .layer(DefaultBodyLimit::max(32 * 1024))
}
fn val<'a>(d: &'a Values, k: &str) -> &'a str {
    d.get(k).map(String::as_str).unwrap_or("")
}
fn reason(d: &Values) -> Option<String> {
    d.get("reason").filter(|v| !v.is_empty()).cloned()
}
fn expected(d: &Values) -> Result<i64, Error> {
    val(d, "expected_revision")
        .parse::<i64>()
        .ok()
        .filter(|n| *n >= 0)
        .ok_or_else(|| {
            invalid(
                "expected_revision",
                "Use the saved revision; zero means new or unconfigured.",
            )
        })
}
fn number(d: &Values) -> Result<usize, Error> {
    if val(d, "page").is_empty() {
        return Ok(1);
    }
    val(d, "page")
        .parse::<usize>()
        .ok()
        .filter(|n| *n > 0 && *n <= usize::MAX / SIZE)
        .ok_or_else(|| invalid("page", "Use a positive page number."))
}
fn decimal(raw: &str, key: &str) -> Result<Option<Decimal>, Error> {
    if raw.is_empty() {
        Ok(None)
    } else {
        raw.parse().map(Some).map_err(|e: String| invalid(key, &e))
    }
}
fn constraints(d: &Values) -> Result<NumericConstraints, Error> {
    let min = decimal(val(d, "minimum"), "minimum");
    let max = decimal(val(d, "maximum"), "maximum");
    let mut errors = Values::new();
    for r in [&min, &max] {
        if let Err(Error::Invalid(e)) = r {
            errors.extend(e.clone());
        }
    }
    if !errors.is_empty() {
        return Err(Error::Invalid(errors));
    }
    let c = NumericConstraints {
        minimum: min?,
        maximum: max?,
        whole_only: val(d, "whole_only") == "yes",
    };
    c.validate().map_err(Error::Invalid)?;
    Ok(c)
}
fn exact(raw: &str, key: &str) -> Result<(String, i64), Error> {
    let (id, rev) = raw
        .rsplit_once('@')
        .ok_or_else(|| invalid(key, "Choose canonical identity@version."))?;
    if !game_data::types::valid_id(id) {
        return Err(invalid(
            key,
            "Use a canonical immutable identity, not a name or slug.",
        ));
    }
    Ok((id.into(), positive(rev, key)?))
}
fn attribute_pin(raw: &str, key: &str) -> Result<AttributePin, Error> {
    let (attribute_id, version) = exact(raw, key)?;
    Ok(AttributePin {
        attribute_id,
        version,
    })
}
fn reference<T>(result: Result<T, Error>, key: &str) -> Result<T, Error> {
    result.map_err(|e| match e {
        Error::NotFound => invalid(key, "This exact pin does not exist in the selected scope. Choose an existing identity@version."),
        e => e,
    })
}
fn chosen<'a>(d: &'a Values, exact: &str, select: &str) -> &'a str {
    if val(d, exact).is_empty() {
        val(d, select)
    } else {
        val(d, exact)
    }
}
fn fields(d: &Values) -> ContentFields {
    ContentFields {
        slug: String::new(),
        display_name: val(d, "display_name").into(),
        description: val(d, "description").into(),
    }
}
fn preview_fields(d: &Values, old: Option<&ContentFields>) -> Result<ContentFields, Error> {
    let mut f = fields(d);
    f.slug = if let Some(o) = old.filter(|o| o.display_name == f.display_name.trim()) {
        o.slug.clone()
    } else {
        game_data::authoring::slug_from_name(&f.display_name)
    };
    f.normalized().map_err(Error::Invalid)
}
fn text_field(d: &Values) -> Result<&str, Error> {
    match val(d, "text_field") {
        "description" => Ok("description"),
        "notes" => Ok("notes"),
        _ => Err(invalid(
            "text_field",
            "Choose and explicitly load Description or Inert notes.",
        )),
    }
}
fn ruleset_prose(d: &Values, old: Option<&Record>) -> Result<(String, String), Error> {
    if ["description", "notes"].iter().any(|k| d.contains_key(*k)) {
        return Err(invalid(
            "text",
            "Submit only the loaded text field. Unexpected description/notes values are retained read-only below; nothing was saved.",
        ));
    }
    let field = text_field(d)?;
    let saved = if let Some(Record::Ruleset(r)) = old {
        Some(r)
    } else {
        None
    };
    if saved.is_none() && field != "description" {
        return Err(invalid(
            "text_field",
            "Save the required Description first. New rulesets start with empty inert notes; edit notes in a later operation.",
        ));
    }
    Ok((
        if field == "description" {
            val(d, "text").into()
        } else {
            saved
                .map(|r| r.fields.description.clone())
                .unwrap_or_default()
        },
        if field == "notes" {
            val(d, "text").into()
        } else {
            saved.map(|r| r.notes.clone()).unwrap_or_default()
        },
    ))
}
fn load_prose(d: &mut Values, old: Option<&Record>, field: &str) -> Result<(), Error> {
    if !matches!(field, "description" | "notes") {
        return Err(invalid(
            "next_text_field",
            "Choose Description or Inert notes.",
        ));
    }
    if old.is_none() && field != "description" {
        return Err(invalid(
            "next_text_field",
            "Save the required Description first; new rulesets start with empty inert notes.",
        ));
    }
    let text = match (old, field) {
        (Some(Record::Ruleset(r)), "notes") => r.notes.clone(),
        (Some(r), "description") => r.fields().description.clone(),
        _ => String::new(),
    };
    d.remove("description");
    d.remove("notes");
    d.insert("text_field".into(), field.into());
    d.insert("next_text_field".into(), field.into());
    d.insert("text".into(), text);
    Ok(())
}
fn digest(v: &impl serde::Serialize) -> Result<String, Error> {
    Ok(hex::encode(Sha256::digest(
        serde_json::to_vec(v).map_err(|_| Error::Unavailable)?,
    )))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Attribute,
    Move,
    Ruleset,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    New,
    Detail,
    Edit,
    Adopt,
}
struct Context {
    world: Option<String>,
    id: String,
    kind: Kind,
    mode: Mode,
}
impl Context {
    fn new(path: &Values, uri: &str) -> Self {
        Self {
            world: path.get("world_id").cloned(),
            id: val(path, "id").into(),
            kind: if uri.contains("/move-attributes") {
                Kind::Attribute
            } else if uri.contains("/ruleset") {
                Kind::Ruleset
            } else {
                Kind::Move
            },
            mode: if uri.ends_with("/new") {
                Mode::New
            } else if uri.ends_with("/edit") {
                Mode::Edit
            } else if uri.ends_with("/adopt") {
                Mode::Adopt
            } else {
                Mode::Detail
            },
        }
    }
    fn world(&self) -> Option<&str> {
        self.world.as_deref()
    }
    fn root(&self) -> String {
        let tail = match self.kind {
            Kind::Attribute => "move-attributes",
            Kind::Move => "moves",
            Kind::Ruleset => "ruleset",
        };
        self.world()
            .map_or_else(|| format!("/{tail}"), |w| format!("/worlds/{w}/{tail}"))
    }
    fn target(&self) -> String {
        if self.kind == Kind::Ruleset || self.mode == Mode::New {
            self.root()
        } else {
            format!("{}/{}", self.root(), self.id)
        }
    }
    fn action(&self) -> String {
        format!(
            "{}{}",
            self.target(),
            match self.mode {
                Mode::New => "/new",
                Mode::Edit => "/edit",
                Mode::Adopt => "/adopt",
                Mode::Detail => "",
            }
        )
    }
    async fn permission(&self, s: &AppState, h: &HeaderMap) -> Result<(), Error> {
        if let Some(w) = self.world() {
            world_permission(s, h, Some(w)).await
        } else {
            library_permission(s, h).await
        }
    }
    async fn saved(&self, s: &AppState) -> Result<Option<Record>, Error> {
        if self.mode == Mode::New {
            return Ok(None);
        }
        Ok(match self.kind {
            Kind::Attribute => Some(Record::Attribute(
                s.moves.attribute(self.world(), &self.id).await?,
            )),
            Kind::Move => Some(Record::Move(s.moves.detail(self.world(), &self.id).await?)),
            Kind::Ruleset => s
                .moves
                .ruleset(self.world().ok_or(Error::NotFound)?)
                .await?
                .map(Record::Ruleset),
        })
    }
    async fn pretty_root(&self, s: &AppState) -> Result<String, Error> {
        let mut root = self.root();
        if let Some(w) = self.world() {
            let world = s.authoring.world_detail(w).await?;
            root = root.replacen(
                &format!("/worlds/{w}/"),
                &format!("/worlds/{}/", addresses::segment(&world.fields.slug)),
                1,
            );
        }
        Ok(root)
    }
    async fn pretty(&self, s: &AppState, row: Option<&Record>) -> Result<String, Error> {
        let root = self.pretty_root(s).await?;
        Ok(if self.kind == Kind::Ruleset {
            root
        } else if let Some(r) = row {
            format!("{root}/{}", addresses::segment(&r.fields().slug))
        } else {
            root
        })
    }
}
#[derive(Clone, PartialEq, Eq, serde::Serialize)]
#[serde(untagged)]
enum Record {
    Attribute(MoveAttributeDefinition),
    Move(MoveRecord),
    Ruleset(RulesetRecord),
}
impl Record {
    fn revision(&self) -> i64 {
        match self {
            Self::Attribute(r) => r.revision,
            Self::Move(r) => r.revision,
            Self::Ruleset(r) => r.revision,
        }
    }
    fn fields(&self) -> &ContentFields {
        match self {
            Self::Attribute(r) => &r.fields,
            Self::Move(r) => &r.fields,
            Self::Ruleset(r) => &r.fields,
        }
    }
    fn len(&self) -> usize {
        match self {
            Self::Attribute(_) => 0,
            Self::Move(r) => r
                .attributes
                .as_ref()
                .map_or(0, Vec::len)
                .max(r.mappings.len()),
            Self::Ruleset(r) => r.expectations.as_ref().map_or(0, Vec::len),
        }
    }
    fn text(&self, n: usize) -> String {
        match self {
            Self::Attribute(r) => attribute_text(r),
            Self::Move(r) => move_text(r, n),
            Self::Ruleset(r) => ruleset_text(r, n),
        }
    }
}
fn constraint_text(c: &NumericConstraints) -> String {
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
            "exact signed decimals allowed"
        }
    )
}
fn attribute_text(r: &MoveAttributeDefinition) -> String {
    format!(
        "{} · {}@{}\n{}\nUnit: {}\n{}\nSource: {}",
        r.fields.display_name,
        r.id,
        r.revision,
        r.fields.description,
        if r.unit.is_empty() {
            "unspecified"
        } else {
            &r.unit
        },
        constraint_text(&r.constraints),
        r.source
            .as_ref()
            .map(|p| format!("{}@{}", p.attribute_id, p.version))
            .unwrap_or_else(|| "Independent — no source".into())
    )
}
fn value_text(v: &NumericValue) -> String {
    match v {
        NumericValue::Known(n) => format!("Known {n}"),
        NumericValue::Unknown => "Unknown".into(),
        NumericValue::NotApplicable => "NotApplicable".into(),
    }
}
fn assignment_text(a: &MoveAttributeValue) -> String {
    format!(
        "{} = {}\n{}",
        a.definition.fields.display_name,
        value_text(&a.value),
        attribute_text(&a.definition)
    )
}
fn move_text(r: &MoveRecord, n: usize) -> String {
    format!(
        "{} · {}@{}\n{}\nSource: {}\nAttributes: {}\n{}\nExplicit mappings:\n{}",
        r.fields.display_name,
        r.id,
        r.revision,
        r.fields.description,
        r.source
            .as_ref()
            .map(|p| format!("{}@{}", p.move_id, p.version))
            .unwrap_or_else(|| "Independent — no source".into()),
        match &r.attributes {
            None => "Unmodeled".into(),
            Some(v) if v.is_empty() => "Defined-empty".into(),
            Some(v) => format!("Defined ({} entries), page {n}", v.len()),
        },
        r.attributes
            .iter()
            .flatten()
            .skip((n - 1) * SIZE)
            .take(SIZE)
            .enumerate()
            .map(|(i, a)| format!(
                "Position {}: {}",
                (n - 1) * SIZE + i + 1,
                assignment_text(a)
            ))
            .collect::<Vec<_>>()
            .join("\n\n"),
        r.mappings
            .iter()
            .skip((n - 1) * SIZE)
            .take(SIZE)
            .map(|m| format!(
                "{} → {}@{}",
                m.shared_attribute_id, m.local_attribute.attribute_id, m.local_attribute.version
            ))
            .collect::<Vec<_>>()
            .join("\n")
    )
}
fn ruleset_text(r: &RulesetRecord, n: usize) -> String {
    format!(
        "{} · {}@{}\n{}\nExpectations: {}\n{}\nExact stat roster: {}\nExact chart: {}\nMaximum moves: {}\nRepeat policy: {:?}\nInert notes: {}",
        r.fields.display_name,
        r.id,
        r.revision,
        r.fields.description,
        match &r.expectations {
            None => "Unmodeled".into(),
            Some(v) if v.is_empty() => "Defined-empty".into(),
            Some(v) => format!("Defined ({} entries), page {n}", v.len()),
        },
        r.expectations
            .iter()
            .flatten()
            .skip((n - 1) * SIZE)
            .take(SIZE)
            .enumerate()
            .map(|(i, e)| format!(
                "Position {}: {}\n{}; NotApplicable {}; additional restrictions: {}",
                (n - 1) * SIZE + i + 1,
                attribute_text(&e.definition),
                if e.required {
                    "Required Known"
                } else {
                    "Optional"
                },
                if e.allow_not_applicable {
                    "allowed if optional"
                } else {
                    "not allowed"
                },
                constraint_text(&e.constraints)
            ))
            .collect::<Vec<_>>()
            .join("\n\n"),
        r.stat_roster
            .as_ref()
            .map(|p| format!("{}@{}", p.roster_id, p.version))
            .unwrap_or_else(|| "None".into()),
        r.chart
            .as_ref()
            .map(|p| format!("{}@{}", p.chart_id, p.version))
            .unwrap_or_else(|| "None".into()),
        r.max_moves
            .map(|n| n.to_string())
            .unwrap_or_else(|| "Unspecified — no default".into()),
        r.repeat_policy,
        r.notes
    )
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
#[template(path = "move_page.html")]
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
    ready: bool,
    reviewed: String,
    draft: Values,
    previous_page: String,
    next_page: String,
    baseline: String,
    review_fingerprint: String,
    notice: String,
    validation_action: String,
    validation_revision: String,
    text_scoped: bool,
}
fn choice(value: &str, label: &str) -> Choice {
    Choice {
        value: value.into(),
        label: label.into(),
    }
}
impl Page {
    fn new(ctx: &Context, draft: Values, failure: Option<&Error>) -> Self {
        Self {
            title: match ctx.kind {
                Kind::Attribute => "Move attribute",
                Kind::Move => "Move",
                Kind::Ruleset => "World ruleset",
            }
            .into(),
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
            ready: false,
            reviewed: String::new(),
            draft,
            previous_page: String::new(),
            next_page: String::new(),
            baseline: String::new(),
            review_fingerprint: String::new(),
            notice: String::new(),
            validation_action: String::new(),
            validation_revision: String::new(),
            text_scoped: false,
        }
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
            choices.push(choice(
                &value,
                &format!("Retained submitted choice: {value}"),
            ));
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
    fn check(&mut self, name: &str, label: &str, help: &str) {
        self.control(name, label, "checkbox", help, vec![]);
    }
    fn select(&mut self, name: &str, label: &str, help: &str, choices: Vec<Choice>) {
        self.control(name, label, "select", help, choices);
    }
    fn numeric(&mut self) {
        self.text("minimum","Minimum (inclusive)","Optional exact signed decimal, at most six fractional digits. Blank is unbounded; exponents are not accepted.");
        self.text(
            "maximum",
            "Maximum (inclusive)",
            "Optional exact signed decimal. Blank is unbounded.",
        );
        self.check(
            "whole_only",
            "Whole numbers only",
            "Restricts Known numbers only; no rounding or clamping.",
        );
    }
    fn paginate(&mut self, url: &str, n: usize, len: usize) {
        if n > 1 {
            if self.editable {
                self.previous_page = format!("page:{}", n - 1);
            } else {
                self.link("Previous page", format!("{url}?page={}", n - 1));
            }
        }
        if n * SIZE < len {
            if self.editable {
                self.next_page = format!("page:{}", n + 1);
            } else {
                self.link("Next page", format!("{url}?page={}", n + 1));
            }
        }
    }
}
fn normalized_draft(p: &Page) -> Values {
    let mut d: Values = p
        .controls
        .iter()
        .filter(|c| c.name != "next_text_field")
        .map(|c| (c.name.clone(), val(&p.draft, &c.name).into()))
        .collect();
    if p.text_scoped {
        d.insert("text_field".into(), val(&p.draft, "text_field").into());
    }
    d
}
fn baseline(p: &Page) -> Result<String, Error> {
    digest(&normalized_draft(p))
}
fn response(s: &AppState, h: &HeaderMap, mut p: Page, status: StatusCode) -> Response {
    let (csrf, set) = match csrf_for(h) {
        Ok(v) => v,
        Err(_) => return authoring::read_error(Error::Unavailable),
    };
    p.csrf = csrf.clone();
    let mut r = no_store(render_status(p, status));
    if set {
        append_cookie(&mut r, csrf_cookie(&csrf, s.cookie_secure));
    }
    r
}
fn draft_for(old: Option<&Record>) -> Values {
    let mut d = Values::from([
        (
            "expected_revision".into(),
            old.map_or(0, Record::revision).to_string(),
        ),
        ("page".into(), "1".into()),
        ("operation".into(), "keep".into()),
        ("state".into(), "unmodeled".into()),
        ("repeat_policy".into(), "unspecified".into()),
        ("value_state".into(), "unknown".into()),
    ]);
    if let Some(r) = old {
        d.insert("display_name".into(), r.fields().display_name.clone());
        d.insert("description".into(), r.fields().description.clone());
        match r {
            Record::Attribute(a) => {
                d.insert("unit".into(), a.unit.clone());
                d.insert(
                    "minimum".into(),
                    a.constraints
                        .minimum
                        .map(|n| n.to_string())
                        .unwrap_or_default(),
                );
                d.insert(
                    "maximum".into(),
                    a.constraints
                        .maximum
                        .map(|n| n.to_string())
                        .unwrap_or_default(),
                );
                d.insert(
                    "whole_only".into(),
                    if a.constraints.whole_only { "yes" } else { "" }.into(),
                );
            }
            Record::Move(m) => {
                d.insert(
                    "state".into(),
                    if m.attributes.is_some() {
                        "defined"
                    } else {
                        "unmodeled"
                    }
                    .into(),
                );
            }
            Record::Ruleset(r) => {
                d.insert(
                    "state".into(),
                    if r.expectations.is_some() {
                        "defined"
                    } else {
                        "unmodeled"
                    }
                    .into(),
                );
                d.insert(
                    "stat_roster".into(),
                    r.stat_roster
                        .as_ref()
                        .map(|p| format!("{}@{}", p.roster_id, p.version))
                        .unwrap_or_default(),
                );
                d.insert(
                    "chart".into(),
                    r.chart
                        .as_ref()
                        .map(|p| format!("{}@{}", p.chart_id, p.version))
                        .unwrap_or_default(),
                );
                d.insert(
                    "max_moves".into(),
                    r.max_moves.map(|n| n.to_string()).unwrap_or_default(),
                );
                d.insert(
                    "repeat_policy".into(),
                    match r.repeat_policy {
                        RepeatPolicy::Unspecified => "unspecified",
                        RepeatPolicy::Allow => "allow",
                        RepeatPolicy::Forbid => "forbid",
                    }
                    .into(),
                );
                d.insert("notes".into(), r.notes.clone());
            }
        }
    }
    d
}
async fn initial(s: &AppState, ctx: &Context) -> Result<Values, Error> {
    let old = ctx.saved(s).await?;
    let mut d = draft_for(old.as_ref());
    if ctx.kind == Kind::Ruleset {
        load_prose(&mut d, old.as_ref(), "description")?;
    }
    if ctx.mode == Mode::Adopt {
        match old {
            Some(Record::Attribute(a)) => {
                if let Some(p) = a.source {
                    d.insert(
                        "target_version".into(),
                        s.moves
                            .attribute(None, &p.attribute_id)
                            .await?
                            .revision
                            .to_string(),
                    );
                }
            }
            Some(Record::Move(m)) => {
                if let Some(p) = m.source {
                    d.insert(
                        "target_version".into(),
                        s.moves.detail(None, &p.move_id).await?.revision.to_string(),
                    );
                }
            }
            _ => {}
        }
    }
    Ok(d)
}
async fn attribute_choices(
    s: &AppState,
    world: Option<&str>,
    n: usize,
) -> Result<Vec<Choice>, Error> {
    let mut choices = vec![choice("", "Choose an exact attribute")];
    choices.extend(
        s.moves
            .attributes(world)
            .await?
            .into_iter()
            .skip((n - 1) * SIZE)
            .take(SIZE)
            .map(|a| {
                choice(
                    &format!("{}@{}", a.id, a.revision),
                    &format!("{} · {}@{}", a.fields.display_name, a.id, a.revision),
                )
            }),
    );
    Ok(choices)
}
async fn make_page(
    s: &AppState,
    h: &HeaderMap,
    ctx: &Context,
    draft: Values,
    failure: Option<&Error>,
) -> Result<Page, Error> {
    let mut p = Page::new(ctx, draft, failure);
    // A malformed submitted page is itself retained, while display uses page one.
    let n = number(&p.draft).unwrap_or(1);
    let old = ctx.saved(s).await?;
    let url = ctx.pretty(s, old.as_ref()).await?;
    p.cancel = url.clone();
    let caps = s
        .authoring
        .capabilities(session(h).as_deref(), ctx.world())
        .await?;
    let allowed = if ctx.world().is_some() {
        caps.can_edit_world
    } else {
        caps.can_edit_library
    };
    p.editable = ctx.mode != Mode::Detail && (allowed || failure.is_some());
    if let Some(w) = ctx.world() {
        let world = s.authoring.world_detail(w).await?;
        let root = format!("/worlds/{}", addresses::segment(&world.fields.slug));
        p.link("World", &root);
        p.link("World moves", format!("{root}/moves"));
        p.link("World move attributes", format!("{root}/move-attributes"));
        p.link("World ruleset", format!("{root}/ruleset"));
    }
    p.link("Shared moves", "/moves");
    p.link("Shared move attributes", "/move-attributes");
    if let Some(r) = &old {
        p.title = format!("{} — {}", p.title, r.fields().display_name);
        p.panel("Current saved state", r.text(n));
        p.link("History", format!("{url}/history"));
        if let Record::Move(m) = r {
            p.link(
                "Card fields and rules",
                format!(
                    "{}/cards/move/{}",
                    m.world_id
                        .as_ref()
                        .map(|w| format!("/worlds/{w}"))
                        .unwrap_or_default(),
                    m.id
                ),
            );
        }
        if ctx.mode == Mode::Detail && allowed {
            p.link("Edit", format!("{url}/edit"));
            let source = match r {
                Record::Attribute(a) => a.source.is_some(),
                Record::Move(m) => m.source.is_some(),
                _ => false,
            };
            if source {
                p.link("Review source adoption", format!("{url}/adopt"));
            }
        }
        match r {
            Record::Attribute(a) => {
                if let Some(pin) = &a.source {
                    p.link(
                        "Exact shared source",
                        format!(
                            "/move-attributes/{}/history/{}",
                            pin.attribute_id, pin.version
                        ),
                    );
                }
            }
            Record::Move(m) => {
                if let Some(pin) = &m.source {
                    p.link(
                        "Exact shared source",
                        format!("/moves/{}/history/{}", pin.move_id, pin.version),
                    );
                }
                for a in m
                    .attributes
                    .iter()
                    .flatten()
                    .skip((n - 1) * SIZE)
                    .take(SIZE)
                {
                    p.link(
                        format!(
                            "Exact attribute: {} revision {}",
                            a.definition.fields.display_name, a.definition.revision
                        ),
                        format!(
                            "{}/{}/history/{}",
                            ctx.root().replace("/moves", "/move-attributes"),
                            a.definition.id,
                            a.definition.revision
                        ),
                    );
                }
                if let Some(w) = ctx.world() {
                    if let Some(head) = s.moves.ruleset(w).await? {
                        let rules = if val(&p.draft, "ruleset_revision").is_empty() {
                            head
                        } else {
                            s.moves
                                .ruleset_version(
                                    w,
                                    &head.id,
                                    positive(
                                        val(&p.draft, "ruleset_revision"),
                                        "ruleset_revision",
                                    )?,
                                )
                                .await?
                        };
                        p.draft
                            .insert("ruleset_revision".into(), rules.revision.to_string());
                        if ctx.mode == Mode::Detail {
                            p.validation_action = url.clone();
                            p.validation_revision = rules.revision.to_string();
                            p.link(
                                "Read this exact move and report",
                                format!(
                                    "{url}/history/{}?ruleset_revision={}",
                                    m.revision, rules.revision
                                ),
                            );
                        }
                        let report = game_data::rulesets::validate_move(m, &rules);
                        p.panel(
                            "Authoring validation — NOT combat readiness",
                            report_text(&report),
                        );
                        p.link(
                            format!("Selected exact ruleset {}@{}", rules.id, rules.revision),
                            format!("/worlds/{w}/ruleset/history/{}", rules.revision),
                        );
                    } else {
                        p.panel(
                            "Authoring validation — NOT combat readiness",
                            "World ruleset is unconfigured. No authoring validation was run.",
                        );
                    }
                }
            }
            Record::Ruleset(r) => p.panel(
                "Saved configuration diagnostics — current policy — NOT combat readiness",
                ruleset_report_text(r),
            ),
        }
    } else if ctx.kind == Kind::Ruleset {
        p.panel(
            "Current saved state",
            "Unconfigured — this world has no ruleset configuration.",
        );
        if ctx.mode == Mode::Detail && allowed {
            p.link("Configure ruleset", format!("{url}/edit"));
        }
    }
    let mut count = old.as_ref().map_or(0, Record::len);
    if let Some(Record::Move(m)) = &old {
        count = count.max(m.mappings.len());
    }
    if p.editable {
        p.text(
            "display_name",
            "Display name",
            "Required, up to 100 characters. Readable scoped slug is generated.",
        );
        if ctx.kind == Kind::Ruleset {
            p.text_scoped = true;
            let field = text_field(&p.draft).unwrap_or("description").to_owned();
            p.hidden
                .insert("text_field".into(), val(&p.draft, "text_field").into());
            if let Some(error) = p.errors.remove(&field) {
                p.errors.insert("text".into(), error);
            }
            p.control("text",if field=="notes" { "Inert interaction and environment notes" } else { "Description" },"textarea",if field=="notes" { "Optional inert plain text, up to 2000 UTF-8 bytes (not characters). Other prose stays saved and read-only. Save or discard this draft before loading another field." } else { "Required plain text, up to 2000 characters. Only this loaded prose field is submitted. New rulesets start with empty notes; save the Description before editing notes." },vec![]);
            let mut choices = vec![choice("description", "Description")];
            if old.is_some() {
                choices.push(choice("notes", "Inert interaction and environment notes"));
            }
            p.select("next_text_field","Text field to load","Choose a field, then use Load selected text field. Save changes or explicitly discard the draft before switching; other saved text is never resubmitted.",choices);
            for key in ["description", "notes"] {
                if let Some(raw) = p.draft.get(key).cloned() {
                    p.panel(
                        format!("Rejected unexpected text field (read-only): {key}"),
                        raw,
                    );
                }
            }
        } else {
            p.control(
                "description",
                "Description",
                "textarea",
                "Required plain text, up to 2000 characters.",
                vec![],
            );
        }
        if ctx.kind == Kind::Attribute {
            p.text(
                "unit",
                "Descriptive unit",
                "Optional descriptive text, up to 100 UTF-8 bytes; not executable mechanics.",
            );
            p.numeric();
        }
        if ctx.mode == Mode::New && ctx.world().is_some() {
            if ctx.kind == Kind::Attribute {
                p.select(
                    "source_pin",
                    "Shared source and exact version",
                    "Optional provenance; local fields remain explicitly entered.",
                    attribute_choices(s, None, n).await?,
                );
                count = count.max(s.moves.attributes(None).await?.len());
            } else {
                let mut choices = vec![choice("", "Independent — no shared source")];
                let moves = s.moves.list(None).await?;
                count = count.max(moves.len());
                choices.extend(moves.into_iter().skip((n - 1) * SIZE).take(SIZE).map(|m| {
                    choice(
                        &format!("{}@{}", m.id, m.revision),
                        &format!("{} · {}@{}", m.fields.display_name, m.id, m.revision),
                    )
                }));
                p.select("source_pin","Shared source and exact version","Optional provenance. Copy attributes later through explicit adoption mappings.",choices);
            }
            p.text("source_exact","Other exact shared source pin","Optional identity@version overrides the catalog choice. Historical revisions are supported.");
        }
        if ctx.mode == Mode::Adopt {
            p.text("target_version","Shared source version","Pinned or newer revision of this same source. Copying at the currently pinned source version is supported.");
            if ctx.kind == Kind::Move {
                p.check("apply_source","Replace the whole attribute collection from source","Unchecked keeps all local values. Replacement uses explicit injective mappings only; review every removal and unmapped source below.");
                p.select("mapping_operation","Mapping operation","Mappings are assembled from verified saved mappings, one operation at a time. Keep-local adoption can save mappings before a later whole replacement.",vec![choice("","Keep saved mappings"),choice("set","Set shared-to-local mapping"),choice("remove","Remove mapping for shared identity"),choice("clear","Clear all mappings")]);
                p.text("shared_attribute_id","Shared attribute identity","Canonical shared attribute identity from the selected source revision; never inferred from names.");
                p.select("attribute_pin","Local attribute and exact version","For set: explicitly select a local definition sourced from this shared identity.",attribute_choices(s,ctx.world(),n).await?);
                p.text(
                    "exact_pin",
                    "Other exact attribute pin",
                    "Optional identity@version overrides the catalog choice.",
                );
                count = count.max(s.moves.attributes(ctx.world()).await?.len());
            }
        } else if ctx.kind != Kind::Attribute {
            p.select("state",if ctx.kind==Kind::Move { "Attribute collection state" } else { "Expectation collection state" },"Unmodeled differs from Defined-empty. Selecting Unmodeled explicitly removes all saved entries after review.",vec![choice("unmodeled","Unmodeled"),choice("defined","Defined")]);
            p.select("operation","Collection operation","One operation per save, assembled from verified saved state. Add uses the entered value/policy; Change replaces only the definition and retains its saved value/policy; Value edits the selected entry's value/policy. Keep ignores entry controls. No fixed global entry count.",vec![choice("keep","Keep entries / define empty"),choice("add","Add exact attribute"),choice("remove","Remove entry at position"),choice("change","Change exact pin at position"),choice("value","Edit value / expectation at position"),choice("reorder","Reorder entry")]);
            p.select("attribute_pin","Attribute and exact version","At most 25 catalog choices per page. Nothing advances existing pins automatically.",attribute_choices(s,ctx.world(),n).await?);
            p.text("exact_pin","Other exact attribute pin","Optional identity@version overrides the catalog choice; use history for older revisions.");
            p.text("position","Existing entry position","One-based position to remove, change, edit or reorder. Current saved page shows positions.");
            p.text(
                "destination",
                "Destination position",
                "For reorder, required. For add, blank appends.",
            );
            if ctx.kind == Kind::Move {
                p.select("value_state","Numeric value state","Unknown, NotApplicable and Known zero are distinct. No Power or Accuracy is mandatory.",vec![choice("unknown","Unknown"),choice("not_applicable","NotApplicable"),choice("known","Known")]);
                p.text("value","Known numeric value","Required only for Known on add/value operations. Exact signed decimal. A change-pin operation preserves the saved value.");
            } else {
                p.check("required","Required Known value","Absent or Unknown is incomplete; required NotApplicable contradicts the configuration.");
                p.check(
                    "allow_not_applicable",
                    "Allow NotApplicable when optional",
                    "This never makes a required NotApplicable value valid.",
                );
                p.numeric();
                p.text(
                    "stat_roster",
                    "Exact world stat roster pin",
                    "Optional identity@version; blank means no roster context. No silent upgrade.",
                );
                p.text(
                    "chart",
                    "Exact world chart pin",
                    "Optional identity@version; blank means no chart context. No silent upgrade.",
                );
                if let Some(w) = ctx.world() {
                    if let Some(r) = s.stats.world_roster(w).await? {
                        p.panel(
                            "Available current roster pin",
                            format!("{}@{} (not selected automatically)", r.id, r.revision),
                        );
                    }
                    if let Some(c) = s.charts.world_chart(w).await? {
                        p.panel(
                            "Available current chart pin",
                            format!("{}@{} (not selected automatically)", c.id, c.revision),
                        );
                    }
                }
                p.text("max_moves","Maximum moves (optional)","Optional positive whole number. Blank is unspecified; there is no four-move default.");
                p.select(
                    "repeat_policy",
                    "Repeat policy",
                    "Declarative policy only.",
                    vec![
                        choice("unspecified", "Unspecified"),
                        choice("allow", "Allow"),
                        choice("forbid", "Forbid"),
                    ],
                );
            }
            count = count.max(s.moves.attributes(ctx.world()).await?.len());
        }
        p.text(
            "reason",
            "Change reason (optional)",
            "One line, up to 500 characters, recorded in public history.",
        );
        for k in ["expected_revision", "page"] {
            p.hidden.insert(k.into(), val(&p.draft, k).into());
        }
        if matches!(failure, Some(Error::Conflict)) {
            p.reviewed = old.as_ref().map_or(0, Record::revision).to_string();
        }
        p.baseline = if p.draft.contains_key("page_baseline") {
            val(&p.draft, "page_baseline").into()
        } else {
            baseline(&p)?
        };
    }
    let page_url = if p.editable {
        ctx.action()
    } else if let Some(r) = &old {
        format!("{url}/history/{}", r.revision())
    } else {
        url
    };
    p.paginate(&page_url, n, count);
    if !p.editable && !val(&p.draft, "ruleset_revision").is_empty() {
        let revision = val(&p.draft, "ruleset_revision").to_owned();
        for link in &mut p.links {
            if link.label == "Previous page" || link.label == "Next page" {
                link.url.push_str(&format!("&ruleset_revision={revision}"));
            }
        }
    }
    Ok(p)
}
fn report_text(r: &ValidationReport) -> String {
    format!(
        "Validator policy version {}\nExact inputs:\n{}\nContradictions:\n{}\nIncomplete:\n{}\nNot evaluated:\n{}",
        r.validator_version,
        r.inputs
            .iter()
            .map(|i| format!("{} {}@{}", i.kind, i.id, i.version))
            .collect::<Vec<_>>()
            .join("\n"),
        if r.contradictions.is_empty() {
            "None".into()
        } else {
            r.contradictions.join("\n")
        },
        if r.incomplete.is_empty() {
            "None".into()
        } else {
            r.incomplete.join("\n")
        },
        r.not_evaluated.join("\n")
    )
}
fn ruleset_report_text(r: &RulesetRecord) -> String {
    format!(
        "Ad-hoc configuration-only diagnostics for this exact ruleset, evaluated using the current validator policy.\n{}",
        report_text(&validate_ruleset_with_policy(r, CURRENT_VALIDATOR_VERSION))
    )
}
async fn catalog(State(s): State<AppState>, h: HeaderMap, uri: Uri, query: QueryInput) -> Response {
    no_store(finish(
        async {
            let Query(q) = query.map_err(|_| invalid("page", "Check the catalog page."))?;
            let n = number(&q)?;
            let mut ctx = Context::new(&Values::new(), uri.path());
            ctx.world = uri
                .path()
                .strip_prefix("/worlds/")
                .and_then(|v| v.split_once('/'))
                .map(|v| v.0.into());
            ctx.mode = Mode::New;
            let rows = if ctx.kind == Kind::Attribute {
                s.moves
                    .attributes(ctx.world())
                    .await?
                    .into_iter()
                    .map(Record::Attribute)
                    .collect::<Vec<_>>()
            } else {
                s.moves
                    .list(ctx.world())
                    .await?
                    .into_iter()
                    .map(Record::Move)
                    .collect()
            };
            let caps = s
                .authoring
                .capabilities(session(&h).as_deref(), ctx.world())
                .await?;
            let url = ctx.pretty_root(&s).await?;
            let mut p = Page::new(&ctx, Values::new(), None);
            p.title = if ctx.kind == Kind::Attribute {
                "Move attributes"
            } else {
                "Moves"
            }
            .into();
            p.cancel = "/".into();
            p.link("Shared moves", "/moves");
            p.link("Shared move attributes", "/move-attributes");
            if let Some(w) = ctx.world() {
                let world = s.authoring.world_detail(w).await?;
                let root = format!("/worlds/{}", addresses::segment(&world.fields.slug));
                p.link("World", &root);
                p.link("World moves", format!("{root}/moves"));
                p.link("World move attributes", format!("{root}/move-attributes"));
                p.link("World ruleset", format!("{root}/ruleset"));
            }
            if if ctx.world().is_some() {
                caps.can_edit_world
            } else {
                caps.can_edit_library
            } {
                p.link(
                    if ctx.kind == Kind::Attribute {
                        "Create move attribute"
                    } else {
                        "Create move"
                    },
                    format!("{url}/new"),
                );
            }
            for r in rows.iter().skip((n - 1) * SIZE).take(SIZE) {
                p.link(&r.fields().display_name, ctx.pretty(&s, Some(r)).await?);
                p.panel(
                    &r.fields().display_name,
                    format!("{}\nRevision {}", r.fields().description, r.revision()),
                );
            }
            p.paginate(&url, n, rows.len());
            Ok(render(p))
        }
        .await,
    ))
}
async fn editor(
    State(s): State<AppState>,
    h: HeaderMap,
    path: Option<Path<Values>>,
    uri: Uri,
    query: QueryInput,
) -> Response {
    no_store(finish(
        async {
            let ctx = Context::new(&path.map(|Path(v)| v).unwrap_or_default(), uri.path());
            if ctx.mode != Mode::Detail {
                ctx.permission(&s, &h).await?;
            }
            let Query(q) = query
                .map_err(|_| invalid("query", "Check the page and exact revision selection."))?;
            let mut d = initial(&s, &ctx).await?;
            if ctx.kind == Kind::Ruleset && q.contains_key("text_field") {
                load_prose(&mut d, ctx.saved(&s).await?.as_ref(), val(&q, "text_field"))?;
            }
            for k in [
                "page",
                "source_pin",
                "source_exact",
                "target_version",
                "ruleset_revision",
            ] {
                if let Some(v) = q.get(k) {
                    d.insert(k.into(), v.clone());
                }
            }
            number(&d)?;
            let p = make_page(&s, &h, &ctx, d, None).await?;
            Ok(response(&s, &h, p, StatusCode::OK))
        }
        .await,
    ))
}

fn position(d: &Values, len: usize, key: &str, append: bool) -> Result<usize, Error> {
    let n = usize::try_from(positive(val(d, key), key)?)
        .map_err(|_| invalid(key, "Choose a position within this collection."))?
        - 1;
    if n < len || (append && n == len) {
        Ok(n)
    } else {
        Err(invalid(
            key,
            "Choose a one-based position within this saved collection.",
        ))
    }
}
fn numeric_value(d: &Values) -> Result<NumericValue, Error> {
    match val(d, "value_state") {
        "unknown" => Ok(NumericValue::Unknown),
        "not_applicable" => Ok(NumericValue::NotApplicable),
        "known" => decimal(val(d, "value"), "value")?
            .map(NumericValue::Known)
            .ok_or_else(|| invalid("value", "Known requires an explicit number; zero is valid.")),
        _ => Err(invalid(
            "value_state",
            "Select Known, Unknown or NotApplicable.",
        )),
    }
}
async fn selected_attribute(
    s: &AppState,
    ctx: &Context,
    d: &Values,
) -> Result<MoveAttributeDefinition, Error> {
    let pin = attribute_pin(chosen(d, "exact_pin", "attribute_pin"), "attribute_pin")?;
    let head = reference(
        s.moves.attribute(ctx.world(), &pin.attribute_id).await,
        "attribute_pin",
    )?;
    let a = reference(
        s.moves
            .attribute_version(ctx.world(), &pin.attribute_id, pin.version)
            .await,
        "attribute_pin",
    )?;
    if head.archived_at.is_some() || a.archived_at.is_some() {
        return Err(invalid(
            "attribute_pin",
            "Choose an unarchived exact definition.",
        ));
    }
    Ok(a)
}
// Collection transformations are transport assembly, not numeric/domain policy.
// Every proposal starts from the verified saved collection after expected checking.
async fn move_attributes(
    s: &AppState,
    ctx: &Context,
    d: &Values,
    old: Option<&MoveRecord>,
) -> Result<Option<Vec<MoveAttributeValue>>, Error> {
    if val(d, "state") == "unmodeled" {
        return Ok(None);
    }
    if val(d, "state") != "defined" {
        return Err(invalid("state", "Choose Unmodeled or Defined."));
    }
    let mut rows = old.and_then(|m| m.attributes.clone()).unwrap_or_default();
    match val(d, "operation") {
        "keep" | "" => {}
        "add" => {
            let a = MoveAttributeValue {
                definition: selected_attribute(s, ctx, d).await?,
                value: numeric_value(d)?,
            };
            let i = if val(d, "destination").is_empty() {
                rows.len()
            } else {
                position(d, rows.len(), "destination", true)?
            };
            rows.insert(i, a);
        }
        "remove" => {
            rows.remove(position(d, rows.len(), "position", false)?);
        }
        "change" => {
            let i = position(d, rows.len(), "position", false)?;
            rows[i].definition = selected_attribute(s, ctx, d).await?;
        }
        "value" => {
            let i = position(d, rows.len(), "position", false)?;
            rows[i].value = numeric_value(d)?;
        }
        "reorder" => {
            let i = position(d, rows.len(), "position", false)?;
            let to = position(d, rows.len(), "destination", false)?;
            let row = rows.remove(i);
            rows.insert(to, row);
        }
        _ => {
            return Err(invalid(
                "operation",
                "Choose a supported bounded operation.",
            ));
        }
    }
    Ok(Some(rows))
}
async fn expectations(
    s: &AppState,
    ctx: &Context,
    d: &Values,
    old: Option<&RulesetRecord>,
) -> Result<Option<Vec<AttributeExpectation>>, Error> {
    if val(d, "state") == "unmodeled" {
        return Ok(None);
    }
    if val(d, "state") != "defined" {
        return Err(invalid("state", "Choose Unmodeled or Defined."));
    }
    let mut rows = old.and_then(|r| r.expectations.clone()).unwrap_or_default();
    match val(d, "operation") {
        "keep" | "" => {}
        "add" => {
            let e = AttributeExpectation {
                definition: selected_attribute(s, ctx, d).await?,
                required: val(d, "required") == "yes",
                allow_not_applicable: val(d, "allow_not_applicable") == "yes",
                constraints: constraints(d)?,
            };
            let i = if val(d, "destination").is_empty() {
                rows.len()
            } else {
                position(d, rows.len(), "destination", true)?
            };
            rows.insert(i, e);
        }
        "remove" => {
            rows.remove(position(d, rows.len(), "position", false)?);
        }
        "change" => {
            let i = position(d, rows.len(), "position", false)?;
            rows[i].definition = selected_attribute(s, ctx, d).await?;
        }
        "value" => {
            let i = position(d, rows.len(), "position", false)?;
            rows[i].required = val(d, "required") == "yes";
            rows[i].allow_not_applicable = val(d, "allow_not_applicable") == "yes";
            rows[i].constraints = constraints(d)?;
        }
        "reorder" => {
            let i = position(d, rows.len(), "position", false)?;
            let to = position(d, rows.len(), "destination", false)?;
            let row = rows.remove(i);
            rows.insert(to, row);
        }
        _ => {
            return Err(invalid(
                "operation",
                "Choose a supported bounded operation.",
            ));
        }
    }
    Ok(Some(rows))
}
fn attribute_edit(d: &Values) -> Result<AttributeEdit, Error> {
    Ok(AttributeEdit {
        expected_revision: expected(d)?,
        fields: fields(d),
        unit: val(d, "unit").into(),
        constraints: constraints(d)?,
        reason: reason(d),
    })
}
fn adoption_edit(d: &Values, old: &MoveRecord) -> Result<MoveAdoptEdit, Error> {
    let mut mappings = old.mappings.clone();
    let shared = val(d, "shared_attribute_id");
    match val(d, "mapping_operation") {
        "" => {}
        "clear" => mappings.clear(),
        "remove" | "set" => {
            if !game_data::types::valid_id(shared) {
                return Err(invalid(
                    "shared_attribute_id",
                    "Choose a canonical shared attribute identity.",
                ));
            }
            mappings.retain(|m| m.shared_attribute_id != shared);
            if val(d, "mapping_operation") == "set" {
                mappings.push(AttributeMapping {
                    shared_attribute_id: shared.into(),
                    local_attribute: attribute_pin(
                        chosen(d, "exact_pin", "attribute_pin"),
                        "attribute_pin",
                    )?,
                });
            }
        }
        _ => {
            return Err(invalid(
                "mapping_operation",
                "Choose a supported mapping operation.",
            ));
        }
    }
    Ok(MoveAdoptEdit {
        expected_revision: expected(d)?,
        target_version: positive(val(d, "target_version"), "target_version")?,
        fields: fields(d),
        apply_source: val(d, "apply_source") == "yes",
        mappings,
        reason: reason(d),
    })
}
struct Proposal {
    row: Record,
    panels: Vec<Panel>,
}
async fn proposal(
    s: &AppState,
    ctx: &Context,
    d: &Values,
    old: Option<&Record>,
) -> Result<Proposal, Error> {
    game_data::authoring::validate_reason(reason(d).as_deref()).map_err(Error::Invalid)?;
    let n = number(d)?;
    let mut composed = d.clone();
    if ctx.kind == Kind::Ruleset {
        let (description, notes) = ruleset_prose(d, old)?;
        composed.insert("description".into(), description);
        composed.insert("notes".into(), notes);
    }
    let fields = preview_fields(&composed, old.map(Record::fields))?;
    let mut panels = vec![];
    let mut add = |label: &str, text: String| {
        panels.push(Panel {
            label: label.into(),
            text,
        })
    };
    if ctx.mode == Mode::Adopt {
        let w = ctx.world().ok_or(Error::NotFound)?;
        let row = match old {
            Some(Record::Attribute(_)) => {
                let p = s
                    .moves
                    .preview_attribute_adoption(
                        w,
                        &ctx.id,
                        positive(val(d, "target_version"), "target_version")?,
                        &attribute_edit(d)?,
                    )
                    .await?;
                add("Old shared source", attribute_text(&p.old_source));
                add("Current local definition", attribute_text(&p.entry));
                add("New shared source", attribute_text(&p.new_source));
                Record::Attribute(p.proposed)
            }
            Some(Record::Move(m)) => {
                let p = s
                    .moves
                    .adoption_preview(w, &ctx.id, &adoption_edit(d, m)?)
                    .await?;
                add("Old shared source", move_text(&p.old_source, n));
                add("Current local move", move_text(&p.entry, n));
                add("New shared source", move_text(&p.new_source, n));
                // Every removed/unmapped assignment is named, even beyond the state display page.
                add(
                    "All removed local assignments",
                    if p.removed.is_empty() {
                        "None".into()
                    } else {
                        p.removed
                            .iter()
                            .map(assignment_text)
                            .collect::<Vec<_>>()
                            .join("\n\n")
                    },
                );
                add(
                    "All unmapped source assignments",
                    if p.unmapped.is_empty() {
                        "None".into()
                    } else {
                        p.unmapped
                            .iter()
                            .map(assignment_text)
                            .collect::<Vec<_>>()
                            .join("\n\n")
                    },
                );
                if val(d, "apply_source") == "yes" {
                    add(
                        "Whole replacement — every proposed assignment",
                        match &p.proposed.attributes {
                            None => {
                                "Unmodeled — the entire local collection will be unmodeled.".into()
                            }
                            Some(v) if v.is_empty() => {
                                "Defined-empty — no assignments survive replacement.".into()
                            }
                            Some(v) => v
                                .iter()
                                .enumerate()
                                .map(|(i, a)| format!("Position {}: {}", i + 1, assignment_text(a)))
                                .collect::<Vec<_>>()
                                .join("\n\n"),
                        },
                    );
                }
                Record::Move(p.proposed)
            }
            _ => return Err(Error::NotFound),
        };
        add("Proposed state", row.text(n));
        return Ok(Proposal { row, panels });
    }
    let revision = old.map_or(1, Record::revision);
    let id = if ctx.mode == Mode::New {
        PROVISIONAL
    } else if let Some(Record::Ruleset(r)) = old {
        &r.id
    } else if ctx.kind == Kind::Ruleset {
        PROVISIONAL
    } else {
        &ctx.id
    };
    let source = chosen(d, "source_exact", "source_pin");
    let mut row = match ctx.kind {
        Kind::Attribute => {
            let source = if let Some(Record::Attribute(a)) = old {
                a.source.clone()
            } else if source.is_empty() {
                None
            } else {
                let pin = attribute_pin(source, "source_pin")?;
                let head = reference(
                    s.moves.attribute(None, &pin.attribute_id).await,
                    "source_pin",
                )?;
                let a = reference(
                    s.moves
                        .attribute_version(None, &pin.attribute_id, pin.version)
                        .await,
                    "source_pin",
                )?;
                if head.archived_at.is_some() || a.archived_at.is_some() {
                    return Err(invalid(
                        "source_pin",
                        "Choose an unarchived source revision.",
                    ));
                }
                Some(pin)
            };
            let a = MoveAttributeDefinition {
                id: id.into(),
                world_id: ctx.world.clone(),
                source,
                fields,
                unit: val(d, "unit").trim().into(),
                revision,
                archived_at: if let Some(Record::Attribute(a)) = old {
                    a.archived_at
                } else {
                    None
                },
                constraints: constraints(d)?,
            };
            a.validate().map_err(Error::Invalid)?;
            Record::Attribute(a)
        }
        Kind::Move => {
            let previous = if let Some(Record::Move(m)) = old {
                Some(m)
            } else {
                None
            };
            let source = if let Some(m) = previous {
                m.source.clone()
            } else if source.is_empty() {
                None
            } else {
                let (move_id, version) = exact(source, "source_pin")?;
                let head = reference(s.moves.detail(None, &move_id).await, "source_pin")?;
                let m = reference(s.moves.version(None, &move_id, version).await, "source_pin")?;
                if head.archived_at.is_some() || m.archived_at.is_some() {
                    return Err(invalid(
                        "source_pin",
                        "Choose an unarchived source revision.",
                    ));
                }
                Some(MovePin { move_id, version })
            };
            let m = MoveRecord {
                id: id.into(),
                world_id: ctx.world.clone(),
                source,
                fields,
                revision,
                archived_at: previous.and_then(|m| m.archived_at),
                attributes: move_attributes(s, ctx, d, previous).await?,
                mappings: previous.map(|m| m.mappings.clone()).unwrap_or_default(),
            };
            m.validate().map_err(Error::Invalid)?;
            Record::Move(m)
        }
        Kind::Ruleset => {
            let w = ctx.world().ok_or(Error::NotFound)?;
            let previous = if let Some(Record::Ruleset(r)) = old {
                Some(r)
            } else {
                None
            };
            let stat_roster = if val(d, "stat_roster").is_empty() {
                None
            } else {
                let (roster_id, version) = exact(val(d, "stat_roster"), "stat_roster")?;
                reference(
                    s.stats.roster_version(w, &roster_id, version).await,
                    "stat_roster",
                )?;
                Some(game_data::stats::RosterPin { roster_id, version })
            };
            let chart = if val(d, "chart").is_empty() {
                None
            } else {
                let (chart_id, version) = exact(val(d, "chart"), "chart")?;
                let pin = game_data::charts::ChartPin { chart_id, version };
                let chart_id = &pin.chart_id;
                let c = reference(s.charts.version(Some(w), chart_id, version).await, "chart")?;
                if previous.and_then(|r| r.chart.as_ref()) != Some(&pin) {
                    let head = reference(s.charts.detail(Some(w), chart_id).await, "chart")?;
                    if head.archived_at.is_some() || c.archived_at.is_some() {
                        return Err(invalid("chart", "Choose an unarchived chart revision."));
                    }
                }
                Some(pin)
            };
            let r = RulesetRecord {
                id: id.into(),
                world_id: w.into(),
                revision,
                fields,
                expectations: expectations(s, ctx, d, previous).await?,
                stat_roster,
                chart,
                max_moves: if val(d, "max_moves").is_empty() {
                    None
                } else {
                    Some(positive(val(d, "max_moves"), "max_moves")?)
                },
                repeat_policy: match val(d, "repeat_policy") {
                    "unspecified" => RepeatPolicy::Unspecified,
                    "allow" => RepeatPolicy::Allow,
                    "forbid" => RepeatPolicy::Forbid,
                    _ => {
                        return Err(invalid(
                            "repeat_policy",
                            "Choose Unspecified, Allow or Forbid.",
                        ));
                    }
                },
                notes: val(&composed, "notes").trim().replace("\r\n", "\n"),
            };
            r.validate().map_err(Error::Invalid)?;
            Record::Ruleset(r)
        }
    };
    if let Some(old) = old.filter(|old| *old != &row) {
        let revision = crate::application::authoring::next(old.revision())?;
        match &mut row {
            Record::Attribute(a) => a.revision = revision,
            Record::Move(m) => m.revision = revision,
            Record::Ruleset(r) => r.revision = revision,
        }
    }
    if let Some(old) = old {
        add("Before — exact saved state", old.text(n));
    } else {
        add(
            "Provisional identity — not persisted",
            format!(
                "{PROVISIONAL} is a preview placeholder only. The service allocates the real immutable identity and scoped address when confirmed."
            ),
        );
    }
    if let Some(old) = old {
        let removed = match (old, &row) {
            (Record::Move(a), Record::Move(b)) => a
                .attributes
                .iter()
                .flatten()
                .filter(|a| {
                    !b.attributes
                        .iter()
                        .flatten()
                        .any(|b| a.definition.id == b.definition.id)
                })
                .map(assignment_text)
                .collect::<Vec<_>>(),
            (Record::Ruleset(a), Record::Ruleset(b)) => a
                .expectations
                .iter()
                .flatten()
                .filter(|a| {
                    !b.expectations
                        .iter()
                        .flatten()
                        .any(|b| a.definition.id == b.definition.id)
                })
                .map(|e| attribute_text(&e.definition))
                .collect(),
            _ => vec![],
        };
        add(
            "All removed entries",
            if removed.is_empty() {
                "None".into()
            } else {
                removed.join("\n\n")
            },
        );
    }
    // A bounded operation may target a later position than the current catalog
    // page. Always expose that operation's exact affected data for review.
    if ctx.kind != Kind::Attribute {
        let affected = |record: &Record| -> String {
            let index = val(d, "position")
                .parse::<usize>()
                .ok()
                .and_then(|n| n.checked_sub(1));
            match record {
                Record::Move(m) => {
                    let rows = m.attributes.as_deref().unwrap_or_default();
                    let i = if val(d, "operation") == "add" {
                        if val(d, "destination").is_empty() {
                            rows.len().checked_sub(1)
                        } else {
                            val(d, "destination")
                                .parse::<usize>()
                                .ok()
                                .and_then(|n| n.checked_sub(1))
                        }
                    } else if val(d, "operation") == "reorder" {
                        val(d, "destination")
                            .parse::<usize>()
                            .ok()
                            .and_then(|n| n.checked_sub(1))
                    } else {
                        index
                    };
                    i.and_then(|i| {
                        rows.get(i)
                            .map(|a| format!("Position {}: {}", i + 1, assignment_text(a)))
                    })
                    .unwrap_or_else(|| "No entry at this operation's affected position.".into())
                }
                Record::Ruleset(r) => {
                    let rows = r.expectations.as_deref().unwrap_or_default();
                    let i = if val(d, "operation") == "add" {
                        if val(d, "destination").is_empty() {
                            rows.len().checked_sub(1)
                        } else {
                            val(d, "destination")
                                .parse::<usize>()
                                .ok()
                                .and_then(|n| n.checked_sub(1))
                        }
                    } else if val(d, "operation") == "reorder" {
                        val(d, "destination")
                            .parse::<usize>()
                            .ok()
                            .and_then(|n| n.checked_sub(1))
                    } else {
                        index
                    };
                    i.and_then(|i|rows.get(i).map(|e|format!("Position {}: {}\n{}; allow NotApplicable: {}; additional restrictions: {}",i+1,attribute_text(&e.definition),if e.required { "Required Known" } else { "Optional" },e.allow_not_applicable,constraint_text(&e.constraints)))).unwrap_or_else(||"No expectation at this operation's affected position.".into())
                }
                _ => String::new(),
            }
        };
        if !matches!(val(d, "operation"), "keep" | "") {
            add("Exact proposed operation result", affected(&row));
        }
    }
    add("Proposed state", row.text(n));
    if let Record::Ruleset(r) = &row {
        add(
            "Proposed configuration diagnostics — current policy — NOT combat readiness",
            ruleset_report_text(r),
        );
    }
    Ok(Proposal { row, panels })
}
async fn persist(
    s: &AppState,
    ctx: &Context,
    d: &Values,
    token: &str,
    p: Record,
    old: Option<&Record>,
) -> Result<String, Error> {
    match p {
        Record::Attribute(a) => {
            let r = if ctx.mode == Mode::New {
                s.moves
                    .create_attribute(
                        token,
                        ctx.world(),
                        NewAttribute {
                            fields: fields(d),
                            unit: a.unit,
                            constraints: a.constraints,
                            source: a.source,
                            reason: reason(d),
                        },
                    )
                    .await?
            } else if ctx.mode == Mode::Adopt {
                s.moves
                    .adopt_attribute(
                        token,
                        ctx.world().ok_or(Error::NotFound)?,
                        &ctx.id,
                        positive(val(d, "target_version"), "target_version")?,
                        attribute_edit(d)?,
                    )
                    .await?
            } else {
                s.moves
                    .save_attribute(token, ctx.world(), &ctx.id, attribute_edit(d)?)
                    .await?
            };
            Ok(format!("{}/{}", ctx.root(), r.id))
        }
        Record::Move(m) => {
            let attributes = m.attributes.as_ref().map(|a| {
                a.iter()
                    .map(|a| AttributeValueInput {
                        attribute: a.definition.pin(),
                        value: a.value.clone(),
                    })
                    .collect()
            });
            let r = if ctx.mode == Mode::New {
                s.moves
                    .create(
                        token,
                        ctx.world(),
                        NewMove {
                            fields: fields(d),
                            source: m.source,
                            attributes,
                            reason: reason(d),
                        },
                    )
                    .await?
            } else if ctx.mode == Mode::Adopt {
                let Some(Record::Move(old)) = old else {
                    return Err(Error::NotFound);
                };
                s.moves
                    .adopt(
                        token,
                        ctx.world().ok_or(Error::NotFound)?,
                        &ctx.id,
                        adoption_edit(d, old)?,
                    )
                    .await?
            } else {
                s.moves
                    .save(
                        token,
                        ctx.world(),
                        &ctx.id,
                        MoveEdit {
                            expected_revision: expected(d)?,
                            fields: fields(d),
                            attributes,
                            reason: reason(d),
                        },
                    )
                    .await?
            };
            Ok(format!("{}/{}", ctx.root(), r.id))
        }
        Record::Ruleset(r) => {
            s.moves
                .save_ruleset(
                    token,
                    ctx.world().ok_or(Error::NotFound)?,
                    RulesetEdit {
                        expected_revision: expected(d)?,
                        fields: r.fields,
                        expectations: r.expectations.map(|v| {
                            v.into_iter()
                                .map(|e| ExpectationInput {
                                    attribute: e.definition.pin(),
                                    required: e.required,
                                    allow_not_applicable: e.allow_not_applicable,
                                    constraints: e.constraints,
                                })
                                .collect()
                        }),
                        stat_roster: r.stat_roster,
                        chart: r.chart,
                        max_moves: r.max_moves,
                        repeat_policy: r.repeat_policy,
                        notes: r.notes,
                        reason: reason(d),
                    },
                )
                .await?;
            Ok(ctx.target())
        }
    }
}
async fn submit(
    State(s): State<AppState>,
    h: HeaderMap,
    path: Option<Path<Values>>,
    uri: Uri,
    form: FormInput,
) -> Response {
    let ctx = Context::new(&path.map(|Path(v)| v).unwrap_or_default(), uri.path());
    let mut d = match form {
        Ok(Form(d)) => d,
        Err(e) => return authoring_form_error(e),
    };
    let result: Result<(Option<String>,Vec<Panel>),Error>=async {
        let token=submit_token(&s,&h,val(&d,"csrf"))?; ctx.permission(&s,&h).await?;
        if val(&d,"intent")=="discard" || val(&d,"intent").starts_with("page:") {
            let n=if val(&d,"intent")=="discard" { number(&d)? } else {
                if ctx.kind==Kind::Ruleset {
                    let old=ctx.saved(&s).await?;
                    if expected(&d)?!=old.as_ref().map_or(0,Record::revision) { return Err(Error::Conflict); }
                    ruleset_prose(&d,old.as_ref())?;
                }
                let page=make_page(&s,&h,&ctx,d.clone(),None).await?;
                if val(&d,"page_baseline").is_empty() || val(&d,"page_baseline")!=baseline(&page)? { return Err(invalid("page","Save the current page before navigation, or explicitly discard this page draft. Raw values and reason are retained.")); }
                let raw=val(&d,"intent").strip_prefix("page:").unwrap_or(""); number(&Values::from([("page".into(),raw.into())]))?
            };
            let field=val(&d,"text_field").to_owned();
            d=initial(&s,&ctx).await?; d.insert("page".into(),n.to_string());
            if ctx.kind==Kind::Ruleset && matches!(field.as_str(),"description"|"notes") { load_prose(&mut d,ctx.saved(&s).await?.as_ref(),&field)?; }
            return Ok((None,vec![]));
        }
        let old=ctx.saved(&s).await?;
        // Expected precedes every submitted dependency/decimal interpretation.
        if let Some(reviewed)=d.remove("reviewed_revision").filter(|v|!v.is_empty()) {
            d.insert("expected_revision".into(),reviewed); d.insert("intent".into(),"preview".into()); d.remove("review_fingerprint");
        }
        if expected(&d)?!=old.as_ref().map_or(0,Record::revision) { return Err(Error::Conflict); }
        if ctx.kind==Kind::Ruleset && !val(&d,"load_text").is_empty() {
            ruleset_prose(&d,old.as_ref())?;
            let page=make_page(&s,&h,&ctx,d.clone(),None).await?;
            if val(&d,"page_baseline").is_empty() || val(&d,"page_baseline")!=baseline(&page)? {
                return Err(invalid("text","Save the current page before loading another text field, or explicitly discard this page draft. Your complete raw text and reason are retained."));
            }
            let field=val(&d,"next_text_field").to_owned(); let n=number(&d)?;
            d=draft_for(old.as_ref()); d.insert("page".into(),n.to_string());
            load_prose(&mut d,old.as_ref(),&field)?;
            return Ok((None,vec![]));
        }
        let proposed=proposal(&s,&ctx,&d,old.as_ref()).await?;
        let page=make_page(&s,&h,&ctx,d.clone(),None).await?;
        let fingerprint=digest(&(ctx.action(),expected(&d)?,val(&d,"page"),normalized_draft(&page),&proposed.row))?;
        if val(&d,"intent")!="save" || val(&d,"review_fingerprint")!=fingerprint {
            if val(&d,"intent")=="save" { d.insert("preview_changed".into(),"yes".into()); }
            d.insert("review_fingerprint".into(),fingerprint); return Ok((None,proposed.panels));
        }
        if val(&d,"confirm")!="yes" { return Err(invalid("confirm","Review and explicitly confirm the exact pins, values, removals and version changes.")); }
        Ok((Some(persist(&s,&ctx,&d,&token,proposed.row,old.as_ref()).await?),vec![]))
    }.await;
    d.remove("csrf");
    d.remove("confirm");
    match result {
        Ok((Some(url), _)) => no_store(Redirect::to(&url).into_response()),
        Ok((None, panels)) => match make_page(&s, &h, &ctx, d, None).await {
            Ok(mut p) => {
                p.ready = !panels.is_empty();
                p.review_fingerprint = val(&p.draft, "review_fingerprint").into();
                if val(&p.draft, "preview_changed") == "yes" {
                    p.notice="Nothing was saved. Review this fresh preview: the inputs or proposal changed. Confirm again.".into();
                }
                p.panels.extend(panels);
                response(&s, &h, p, StatusCode::OK)
            }
            Err(e) => authoring::read_error(e),
        },
        Err(e) => match make_page(&s, &h, &ctx, d, Some(&e)).await {
            Ok(p) => response(&s, &h, p, error_status(&e)),
            Err(e) => authoring::read_error(e),
        },
    }
}

#[derive(Template)]
#[template(path = "move_history.html")]
struct History {
    title: String,
    url: String,
    entries: Vec<HistoryEntry>,
    next: Option<i64>,
    panels: Vec<Panel>,
    links: Vec<Link>,
}
fn snapshot(kind: Kind, json: &str) -> Result<Record, Error> {
    let value: serde_json::Value = serde_json::from_str(json).map_err(|_| Error::Unavailable)?;
    let content = value.get("content").ok_or(Error::Unavailable)?.clone();
    match kind {
        Kind::Attribute => serde_json::from_value(content).map(Record::Attribute),
        Kind::Move => serde_json::from_value(content).map(Record::Move),
        Kind::Ruleset => serde_json::from_value(content).map(Record::Ruleset),
    }
    .map_err(|_| Error::Unavailable)
}
async fn history(
    State(s): State<AppState>,
    Path(path): Path<Values>,
    uri: Uri,
    query: QueryInput,
) -> Response {
    no_store(finish(
        async {
            let ctx = Context::new(&path, uri.path());
            let Query(q) =
                query.map_err(|_| invalid("page", "Check the history cursor and page."))?;
            let n = number(&q)?;
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
            let old = ctx.saved(&s).await?;
            let url = ctx.pretty(&s, old.as_ref()).await?;
            let w = ctx.world();
            let entries = match (ctx.kind, revision) {
                (Kind::Attribute, Some(r)) => {
                    vec![s.moves.attribute_history_entry(w, &ctx.id, r).await?]
                }
                (Kind::Attribute, None) => s.moves.attribute_history(w, &ctx.id, before).await?,
                (Kind::Move, Some(r)) => vec![s.moves.history_entry(w, &ctx.id, r).await?],
                (Kind::Move, None) => s.moves.history(w, &ctx.id, before).await?,
                (Kind::Ruleset, Some(r)) => vec![
                    s.moves
                        .ruleset_history_entry(w.ok_or(Error::NotFound)?, r)
                        .await?,
                ],
                (Kind::Ruleset, None) => {
                    s.moves
                        .ruleset_history(w.ok_or(Error::NotFound)?, before)
                        .await?
                }
            };
            let next = if revision.is_none() && entries.len() == 50 {
                entries.last().map(|e| e.revision)
            } else {
                None
            };
            let mut panels = vec![];
            let mut links = vec![];
            if let Some(r) = revision {
                let e = entries.first().ok_or(Error::NotFound)?;
                let after = snapshot(ctx.kind, &e.after_json)?;
                let before = e
                    .before_json
                    .as_deref()
                    .map(|v| snapshot(ctx.kind, v))
                    .transpose()?;
                let count = after.len().max(before.as_ref().map_or(0, Record::len)).max(match &after { Record::Move(m)=>m.mappings.len(),_=>0 });
                let (summary,changed_pages)=change_summary(before.as_ref(),&after);
                panels.push(Panel { label:"Complete revision change summary".into(),text:summary });
                panels.push(Panel {
                    label: "Before — readable historical state".into(),
                    text: before
                        .as_ref()
                        .map(|r| r.text(n))
                        .unwrap_or_else(|| "No previous state (creation).".into()),
                });
                panels.push(Panel {
                    label: "After — readable historical state".into(),
                    text: after.text(n),
                });
                if let Record::Ruleset(r) = &after {
                    panels.push(Panel {
                        label: "Historical configuration diagnostics — evaluated now with current policy — NOT combat readiness".into(),
                        text: ruleset_report_text(r),
                    });
                }
                // Identify every changed identity, including later pages; values and
                // definition/source changes are readable in their pinned state pages.
                panels.push(Panel {
                    label: "Readable changes".into(),
                    text: changes(before.as_ref(), &after,n),
                });
                let ruleset_query=if let (Record::Move(m),Some(w))=(&after,w) {
                    if val(&q,"ruleset_revision").is_empty() {
                        panels.push(Panel { label:"Historical authoring validation".into(),text:"No ruleset was selected for this historical move. The historical state is not reinterpreted against a latest-head ruleset. Open a current move's pinned report to select an exact ruleset revision.".into() });
                        String::new()
                    } else {
                        let revision=positive(val(&q,"ruleset_revision"),"ruleset_revision")?;
                        let current=s.moves.ruleset(w).await?.ok_or(Error::NotFound)?;
                        let rules=s.moves.ruleset_version(w,&current.id,revision).await?;
                        panels.push(Panel { label:"Authoring validation — NOT combat readiness".into(),text:report_text(&game_data::rulesets::validate_move(m,&rules)) });
                        format!("&ruleset_revision={revision}")
                    }
                } else { String::new() };
                for page in changed_pages {
                    links.push(Link { label:format!("Changed state page {page}"),url:format!("{url}/history/{r}?page={page}{ruleset_query}") });
                }
                if n > 1 {
                    links.push(Link {
                        label: "Previous historical page".into(),
                        url: format!("{url}/history/{r}?page={}{ruleset_query}", n - 1),
                    });
                }
                if n * SIZE < count {
                    links.push(Link {
                        label: "Next historical page".into(),
                        url: format!("{url}/history/{r}?page={}{ruleset_query}", n + 1),
                    });
                }
            }
            Ok(render(History {
                title: "Move / attribute / ruleset history".into(),
                url,
                entries,
                next,
                panels,
                links,
            }))
        }
        .await,
    ))
}
fn changes(before: Option<&Record>, after: &Record, n: usize) -> String {
    let Some(before) = before else {
        return "Created this independent aggregate; exact initial state is shown above.".into();
    };
    let mut lines = vec![];
    if before.fields() != after.fields() {
        lines.push(format!(
            "Metadata: {} → {}\nDescription: {} → {}",
            before.fields().display_name,
            after.fields().display_name,
            before.fields().description,
            after.fields().description
        ));
    }
    match (before, after) {
        (Record::Attribute(a), Record::Attribute(b)) => {
            if a.unit != b.unit {
                lines.push(format!("Unit: {} → {}", a.unit, b.unit));
            }
            if a.constraints != b.constraints {
                lines.push(format!(
                    "Constraints: {} → {}",
                    constraint_text(&a.constraints),
                    constraint_text(&b.constraints)
                ));
            }
            if a.source != b.source {
                lines.push(format!("Exact source: {:?} → {:?}", a.source, b.source));
            }
        }
        (Record::Move(a), Record::Move(b)) => {
            if a.source != b.source {
                lines.push(format!("Exact source: {:?} → {:?}", a.source, b.source));
            }
            for i in changed_positions(&a.mappings, &b.mappings)
                .into_iter()
                .filter(|i| *i / SIZE + 1 == n)
            {
                let text = |m: Option<&AttributeMapping>| {
                    m.map(|m| {
                        format!(
                            "{} → {}@{}",
                            m.shared_attribute_id,
                            m.local_attribute.attribute_id,
                            m.local_attribute.version
                        )
                    })
                    .unwrap_or_else(|| "Not mapped".into())
                };
                lines.push(format!(
                    "Mapping position {}: {} → {}",
                    i + 1,
                    text(a.mappings.get(i)),
                    text(b.mappings.get(i))
                ));
            }
            if a.attributes.is_some() != b.attributes.is_some() {
                lines.push("Collection modeling state changed (Unmodeled / Defined).".into());
            }
            for (i, v) in a
                .attributes
                .iter()
                .flatten()
                .enumerate()
                .skip((n - 1) * SIZE)
                .take(SIZE)
            {
                if !b
                    .attributes
                    .iter()
                    .flatten()
                    .any(|n| n == v && b.attributes.as_ref().and_then(|r| r.get(i)) == Some(n))
                {
                    lines.push(format!("Before position {}: {}", i + 1, assignment_text(v)));
                }
            }
            for (i, v) in b
                .attributes
                .iter()
                .flatten()
                .enumerate()
                .skip((n - 1) * SIZE)
                .take(SIZE)
            {
                if a.attributes.as_ref().and_then(|r| r.get(i)) != Some(v) {
                    lines.push(format!("After position {}: {}", i + 1, assignment_text(v)));
                }
            }
        }
        (Record::Ruleset(a), Record::Ruleset(b)) => {
            if a.expectations.is_some() != b.expectations.is_some() {
                lines.push("Expectation modeling state changed (Unmodeled / Defined).".into());
            }
            let old = a.expectations.as_deref().unwrap_or_default();
            let new = b.expectations.as_deref().unwrap_or_default();
            for i in changed_positions(old, new)
                .into_iter()
                .filter(|i| *i / SIZE + 1 == n)
            {
                let text = |e: Option<&AttributeExpectation>| {
                    e.map(|e| {
                        format!(
                            "{}\n{}; allow NotApplicable: {}; additional restrictions: {}",
                            attribute_text(&e.definition),
                            if e.required {
                                "Required Known"
                            } else {
                                "Optional"
                            },
                            e.allow_not_applicable,
                            constraint_text(&e.constraints)
                        )
                    })
                    .unwrap_or_else(|| "No expectation".into())
                };
                lines.push(format!(
                    "Expectation position {} — before:\n{}\nAfter:\n{}",
                    i + 1,
                    text(old.get(i)),
                    text(new.get(i))
                ));
            }
            if a.stat_roster != b.stat_roster {
                lines.push(format!(
                    "Stat roster: {:?} → {:?}",
                    a.stat_roster, b.stat_roster
                ));
            }
            if a.chart != b.chart {
                lines.push(format!("Chart: {:?} → {:?}", a.chart, b.chart));
            }
            if a.max_moves != b.max_moves {
                lines.push(format!(
                    "Maximum moves: {:?} → {:?}",
                    a.max_moves, b.max_moves
                ));
            }
            if a.repeat_policy != b.repeat_policy {
                lines.push(format!(
                    "Repeat policy: {:?} → {:?}",
                    a.repeat_policy, b.repeat_policy
                ));
            }
            if a.notes != b.notes {
                lines.push(format!("Inert notes: {} → {}", a.notes, b.notes));
            }
        }
        _ => {}
    }
    if lines.is_empty() {
        "No differences on this page. See the complete revision change summary and changed-page links for the rest of this aggregate.".into()
    } else {
        lines.join("\n\n")
    }
}

fn changed_positions<T: PartialEq>(before: &[T], after: &[T]) -> Vec<usize> {
    (0..before.len().max(after.len()))
        .filter(|i| before.get(*i) != after.get(*i))
        .collect()
}
// Compare entire immutable aggregates before choosing a display page. A page
// with no changed rows must never claim that the whole revision is unchanged.
fn change_summary(before: Option<&Record>, after: &Record) -> (String, Vec<usize>) {
    let mut lines = vec![];
    let mut pages = std::collections::BTreeSet::new();
    if let Some(before) = before {
        if before.fields() != after.fields() {
            lines.push("Metadata changed (name, description or scoped address).".into());
        }
    } else {
        lines.push("Created this independent aggregate.".into());
    }
    let mut collection = |label: &str, positions: Vec<usize>| {
        lines.push(format!(
            "{label}: {} changed positions across the complete aggregate.",
            positions.len()
        ));
        pages.extend(positions.into_iter().map(|i| i / SIZE + 1));
    };
    match after {
        Record::Move(b) => {
            let a = if let Some(Record::Move(a)) = before {
                Some(a)
            } else {
                None
            };
            collection(
                "Attribute assignments",
                changed_positions(
                    a.and_then(|a| a.attributes.as_deref()).unwrap_or_default(),
                    b.attributes.as_deref().unwrap_or_default(),
                ),
            );
            collection(
                "Attribute mappings",
                changed_positions(
                    a.map(|a| a.mappings.as_slice()).unwrap_or_default(),
                    &b.mappings,
                ),
            );
            if a.and_then(|a| a.attributes.as_ref()).is_some() != b.attributes.is_some() {
                lines.push(
                    "Attribute collection modeling state changed (Unmodeled / Defined).".into(),
                );
            }
            if a.and_then(|a| a.source.as_ref()) != b.source.as_ref() {
                lines.push("Exact shared Move source changed.".into());
            }
        }
        Record::Ruleset(b) => {
            let a = if let Some(Record::Ruleset(a)) = before {
                Some(a)
            } else {
                None
            };
            collection(
                "Attribute expectations",
                changed_positions(
                    a.and_then(|a| a.expectations.as_deref())
                        .unwrap_or_default(),
                    b.expectations.as_deref().unwrap_or_default(),
                ),
            );
            if a.and_then(|a| a.expectations.as_ref()).is_some() != b.expectations.is_some() {
                lines.push(
                    "Expectation collection modeling state changed (Unmodeled / Defined).".into(),
                );
            }
            if let Some(a) = a {
                if a.stat_roster != b.stat_roster {
                    lines.push("Exact stat roster pin changed.".into());
                }
                if a.chart != b.chart {
                    lines.push("Exact chart pin changed.".into());
                }
                if a.max_moves != b.max_moves {
                    lines.push("Optional maximum moves changed.".into());
                }
                if a.repeat_policy != b.repeat_policy {
                    lines.push("Repeat policy changed.".into());
                }
                if a.notes != b.notes {
                    lines.push("Inert notes changed.".into());
                }
            }
        }
        Record::Attribute(b) => {
            if let Some(Record::Attribute(a)) = before {
                if a.unit != b.unit {
                    lines.push("Descriptive unit changed.".into());
                }
                if a.constraints != b.constraints {
                    lines.push("Exact numeric constraints changed.".into());
                }
                if a.source != b.source {
                    lines.push("Exact shared attribute source changed.".into());
                }
            }
        }
    }
    if pages.is_empty() {
        lines.push("No changed collection pages; any metadata, source, modeling or configuration changes are identified above.".into());
    } else {
        lines.push(format!(
            "Changed state pages: {}. Each link retains this exact revision.",
            pages
                .iter()
                .map(usize::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    (lines.join("\n"), pages.into_iter().collect())
}
