//! Merge one operation into the current full component; the writer rejects races.
use super::*;
#[derive(Clone)]
pub(super) struct OptionView {
    value: String,
    label: String,
    selected: bool,
}
pub(super) struct Control {
    name: String,
    label: String,
    value: String,
    multiline: bool,
    checkbox: bool,
    options: Vec<OptionView>,
}
pub(super) fn input(d: &Values, k: &str, label: &str, multiline: bool) -> Control {
    Control {
        name: k.into(),
        label: label.into(),
        value: val(d, k).into(),
        multiline,
        checkbox: false,
        options: vec![],
    }
}
pub(super) fn select(d: &Values, k: &str, label: &str, options: &[(&str, &str)]) -> Control {
    let mut c = input(d, k, label, false);
    c.options = options
        .iter()
        .map(|(v, l)| OptionView {
            value: (*v).into(),
            label: (*l).into(),
            selected: val(d, k) == *v,
        })
        .collect();
    if !options.iter().any(|(v, _)| *v == val(d, k)) {
        c.options.push(OptionView {
            value: val(d, k).into(),
            label: format!("Unselected/raw: {}", val(d, k)),
            selected: true,
        });
    }
    c
}
fn checkbox(d: &Values, k: &str, label: &str) -> Control {
    let mut c = input(d, k, label, false);
    c.checkbox = true;
    c
}
fn named_select(d: &Values, k: &str, label: &str, options: Vec<(String, String)>) -> Control {
    let pairs = options
        .iter()
        .map(|(v, l)| (v.as_str(), l.as_str()))
        .collect::<Vec<_>>();
    select(d, k, label, &pairs)
}
#[derive(Template)]
#[template(path = "records_editor.html")]
struct Editor {
    current_revision: String,
    title: String,
    action: String,
    csrf: String,
    revision: String,
    operation: String,
    controls: Vec<Control>,
    search: Vec<Control>,
    hidden: Vec<(String, String)>,
    unexpected: Vec<(String, String)>,
    error: String,
    errors: Vec<(String, String)>,
    sections: Vec<Section>,
    picks: Vec<selectors::Pick>,
    review: String,
    back: String,
    mapping: bool,
    cell: bool,
    preview_operation: bool,
}

