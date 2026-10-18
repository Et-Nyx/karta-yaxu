//! Stateless, bounded selectors. A transition changes only the submitted draft.
use super::*;

pub(super) const KEYS: &[&str] = &[
    "picker",
    "picker_query",
    "picker_label",
    "picker_namespace",
    "picker_kind",
    "picker_archive",
    "picker_scope",
    "picker_page",
    "picker_before",
    "mapping_shared",
    "mapping_page",
    "pending_pick",
    "loaded_field_marker",
];

pub(super) fn prune_defaults(c: &Context, d: &mut Values) {
    let names = controls(c, d)
        .into_iter()
        .map(|c| c.name)
        .collect::<Vec<_>>();
    d.retain(|k, _| {
        names.contains(k)
            || matches!(
                k.as_str(),
                "operation" | "expected_revision" | "loaded_field_id" | "loaded_field_marker"
            )
    });
}
fn number(d: &Values, key: &str) -> Result<usize, Error> {
    if value(d, key).is_empty() {
        return Ok(1);
    }
    value(d, key)
        .parse()
        .ok()
        .filter(|n| (1..=10000).contains(n))
        .ok_or_else(|| invalid(key, "Use a positive page number up to 10000."))
}
fn archive_filter(s: &str) -> Result<ArchiveFilter, Error> {
    match s {
        "" | "all" => Ok(ArchiveFilter::All),
        "active" => Ok(ArchiveFilter::Active),
        "archived" => Ok(ArchiveFilter::Archived),
        _ => Err(invalid("archive", "Choose All, Active or Archived.")),
    }
}
fn kind_tag(s: &str) -> Result<Option<FieldKindTag>, Error> {
    Ok(match s {
        "" => None,
        "number" => Some(FieldKindTag::Number),
        "text" => Some(FieldKindTag::Text),
        "boolean" => Some(FieldKindTag::Boolean),
        "choice" => Some(FieldKindTag::Choice),
        "card_reference" => Some(FieldKindTag::CardReference),
        _ => return Err(invalid("kind", "Choose a supported field kind.")),
    })
}
pub(super) fn field_search(d: &Values, prefix: &str) -> Result<FieldSearch, Error> {
    Ok(FieldSearch {
        query: value(d, &format!("{prefix}query")).into(),
        kind: kind_tag(value(d, &format!("{prefix}kind")))?,
        archive: archive_filter(value(d, &format!("{prefix}archive")))?,
        page: number(d, &format!("{prefix}page"))?,
    })
}
pub(super) fn card_search(d: &Values, prefix: &str) -> Result<CardSearch, Error> {
    let ns = value(d, &format!("{prefix}namespace"));
    Ok(CardSearch {
        query: value(d, &format!("{prefix}query")).into(),
        label: value(d, &format!("{prefix}label")).into(),
        namespace: if ns.is_empty() {
            None
        } else {
            Some(namespace(ns)?)
        },
        archive: archive_filter(value(d, &format!("{prefix}archive")))?,
        page: number(d, &format!("{prefix}page"))?,
    })
}
pub(super) fn catalog_allow(q: &Values, field: bool) -> Result<(), Error> {
    let keys = if field {
        &["query", "kind", "archive", "page"][..]
    } else {
        &["query", "label", "namespace", "archive", "page"][..]
    };
    if q.keys().any(|k| !keys.contains(&k.as_str())) {
        return Err(invalid(
            "query",
            "Use the named catalog filters and page links.",
        ));
    }
    Ok(())
}
pub(super) fn catalog_pages(root: &str, q: &Values, n: usize, more: bool) -> (String, String) {
    let url = |page| {
        let mut d = q.clone();
        put(&mut d, "page", page);
        format!(
            "{root}?{}",
            d.iter()
                .map(|(k, v)| format!("{}={}", query_text(k), query_text(v)))
                .collect::<Vec<_>>()
                .join("&")
        )
    };
    (
        if n > 1 { url(n - 1) } else { String::new() },
        if more { url(n + 1) } else { String::new() },
    )
}
pub(super) fn catalog_controls(d: &Values, field: bool) -> Vec<Control> {
    let mut out = vec![control(d, "query", "Name contains (literal text)", false)];
    if field {
        out.push(select(
            d,
            "kind",
            "Field kind filter",
            &[
                ("", "All kinds"),
                ("number", "Number"),
                ("text", "Text"),
                ("boolean", "Boolean"),
                ("choice", "Choice"),
                ("card_reference", "Card reference"),
            ],
        ));
    } else {
        out.push(control(d, "label", "Exact label", false));
        out.push(select(
            d,
            "namespace",
            "Technical origin filter",
            &[
                ("", "All origins"),
                ("form", "Form"),
                ("variant", "Variant"),
                ("type", "Type"),
                ("move", "Move"),
                ("standalone", "Standalone"),
            ],
        ));
    }
    let mut archive = select(
        d,
        "archive",
        "Archive filter",
        &[
            ("", "All"),
            ("active", "Active"),
            ("archived", "Archived"),
            ("all", "All (explicit)"),
        ],
    );
    archive.help =
        "Filters apply before pagination. Search treats %, _ and backslashes literally.".into();
    out.push(archive);
    out
}

