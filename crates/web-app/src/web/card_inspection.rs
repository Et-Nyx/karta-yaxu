//! Public card inspection, rendered entirely from one completed public capture.
use super::authoring::{error_status, error_text, invalid, no_store};
use super::*;
use crate::application::{
    authoring::{AddressKey, Error},
    inspection::InspectionResult,
};
use axum::extract::rejection::QueryRejection;
use game_data::{
    card_fields::{FieldKind, FieldValue},
    card_inspection::{self as analysis, Identity, Node, Query as UsageQuery, Site, Usage},
    cards::{CardIdentity, CardNamespace},
    inspection::{CapturedWorld, InspectionPayload},
};

const SIZE: usize = 25;

pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/worlds/{world_id}/inspection/cards", get(inventory))
        .route("/worlds/{world_id}/inspection/cards/where-used", get(usage))
        .route("/worlds/{world_id}/inspection/records", get(inventory))
        .route(
            "/worlds/{world_id}/inspection/records/where-used",
            get(usage),
        )
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawQuery {
    kind: Option<String>,
    target: Option<String>,
    namespace: Option<String>,
    scope: Option<String>,
    id: Option<String>,
    version: Option<String>,
    view: Option<String>,
    page: Option<String>,
    fingerprint: Option<String>,
}
struct Parsed {
    raw: RawQuery,
    namespace: Option<CardNamespace>,
    version: Option<i64>,
    page: usize,
}
fn hex(s: &str, len: usize) -> bool {
    s.len() == len
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn parse(raw: RawQuery, usage: bool) -> Result<Parsed, Error> {
    if raw.fingerprint.as_deref().is_some_and(|s| !hex(s, 64)) {
        return Err(invalid(
            "fingerprint",
            "Use a 64-character lowercase hexadecimal fingerprint.",
        ));
    }
    let page = raw.page.as_deref().unwrap_or("1");
    let page = page
        .parse::<usize>()
        .ok()
        .filter(|n| (1..=10_000).contains(n) && page.bytes().all(|b| b.is_ascii_digit()))
        .ok_or_else(|| invalid("page", "Use a positive page up to 10000."))?;
    let namespace = raw
        .namespace
        .as_deref()
        .map(|s| match s {
            "form" => Ok(CardNamespace::Form),
            "variant" => Ok(CardNamespace::Variant),
            "type" => Ok(CardNamespace::Type),
            "move" => Ok(CardNamespace::Move),
            "standalone" => Ok(CardNamespace::Standalone),
            _ => Err(invalid(
                "namespace",
                "Choose form, variant, type, move or standalone.",
            )),
        })
        .transpose()?;
    if raw.target.as_deref().is_some_and(|s| {
        !matches!(
            s,
            "card" | "field" | "stat" | "layout" | "collection" | "typed"
        )
    }) {
        return Err(invalid(
            "target",
            "Choose card, field, stat, layout, collection or typed.",
        ));
    }
    if raw
        .scope
        .as_deref()
        .is_some_and(|s| !matches!(s, "shared" | "local"))
    {
        return Err(invalid("scope", "Choose shared or local to this world."));
    }
    if raw.id.as_deref().is_some_and(|s| !hex(s, 32)) {
        return Err(invalid(
            "id",
            "Use a 32-character lowercase hexadecimal identity.",
        ));
    }
    if raw
        .view
        .as_deref()
        .is_some_and(|s| !matches!(s, "direct" | "paths"))
    {
        return Err(invalid("view", "Choose direct or paths."));
    }
    let version = match raw.version.as_deref().unwrap_or("all") {
        "all" => None,
        s => Some(
            s.parse::<i64>()
                .ok()
                .filter(|n| *n > 0 && s.bytes().all(|b| b.is_ascii_digit()))
                .ok_or_else(|| invalid("version", "Use all or a positive exact revision."))?,
        ),
    };
    if usage
        && (raw.target.is_none()
            || raw.scope.is_none()
            || raw.id.is_none()
            || (raw.target.as_deref() == Some("card") && namespace.is_none()))
    {
        return Err(invalid(
            "target",
            "Choose target, scope and identity; cards also require a namespace.",
        ));
    }
    if let Some(kind) = &raw.kind {
        kind.parse::<game_data::inspection::DomainKind>()
            .map_err(|_| invalid("kind", "Choose a supported typed object kind."))?;
    }
    if raw.target.as_deref() == Some("typed") && raw.kind.is_none() {
        return Err(invalid(
            "kind",
            "A typed target requires its explicit kind.",
        ));
    }
    if raw.target.as_deref() == Some("card")
        && ((namespace == Some(CardNamespace::Form) && raw.scope.as_deref() == Some("local"))
            || (namespace == Some(CardNamespace::Variant)
                && raw.scope.as_deref() == Some("shared")))
    {
        return Err(invalid("scope", "Forms are shared; variants are local."));
    }
    Ok(Parsed {
        raw,
        namespace,
        version,
        page,
    })
}
fn target(p: &Parsed, world: &str) -> Identity {
    let world_id = (p.raw.scope.as_deref() == Some("local")).then(|| world.to_owned());
    let id = p.raw.id.clone().expect("validated target");
    match p.raw.target.as_deref() {
        Some("field") => Identity::Field { world_id, id },
        Some("layout") => Identity::Layout { world_id, id },
        Some("collection") => Identity::Collection { world_id, id },
        Some("stat") | Some("typed") => Identity::Typed(game_data::inspection::DomainIdentity {
            kind: if p.raw.target.as_deref() == Some("stat") {
                game_data::inspection::DomainKind::Stat
            } else {
                p.raw
                    .kind
                    .as_ref()
                    .expect("validated kind")
                    .parse()
                    .expect("validated kind")
            },
            world_id,
            id,
        }),
        _ => Identity::Card(CardIdentity {
            world_id,
            id,
            namespace: p.namespace.expect("validated namespace"),
        }),
    }
}
fn root(c: &CapturedWorld) -> String {
    format!("/worlds/{}/inspection/cards", c.world.id)
}
fn bounded(s: &str) -> String {
    let mut out: String = s.chars().take(2000).collect();
    if s.chars().nth(2000).is_some() {
        out.push_str("… [preview truncated]");
    }
    out
}
struct Link {
    label: String,
    url: String,
}
struct Entry {
    label: String,
    meta: String,
    text: Vec<String>,
    links: Vec<Link>,
}
struct EdgeView {
    from: Entry,
    to: Entry,
    site: String,
}
struct PathView {
    root: Entry,
    edges: Vec<EdgeView>,
}
#[derive(Template)]
#[template(path = "card_inspection.html")]
struct Page {
    layouts: Vec<Entry>,
    layout_total: usize,
    collections: Vec<Entry>,
    collection_total: usize,
    world: String,
    world_url: String,
    typed_url: String,
    inventory_url: String,
    fingerprint: String,
    complete: bool,
    notice: String,
    gaps: Vec<String>,
    page: usize,
    cards: Vec<Entry>,
    card_total: usize,
    fields: Vec<Entry>,
    field_total: usize,
    usage: bool,
    view: String,
    captured: Vec<Entry>,
    captured_total: usize,
    direct: Vec<EdgeView>,
    paths: Vec<PathView>,
    total: usize,
    truncated: bool,
    empty: String,
    links: Vec<Link>,
}
#[derive(Template)]
#[template(path = "inspection_error.html")]
struct Failure {
    errors: Vec<(String, String)>,
}
fn failure(e: Error) -> Response {
    let status = error_status(&e);
    let errors = if let Error::Invalid(fields) = &e {
        fields.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    } else {
        vec![("Inspection".into(), error_text(&e).into())]
    };
    no_store(render_status(Failure { errors }, status))
}
fn usage_url(
    c: &CapturedWorld,
    identity: &Identity,
    version: Option<i64>,
    view: &str,
    fp: Option<&str>,
) -> String {
    let (target, namespace, world, id) = match identity {
        Identity::Card(i) => (
            "card",
            format!("&namespace={}", i.namespace.as_str()),
            &i.world_id,
            &i.id,
        ),
        Identity::Field { world_id, id } => ("field", String::new(), world_id, id),
        Identity::Typed(i) => (
            "typed",
            format!("&kind={}", i.kind.as_str()),
            &i.world_id,
            &i.id,
        ),
        Identity::Layout { world_id, id } => ("layout", String::new(), world_id, id),
        Identity::Collection { world_id, id } => ("collection", String::new(), world_id, id),
    };
    format!(
        "{}/where-used?target={target}{namespace}&scope={}&id={id}&version={}&view={view}{}",
        root(c),
        if world.is_some() { "local" } else { "shared" },
        version
            .map(|v| v.to_string())
            .unwrap_or_else(|| "all".into()),
        fp.map(|f| format!("&fingerprint={f}")).unwrap_or_default()
    )
}
fn entry(c: &CapturedWorld, n: &Node, fp: Option<&str>, full: bool) -> Entry {
    let identity = n.identity();
    let mut text = Vec::new();
    let mut component_links = Vec::new();
    let history;
    let (label, meta) = match n {
        Node::Card(p) => {
            let scope = p
                .identity
                .world_id
                .as_ref()
                .map(|w| format!("/worlds/{w}"))
                .unwrap_or_default();
            history = Some(format!(
                "{scope}/cards/{}/{}/history/{}",
                p.identity.namespace.as_str(),
                p.identity.id,
                p.version
            ));
            let r = c.card_data.cards.get(p);
            if let Some(r) = r
                && full
            {
                if let Some(source) = &r.source {
                    let e = entry(c, &Node::Card(source.clone()), fp, false);
                    text.push(format!("Exact source: {} · {}", e.label, e.meta));
                    component_links.extend(e.links.into_iter().map(|l| Link {
                        label: format!("Source · {}", l.label),
                        url: l.url,
                    }));
                }
                text.push(format!("Description: {}", bounded(&r.fields.description)));
                text.push(format!(
                    "Rules text (inert): {}",
                    r.component
                        .rules_text
                        .as_deref()
                        .map(bounded)
                        .unwrap_or_else(|| "Unmodeled".into())
                ));
                match &r.component.labels {
                    None => text.push("Labels: Unmodeled".into()),
                    Some(ls) if ls.is_empty() => text.push("Labels: Defined empty".into()),
                    Some(ls) => text.extend(
                        ls.iter()
                            .take(SIZE)
                            .map(|l| format!("Label: {}", bounded(l))),
                    ),
                }
                if let Some(labels) = &r.component.labels
                    && labels.len() > SIZE
                {
                    text.push(format!("Showing 25 of {} labels.", labels.len()));
                }
                match &r.component.fields {
                    None => text.push("Custom fields: Unmodeled".into()),
                    Some(fs) if fs.is_empty() => text.push("Custom fields: Defined empty".into()),
                    Some(fs) => {
                        for f in fs.iter().take(SIZE) {
                            let e = entry(c, &Node::Field(f.definition.pin()), fp, false);
                            component_links.extend(e.links.into_iter().map(|l| Link {
                                label: format!(
                                    "Field {} · {}",
                                    bounded(&f.definition.fields.display_name),
                                    l.label
                                ),
                                url: l.url,
                            }));
                            if let FieldValue::CardReference(pin) = &f.value {
                                let e = entry(c, &Node::Card(pin.clone()), fp, false);
                                component_links.extend(e.links.into_iter().map(|l| Link {
                                    label: format!(
                                        "Reference from field {} · {}",
                                        bounded(&f.definition.fields.display_name),
                                        l.label
                                    ),
                                    url: l.url,
                                }));
                            }
                            let value = match &f.value {
                                FieldValue::Missing => "Missing".into(),
                                FieldValue::Unknown => "Unknown".into(),
                                FieldValue::NotApplicable => "NotApplicable".into(),
                                FieldValue::Number(n) => format!("Known number: {n}"),
                                FieldValue::Text(s) => format!("Known text: {}", bounded(s)),
                                FieldValue::Boolean(b) => format!("Known boolean: {b}"),
                                FieldValue::Choice(id) => {
                                    let label = match &f.definition.kind {
                                        FieldKind::Choice { options } => options
                                            .iter()
                                            .find(|o| &o.id == id)
                                            .map(|o| o.label.as_str()),
                                        _ => None,
                                    };
                                    format!(
                                        "Known choice: {} · {id}",
                                        label.unwrap_or("Captured label unavailable")
                                    )
                                }
                                FieldValue::CardReference(pin) => format!(
                                    "Exact card reference: {} {} {} revision {}",
                                    pin.identity.namespace.as_str(),
                                    if pin.identity.world_id.is_some() {
                                        "local"
                                    } else {
                                        "shared"
                                    },
                                    pin.identity.id,
                                    pin.version
                                ),
                            };
                            text.push(format!(
                                "{} · field {} revision {}: {value}",
                                bounded(&f.definition.fields.display_name),
                                f.definition.id,
                                f.definition.revision
                            ));
                        }
                        if fs.len() > SIZE {
                            text.push(format!("Showing 25 of {} assigned fields/values; remaining values are in exact card history.", fs.len()));
                        }
                    }
                }
            }
            if full && let Some(records) = c.card_data.records.relations.get(p) {
                component_links.push(Link {
                    label: "Exact record collections".into(),
                    url: format!(
                        "{scope}/cards/{}/{}/records/history/{}",
                        p.identity.namespace.as_str(),
                        p.identity.id,
                        p.version
                    ),
                });
                text.push(match &records.relations.collections {None=>"Record inventory: Unmodeled".into(),Some(cols)=>format!("Record inventory: {} selected collections; at most 25 cell occurrences previewed",cols.len())});
                for assignment in records.relations.collections.iter().flatten().take(SIZE) {
                    text.push(format!(
                        "Collection {} revision {}: {}",
                        bounded(&assignment.definition.fields.display_name),
                        assignment.definition.revision,
                        match &assignment.value {
                            game_data::relations::CollectionValue::Missing => "Missing".into(),
                            game_data::relations::CollectionValue::Unknown => "Unknown".into(),
                            game_data::relations::CollectionValue::NotApplicable =>
                                "NotApplicable".into(),
                            game_data::relations::CollectionValue::Entries(es) =>
                                format!("Known entries: {}", es.len()),
                        }
                    ));
                }
                let cells = records
                    .relations
                    .collections
                    .iter()
                    .flatten()
                    .filter_map(|a| {
                        if let game_data::relations::CollectionValue::Entries(es) = &a.value {
                            Some((a, es))
                        } else {
                            None
                        }
                    })
                    .flat_map(|(a, es)| {
                        es.iter()
                            .flat_map(move |e| e.cells.iter().map(move |cell| (a, e, cell)))
                    });
                for (a, e, cell) in cells.take(SIZE) {
                    let column = a
                        .definition
                        .layout
                        .columns
                        .iter()
                        .find(|col| col.id == cell.column_id);
                    let value = match &cell.value {
                        game_data::relations::RecordValue::Reference(pin) => {
                            let target = analysis::record_target(pin);
                            let entry = entry(c, &target, fp, false);
                            let name = entry.label;
                            component_links.extend(entry.links);
                            format!("Exact reference: {name}")
                        }
                        game_data::relations::RecordValue::Missing => "Missing".into(),
                        game_data::relations::RecordValue::Unknown => "Unknown".into(),
                        game_data::relations::RecordValue::NotApplicable => "NotApplicable".into(),
                        game_data::relations::RecordValue::Number(n) => {
                            format!("Known number: {n}")
                        }
                        game_data::relations::RecordValue::Text(v) => {
                            format!("Known text: {}", bounded(v))
                        }
                        game_data::relations::RecordValue::Boolean(v) => {
                            format!("Known boolean: {v}")
                        }
                        game_data::relations::RecordValue::Choice(id) => {
                            let label = column
                                .and_then(|col| {
                                    if let game_data::relations::ColumnKind::Field(f) = &col.kind {
                                        if let FieldKind::Choice { options } = &f.kind {
                                            options
                                                .iter()
                                                .find(|o| &o.id == id)
                                                .map(|o| o.label.as_str())
                                        } else {
                                            None
                                        }
                                    } else {
                                        None
                                    }
                                })
                                .unwrap_or("Unavailable exact option");
                            format!("Known choice: {}", bounded(label))
                        }
                    };
                    text.push(format!(
                        "{} · entry {} · {}: {value}",
                        bounded(&a.definition.fields.display_name),
                        e.id,
                        column
                            .map(|c| bounded(&c.label))
                            .unwrap_or_else(|| cell.column_id.clone())
                    ));
                }
            }
            (
                r.map(|r| bounded(&r.fields.display_name))
                    .or_else(|| {
                        let kind = match p.identity.namespace {
                            CardNamespace::Form => game_data::inspection::DomainKind::Form,
                            CardNamespace::Variant => game_data::inspection::DomainKind::Variant,
                            CardNamespace::Type => game_data::inspection::DomainKind::Type,
                            CardNamespace::Move => game_data::inspection::DomainKind::Move,
                            CardNamespace::Standalone => return None,
                        };
                        c.nodes
                            .get(&game_data::inspection::DomainNodeKey {
                                kind,
                                world_id: p.identity.world_id.clone(),
                                id: p.identity.id.clone(),
                                revision: p.version,
                            })
                            .and_then(InspectionPayload::fields)
                            .map(|f| bounded(&f.display_name))
                    })
                    .unwrap_or_else(|| "Captured card identity; component unavailable".into()),
                format!(
                    "{} · {} · {} · revision {} · {}",
                    p.identity.namespace.as_str(),
                    if p.identity.world_id.is_some() {
                        "Local"
                    } else {
                        "Shared"
                    },
                    p.identity.id,
                    p.version,
                    r.map(|r| if r.archived_at.is_some() {
                        "Archived"
                    } else {
                        "Active"
                    })
                    .unwrap_or("metadata unavailable")
                ),
            )
        }
        Node::Field(p) => {
            let scope = p
                .world_id
                .as_ref()
                .map(|w| format!("/worlds/{w}"))
                .unwrap_or_default();
            history = Some(format!(
                "{scope}/card-fields/{}/history/{}",
                p.field_id, p.version
            ));
            let r = c.card_data.fields.get(p);
            if let Some(r) = r
                && full
            {
                if let Some(source) = &r.source {
                    let e = entry(c, &Node::Field(source.clone()), fp, false);
                    text.push(format!("Exact source: {} · {}", e.label, e.meta));
                    component_links.extend(e.links.into_iter().map(|l| Link {
                        label: format!("Source · {}", l.label),
                        url: l.url,
                    }));
                }
                text.push(format!("Description: {}", bounded(&r.fields.description)));
                let kind = match &r.kind {
                    FieldKind::Number(c) => format!("Exact number constraints: {c:?}"),
                    FieldKind::Text { max_length } => {
                        format!("Text, maximum {max_length} characters")
                    }
                    FieldKind::Boolean => "Boolean".into(),
                    FieldKind::CardReference => "Exact qualified card reference".into(),
                    FieldKind::Choice { options } => {
                        for o in options.iter().take(SIZE) {
                            text.push(format!("Choice {}: {}", o.id, bounded(&o.label)));
                        }
                        format!(
                            "Single choice; showing {} of {} options",
                            options.len().min(SIZE),
                            options.len()
                        )
                    }
                };
                text.push(kind);
            }
            (
                r.map(|r| bounded(&r.fields.display_name))
                    .unwrap_or_else(|| "Captured field identity; payload unavailable".into()),
                format!(
                    "Field · {} · {} · revision {} · {}",
                    if p.world_id.is_some() {
                        "Local"
                    } else {
                        "Shared"
                    },
                    p.field_id,
                    p.version,
                    r.map(|r| if r.archived_at.is_some() {
                        "Archived"
                    } else {
                        "Active"
                    })
                    .unwrap_or("metadata unavailable")
                ),
            )
        }
        Node::Typed(k) => {
            let payload = c.nodes.get(k);
            // These owners are captured public metadata, never current DB lookups.
            history = match payload {
                Some(InspectionPayload::Learnset(r)) => r
                    .variant_id
                    .as_ref()
                    .zip(r.world_id.as_ref())
                    .map(|(v, w)| {
                        format!("/worlds/{w}/variants/{v}/learnset/history/{}", k.revision)
                    })
                    .or_else(|| {
                        r.form_id
                            .as_ref()
                            .map(|f| format!("/forms/{f}/learnset/history/{}", k.revision))
                    }),
                Some(InspectionPayload::Preset(r)) => Some(format!(
                    "/worlds/{}/variants/{}/movesets/{}/history/{}",
                    r.world_id, r.variant_id, r.id, k.revision
                )),
                Some(InspectionPayload::Stat(_)) => Some(format!(
                    "{}/stats/{}/history/{}",
                    k.world_id
                        .as_ref()
                        .map(|w| format!("/worlds/{w}"))
                        .unwrap_or_default(),
                    k.id,
                    k.revision
                )),
                _ => None,
            };
            (
                payload
                    .and_then(InspectionPayload::fields)
                    .map(|f| bounded(&f.display_name))
                    .unwrap_or_else(|| k.kind.as_str().into()),
                format!(
                    "Native {} · {} · revision {}",
                    k.kind.as_str(),
                    k.id,
                    k.revision
                ),
            )
        }
        Node::Layout(p) => {
            history = Some(format!(
                "{}/record-layouts/{}/history/{}",
                p.world_id
                    .as_ref()
                    .map(|w| format!("/worlds/{w}"))
                    .unwrap_or_default(),
                p.layout_id,
                p.version
            ));
            let row = c.card_data.records.layouts.get(p);
            if full && let Some(row) = row {
                text.push(bounded(&row.fields.description));
                for column in row.columns.iter().take(SIZE) {
                    text.push(format!(
                        "Column {} · {} · {}",
                        column.id,
                        bounded(&column.label),
                        match &column.kind {
                            game_data::relations::ColumnKind::Field(f) => format!(
                                "Scalar {} revision {}",
                                bounded(&f.fields.display_name),
                                f.revision
                            ),
                            game_data::relations::ColumnKind::Reference { allowed } =>
                                format!("Allowed target kinds {allowed:?}"),
                        }
                    ));
                }
                if row.columns.len() > SIZE {
                    text.push(format!(
                        "{} more columns; exact history retains all",
                        row.columns.len() - SIZE
                    ));
                }
            }
            (
                row.map(|r| bounded(&r.fields.display_name))
                    .unwrap_or_else(|| "Layout evidence unavailable".into()),
                format!("Record layout · {} · revision {}", p.layout_id, p.version),
            )
        }
        Node::Collection(p) => {
            history = Some(format!(
                "{}/record-collections/{}/history/{}",
                p.world_id
                    .as_ref()
                    .map(|w| format!("/worlds/{w}"))
                    .unwrap_or_default(),
                p.collection_id,
                p.version
            ));
            let row = c.card_data.records.collections.get(p);
            if full && let Some(row) = row {
                text.push(bounded(&row.fields.description));
                let e = entry(c, &Node::Layout(row.layout.pin()), fp, false);
                text.push(format!(
                    "Exact layout: {} revision {}",
                    e.label, row.layout.revision
                ));
                component_links.extend(e.links);
            }
            (
                row.map(|r| bounded(&r.fields.display_name))
                    .unwrap_or_else(|| "Collection evidence unavailable".into()),
                format!(
                    "Collection definition · {} · revision {}",
                    p.collection_id, p.version
                ),
            )
        }
    };
    let mut links = vec![];
    if let Some(url) = history {
        links.push(Link {
            label: "Exact history".into(),
            url,
        });
    }
    for (label, version) in [
        ("Where used · all captured revisions", None),
        ("Where used · exact revision", Some(n.version())),
    ] {
        links.push(Link {
            label: label.into(),
            url: usage_url(c, &identity, version, "direct", fp),
        });
    }
    links.extend(component_links);
    Entry {
        label,
        meta,
        text,
        links,
    }
}
fn edge(c: &CapturedWorld, e: &analysis::Edge, fp: Option<&str>) -> EdgeView {
    let to =
        e.to.as_ref()
            .map(|n| entry(c, n, fp, false))
            .unwrap_or_else(|| Entry {
                label: "Identity-only native reference (not an exact revision pin)".into(),
                meta: format!("{:?}", e.identity),
                text: vec![],
                links: vec![Link {
                    label: "All captured revisions".into(),
                    url: usage_url(c, &e.identity, None, "direct", fp),
                }],
            });
    let site = match &e.site {
        Site::Native(e) => format!("Native {:?} · {:?}", e.kind, e.site),
        Site::CardSource => "Explicit exact card source".into(),
        Site::FieldSource => "Explicit exact field source".into(),
        Site::Assignment { slot } => format!("Field assignment slot {slot}"),
        Site::CardReference { slot, field } => format!(
            "Card reference value · field slot {slot} · field {} revision {}",
            field.field_id, field.version
        ),
        Site::RecordCollection { slot } => format!("Record collection assignment slot {slot}"),
        Site::CollectionLayout => "Exact collection layout".into(),
        Site::LayoutField { slot, column_id } => {
            format!("Layout scalar field column {slot} · {column_id}")
        }
        Site::LayoutSource => "Exact layout source".into(),
        Site::CollectionSource => "Exact collection definition source".into(),
        Site::RecordReference {
            collection,
            entry_id,
            column_id,
            row_slot,
            cell_slot,
        } => format!(
            "Record reference · collection {} revision {} · entry {entry_id} (row {row_slot}) · column {column_id} (cell {cell_slot})",
            collection.collection_id, collection.version
        ),
    };
    EdgeView {
        from: entry(c, &e.from, fp, false),
        to,
        site,
    }
}
fn selected(n: &Node, p: &Parsed) -> bool {
    let (kind, ns, scope, id) = match n {
        Node::Card(k) => (
            "card",
            Some(k.identity.namespace),
            &k.identity.world_id,
            &k.identity.id,
        ),
        Node::Field(k) => ("field", None, &k.world_id, &k.field_id),
        Node::Typed(k) => ("typed", None, &k.world_id, &k.id),
        Node::Layout(k) => ("layout", None, &k.world_id, &k.layout_id),
        Node::Collection(k) => ("collection", None, &k.world_id, &k.collection_id),
    };
    p.raw.target.as_deref().is_none_or(|s| s == kind)
        && p.namespace.is_none_or(|s| ns == Some(s))
        && p.raw
            .scope
            .as_deref()
            .is_none_or(|s| (s == "local") == scope.is_some())
        && p.raw.id.as_deref().is_none_or(|s| s == id)
        && p.version.is_none_or(|v| v == n.version())
}
fn window<T>(it: impl Iterator<Item = T>, page: usize) -> impl Iterator<Item = T> {
    it.skip((page - 1) * SIZE).take(SIZE)
}
fn build(result: &InspectionResult, p: &Parsed, found: Option<&Usage>) -> Page {
    let c = &result.capture;
    let fp = result.fingerprint.as_deref();
    let notice = match (p.raw.fingerprint.as_deref(), fp) {
        (Some(old), Some(new)) if old != new => {
            "World changed — results refreshed from one new capture. Pagination reset to page 1; these are not mixed-snapshot results."
        }
        (Some(_), None) => {
            "Coverage is now Partial — the previous fingerprint cannot be confirmed. Pagination reset to page 1."
        }
        _ => "",
    };
    let page = if notice.is_empty() { p.page } else { 1 };
    let cards: Vec<_> = c
        .card_data
        .cards
        .keys()
        .cloned()
        .map(Node::Card)
        .filter(|n| selected(n, p))
        .collect();
    let fields: Vec<_> = c
        .card_data
        .fields
        .keys()
        .cloned()
        .map(Node::Field)
        .filter(|n| selected(n, p))
        .collect();
    let mut base = if found.is_some() {
        usage_url(
            c,
            &target(p, &c.world.id),
            p.version,
            p.raw.view.as_deref().unwrap_or("direct"),
            fp,
        )
    } else {
        let mut url = format!("{}?", root(c));
        for (key, value) in [
            ("target", &p.raw.target),
            ("kind", &p.raw.kind),
            ("namespace", &p.raw.namespace),
            ("scope", &p.raw.scope),
            ("id", &p.raw.id),
            ("version", &p.raw.version),
            ("view", &p.raw.view),
        ] {
            if let Some(value) = value {
                url.push_str(&format!("{key}={value}&"));
            }
        }
        if let Some(fp) = fp {
            url.push_str(&format!("fingerprint={fp}"));
        }
        url
    };
    if !base.contains('?') {
        base.push('?');
    }
    let layouts = c
        .card_data
        .records
        .layouts
        .keys()
        .cloned()
        .map(Node::Layout)
        .filter(|n| selected(n, p))
        .collect::<Vec<_>>();
    let collections = c
        .card_data
        .records
        .collections
        .keys()
        .cloned()
        .map(Node::Collection)
        .filter(|n| selected(n, p))
        .collect::<Vec<_>>();
    let total = found
        .map(|f| f.total.min(256).max(f.captured.len()))
        .unwrap_or(
            cards
                .len()
                .max(fields.len())
                .max(layouts.len())
                .max(collections.len()),
        );
    let mut links = vec![];
    if page > 1 {
        links.push(Link {
            label: "Previous page".into(),
            url: format!("{base}&page={}", page - 1),
        });
    }
    if page * SIZE < total && page < 10_000 {
        links.push(Link {
            label: "Next page".into(),
            url: format!("{base}&page={}", page + 1),
        });
    }
    if found.is_some() {
        for view in ["direct", "paths"] {
            links.push(Link {
                label: if view == "direct" {
                    "Direct uses"
                } else {
                    "Representative root paths"
                }
                .into(),
                url: usage_url(c, &target(p, &c.world.id), p.version, view, fp),
            });
        }
    }
    let complete = found.map(|f| f.complete).unwrap_or_else(|| c.is_complete());
    let empty = match found {
        Some(f) if f.captured.is_empty() => {
            "Target revision or identity is not captured here. This is not a global or all-history search."
        }
        Some(f) if f.total == 0 && !f.complete => {
            "No observed uses in the inspected portion. Partial coverage or traversal limits prevent an unused claim."
        }
        Some(f) if f.total == 0 && p.raw.view.as_deref() == Some("paths") => {
            "No representative root paths observed. This is not an unused claim; inspect direct uses."
        }
        Some(f) if f.total == 0 => {
            "Unused in this world capture: no incoming content uses. Membership and ownership are excluded. This does not mean globally unused or safe to delete."
        }
        _ => "",
    };
    Page {
        layout_total: layouts.len(),
        collection_total: collections.len(),
        layouts: if found.is_none() {
            window(layouts.iter(), page)
                .map(|n| entry(c, n, fp, true))
                .collect()
        } else {
            vec![]
        },
        collections: if found.is_none() {
            window(collections.iter(), page)
                .map(|n| entry(c, n, fp, true))
                .collect()
        } else {
            vec![]
        },
        world: bounded(&c.world.fields.display_name),
        world_url: format!("/worlds/{}", c.world.id),
        typed_url: format!("/worlds/{}/inspection", c.world.id),
        inventory_url: format!(
            "{}{}",
            root(c),
            fp.map(|f| format!("?fingerprint={f}")).unwrap_or_default()
        ),
        fingerprint: fp.unwrap_or("").into(),
        complete,
        notice: notice.into(),
        gaps: c
            .gaps
            .iter()
            .take(SIZE)
            .map(|g| bounded(&g.message))
            .chain(
                (c.gaps.len() > SIZE)
                    .then(|| format!("Showing 25 of {} coverage gaps.", c.gaps.len())),
            )
            .collect(),
        page,
        card_total: cards.len(),
        field_total: fields.len(),
        cards: if found.is_none() {
            window(cards.iter(), page)
                .map(|n| entry(c, n, fp, true))
                .collect()
        } else {
            vec![]
        },
        fields: if found.is_none() {
            window(fields.iter(), page)
                .map(|n| entry(c, n, fp, true))
                .collect()
        } else {
            vec![]
        },
        usage: found.is_some(),
        view: p.raw.view.clone().unwrap_or_else(|| "direct".into()),
        captured: found
            .map(|f| {
                window(f.captured.iter(), page)
                    .map(|n| entry(c, n, fp, false))
                    .collect()
            })
            .unwrap_or_default(),
        captured_total: found.map(|f| f.captured.len()).unwrap_or(0),
        direct: found
            .map(|f| {
                window(f.direct.iter(), page)
                    .map(|e| edge(c, e, fp))
                    .collect()
            })
            .unwrap_or_default(),
        paths: found
            .map(|f| {
                window(f.paths.iter(), page)
                    .map(|p| PathView {
                        root: entry(c, &p.root, fp, false),
                        edges: p.edges.iter().map(|e| edge(c, e, fp)).collect(),
                    })
                    .collect()
            })
            .unwrap_or_default(),
        total: found.map(|f| f.total).unwrap_or(0),
        truncated: found.is_some_and(|f| f.truncated),
        empty: empty.into(),
        links,
    }
}
async fn respond(
    state: AppState,
    world: String,
    raw: Result<Query<RawQuery>, QueryRejection>,
    usage: bool,
) -> Response {
    let result = async {
        let Query(raw) =
            raw.map_err(|_| invalid("query", "Use only the listed query fields, once each."))?;
        let p = parse(raw, usage)?;
        let result = state
            .inspection
            .inspect(AddressKey::Id(world), false)
            .await?;
        // Moving the whole result retains its private admission permit, including
        // cancellation, through graph work AND rendering in this blocking task.
        tokio::task::spawn_blocking(move || {
            let found = usage.then(|| {
                analysis::where_used(
                    &result.capture,
                    &UsageQuery {
                        target: target(&p, &result.capture.world.id),
                        version: p.version,
                        paths: p.raw.view.as_deref() == Some("paths"),
                    },
                )
            });
            let response = render(build(&result, &p, found.as_ref()));
            drop(result);
            response
        })
        .await
        .map_err(|_| Error::Unavailable)
    }
    .await;
    match result {
        Ok(r) => no_store(r),
        Err(e) => failure(e),
    }
}
async fn inventory(
    State(state): State<AppState>,
    Path(world): Path<String>,
    raw: Result<Query<RawQuery>, QueryRejection>,
) -> Response {
    respond(state, world, raw, false).await
}
async fn usage(
    State(state): State<AppState>,
    Path(world): Path<String>,
    raw: Result<Query<RawQuery>, QueryRejection>,
) -> Response {
    respond(state, world, raw, true).await
}

#[cfg(test)]
#[path = "card_inspection_tests.rs"]
mod tests;