async fn current_revision(s: &AppState, c: &Context) -> Result<i64, Error> {
    if c.id.is_empty() {
        Ok(0)
    } else {
        Ok(match c.kind {
            Kind::Layout => api(s).layout(c.world.as_deref(), &c.id).await?.revision,
            Kind::Collection => api(s).collection(c.world.as_deref(), &c.id).await?.revision,
            Kind::Card => s.cards.detail(&c.identity()).await?.revision,
        })
    }
}
pub(super) async fn selected_collection(
    s: &AppState,
    c: &Context,
    d: &Values,
) -> Result<RecordCollection, Error> {
    let pin: CollectionPin = unpack(d, "collection")?;
    let scalar = s.cards.detail(&c.identity()).await?;
    let saved = api(s)
        .card(&scalar.pin())
        .await?
        .relations
        .collections
        .into_iter()
        .flatten()
        .find(|r| r.definition.id == pin.collection_id)
        .ok_or_else(|| {
            invalid(
                "collection",
                "Choose an attached collection. Add a definition using Edit collection first.",
            )
        })?;
    if val(d, "operation") != "reconcile" && saved.definition.pin() != pin {
        return Err(invalid(
            "collection",
            "Choose the retained exact attached definition for row/cell edits; reconcile explicitly to change its revision.",
        ));
    }
    Ok(saved)
}
async fn defaults(s: &AppState, c: &Context, q: &Values) -> Result<Values, Error> {
    let mut d = Values::new();
    let op = if c.id.is_empty() {
        "create"
    } else {
        val(q, "operation")
    };
    for (k, v) in [
        ("operation", op),
        ("mode", "set"),
        ("state", "missing"),
        ("value_state", "unknown"),
        ("boolean", "false"),
        (
            "find",
            match c.kind {
                Kind::Layout => "field",
                Kind::Collection => "layout",
                Kind::Card => "attached",
            },
        ),
        ("search_scope", "local"),
        ("page", "1"),
        ("column_kind", "reference"),
        ("allow_card", "yes"),
        ("archived", "no"),
    ] {
        put(&mut d, k, v);
    }
    put(&mut d, "expected_revision", current_revision(s, c).await?);
    if c.id.is_empty() {
        return Ok(d);
    }
    match c.kind {
        Kind::Layout => {
            let r = api(s).layout(c.world.as_deref(), &c.id).await?;
            put(&mut d, "display_name", r.fields.display_name);
            if op == "description" {
                put(&mut d, "text", r.fields.description);
            }
            if r.archived_at.is_some() {
                put(&mut d, "archived", "yes");
            }
            if let Some(p) = r.source {
                put(&mut d, "source_layout", packed(&p)?);
            }
        }
        Kind::Collection => {
            let r = api(s).collection(c.world.as_deref(), &c.id).await?;
            put(&mut d, "display_name", r.fields.display_name);
            if op == "description" {
                put(&mut d, "text", r.fields.description);
            }
            put(&mut d, "layout", packed(&r.layout.pin())?);
            if r.archived_at.is_some() {
                put(&mut d, "archived", "yes");
            }
            if let Some(p) = r.source {
                put(&mut d, "source_collection", packed(&p)?);
            }
        }
        Kind::Card => {
            let r = s.cards.detail(&c.identity()).await?;
            let row = api(s).card(&r.pin()).await?;
            if let Some(col) = row.relations.collections.iter().flatten().next() {
                put(&mut d, "collection", packed(&col.definition.pin())?);
                if let Some(column) = col.definition.layout.columns.first() {
                    put(&mut d, "column_id", &column.id);
                }
                if let CollectionValue::Entries(es) = &col.value
                    && let Some(e) = es.first()
                {
                    put(&mut d, "entry_id", &e.id);
                }
            }
            if op == "cell" && !val(&d, "entry_id").is_empty() && !val(&d, "column_id").is_empty() {
                load_cell(s, c, &mut d, true).await?;
            }
        }
    }
    Ok(d)
}
pub(super) async fn editor(
    State(s): State<AppState>,
    Path(p): Path<Values>,
    uri: Uri,
    h: HeaderMap,
    q: Result<Query<Vec<(String, String)>>, QueryRejection>,
) -> Response {
    finish(
        async {
            let c = Context::new(&p, &uri)?.resolved(&s).await?;
            c.permission(&s, &h).await?;
            let q = read_query(q, &["operation"])?;
            let d = defaults(&s, &c, &q).await?;
            let (csrf, set) = csrf_for(&h).map_err(|_| Error::Unavailable)?;
            let mut r = show(&s, &c, &d, &csrf, None, vec![], "").await;
            if set {
                append_cookie(&mut r, csrf_cookie(&csrf, s.cookie_secure));
            }
            Ok(r)
        }
        .await,
    )
}
async fn controls(s: &AppState, c: &Context, d: &Values) -> Result<Vec<Control>, Error> {
    let mut out = vec![];
    let op = val(d, "operation");
    match op {
        "create" => {
            out.push(input(d, "display_name", "Display name", false));
            out.push(input(d, "text", "Description (inert plain text)", true));
        }
        "name" => out.push(input(d, "display_name", "Display name", false)),
        "description" => out.push(input(
            d,
            "text",
            "Loaded description (inert plain text)",
            true,
        )),
        "archive" => out.push(select(
            d,
            "archived",
            "Archive state",
            &[("no", "Active"), ("yes", "Archived")],
        )),
        "column" => {
            out.push(select(
                d,
                "mode",
                "Column operation",
                &[
                    ("set", "Add or update column"),
                    ("remove", "Remove column"),
                    ("up", "Move earlier"),
                    ("down", "Move later"),
                ],
            ));
            out.push(input(d, "label", "Column label", false));
            out.push(select(
                d,
                "column_kind",
                "Column kind",
                &[
                    ("field", "Exact scalar field"),
                    ("reference", "Public reference"),
                ],
            ));
            for (k, l) in [
                ("allow_card", "Allow Card targets"),
                ("allow_stat", "Allow Stat targets"),
                ("allow_field", "Allow Field targets"),
            ] {
                out.push(checkbox(d, k, l));
            }
            if !c.id.is_empty() {
                let row = api(s).layout(c.world.as_deref(), &c.id).await?;
                let n = page(d)?;
                out.push(named_select(
                    d,
                    "column_id",
                    "Existing column (blank adds a distinct column)",
                    std::iter::once((String::new(), "New distinct column".into()))
                        .chain(
                            row.columns
                                .iter()
                                .skip((n - 1) * SIZE)
                                .take(SIZE)
                                .map(|v| (v.id.clone(), v.label.clone())),
                        )
                        .collect(),
                ));
            }
        }
        "inventory" => out.push(select(
            d,
            "mode",
            "Inventory operation",
            &[
                ("define", "Define empty inventory only if unmodeled"),
                ("unmodel", "Unmodel only if empty"),
            ],
        )),
        "collection" => {
            out.push(select(
                d,
                "mode",
                "Collection operation",
                &[
                    ("add", "Add selected definition"),
                    ("state", "Replace state (entries must already be empty)"),
                    ("remove", "Remove selected collection"),
                    ("up", "Move earlier"),
                    ("down", "Move later"),
                ],
            ));
            out.push(select(
                d,
                "state",
                "Collection value state",
                &[
                    ("missing", "Missing"),
                    ("unknown", "Unknown"),
                    ("not_applicable", "NotApplicable"),
                    ("entries", "Known entries (empty)"),
                ],
            ));
        }
        "row" => out.push(select(
            d,
            "mode",
            "Entry operation",
            &[
                ("add", "Add new occurrence"),
                ("remove", "Remove selected entry"),
                ("up", "Move earlier"),
                ("down", "Move later"),
            ],
        )),
        "cell" => {
            out.push(select(
                d,
                "value_state",
                "Cell state",
                &[
                    ("absent", "Absent (remove cell)"),
                    ("missing", "Missing"),
                    ("unknown", "Unknown"),
                    ("not_applicable", "NotApplicable"),
                    ("known", "Known (typed by the exact column)"),
                ],
            ));
            let col = selected_collection(s, c, d).await?;
            if let Some(column) = col
                .definition
                .layout
                .columns
                .iter()
                .find(|r| r.id == val(d, "column_id"))
            {
                match &column.kind {
                    ColumnKind::Field(f) => match &f.kind {
                        FieldKind::Number(_) => out.push(input(
                            d,
                            "text",
                            "Loaded numeric value (zero is known)",
                            val(d, "text").contains(['\n', '\r']),
                        )),
                        FieldKind::Text { .. } => out.push(input(
                            d,
                            "text",
                            "Loaded cell text (empty is known, inert)",
                            true,
                        )),
                        FieldKind::Boolean => out.push(select(
                            d,
                            "boolean",
                            "Known boolean",
                            &[("false", "false"), ("true", "true")],
                        )),
                        FieldKind::Choice { .. } => {}
                        FieldKind::CardReference => {}
                    },
                    ColumnKind::Reference { .. } => {}
                }
            }
        }
        "reconcile" | "copy" => {
            let (source, target) = mapping_definitions(s, c, d).await?;
            if let (Some(source), Some(target)) = (source, target) {
                let n = page(d)?;
                out.push(named_select(
                    d,
                    "from_column",
                    "Source column",
                    source
                        .layout
                        .columns
                        .iter()
                        .skip((n - 1) * SIZE)
                        .take(SIZE)
                        .map(|x| (x.id.clone(), x.label.clone()))
                        .collect(),
                ));
                out.push(named_select(
                    d,
                    "to_column",
                    "Destination column",
                    target
                        .layout
                        .columns
                        .iter()
                        .skip((n - 1) * SIZE)
                        .take(SIZE)
                        .map(|x| (x.id.clone(), x.label.clone()))
                        .collect(),
                ));
            }
        }
        "layout" | "adopt" => {}
        _ => {
            return Err(invalid(
                "operation",
                "Choose an operation from the saved record.",
            ));
        }
    }
    if !out.iter().any(|control| control.name == "text") && !val(d, "text").is_empty() {
        out.push(input(
            d,
            "text",
            "Retained inactive text (save or discard before loading)",
            true,
        ));
    }
    out.push(input(d, "reason", "Change reason (optional)", false));
    Ok(out)
}
async fn saved_context(s: &AppState, c: &Context) -> Result<Vec<Section>, Error> {
    if c.id.is_empty() {
        return Ok(vec![]);
    }
    match c.kind {
        Kind::Layout => Ok(layout_sections(
            &api(s).layout(c.world.as_deref(), &c.id).await?,
            1,
        )),
        Kind::Collection => Ok(collection_sections(
            &api(s).collection(c.world.as_deref(), &c.id).await?,
            1,
        )),
        Kind::Card => {
            let r = s.cards.detail(&c.identity()).await?;
            card_sections(s, &api(s).card(&r.pin()).await?, 1).await
        }
    }
}
async fn show(
    s: &AppState,
    c: &Context,
    d: &Values,
    csrf: &str,
    error: Option<&Error>,
    mut sections: Vec<Section>,
    review: &str,
) -> Response {
    let mut local_error = None;
    let controls = match controls(s, c, d).await {
        Ok(v) => v,
        Err(e) => {
            local_error = Some(e);
            vec![
                input(d, "text", "Retained raw text", true),
                input(d, "reason", "Change reason (optional)", false),
            ]
        }
    };
    let picks = match selectors::results(s, c, d).await {
        Ok(v) => v,
        Err(e) => {
            if local_error.is_none() {
                local_error = Some(e);
            }
            vec![]
        }
    };
    match selectors::selected(s, d).await {
        Ok(v) => sections.extend(v),
        Err(e) => {
            if local_error.is_none() {
                local_error = Some(e);
            }
        }
    }
    if !c.id.is_empty()
        && let Ok(rev) = current_revision(s, c).await
    {
        sections.push(section("Saved revision context",vec![format!("Current revision: {rev}. Draft starting revision: {}. Open the saved record in another tab to compare; re-open an operation to deliberately load its latest revision.",val(d,"expected_revision"))]));
    }
    if let Ok(v) = mapping_sections(s, c, d).await {
        sections.extend(v);
    }
    let error = error.or(local_error.as_ref());
    let search = selectors::search_controls(d);
    let names = controls
        .iter()
        .chain(search.iter())
        .map(|c| c.name.as_str())
        .collect::<Vec<_>>();
    let hidden = d
        .iter()
        .filter(|(k, _)| {
            allowed_key(k)
                && !names.contains(&k.as_str())
                && !matches!(
                    k.as_str(),
                    "csrf"
                        | "expected_revision"
                        | "operation"
                        | "intent"
                        | "choose"
                        | "digest"
                        | "text"
                )
        })
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    no_store(render_status(
        Editor {
            current_revision: if matches!(error, Some(Error::Conflict)) {
                current_revision(s, c)
                    .await
                    .map(|r| r.to_string())
                    .unwrap_or_default()
            } else {
                String::new()
            },
            title: format!("Record {}", val(d, "operation")),
            action: c.action(),
            csrf: csrf.into(),
            revision: val(d, "expected_revision").into(),
            operation: val(d, "operation").into(),
            controls,
            search,
            hidden,
            unexpected: d
                .iter()
                .filter(|(key, _)| !allowed_key(key))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            error: error.map(error_text).unwrap_or("").into(),
            errors: if let Some(Error::Invalid(e)) = error {
                e.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
            } else {
                vec![]
            },
            sections,
            picks,
            review: review.into(),
            back: if c.id.is_empty() { c.root() } else { c.url() },
            mapping: matches!(val(d, "operation"), "copy" | "reconcile"),
            cell: val(d, "operation") == "cell",
            preview_operation: matches!(
                val(d, "operation"),
                "copy" | "reconcile" | "adopt" | "archive"
            ),
        },
        error.map(error_status).unwrap_or(StatusCode::OK),
    ))
}
fn allowed_key(key: &str) -> bool {
    [
        "operation",
        "expected_revision",
        "mode",
        "state",
        "value_state",
        "boolean",
        "find",
        "search_scope",
        "page",
        "before",
        "column_kind",
        "allow_card",
        "allow_stat",
        "allow_field",
        "archived",
        "display_name",
        "label",
        "text",
        "reason",
        "column_id",
        "layout",
        "field",
        "collection",
        "reference",
        "choice",
        "entry_id",
        "source_layout",
        "source_collection",
        "mapping_source",
        "mappings",
        "from_column",
        "to_column",
        "loaded_text",
        "loaded_cell",
        "csrf",
        "intent",
        "choose",
        "digest",
        "query",
        "reviewed_revision",
    ]
    .contains(&key)
}
fn allow(d: &Values) -> Result<(), Error> {
    if d.keys().any(|key| !allowed_key(key)) {
        return Err(invalid(
            "form",
            "Use only the loaded operation's controls; unexpected values are retained read-only.",
        ));
    }
    Ok(())
}
fn guard_size(d: &Values) -> Result<(), Error> {
    // Worst-case percent encoding, including button and CSRF/review overhead.
    let len = d
        .iter()
        .map(|(k, v)| encode(k).len() + encode(v).len() + 2)
        .sum::<usize>();
    if len > 30 * 1024 {
        return Err(invalid(
            "draft",
            "This addition would exceed the 32 KiB form budget. The preceding draft has been retained. Save a smaller operation first.",
        ));
    }
    Ok(())
}
pub(super) async fn submit(
    State(s): State<AppState>,
    Path(p): Path<Values>,
    uri: Uri,
    h: HeaderMap,
    form: Result<Form<Vec<(String, String)>>, FormRejection>,
) -> Response {
    let Form(pairs) = match form {
        Ok(v) => v,
        Err(e) => return authoring_form_error(e),
    };
    let mut d = Values::new();
    let mut duplicates = Vec::new();
    for (k, v) in pairs {
        match d.entry(k) {
            std::collections::btree_map::Entry::Occupied(entry) => {
                duplicates.push((entry.key().clone(), v))
            }
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(v);
            }
        }
    }
    let mut c = match Context::new(&p, &uri) {
        Ok(c) => c,
        Err(e) => return finish(Err(e)),
    };
    // Mutation paths are deliberately never slug-resolved.
    let result: Result<Option<(Vec<Section>, String)>, Error> = async {
        let token = submit_token(&s, &h, val(&d, "csrf"))?;
        c.permission(&s, &h).await?;
        if !duplicates.is_empty() {
            return Err(invalid("form", "Submit each control once."));
        }
        if !c.id.is_empty() && !game_data::types::valid_id(&c.id) {
            return Err(Error::NotFound);
        }
        // Reject stale drafts before parsing any submitted values or dependencies.
        if !c.id.is_empty() {
            if !val(&d, "reviewed_revision").is_empty() {
                let reviewed = positive(val(&d, "reviewed_revision"), "reviewed_revision")?;
                if current_revision(&s, &c).await? != reviewed {
                    return Err(Error::Conflict);
                }
                put(&mut d, "expected_revision", reviewed);
                d.remove("reviewed_revision");
                let intent = if matches!(
                    val(&d, "operation"),
                    "copy" | "reconcile" | "archive" | "adopt"
                ) {
                    "preview"
                } else {
                    "save"
                };
                put(&mut d, "intent", intent);
            }
            let expected = positive(val(&d, "expected_revision"), "expected_revision")?;
            if current_revision(&s, &c).await? != expected {
                return Err(Error::Conflict);
            }
        }
        allow(&d)?;
        if d.contains_key("choose") {
            let mut next = d.clone();
            selectors::choose(&mut next)?;
            guard_size(&next)?;
            d = next;
            return Ok(Some((vec![], String::new())));
        }
        match val(&d, "intent") {
            "search" => return Ok(Some((vec![], String::new()))),
            "load" | "discard_load" => {
                let discard = val(&d, "intent") == "discard_load";
                load_cell(&s, &c, &mut d, discard).await?;
                return Ok(Some((vec![], String::new())));
            }
            "map" => {
                let mut next = d.clone();
                add_mapping(&s, &c, &mut next).await?;
                guard_size(&next)?;
                d = next;
                return Ok(Some((vec![], String::new())));
            }
            "clear_mappings" => {
                put(&mut d, "mappings", "[]");
                return Ok(Some((vec![], String::new())));
            }
            "load_column" => {
                let r = api(&s).layout(c.world.as_deref(), &c.id).await?;
                let col = r
                    .columns
                    .iter()
                    .find(|v| v.id == val(&d, "column_id"))
                    .ok_or(Error::NotFound)?;
                put(&mut d, "label", &col.label);
                match &col.kind {
                    ColumnKind::Field(f) => {
                        put(&mut d, "column_kind", "field");
                        put(&mut d, "field", packed(&f.pin())?);
                    }
                    ColumnKind::Reference { allowed } => {
                        put(&mut d, "column_kind", "reference");
                        for (k, kind) in [
                            ("allow_card", ReferenceKind::Card),
                            ("allow_stat", ReferenceKind::Stat),
                            ("allow_field", ReferenceKind::Field),
                        ] {
                            if allowed.contains(&kind) {
                                put(&mut d, k, "yes");
                            } else {
                                d.remove(k);
                            }
                        }
                    }
                }
                return Ok(Some((vec![], String::new())));
            }
            "preview" | "confirm" | "save" => {}
            _ => return Err(invalid("intent", "Choose a native operation button.")),
        }
        apply(&s, &mut c, &d, &token).await
    }
    .await;
    match result {
        Ok(None) => no_store(Redirect::to(&c.url()).into_response()),
        Ok(Some((sections, review))) => {
            let csrf = val(&d, "csrf").to_owned();
            show(&s, &c, &d, &csrf, None, sections, &review).await
        }
        Err(e) => {
            let csrf = csrf_for(&h).map(|v| v.0).unwrap_or_default();
            let mut sections = saved_context(&s, &c).await.unwrap_or_default();
            if !duplicates.is_empty() {
                sections.push(section(
                    "Duplicate raw values (not submitted)",
                    duplicates
                        .iter()
                        .map(|(k, v)| format!("{k}: {v}"))
                        .collect(),
                ));
            }
            if e == Error::Conflict {
                sections.insert(0,section("Conflict: saved state",vec!["Your raw draft is retained below. It has not been applied to the newer revision.".into()]));
            }
            show(&s, &c, &d, &csrf, Some(&e), sections, "").await
        }
    }
}
fn fields(d: &Values) -> ContentFields {
    ContentFields {
        slug: String::new(),
        display_name: val(d, "display_name").into(),
        description: normalize(val(d, "text")),
    }
}
fn normalize(s: &str) -> String {
    s.replace("\r\n", "\n").replace('\r', "\n")
}
fn move_item<T>(v: &mut Vec<T>, i: usize, mode: &str) -> Result<(), Error> {
    match mode {
        "remove" => {
            v.remove(i);
        }
        "up" => {
            if i > 0 {
                v.swap(i, i - 1);
            }
        }
        "down" => {
            if i + 1 < v.len() {
                v.swap(i, i + 1);
            }
        }
        _ => {
            return Err(invalid(
                "mode",
                "Choose remove, move earlier or move later.",
            ));
        }
    }
    Ok(())
}
fn collection_state(d: &Values) -> Result<CollectionValue, Error> {
    match val(d, "state") {
        "missing" => Ok(CollectionValue::Missing),
        "unknown" => Ok(CollectionValue::Unknown),
        "not_applicable" => Ok(CollectionValue::NotApplicable),
        "entries" => Ok(CollectionValue::Entries(vec![])),
        _ => Err(invalid("state", "Choose an explicit collection state.")),
    }
}
async fn load_cell(s: &AppState, c: &Context, d: &mut Values, discard: bool) -> Result<(), Error> {
    if !discard
        && ((!val(d, "loaded_text").is_empty()
            && text_hash(&normalize(val(d, "text"))) != val(d, "loaded_text"))
            || (val(d, "loaded_text").is_empty() && !val(d, "text").is_empty()))
    {
        return Err(invalid(
            "text",
            "Save or explicitly discard dirty text before loading a different cell.",
        ));
    }
    let col = selected_collection(s, c, d).await?;
    let column = col
        .definition
        .layout
        .columns
        .iter()
        .find(|v| v.id == val(d, "column_id"))
        .ok_or_else(|| invalid("column", "Choose a named column."))?;
    let CollectionValue::Entries(es) = &col.value else {
        return Err(invalid("entry", "This collection has no known entries."));
    };
    let entry = es
        .iter()
        .find(|e| e.id == val(d, "entry_id"))
        .ok_or_else(|| invalid("entry", "Choose a named entry."))?;
    let cell = entry.cells.iter().find(|v| v.column_id == column.id);
    let mut text = String::new();
    let mut state = "known";
    d.remove("reference");
    d.remove("choice");
    match cell.map(|v| &v.value) {
        None => state = "absent",
        Some(RecordValue::Missing) => state = "missing",
        Some(RecordValue::Unknown) => state = "unknown",
        Some(RecordValue::NotApplicable) => state = "not_applicable",
        Some(RecordValue::Text(t)) => text = t.clone(),
        Some(RecordValue::Number(v)) => text = v.to_string(),
        Some(RecordValue::Boolean(v)) => put(d, "boolean", v),
        Some(RecordValue::Choice(v)) => put(d, "choice", v),
        Some(RecordValue::Reference(v)) => put(d, "reference", packed(v)?),
    }
    put(d, "value_state", state);
    put(d, "text", &text);
    put(d, "loaded_text", text_hash(&text));
    put(
        d,
        "loaded_cell",
        packed(&(
            col.definition.id,
            entry.id.clone(),
            column.id.clone(),
            val(d, "expected_revision"),
        ))?,
    );
    Ok(())
}
async fn cell_value(
    s: &AppState,
    c: &Context,
    d: &Values,
    col: &RecordCollection,
) -> Result<Option<RecordValue>, Error> {
    let column = col
        .definition
        .layout
        .columns
        .iter()
        .find(|v| v.id == val(d, "column_id"))
        .ok_or_else(|| invalid("column", "Choose a named column."))?;
    let loaded = packed(&(
        col.definition.id.clone(),
        val(d, "entry_id"),
        column.id.clone(),
        val(d, "expected_revision"),
    ))?;
    if val(d, "loaded_cell") != loaded {
        return Err(invalid(
            "text",
            "Load the selected cell before editing. Save or discard dirty text before switching.",
        ));
    }
    let value = match val(d, "value_state") {
        "absent" => return Ok(None),
        "missing" => RecordValue::Missing,
        "unknown" => RecordValue::Unknown,
        "not_applicable" => RecordValue::NotApplicable,
        "known" => match &column.kind {
            ColumnKind::Reference { .. } => RecordValue::Reference(unpack(d, "reference")?),
            ColumnKind::Field(f) => match &f.kind {
                FieldKind::Number(_) => RecordValue::Number(
                    val(d, "text")
                        .parse()
                        .map_err(|e: String| invalid("text", &e))?,
                ),
                FieldKind::Text { .. } => RecordValue::Text(normalize(val(d, "text"))),
                FieldKind::Boolean => RecordValue::Boolean(match val(d, "boolean") {
                    "true" => true,
                    "false" => false,
                    _ => return Err(invalid("boolean", "Choose true or false.")),
                }),
                FieldKind::Choice { .. } => RecordValue::Choice(val(d, "choice").into()),
                FieldKind::CardReference => {
                    let p: DefinitionPin = unpack(d, "reference")?;
                    if !matches!(p, DefinitionPin::Card(_)) {
                        return Err(invalid(
                            "reference",
                            "This scalar column accepts Card targets only.",
                        ));
                    }
                    RecordValue::Reference(p)
                }
            },
        },
        _ => return Err(invalid("value_state", "Choose an explicit cell state.")),
    };
    value
        .validate(column, c.world.as_deref())
        .map_err(Error::Invalid)?;
    let _ = s;
    Ok(Some(value))
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct MappingDraft {
    shared_collection_id: String,
    local_collection: CollectionPin,
    columns: Vec<ColumnDraft>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ColumnDraft {
    source_column_id: String,
    local_column_id: String,
}
impl ColumnDraft {
    fn backend(&self) -> ColumnMapping {
        ColumnMapping {
            source_column_id: self.source_column_id.clone(),
            local_column_id: self.local_column_id.clone(),
        }
    }
}
fn mappings(d: &Values) -> Result<Vec<MappingDraft>, Error> {
    if val(d, "mappings").is_empty() {
        Ok(vec![])
    } else {
        unpack(d, "mappings")
    }
}
async fn mapping_definitions(
    s: &AppState,
    c: &Context,
    d: &Values,
) -> Result<(Option<CollectionDefinition>, Option<CollectionDefinition>), Error> {
    let target = if val(d, "collection").is_empty() {
        None
    } else {
        Some(api(s).collection_version(&unpack(d, "collection")?).await?)
    };
    let source = if val(d, "operation") == "reconcile" {
        if target.is_some() {
            Some(selected_collection(s, c, d).await?.definition)
        } else {
            None
        }
    } else if val(d, "mapping_source").is_empty() {
        None
    } else {
        Some(
            api(s)
                .collection_version(&unpack(d, "mapping_source")?)
                .await?,
        )
    };
    Ok((source, target))
}
async fn add_mapping(s: &AppState, c: &Context, d: &mut Values) -> Result<(), Error> {
    let (source, target) = mapping_definitions(s, c, d).await?;
    let source = source.ok_or_else(|| invalid("source", "Choose the source card collection."))?;
    let target =
        target.ok_or_else(|| invalid("target", "Choose a destination collection definition."))?;
    let mut maps = mappings(d)?;
    let i = maps
        .iter()
        .position(|m| m.shared_collection_id == source.id);
    let i = if let Some(i) = i {
        if maps[i].local_collection != target.pin() {
            return Err(invalid(
                "mappings",
                "Clear reviewed draft mappings before changing an existing destination.",
            ));
        }
        i
    } else {
        maps.push(MappingDraft {
            shared_collection_id: source.id.clone(),
            local_collection: target.pin(),
            columns: vec![],
        });
        maps.len() - 1
    };
    if !val(d, "from_column").is_empty() || !val(d, "to_column").is_empty() {
        let from = val(d, "from_column");
        let to = val(d, "to_column");
        if !source.layout.columns.iter().any(|v| v.id == from)
            || !target.layout.columns.iter().any(|v| v.id == to)
        {
            return Err(invalid("mappings", "Choose declared named columns."));
        }
        if maps[i]
            .columns
            .iter()
            .any(|m| m.local_column_id == to && m.source_column_id != from)
        {
            return Err(invalid("mappings", "Destination columns must be distinct."));
        }
        maps[i].columns.retain(|m| m.source_column_id != from);
        maps[i].columns.push(ColumnDraft {
            source_column_id: from.into(),
            local_column_id: to.into(),
        });
    }
    put(d, "mappings", packed(&maps)?);
    Ok(())
}
async fn mapping_sections(s: &AppState, c: &Context, d: &Values) -> Result<Vec<Section>, Error> {
    let maps = mappings(d)?;
    let mut out = vec![];
    for m in maps.iter().skip((page(d)? - 1) * SIZE).take(SIZE) {
        let target = api(s).collection_version(&m.local_collection).await?;
        let source = if val(d, "operation") == "reconcile" {
            selected_collection(s, c, d).await?.definition
        } else {
            let p: CollectionPin = unpack(d, "mapping_source")?;
            let scalar = s.cards.detail(&c.identity()).await?;
            let source = api(s)
                .card(scalar.source.as_ref().ok_or(Error::NotFound)?)
                .await?;
            source
                .relations
                .collections
                .into_iter()
                .flatten()
                .find(|x| x.definition.id == m.shared_collection_id)
                .map(|x| x.definition)
                .unwrap_or(api(s).collection_version(&p).await?)
        };
        let mut lines = vec![format!(
            "{} → {} · {} explicit columns",
            source.fields.display_name,
            target.fields.display_name,
            m.columns.len()
        )];
        for col in m.columns.iter().take(SIZE) {
            let from = source
                .layout
                .columns
                .iter()
                .find(|x| x.id == col.source_column_id)
                .ok_or(Error::NotFound)?;
            let to = target
                .layout
                .columns
                .iter()
                .find(|x| x.id == col.local_column_id)
                .ok_or(Error::NotFound)?;
            lines.push(format!("{} → {}", from.label, to.label));
        }
        out.push(section("Draft mapping (not saved)", lines));
    }
    Ok(out)
}
async fn relation_preview(s: &AppState, c: &Context, d: &Values) -> Result<RelationPreview, Error> {
    let a = api(s);
    let expected = positive(val(d, "expected_revision"), "expected_revision")?;
    let maps = mappings(d)?;
    if val(d, "operation") == "reconcile" {
        let target: CollectionPin = unpack(d, "collection")?;
        let cols = maps
            .iter()
            .find(|m| m.shared_collection_id == target.collection_id)
            .map(|m| {
                m.columns
                    .iter()
                    .map(ColumnDraft::backend)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        a.preview_reconcile_card(
            &c.identity(),
            expected,
            &target.collection_id,
            &target,
            &cols,
        )
        .await
    } else {
        let maps = maps
            .iter()
            .map(|m| CollectionMapping {
                shared_collection_id: m.shared_collection_id.clone(),
                local_collection: m.local_collection.clone(),
                columns: m.columns.iter().map(ColumnDraft::backend).collect(),
            })
            .collect::<Vec<_>>();
        a.preview_source_collections(&c.identity(), expected, &maps)
            .await
    }
}
async fn preview_sections(
    s: &AppState,
    p: &RelationPreview,
    n: usize,
) -> Result<Vec<Section>, Error> {
    let mut out=vec![section("Reviewed replacement",vec![format!("Removed/replaced collections: {}. Unmapped source collections: {}. Removed destination cells: {}. Unmapped source cells: {}.",p.removed.len(),p.unmapped.len(),p.removed_cells.len(),p.unmapped_cells.len()),"Every occurrence is retained by its stable entry ID when mapped. Unmapped cells and destination collections are removed; no name inference or number coercion occurs.".into()])];
    for (title, row) in [
        ("Old local values", Some(&p.entry)),
        ("Exact source values", p.source.as_ref()),
        ("Proposed values", Some(&p.proposed)),
    ] {
        if let Some(row) = row {
            out.push(section(title, vec![]));
            out.extend(card_sections(s, row, n).await?);
        }
    }
    for (title, items) in [
        ("Removed collections", &p.removed),
        ("Unmapped source collections", &p.unmapped),
    ] {
        out.push(section(
            title,
            items
                .iter()
                .skip((n - 1) * SIZE)
                .take(SIZE)
                .map(|v| {
                    format!(
                        "{} · exact revision {}",
                        v.definition.fields.display_name, v.definition.revision
                    )
                })
                .chain(std::iter::once(format!(
                    "{} total; page {n}, at most 25 displayed",
                    items.len()
                )))
                .collect(),
        ));
    }
    for (title, items) in [
        ("Removed destination cells", &p.removed_cells),
        ("Unmapped source cells", &p.unmapped_cells),
    ] {
        out.push(section(
            title,
            items
                .iter()
                .skip((n - 1) * SIZE)
                .take(SIZE)
                .map(|v| {
                    format!(
                        "Collection {} / entry {} / column {}: {}",
                        v.collection_id,
                        v.entry_id,
                        v.cell.column_id,
                        value_text(&v.cell.value)
                    )
                })
                .chain(std::iter::once(format!(
                    "{} total; page {n}, at most 25 displayed",
                    items.len()
                )))
                .collect(),
        ));
    }
    Ok(out)
}
async fn apply(
    s: &AppState,
    c: &mut Context,
    d: &Values,
    token: &str,
) -> Result<Option<(Vec<Section>, String)>, Error> {
    let a = api(s);
    let op = val(d, "operation");
    let intent = val(d, "intent");
    let expected = if c.id.is_empty() {
        0
    } else {
        positive(val(d, "expected_revision"), "expected_revision")?
    };
    if matches!(op, "reconcile" | "copy") {
        let p = relation_preview(s, c, d).await?;
        let digest = hash(&(&p, reason(d)));
        if intent == "confirm" {
            if val(d, "digest") != digest {
                return Err(Error::Conflict);
            }
            a.confirm_card(token, &p, reason(d).as_deref()).await?;
            return Ok(None);
        }
        if intent != "preview" {
            return Err(invalid(
                "intent",
                "Preview the whole replacement before confirming.",
            ));
        }
        return Ok(Some((preview_sections(s, &p, page(d)?).await?, digest)));
    }
    if !matches!(op, "archive" | "adopt") && intent != "save" {
        return Err(invalid("intent", "Use Save operation for this operation."));
    }
    match c.kind {
        Kind::Layout => {
            if op == "create" {
                let row = a
                    .create_layout(
                        token,
                        c.world.as_deref(),
                        NewLayout {
                            fields: fields(d),
                            source: if val(d, "source_layout").is_empty() {
                                None
                            } else {
                                Some(unpack(d, "source_layout")?)
                            },
                            columns: vec![],
                            reason: reason(d),
                        },
                    )
                    .await?;
                c.id = row.id;
                return Ok(None);
            }
            let old = a.layout(c.world.as_deref(), &c.id).await?;
            let mut row = old.clone();
            match op {
                "name" => row.fields.display_name = val(d, "display_name").into(),
                "description" => row.fields.description = normalize(val(d, "text")),
                "column" => {
                    let id = val(d, "column_id");
                    let i = row.columns.iter().position(|x| x.id == id);
                    if val(d, "mode") == "set" {
                        let kind = match val(d, "column_kind") {
                            "field" => ColumnKind::Field(
                                s.cards.field_version(&unpack(d, "field")?).await?,
                            ),
                            "reference" => ColumnKind::Reference {
                                allowed: [
                                    ("allow_card", ReferenceKind::Card),
                                    ("allow_stat", ReferenceKind::Stat),
                                    ("allow_field", ReferenceKind::Field),
                                ]
                                .into_iter()
                                .filter(|(k, _)| val(d, k) == "yes")
                                .map(|(_, v)| v)
                                .collect(),
                            },
                            _ => {
                                return Err(invalid(
                                    "column_kind",
                                    "Choose scalar field or reference.",
                                ));
                            }
                        };
                        let col = RecordColumn {
                            id: if id.is_empty() { new_id()? } else { id.into() },
                            label: val(d, "label").into(),
                            kind,
                        };
                        if let Some(i) = i {
                            row.columns[i] = col;
                        } else if id.is_empty() {
                            row.columns.push(col);
                        } else {
                            return Err(Error::NotFound);
                        }
                    } else {
                        move_item(&mut row.columns, i.ok_or(Error::NotFound)?, val(d, "mode"))?;
                    }
                }
                "archive" | "adopt" => {
                    let target = if op == "adopt" {
                        let pin: LayoutPin = unpack(d, "source_layout")?;
                        let previous = old.source.as_ref().ok_or_else(|| {
                            invalid("source", "This layout has no shared source.")
                        })?;
                        if pin.layout_id != previous.layout_id || pin.world_id.is_some() {
                            return Err(invalid(
                                "source",
                                "Choose the same shared source identity.",
                            ));
                        }
                        Some(a.layout_version(&pin).await?)
                    } else {
                        None
                    };
                    let digest = hash(&(&old, &target, val(d, "archived"), reason(d)));
                    if intent != "confirm" {
                        if intent != "preview" {
                            return Err(invalid("intent", "Preview before confirming."));
                        }
                        let mut sections = layout_sections(&old, 1);
                        sections.push(section("Proposed change",vec![if op=="archive"{format!("Archive state: {} → {}",archive(old.archived_at),val(d,"archived"))}else{"Keep all local metadata and columns; update the explicit shared source pin only. Consumers retain exact existing pins.".into()}]));
                        if let Some(target) = target {
                            sections.push(section("New shared source", vec![]));
                            sections.extend(layout_sections(&target, 1));
                        }
                        return Ok(Some((sections, digest)));
                    }
                    if digest != val(d, "digest") {
                        return Err(Error::Conflict);
                    }
                    if op == "archive" {
                        row.archived_at = archived(d, old.archived_at)?;
                    } else {
                        let target = target.ok_or(Error::NotFound)?;
                        a.adopt_layout(
                            token,
                            c.world.as_deref().ok_or(Error::Forbidden)?,
                            &c.id,
                            target.revision,
                            LayoutEdit {
                                expected_revision: expected,
                                fields: row.fields,
                                columns: row.columns,
                                archived_at: row.archived_at,
                                reason: reason(d),
                            },
                        )
                        .await?;
                        return Ok(None);
                    }
                }
                _ => return Err(invalid("operation", "Choose a layout operation.")),
            }
            a.save_layout(
                token,
                c.world.as_deref(),
                &c.id,
                LayoutEdit {
                    expected_revision: expected,
                    fields: row.fields,
                    columns: row.columns,
                    archived_at: row.archived_at,
                    reason: reason(d),
                },
            )
            .await?;
        }
        Kind::Collection => {
            if op == "create" {
                let row = a
                    .create_collection(
                        token,
                        c.world.as_deref(),
                        NewCollection {
                            fields: fields(d),
                            source: if val(d, "source_collection").is_empty() {
                                None
                            } else {
                                Some(unpack(d, "source_collection")?)
                            },
                            layout: unpack(d, "layout")?,
                            reason: reason(d),
                        },
                    )
                    .await?;
                c.id = row.id;
                return Ok(None);
            }
            let old = a.collection(c.world.as_deref(), &c.id).await?;
            let mut row = old.clone();
            match op {
                "name" => row.fields.display_name = val(d, "display_name").into(),
                "description" => row.fields.description = normalize(val(d, "text")),
                "layout" => row.layout = a.layout_version(&unpack(d, "layout")?).await?,
                "archive" | "adopt" => {
                    let target = if op == "adopt" {
                        let pin: CollectionPin = unpack(d, "source_collection")?;
                        let prev = old.source.as_ref().ok_or_else(|| {
                            invalid("source", "This collection has no shared source.")
                        })?;
                        if pin.collection_id != prev.collection_id || pin.world_id.is_some() {
                            return Err(invalid(
                                "source",
                                "Choose the same shared source identity.",
                            ));
                        }
                        Some(a.collection_version(&pin).await?)
                    } else {
                        None
                    };
                    let digest = hash(&(&old, &target, val(d, "archived"), reason(d)));
                    if intent != "confirm" {
                        if intent != "preview" {
                            return Err(invalid("intent", "Preview before confirming."));
                        }
                        let mut sections = collection_sections(&old, 1);
                        sections.push(section("Proposed change",vec![if op=="archive"{format!("Archive state: {} → {}",archive(old.archived_at),val(d,"archived"))}else{"Keep local metadata and exact layout; update the explicit shared source pin only. Existing consumers retain their pins.".into()}]));
                        if let Some(target) = target {
                            sections.push(section("New shared source", vec![]));
                            sections.extend(collection_sections(&target, 1));
                        }
                        return Ok(Some((sections, digest)));
                    }
                    if digest != val(d, "digest") {
                        return Err(Error::Conflict);
                    }
                    if op == "archive" {
                        row.archived_at = archived(d, old.archived_at)?;
                    } else {
                        let target = target.ok_or(Error::NotFound)?;
                        a.adopt_collection(
                            token,
                            c.world.as_deref().ok_or(Error::Forbidden)?,
                            &c.id,
                            target.revision,
                            CollectionEdit {
                                expected_revision: expected,
                                fields: row.fields,
                                layout: row.layout.pin(),
                                archived_at: row.archived_at,
                                reason: reason(d),
                            },
                        )
                        .await?;
                        return Ok(None);
                    }
                }
                _ => return Err(invalid("operation", "Choose a collection operation.")),
            }
            a.save_collection(
                token,
                c.world.as_deref(),
                &c.id,
                CollectionEdit {
                    expected_revision: expected,
                    fields: row.fields,
                    layout: row.layout.pin(),
                    archived_at: row.archived_at,
                    reason: reason(d),
                },
            )
            .await?;
        }
        Kind::Card => {
            let scalar = s.cards.detail(&c.identity()).await?;
            if scalar.revision != expected {
                return Err(Error::Conflict);
            }
            let mut row = a.card(&scalar.pin()).await?;
            if op == "inventory" {
                match val(d, "mode") {
                    "define" => {
                        if row.relations.collections.is_none() {
                            row.relations.collections = Some(vec![]);
                        }
                    }
                    "unmodel" => {
                        if row
                            .relations
                            .collections
                            .as_ref()
                            .is_some_and(|v| !v.is_empty())
                        {
                            return Err(invalid(
                                "inventory",
                                "Remove collections explicitly before unmodeling.",
                            ));
                        }
                        row.relations.collections = None;
                    }
                    _ => return Err(invalid("mode", "Choose define or unmodel.")),
                }
            } else {
                let pin: CollectionPin = unpack(d, "collection")?;
                let collections = row.relations.collections.get_or_insert_with(Vec::new);
                let i = collections
                    .iter()
                    .position(|x| x.definition.id == pin.collection_id);
                if op == "collection" {
                    match val(d, "mode") {
                        "add" => {
                            if i.is_some() {
                                return Err(invalid(
                                    "collection",
                                    "This collection is already attached; use its state/row/cell operations.",
                                ));
                            }
                            collections.push(RecordCollection {
                                definition: a.collection_version(&pin).await?,
                                value: collection_state(d)?,
                            });
                        }
                        "state" => {
                            let col = &mut collections[i.ok_or(Error::NotFound)?];
                            if matches!(&col.value,CollectionValue::Entries(es) if !es.is_empty()) {
                                return Err(invalid(
                                    "state",
                                    "Remove entries explicitly before replacing their known state.",
                                ));
                            }
                            col.value = collection_state(d)?;
                        }
                        mode => move_item(collections, i.ok_or(Error::NotFound)?, mode)?,
                    }
                } else {
                    let col = &mut collections[i.ok_or(Error::NotFound)?];
                    if op == "row" {
                        let CollectionValue::Entries(es) = &mut col.value else {
                            return Err(invalid(
                                "entry",
                                "Set the collection to known entries before adding rows.",
                            ));
                        };
                        if val(d, "mode") == "add" {
                            es.push(RecordEntry {
                                id: new_id()?,
                                cells: vec![],
                            });
                        } else {
                            let i = es
                                .iter()
                                .position(|e| e.id == val(d, "entry_id"))
                                .ok_or_else(|| invalid("entry", "Choose a named entry."))?;
                            move_item(es, i, val(d, "mode"))?;
                        }
                    } else if op == "cell" {
                        let value = cell_value(s, c, d, col).await?;
                        let CollectionValue::Entries(es) = &mut col.value else {
                            return Err(invalid("entry", "Select a known entry."));
                        };
                        let e = es
                            .iter_mut()
                            .find(|e| e.id == val(d, "entry_id"))
                            .ok_or(Error::NotFound)?;
                        let id = val(d, "column_id");
                        if let Some(v) = value {
                            if let Some(cell) = e.cells.iter_mut().find(|v| v.column_id == id) {
                                cell.value = v;
                            } else {
                                e.cells.push(RecordCell {
                                    column_id: id.into(),
                                    value: v,
                                });
                            }
                        } else {
                            e.cells.retain(|v| v.column_id != id);
                        }
                    } else {
                        return Err(invalid("operation", "Choose a card record operation."));
                    }
                }
            }
            a.save_card(
                token,
                &c.identity(),
                RelationEdit {
                    expected_revision: expected,
                    relations: row.relations,
                    reason: reason(d),
                },
            )
            .await?;
        }
    }
    Ok(None)
}
fn archived(d: &Values, old: Option<i64>) -> Result<Option<i64>, Error> {
    match val(d, "archived") {
        "no" => Ok(None),
        "yes" => Ok(Some(if let Some(t) = old {
            t
        } else {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|_| Error::Unavailable)?
                .as_secs()
                .try_into()
                .map_err(|_| Error::Unavailable)?
        })),
        _ => Err(invalid("archived", "Choose active or archived.")),
    }
}