pub(super) struct Candidate {
    pub name: String,
    pub description: String,
    pub context: String,
    pub intent: String,
}
pub(super) struct Picker {
    pub enabled: bool,
    pub title: String,
    pub selected: Vec<String>,
    pub links: Vec<Link>,
    pub candidates: Vec<Candidate>,
    pub previous: String,
    pub next: String,
    pub mappings: Vec<Candidate>,
    pub mapping_previous: String,
    pub mapping_next: String,
    pub pending: String,
    pub assignment: bool,
}
impl Picker {
    pub(super) fn empty() -> Self {
        Self {
            enabled: false,
            title: String::new(),
            selected: vec![],
            links: vec![],
            candidates: vec![],
            previous: String::new(),
            next: String::new(),
            mappings: vec![],
            mapping_previous: String::new(),
            mapping_next: String::new(),
            pending: String::new(),
            assignment: false,
        }
    }
}
fn field_candidate(f: FieldSummary, purpose: &str) -> Candidate {
    Candidate {
        name: f.fields.display_name,
        description: f.fields.description,
        context: format!(
            "{} · {} · revision {} · {} · current identity {} · exact coordinates {}",
            f.kind.as_str(),
            if f.pin.world_id.is_some() {
                "This world"
            } else {
                "Shared library"
            },
            f.pin.version,
            archive_text(f.archived_at),
            archive_text(f.current_archived_at),
            field_pin_text(&f.pin)
        ),
        intent: format!("pick:{purpose}:{}:{}", f.pin.field_id, f.pin.version),
    }
}
fn card_candidate(card: CardSummary, purpose: &str) -> Candidate {
    Candidate {
        name: card.fields.display_name,
        description: card.fields.description,
        context: format!(
            "{} · {} · revision {} · {} · current identity {} · exact coordinates {}",
            card.pin.identity.namespace.as_str(),
            if card.pin.identity.world_id.is_some() {
                "This world"
            } else {
                "Shared library"
            },
            card.pin.version,
            archive_text(card.archived_at),
            archive_text(card.current_archived_at),
            pin_text(&card.pin)
        ),
        intent: format!(
            "pick:{purpose}:{}:{}:{}:{}",
            card.pin.identity.world_id.as_deref().unwrap_or("shared"),
            card.pin.identity.namespace.as_str(),
            card.pin.identity.id,
            card.pin.version
        ),
    }
}
fn set_page(p: &mut Picker, n: usize, more: bool) {
    if n > 1 {
        p.previous = format!("picker-page:{}", n - 1);
    }
    if more {
        p.next = format!("picker-page:{}", n + 1);
    }
}
fn exact_field(d: &Values, c: &Context) -> Result<FieldPin, Error> {
    field_pin(d, c.world.clone())
}
pub(super) fn loaded_marker(pin: &FieldPin, revision: i64) -> String {
    format!(
        "{}:{}:{}:{}",
        pin.world_id.as_deref().unwrap_or("shared"),
        pin.field_id,
        pin.version,
        revision
    )
}
pub(super) fn load_values(d: &mut Values, field: &CardField, revision: i64) {
    put(d, "loaded_field_id", &field.definition.id);
    put(
        d,
        "loaded_field_marker",
        loaded_marker(&field.definition.pin(), revision),
    );
    put(d, "field_id", &field.definition.id);
    put(d, "field_revision", field.definition.revision);
    put(d, "text", "");
    put(d, "boolean", "false");
    put(d, "reference_scope", "shared");
    put(d, "reference_namespace", "standalone");
    put(d, "reference_id", "");
    put(d, "reference_revision", "");
    let state = match &field.value {
        FieldValue::Missing => "missing",
        FieldValue::Unknown => "unknown",
        FieldValue::NotApplicable => "not_applicable",
        FieldValue::Number(value) => {
            put(d, "text", value);
            "number"
        }
        FieldValue::Text(value) => {
            put(d, "text", value);
            "text"
        }
        FieldValue::Boolean(value) => {
            put(d, "boolean", value);
            "boolean"
        }
        FieldValue::Choice(value) => {
            put(d, "text", value);
            "choice"
        }
        FieldValue::CardReference(pin) => {
            put(
                d,
                "reference_scope",
                pin.identity.world_id.as_deref().unwrap_or("shared"),
            );
            put(d, "reference_namespace", pin.identity.namespace.as_str());
            put(d, "reference_id", &pin.identity.id);
            put(d, "reference_revision", pin.version);
            "card_reference"
        }
    };
    put(d, "value_state", state);
}
pub(super) fn check_reference_scope(c: &Context, pin: &CardPin) -> Result<(), Error> {
    if pin.identity.world_id.is_some() && pin.identity.world_id != c.world {
        return Err(invalid(
            "reference_scope",
            "Choose a shared card or a card from the edited card's world.",
        ));
    }
    Ok(())
}
fn known_kind(kind: &FieldKind) -> &'static str {
    match kind {
        FieldKind::Number(_) => "number",
        FieldKind::Text { .. } => "text",
        FieldKind::Boolean => "boolean",
        FieldKind::Choice { .. } => "choice",
        FieldKind::CardReference => "card_reference",
    }
}
pub(super) fn typed_value(d: &Values, f: &FieldDefinition) -> Result<FieldValue, Error> {
    let mut typed = d.clone();
    if value(d, "value_state") == "known" {
        put(&mut typed, "value_state", known_kind(&f.kind));
    }
    field_value(&typed)
}
fn typed_controls(d: &Values, definition: &FieldDefinition, controls: &mut [Control]) {
    for control in controls {
        if control.name == "value_state" {
            *control = select(
                d,
                "value_state",
                "Field value state",
                &[
                    ("missing", "Missing"),
                    ("unknown", "Unknown"),
                    ("not_applicable", "NotApplicable"),
                    ("known", "Known value"),
                    ("number", "Number (advanced exact kind)"),
                    ("text", "Text (advanced exact kind)"),
                    ("boolean", "Boolean (advanced exact kind)"),
                    ("choice", "Choice (advanced exact kind)"),
                    ("card_reference", "CardReference (advanced exact kind)"),
                ],
            );
        }
        if control.name == "text" {
            control.multiline = match definition.kind {
                FieldKind::Number(_) => value(d, "text").contains(['\n', '\r']),
                FieldKind::Text { .. } => true,
                _ => !value(d, "text").is_empty(),
            };
            if matches!(definition.kind, FieldKind::Number(_)) {
                control.inputmode = "decimal".into();
            }
            control.advanced = !matches!(
                definition.kind,
                FieldKind::Text { .. } | FieldKind::Number(_)
            );
            control.help = format!(
                "{}; {}. This raw draft is retained even when its value state is inactive.",
                definition.fields.display_name,
                kind_text(&definition.kind)
            );
        }
        if control.name == "boolean" {
            control.advanced = !matches!(definition.kind, FieldKind::Boolean);
        }
    }
}

pub(super) async fn prepare(
    api: &Cards,
    c: &Context,
    d: &Values,
    controls: &mut Vec<Control>,
) -> Result<Picker, Error> {
    let op = value(d, "operation");
    if !matches!(op, "create" | "assignment" | "copy" | "adopt") {
        return Ok(Picker::empty());
    }
    if op == "create" && c.world.is_none() {
        for control in controls.iter_mut() {
            if matches!(control.name.as_str(), "source_id" | "source_revision") {
                control.advanced = true;
            }
        }
        return Ok(Picker::empty());
    }
    let mut p = Picker::empty();
    p.enabled = true;
    p.assignment = op == "assignment";
    p.pending = value(d, "pending_pick").into();
    for control in controls.iter_mut() {
        if matches!(
            control.name.as_str(),
            "field_id"
                | "field_revision"
                | "reference_scope"
                | "reference_namespace"
                | "reference_id"
                | "reference_revision"
                | "source_id"
                | "source_revision"
                | "shared_field_id"
                | "mappings"
        ) {
            control.advanced = true;
        }
    }
    let mut selected_definition = None;
    if op == "assignment" && !value(d, "field_id").is_empty() {
        match exact_field(d,c) {Ok(pin)=>match api.field_version(&pin).await{Ok(f)=>{p.selected.push(format!("Selected field: {} · {} · {}",f.fields.display_name,field_pin_text(&pin),kind_text(&f.kind)));p.links.push(link("Selected exact field",field_url(&pin)));typed_controls(d,&f,controls);selected_definition=Some(f);},Err(_)=>p.selected.push(format!("Unresolved selected field (raw pin retained): {}",field_pin_text(&pin)))},Err(_)=>p.selected.push("Incomplete raw field pin retained; choose a named definition or correct its exact coordinates.".into())}
    }
    if op == "assignment"
        && !value(d, "reference_id").is_empty()
        && let Ok(pin) = card_pin(d)
    {
        check_reference_scope(c, &pin)?;
        let card = api.version(&pin).await?;
        p.selected.push(format!(
            "Selected reference: {} · {} · {}",
            card.fields.display_name,
            pin_text(&pin),
            archive_text(card.archived_at)
        ));
        p.links
            .push(link("Selected exact reference", card_url(&pin)));
    }
    if let Some(FieldDefinition {
        kind: FieldKind::Choice { options },
        ..
    }) = &selected_definition
    {
        p.selected
            .push(match options.iter().find(|o| o.id == value(d, "text")) {
                Some(option) => format!(
                    "Selected choice: {} · stable option {}",
                    option.label, option.id
                ),
                None => format!(
                    "No checked choice selected; raw value retained: {}",
                    value(d, "text")
                ),
            });
    }
    let source = if op == "create" {
        if value(d, "source_id").is_empty() {
            None
        } else if c.field {
            let pin = FieldPin {
                world_id: None,
                field_id: value(d, "source_id").into(),
                version: positive(value(d, "source_revision"), "source_revision")?,
            };
            let f = api.field_version(&pin).await?;
            p.selected.push(format!(
                "Selected shared source: {} · {}",
                f.fields.display_name,
                field_pin_text(&pin)
            ));
            p.links.push(link("Selected exact source", field_url(&pin)));
            None
        } else {
            Some(CardPin {
                identity: CardIdentity {
                    namespace: CardNamespace::Standalone,
                    world_id: None,
                    id: value(d, "source_id").into(),
                },
                version: positive(value(d, "source_revision"), "source_revision")?,
            })
        }
    } else if matches!(op, "copy" | "adopt") && !c.field {
        api.detail(&c.identity()).await?.source
    } else {
        None
    };
    if let Some(pin) = &source {
        let card = api.version(pin).await?;
        p.selected.push(format!(
            "Pinned shared source: {} · {}",
            card.fields.display_name,
            pin_text(pin)
        ));
        p.links.push(link("Pinned exact source", card_url(pin)));
    }
    let default_picker = match op {
        "assignment" => "field",
        "copy" => "mapping_source",
        "adopt" => "source_versions",
        _ => "source",
    };
    let target = if value(d, "picker").is_empty() {
        default_picker
    } else {
        value(d, "picker")
    };
    let mut selector = select(
        d,
        "picker",
        "Find by name",
        &[(
            default_picker,
            match op {
                "assignment" => "Assignment fields",
                "copy" => "Shared fields to map",
                "adopt" => "Source revisions",
                _ => "Shared creation source",
            },
        )],
    );
    if op == "assignment" {
        for (v, l) in [
            ("field_versions", "Selected field revisions"),
            ("reference", "Reference cards"),
            ("reference_versions", "Selected reference revisions"),
            ("choice", "Exact field choice options"),
        ] {
            selector.options.push(OptionView {
                value: v.into(),
                label: l.into(),
                selected: target == v,
            });
        }
    }
    if op == "copy" {
        selector.options.push(OptionView {
            value: "mapping".into(),
            label: "Eligible local mapping fields".into(),
            selected: target == "mapping",
        });
    }
    if op == "create" && c.world.is_some() {
        selector.options.push(OptionView {
            value: "source_versions".into(),
            label: "Selected creation source revisions".into(),
            selected: target == "source_versions",
        });
    }
    // Empty is a real initial selector value, never a selected candidate or option.
    if value(d, "picker").is_empty() {
        for option in &mut selector.options {
            option.selected = option.value == default_picker;
        }
    }
    controls.push(selector);
    controls.push(control(d, "picker_query", "Picker name contains", false));
    controls.push(select(
        d,
        "picker_archive",
        "Picker archive filter",
        &[
            ("", "All"),
            ("all", "All (explicit)"),
            ("active", "Active"),
            ("archived", "Archived"),
        ],
    ));
    if target == "reference" && c.world.is_some() {
        controls.push(select(
            d,
            "picker_scope",
            "Reference candidate scope",
            &[
                ("", "This world"),
                ("local", "This world (explicit)"),
                ("shared", "Shared library"),
            ],
        ));
    }
    if matches!(target, "reference" | "source") && !c.field {
        if target == "reference" {
            controls.push(select(
                d,
                "picker_namespace",
                "Picker technical origin",
                &[
                    ("", "All origins"),
                    ("standalone", "Standalone"),
                    ("form", "Form"),
                    ("variant", "Variant"),
                    ("type", "Type"),
                    ("move", "Move"),
                ],
            ));
        }
        controls.push(control(d, "picker_label", "Picker exact label", false));
    }
    if matches!(target, "field" | "mapping") || (target == "source" && c.field) {
        controls.push(select(
            d,
            "picker_kind",
            "Picker field kind",
            &[
                ("", "All kinds"),
                ("number", "Number"),
                ("text", "Text"),
                ("boolean", "Boolean"),
                ("choice", "Choice"),
                ("card_reference", "Card reference"),
            ],
        ));
    }
    let n = number(d, "picker_page")?;
    match target {
        "field" if op == "assignment" => {
            p.title = "Assignment fields in this scope".into();
            let page = api
                .search_fields(c.world.as_deref(), &field_search(d, "picker_")?)
                .await?;
            set_page(&mut p, n, page.has_more);
            p.candidates = page
                .items
                .into_iter()
                .map(|f| field_candidate(f, "field"))
                .collect();
        }
        "source" if op == "create" && c.world.is_some() => {
            p.title = "Shared creation sources".into();
            if c.field {
                let page = api
                    .search_fields(None, &field_search(d, "picker_")?)
                    .await?;
                set_page(&mut p, n, page.has_more);
                p.candidates = page
                    .items
                    .into_iter()
                    .map(|f| field_candidate(f, "source_field"))
                    .collect();
            } else {
                let mut search = card_search(d, "picker_")?;
                search.namespace = Some(CardNamespace::Standalone);
                let page = api.search_cards(None, &search).await?;
                set_page(&mut p, n, page.has_more);
                p.candidates = page
                    .items
                    .into_iter()
                    .map(|f| card_candidate(f, "source_card"))
                    .collect();
            }
        }
        "reference" if op == "assignment" => {
            p.title = "Reference cards".into();
            let world = match value(d, "picker_scope") {
                "shared" => None,
                "" | "local" => c.world.as_deref(),
                _ => {
                    return Err(invalid(
                        "picker_scope",
                        "Choose this world or the shared library.",
                    ));
                }
            };
            let page = api.search_cards(world, &card_search(d, "picker_")?).await?;
            set_page(&mut p, n, page.has_more);
            p.candidates = page
                .items
                .into_iter()
                .map(|f| card_candidate(f, "reference"))
                .collect();
        }
        "choice" if op == "assignment" => {
            p.title = "Options from the selected exact field".into();
            if let Some(f) = &selected_definition
                && let FieldKind::Choice { .. } = &f.kind
            {
                let page = api
                    .choice_options(&f.pin(), value(d, "picker_query"), n)
                    .await?;
                set_page(&mut p, n, page.has_more);
                p.candidates = page
                    .items
                    .into_iter()
                    .map(|o| Candidate {
                        name: o.label,
                        description: String::new(),
                        context: format!("Stable option {}", o.id),
                        intent: format!("pick:choice:{}", o.id),
                    })
                    .collect();
            }
        }
        "field_versions" if op == "assignment" => {
            p.title = "Exact selected field revisions".into();
            if !value(d, "field_id").is_empty() {
                let before = before(d)?;
                let page = api
                    .field_revisions(c.world.as_deref(), value(d, "field_id"), before)
                    .await?;
                if let Some(last) = page.items.last()
                    && page.has_more
                {
                    p.next = format!("picker-before:{}", last.pin.version);
                }
                p.candidates = page
                    .items
                    .into_iter()
                    .map(|f| field_candidate(f, "field"))
                    .collect();
            }
        }
        "reference_versions" if op == "assignment" => {
            p.title = "Exact selected reference revisions".into();
            if let Ok(pin) = card_pin(d) {
                check_reference_scope(c, &pin)?;
                let page = api.card_revisions(&pin.identity, before(d)?).await?;
                if let Some(last) = page.items.last()
                    && page.has_more
                {
                    p.next = format!("picker-before:{}", last.pin.version);
                }
                p.candidates = page
                    .items
                    .into_iter()
                    .map(|f| card_candidate(f, "reference"))
                    .collect();
            }
        }
        "source_versions" if matches!(op, "adopt" | "create") && c.world.is_some() => {
            p.title = "Exact revisions of the pinned shared source".into();
            if c.field {
                let pin = if op == "create" {
                    if value(d, "source_id").is_empty() {
                        None
                    } else {
                        Some(FieldPin {
                            world_id: None,
                            field_id: value(d, "source_id").into(),
                            version: positive(value(d, "source_revision"), "source_revision")?,
                        })
                    }
                } else {
                    api.field(c.world.as_deref(), &c.id).await?.source
                };
                if let Some(pin) = pin {
                    let selected = FieldPin {
                        version: positive(value(d, "source_revision"), "source_revision")?,
                        ..pin.clone()
                    };
                    let f = api.field_version(&selected).await?;
                    p.selected.push(format!(
                        "Selected adoption: {} · {}",
                        f.fields.display_name,
                        field_pin_text(&selected)
                    ));
                    let page = api.field_revisions(None, &pin.field_id, before(d)?).await?;
                    if let Some(last) = page.items.last()
                        && page.has_more
                    {
                        p.next = format!("picker-before:{}", last.pin.version);
                    }
                    p.candidates = page
                        .items
                        .into_iter()
                        .map(|f| {
                            field_candidate(
                                f,
                                if op == "create" {
                                    "source_field"
                                } else {
                                    "adopt_field"
                                },
                            )
                        })
                        .collect();
                }
            } else if let Some(pin) = &source {
                let selected = CardPin {
                    version: positive(value(d, "source_revision"), "source_revision")?,
                    ..pin.clone()
                };
                let card = api.version(&selected).await?;
                p.selected.push(format!(
                    "Selected adoption: {} · {}",
                    card.fields.display_name,
                    pin_text(&selected)
                ));
                let page = api.card_revisions(&pin.identity, before(d)?).await?;
                if let Some(last) = page.items.last()
                    && page.has_more
                {
                    p.next = format!("picker-before:{}", last.pin.version);
                }
                p.candidates = page
                    .items
                    .into_iter()
                    .map(|f| {
                        card_candidate(
                            f,
                            if op == "create" {
                                "source_card"
                            } else {
                                "adopt_card"
                            },
                        )
                    })
                    .collect();
            }
        }
        "mapping_source" if op == "copy" => {
            p.title = "Shared source assignments to map".into();
            if let Some(pin) = &source {
                let card = api.version(pin).await?;
                let fields = card
                    .component
                    .fields
                    .iter()
                    .flatten()
                    .filter(|f| {
                        f.definition
                            .fields
                            .display_name
                            .to_ascii_lowercase()
                            .contains(&value(d, "picker_query").to_ascii_lowercase())
                    })
                    .collect::<Vec<_>>();
                set_page(&mut p, n, fields.len() > n * SIZE);
                p.candidates = fields
                    .into_iter()
                    .skip((n - 1) * SIZE)
                    .take(SIZE)
                    .map(|f| Candidate {
                        name: f.definition.fields.display_name.clone(),
                        description: f.definition.fields.description.clone(),
                        context: field_pin_text(&f.definition.pin()),
                        intent: format!("pick:mapping_source:{}", f.definition.id),
                    })
                    .collect();
            }
        }
        "mapping" if op == "copy" => {
            p.title = "Local fields explicitly sourced from this shared identity".into();
            if let (Some(world), Some(pin)) = (&c.world, &source) {
                let card = api.version(pin).await?;
                if let Some(f) = card
                    .component
                    .fields
                    .iter()
                    .flatten()
                    .find(|f| f.definition.id == value(d, "mapping_shared"))
                {
                    p.selected.push(format!(
                        "Mapping shared field: {} · {}",
                        f.definition.fields.display_name,
                        field_pin_text(&f.definition.pin())
                    ));
                    let page = api
                        .mapping_fields(world, &f.definition.id, &field_search(d, "picker_")?)
                        .await?;
                    set_page(&mut p, n, page.has_more);
                    p.candidates = page
                        .items
                        .into_iter()
                        .map(|f| field_candidate(f, "mapping"))
                        .collect();
                }
            }
        }
        _ => {
            p.title = "Choose a selector for this operation".into();
        }
    }
    if op == "copy" {
        let mappings = parse_mappings(d, c.world.clone());
        if let Ok(mappings) = mappings {
            let n = number(d, "mapping_page")?;
            if n > 1 {
                p.mapping_previous = format!("mapping-page:{}", n - 1);
            }
            if mappings.len() > n * SIZE {
                p.mapping_next = format!("mapping-page:{}", n + 1);
            }
            let source = if let Some(pin) = &source {
                Some(api.version(pin).await?)
            } else {
                None
            };
            for m in mappings.iter().skip((n - 1) * SIZE).take(SIZE) {
                let f = api.field_version(&m.local_field).await?;
                let shared = source
                    .as_ref()
                    .and_then(|card| {
                        card.component
                            .fields
                            .iter()
                            .flatten()
                            .find(|f| f.definition.id == m.shared_field_id)
                    })
                    .map(|f| f.definition.fields.display_name.as_str())
                    .unwrap_or("Unmapped source identity");
                p.mappings.push(Candidate {
                    name: format!("{shared} → {}", f.fields.display_name),
                    description: f.fields.description,
                    context: format!("{} → {}", m.shared_field_id, field_pin_text(&m.local_field)),
                    intent: format!("mapping-remove:{}", m.shared_field_id),
                });
            }
        }
    }
    Ok(p)
}
fn before(d: &Values) -> Result<Option<i64>, Error> {
    if value(d, "picker_before").is_empty() {
        Ok(None)
    } else {
        positive(value(d, "picker_before"), "picker_before").map(Some)
    }
}
pub(super) fn field_url(pin: &FieldPin) -> String {
    format!(
        "{}/card-fields/{}/history/{}",
        pin.world_id
            .as_ref()
            .map(|w| format!("/worlds/{w}"))
            .unwrap_or_default(),
        pin.field_id,
        pin.version
    )
}
pub(super) fn card_url(pin: &CardPin) -> String {
    format!(
        "{}/cards/{}/{}/history/{}",
        pin.identity
            .world_id
            .as_ref()
            .map(|w| format!("/worlds/{w}"))
            .unwrap_or_default(),
        pin.identity.namespace.as_str(),
        pin.identity.id,
        pin.version
    )
}

pub(super) fn is_transition(intent: &str) -> bool {
    matches!(
        intent,
        "picker-search" | "load-assignment" | "discard-load" | "discard-value" | "retry-pick"
    ) || [
        "pick:",
        "picker-page:",
        "picker-before:",
        "mapping-page:",
        "mapping-remove:",
    ]
    .iter()
    .any(|prefix| intent.starts_with(prefix))
}
/// Conservative URL encoding, including textarea LF→CRLF and next-request controls.
fn form_size(d: &Values) -> usize {
    fn size(s: &str) -> usize {
        s.bytes()
            .map(|b| {
                if b == b'\n' {
                    6
                } else if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
                    1
                } else {
                    3
                }
            })
            .sum()
    }
    d.iter().map(|(k, v)| size(k) + size(v) + 2).sum::<usize>() + 1024
}
fn eligible_field(f: &FieldDefinition, retained: bool) -> Result<(), Error> {
    if f.archived_at.is_some() && !retained {
        Err(invalid(
            "field_id",
            "Archived definitions remain readable, but cannot be newly selected.",
        ))
    } else {
        Ok(())
    }
}
fn eligible_card(f: &CardRecord, retained: bool) -> Result<(), Error> {
    if f.archived_at.is_some() && !retained {
        Err(invalid(
            "reference_id",
            "Archived cards remain readable, but cannot be newly selected.",
        ))
    } else {
        Ok(())
    }
}
async fn retained_field(api: &Cards, c: &Context, pin: &FieldPin) -> Result<bool, Error> {
    if c.id.is_empty() {
        return Ok(false);
    }
    let card = api.detail(&c.identity()).await?;
    Ok(card
        .component
        .fields
        .iter()
        .flatten()
        .any(|f| f.definition.pin() == *pin))
}
pub(super) async fn transition(api: &Cards, c: &Context, d: &mut Values) -> Result<(), Error> {
    let mut next = d.clone();
    let requested = if value(d, "intent") == "retry-pick" {
        value(d, "pending_pick")
    } else {
        value(d, "intent")
    }
    .to_owned();
    let op = value(d, "operation");
    if requested == "discard-value" && op == "assignment" {
        for key in [
            "text",
            "boolean",
            "reference_id",
            "reference_revision",
            "loaded_field_id",
            "loaded_field_marker",
        ] {
            put(&mut next, key, "");
        }
        put(&mut next, "value_state", "missing");
    } else if requested == "picker-search" {
        put(&mut next, "picker_page", 1);
        next.remove("picker_before");
    } else if let Some(n) = requested.strip_prefix("picker-page:") {
        put(&mut next, "picker_page", n);
        number(&next, "picker_page")?;
    } else if let Some(n) = requested.strip_prefix("picker-before:") {
        put(&mut next, "picker_before", positive(n, "picker_before")?);
    } else if let Some(n) = requested.strip_prefix("mapping-page:") {
        put(&mut next, "mapping_page", n);
        number(&next, "mapping_page")?;
    } else if matches!(requested.as_str(), "load-assignment" | "discard-load") && op == "assignment"
    {
        let pin = exact_field(d, c)?;
        let start = positive(value(d, "expected_revision"), "expected_revision")?;
        let saved = api
            .version(&CardPin {
                identity: c.identity(),
                version: start,
            })
            .await?;
        let assignment=saved.component.fields.iter().flatten().find(|f|f.definition.pin()==pin).ok_or_else(||invalid("field_id","This exact field is not assigned in the starting card revision. Select the retained exact pin before loading."))?;
        if requested != "discard-load"
            && (!value(d, "text").is_empty()
                || !value(d, "loaded_field_marker").is_empty()
                || (matches!(assignment.definition.kind, FieldKind::Text { .. })
                    && matches!(value(d, "value_state"), "known" | "text")))
        {
            let marker = value(d, "loaded_field_marker");
            let clean = if marker.is_empty() {
                false
            } else {
                // Compare against the previously loaded exact assignment, even
                // when the pending field selection is now a different identity.
                let previous = saved
                    .component
                    .fields
                    .iter()
                    .flatten()
                    .find(|f| loaded_marker(&f.definition.pin(), start) == marker);
                if let Some(previous) = previous {
                    let mut clean = Values::new();
                    load_values(&mut clean, previous, start);
                    textarea_text(value(d, "text")) == textarea_text(value(&clean, "text"))
                } else {
                    false
                }
            };
            if !clean {
                return Err(invalid(
                    "text",
                    "Save the dirty loaded value, or explicitly Discard text and load selected assignment.",
                ));
            }
        }
        load_values(&mut next, assignment, start);
    } else if let Some(id) = requested.strip_prefix("mapping-remove:") {
        if op != "copy" {
            return Err(invalid("operation", "Mapping edits belong to source copy."));
        }
        let mut mappings = parse_mappings(d, c.world.clone())?;
        mappings.retain(|m| m.shared_field_id != id);
        store_mappings(&mut next, &mappings);
    } else if let Some(pick) = requested.strip_prefix("pick:") {
        let parts = pick.split(':').collect::<Vec<_>>();
        match parts.as_slice() {
            [purpose, id, rev]
                if matches!(
                    *purpose,
                    "field" | "source_field" | "mapping" | "adopt_field"
                ) =>
            {
                let world = if matches!(*purpose, "source_field" | "adopt_field") {
                    None
                } else {
                    c.world.clone()
                };
                let pin = FieldPin {
                    world_id: world,
                    field_id: (*id).into(),
                    version: positive(rev, "field_revision")?,
                };
                pin.validate().map_err(Error::Invalid)?;
                let f = api.field_version(&pin).await?;
                match *purpose {
                    "field" if op == "assignment" => {
                        let retained = retained_field(api, c, &pin).await?;
                        eligible_field(&f, retained)?;
                        eligible_field(&api.field(pin.world_id.as_deref(), id).await?, retained)?;
                        put(&mut next, "field_id", id);
                        put(&mut next, "field_revision", rev);
                        // Choosing a kind never changes state or the one raw value.
                    }
                    "source_field" if op == "create" && c.field && c.world.is_some() => {
                        eligible_field(&f, false)?;
                        eligible_field(&api.field(None, id).await?, false)?;
                        put(&mut next, "source_id", id);
                        put(&mut next, "source_revision", rev);
                    }
                    "adopt_field" if op == "adopt" && c.field => {
                        let original = api
                            .field(c.world.as_deref(), &c.id)
                            .await?
                            .source
                            .ok_or_else(|| invalid("source", "No shared source."))?;
                        if original.field_id != *id || pin.version < original.version {
                            return Err(invalid(
                                "source_revision",
                                "Choose the pinned shared identity at its retained or a newer exact revision.",
                            ));
                        }
                        eligible_field(&f, pin == original)?;
                        eligible_field(&api.field(None, id).await?, pin == original)?;
                        put(&mut next, "source_revision", rev);
                    }
                    "mapping" if op == "copy" => {
                        let shared = value(d, "mapping_shared");
                        if shared.is_empty() {
                            return Err(invalid(
                                "mapping_shared",
                                "Choose a named shared assignment first.",
                            ));
                        }
                        let source = api
                            .detail(&c.identity())
                            .await?
                            .source
                            .ok_or_else(|| invalid("source", "No source card."))?;
                        let source = api.version(&source).await?;
                        if !source
                            .component
                            .fields
                            .iter()
                            .flatten()
                            .any(|f| f.definition.id == shared)
                        {
                            return Err(invalid(
                                "mapping_shared",
                                "Choose a field in the exact pinned source.",
                            ));
                        }
                        if !f
                            .source
                            .as_ref()
                            .is_some_and(|s| s.world_id.is_none() && s.field_id == shared)
                        {
                            return Err(invalid(
                                "field_id",
                                "This local definition does not explicitly source the selected shared identity.",
                            ));
                        }
                        let mut mappings = parse_mappings(d, c.world.clone())?;
                        let retained = retained_field(api, c, &pin).await?;
                        eligible_field(&f, retained)?;
                        eligible_field(&api.field(c.world.as_deref(), id).await?, retained)?;
                        if mappings
                            .iter()
                            .any(|m| m.shared_field_id != shared && m.local_field.field_id == *id)
                        {
                            return Err(invalid(
                                "mappings",
                                "A local definition may map only one shared identity.",
                            ));
                        }
                        mappings.retain(|m| m.shared_field_id != shared);
                        mappings.push(FieldMapping {
                            shared_field_id: shared.into(),
                            local_field: pin,
                        });
                        store_mappings(&mut next, &mappings);
                    }
                    _ => {
                        return Err(invalid(
                            "picker",
                            "This selection does not belong to the loaded operation.",
                        ));
                    }
                }
            }
            [purpose, scope, ns, id, rev]
                if matches!(*purpose, "reference" | "source_card" | "adopt_card") =>
            {
                let pin = CardPin {
                    identity: CardIdentity {
                        namespace: namespace(ns)?,
                        world_id: if *scope == "shared" {
                            None
                        } else {
                            Some((*scope).into())
                        },
                        id: (*id).into(),
                    },
                    version: positive(rev, "reference_revision")?,
                };
                pin.validate().map_err(Error::Invalid)?;
                check_reference_scope(c, &pin)?;
                let card = api.version(&pin).await?;
                match *purpose {
                    "reference" if op == "assignment" => {
                        let saved = api.detail(&c.identity()).await?;
                        let retained = saved.component.fields.iter().flatten().any(|f| {
                            exact_field(d, c).ok().as_ref() == Some(&f.definition.pin())
                                && matches!(&f.value, FieldValue::CardReference(p) if *p == pin)
                        });
                        eligible_card(&card, retained)?;
                        eligible_card(&api.detail(&pin.identity).await?, retained)?;
                        put(&mut next, "reference_scope", scope);
                        put(&mut next, "reference_namespace", ns);
                        put(&mut next, "reference_id", id);
                        put(&mut next, "reference_revision", rev);
                    }
                    "source_card"
                        if op == "create"
                            && !c.field
                            && c.world.is_some()
                            && pin.identity.world_id.is_none()
                            && pin.identity.namespace == CardNamespace::Standalone =>
                    {
                        eligible_card(&card, false)?;
                        eligible_card(&api.detail(&pin.identity).await?, false)?;
                        put(&mut next, "source_id", id);
                        put(&mut next, "source_revision", rev);
                    }
                    "adopt_card" if op == "adopt" && !c.field => {
                        let original = api
                            .detail(&c.identity())
                            .await?
                            .source
                            .ok_or_else(|| invalid("source", "No shared source."))?;
                        if original.identity != pin.identity || pin.version < original.version {
                            return Err(invalid(
                                "source_revision",
                                "Choose the pinned shared identity at its retained or a newer exact revision.",
                            ));
                        }
                        eligible_card(&card, pin == original)?;
                        eligible_card(&api.detail(&pin.identity).await?, pin == original)?;
                        put(&mut next, "source_revision", rev);
                    }
                    _ => {
                        return Err(invalid(
                            "picker",
                            "This selection does not belong to the loaded operation.",
                        ));
                    }
                }
            }
            ["choice", id] if op == "assignment" => {
                let f = api.field_version(&exact_field(d, c)?).await?;
                let FieldKind::Choice { options } = f.kind else {
                    return Err(invalid(
                        "field_id",
                        "Choose an exact Choice definition first.",
                    ));
                };
                if !options.iter().any(|o| o.id == *id) {
                    return Err(invalid(
                        "text",
                        "Choose an option in this exact field revision.",
                    ));
                }
                // Replacing dirty prose is an explicit load/discard operation, not a choice side effect.
                if !value(d, "text").is_empty() && !options.iter().any(|o| o.id == value(d, "text"))
                {
                    return Err(invalid(
                        "text",
                        "Save or clear the existing raw value before choosing a different kind's option.",
                    ));
                }
                put(&mut next, "text", id);
            }
            ["mapping_source", id] if op == "copy" => {
                let source = api
                    .detail(&c.identity())
                    .await?
                    .source
                    .ok_or_else(|| invalid("source", "No shared source."))?;
                let source = api.version(&source).await?;
                if !source
                    .component
                    .fields
                    .iter()
                    .flatten()
                    .any(|f| f.definition.id == *id)
                {
                    return Err(invalid(
                        "mapping_shared",
                        "Choose a field in the exact pinned source.",
                    ));
                }
                put(&mut next, "mapping_shared", id);
                put(&mut next, "picker", "mapping");
                put(&mut next, "picker_query", "");
                put(&mut next, "picker_page", 1);
            }
            _ => return Err(invalid("picker", "Choose a named exact result.")),
        }
    } else {
        return Err(invalid("intent", "Use a named selector action."));
    }
    next.remove("digest");
    next.remove("pending_pick");
    next.remove("intent");
    if form_size(&next) > 32 * 1024 {
        // The original raw values and attempted selection remain usable. Nothing is truncated.
        put(d, "pending_pick", &requested);
        return Err(invalid(
            "form",
            "This addition would exceed the 32 KiB encoded form budget. The prior draft and pending selection are retained. Shorten the draft or remove a mapping before retrying.",
        ));
    }
    *d = next;
    Ok(())
}
fn store_mappings(d: &mut Values, mappings: &[FieldMapping]) {
    put(
        d,
        "mappings",
        mappings
            .iter()
            .map(|m| {
                format!(
                    "{} {} {}",
                    m.shared_field_id, m.local_field.field_id, m.local_field.version
                )
            })
            .collect::<Vec<_>>()
            .join("\n"),
    );
    for key in ["shared_field_id", "field_id", "field_revision"] {
        put(d, key, "");
    }
}
