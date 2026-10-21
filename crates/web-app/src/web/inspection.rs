//! Public native inspection. Every rendered value belongs to one materialized capture.
use super::authoring::{error_status, error_text, invalid, no_store};
use super::*;
use crate::application::authoring::{AddressKey, Error};
use crate::application::inspection::InspectionResult;
use axum::extract::rejection::QueryRejection;
use game_data::inspection::*;
use game_data::inspection_analysis::*;

const PAGE_SIZE: usize = 25;
const MAX_PAGE: usize = 10_000;

pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/worlds/{world_id}/inspection", get(report))
        .route("/worlds/{world_id}/inspection/where-used", get(usage))
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReportQuery {
    category: Option<String>,
    kind: Option<String>,
    compare: Option<String>,
    page: Option<String>,
    fingerprint: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UsageQuery {
    kind: String,
    scope: String,
    id: String,
    version: Option<String>,
    view: Option<String>,
    page: Option<String>,
    compare: Option<String>,
    fingerprint: Option<String>,
}
fn hex_value(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn fingerprint_input(value: Option<&str>) -> Result<(), Error> {
    if value.is_some_and(|v| !hex_value(v, 64)) {
        return Err(invalid(
            "fingerprint",
            "Use a 64-character lowercase hexadecimal inspection fingerprint.",
        ));
    }
    Ok(())
}
fn page_input(value: Option<&str>) -> Result<usize, Error> {
    let value = value.unwrap_or("1");
    if !value.bytes().all(|b| b.is_ascii_digit()) {
        return Err(invalid("page", "Use a positive page number up to 10000."));
    }
    value
        .parse()
        .ok()
        .filter(|n| (1..=MAX_PAGE).contains(n))
        .ok_or_else(|| invalid("page", "Use a positive page number up to 10000."))
}
fn compare_input(value: Option<&str>) -> Result<bool, Error> {
    match value.unwrap_or("no") {
        "yes" => Ok(true),
        "no" => Ok(false),
        _ => Err(invalid("compare", "Choose yes or no.")),
    }
}
fn kind_input(value: &str) -> Result<DomainKind, Error> {
    value
        .parse()
        .map_err(|_| invalid("kind", "Choose a listed hyphenated object kind."))
}
fn versions_input(value: Option<&str>) -> Result<VersionFilter, Error> {
    match value.unwrap_or("all") {
        "all" => Ok(VersionFilter::All),
        v if !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit()) => v
            .parse::<i64>()
            .ok()
            .filter(|n| *n > 0)
            .map(VersionFilter::Exact)
            .ok_or_else(|| invalid("version", "Use all or an exact positive revision.")),
        _ => Err(invalid("version", "Use all or an exact positive revision.")),
    }
}
fn usage_input(raw: &UsageQuery, world: &str) -> Result<WhereUsedQuery, Error> {
    let kind = kind_input(&raw.kind)?;
    if !hex_value(&raw.id, 32) {
        return Err(invalid(
            "id",
            "Use a 32-character lowercase hexadecimal content identity.",
        ));
    }
    let world_id = match raw.scope.as_str() {
        "shared" => None,
        "local" if kind != DomainKind::World => Some(world.into()),
        "local" => {
            return Err(invalid(
                "scope",
                "The world anchor uses shared storage scope; it is labeled World anchor, not shared content.",
            ));
        }
        _ => {
            return Err(invalid(
                "scope",
                "Choose shared or local to this world. Foreign world scopes are not accepted.",
            ));
        }
    };
    if kind == DomainKind::World && raw.id != world {
        return Err(invalid(
            "id",
            "Only this world's anchor can be inspected here.",
        ));
    }
    Ok(WhereUsedQuery {
        target: DomainIdentity {
            kind,
            world_id,
            id: raw.id.clone(),
        },
        versions: versions_input(raw.version.as_deref())?,
        view: match raw.view.as_deref().unwrap_or("direct") {
            "direct" => UseView::DirectUses,
            "paths" => UseView::RootPaths,
            _ => return Err(invalid("view", "Choose direct or paths.")),
        },
        page: page_input(raw.page.as_deref())?,
    })
}

#[derive(Template)]
#[template(path = "inspection_error.html")]
struct InspectionError {
    errors: Vec<(String, String)>,
}
fn failure(error: Error) -> Response {
    let status = error_status(&error);
    let errors = match &error {
        Error::Invalid(fields) => fields.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
        Error::Unavailable => vec![("Inspection".into(), "Inspection is temporarily unavailable or busy. Retry; no complete report was produced.".into())],
        _ => vec![("Inspection".into(), error_text(&error).into())],
    };
    no_store(render_status(InspectionError { errors }, status))
}
fn query_error() -> Error {
    invalid(
        "query",
        "Use the listed query fields once each. Unknown fields and malformed query values are rejected.",
    )
}

struct Link {
    label: String,
    url: String,
}
struct Node {
    label: String,
    kind: &'static str,
    scope: &'static str,
    id: String,
    revision: i64,
    state: String,
    owner: String,
    policy: String,
    links: Vec<Link>,
}
struct Finding {
    category: &'static str,
    code: String,
    subject: Node,
    site: String,
    message: String,
    policy: String,
    comparison: &'static str,
    evidence: Vec<Node>,
    evidence_note: String,
}
struct Gap {
    kind: String,
    message: String,
    subject: Vec<Node>,
}
struct Common {
    world_name: String,
    world_description: String,
    world_slug: String,
    world_state: &'static str,
    world_id: String,
    revision: i64,
    contract: u32,
    complete: bool,
    fingerprint: String,
    notice: String,
    page: usize,
    active: usize,
    archived: usize,
    dependencies: usize,
    nodes: usize,
    edges: usize,
    report_url: String,
    refresh_url: String,
    world_url: String,
    gaps: Vec<Gap>,
}
fn scope(key: &DomainNodeKey) -> &'static str {
    if key.kind == DomainKind::World {
        "World anchor"
    } else if key.world_id.is_some() {
        "Local to this world"
    } else {
        "Shared"
    }
}
fn root_role(role: RootRole) -> &'static str {
    match role {
        RootRole::Active => "Active root",
        RootRole::Archived => "Archived root",
        RootRole::Dependency => "Exact dependency",
    }
}
fn root_url(capture: &CapturedWorld) -> String {
    // Immutable navigation survives a rename between captures. The main address
    // adapter redirects GET/HEAD to the current readable address on the next request.
    format!("/worlds/{}/inspection", capture.world.id)
}
fn suffix(fingerprint: Option<&str>, compare: bool) -> String {
    format!(
        "compare={}{}",
        if compare { "yes" } else { "no" },
        fingerprint
            .map(|f| format!("&fingerprint={f}"))
            .unwrap_or_default()
    )
}
fn usage_url(
    capture: &CapturedWorld,
    key: &DomainNodeKey,
    exact: bool,
    fingerprint: Option<&str>,
    compare: bool,
) -> String {
    format!(
        "{}/where-used?kind={}&scope={}&id={}&version={}&view=direct&{}",
        root_url(capture),
        key.kind.as_str(),
        if key.world_id.is_some() {
            "local"
        } else {
            "shared"
        },
        key.id,
        if exact {
            key.revision.to_string()
        } else {
            "all".into()
        },
        suffix(fingerprint, compare)
    )
}
// Owner identities come only from the captured payload, never a current-head read.
fn content_url(key: &DomainNodeKey, payload: Option<&InspectionPayload>) -> Option<String> {
    let local = key.world_id.as_ref().map(|w| format!("/worlds/{w}"));
    let catalog = |name: &str| format!("{}/{name}/{}", local.as_deref().unwrap_or(""), key.id);
    Some(match key.kind {
        DomainKind::World => format!("/worlds/{}", key.id),
        DomainKind::Species => format!("/creatures/{}", key.id),
        DomainKind::Form => format!("/forms/{}", key.id),
        DomainKind::Variant => catalog("variants"),
        DomainKind::Type => catalog("types"),
        DomainKind::Stat => catalog("stats"),
        DomainKind::MoveAttribute => catalog("move-attributes"),
        DomainKind::Move => catalog("moves"),
        DomainKind::Chart => catalog("charts"),
        DomainKind::StatRoster => format!("{}/stat-roster", local?),
        DomainKind::Ruleset => format!("{}/ruleset", local?),
        DomainKind::Learnset => match payload? {
            InspectionPayload::Learnset(r) => {
                if let (Some(w), Some(v)) = (&r.world_id, &r.variant_id) {
                    format!("/worlds/{w}/variants/{v}/learnset")
                } else {
                    format!("/forms/{}/learnset", r.form_id.as_ref()?)
                }
            }
            _ => return None,
        },
        DomainKind::Preset => match payload? {
            InspectionPayload::Preset(r) => format!(
                "/worlds/{}/variants/{}/movesets/{}",
                r.world_id, r.variant_id, r.id
            ),
            _ => return None,
        },
    })
}
fn node(
    capture: &CapturedWorld,
    key: &DomainNodeKey,
    fingerprint: Option<&str>,
    compare: bool,
) -> Node {
    let payload = capture.nodes.get(key);
    let label = payload
        .and_then(InspectionPayload::fields)
        .map(|f| f.display_name.clone())
        .unwrap_or_else(|| {
            if payload.is_some() {
                key.kind.as_str().into()
            } else {
                "Payload unavailable — immutable identity only".into()
            }
        });
    let owner = match payload {
        Some(InspectionPayload::Learnset(r)) => r
            .variant_id
            .as_ref()
            .map(|id| format!("Variant owner identity {id}"))
            .or_else(|| {
                r.form_id
                    .as_ref()
                    .map(|id| format!("Form owner identity {id}"))
            })
            .unwrap_or_default(),
        Some(InspectionPayload::Preset(r)) => format!("Variant owner identity {}", r.variant_id),
        Some(InspectionPayload::Form(r)) => format!("Species owner identity {}", r.species_id),
        _ => String::new(),
    };
    let policy = match payload {
        Some(InspectionPayload::Preset(r)) => {
            format!("Saved validator policy {} (retained)", r.validator_version)
        }
        _ => String::new(),
    };
    let mut links = vec![];
    if let Some(url) = content_url(key, payload) {
        links.push(Link {
            label: "Exact history".into(),
            url: format!("{url}/history/{}", key.revision),
        });
    }
    links.push(Link {
        label: "Where used · all captured revisions".into(),
        url: usage_url(capture, key, false, fingerprint, compare),
    });
    links.push(Link {
        label: "Where used · exact revision".into(),
        url: usage_url(capture, key, true, fingerprint, compare),
    });
    let role = capture
        .roots
        .get(key)
        .map(|r| root_role(*r))
        .unwrap_or("Exact dependency");
    let state = match payload {
        None => format!("{role}; payload unavailable"),
        Some(p) => format!(
            "{role}; captured metadata {}",
            if p.archived_at().is_some() {
                "Archived"
            } else {
                "Active"
            }
        ),
    };
    Node {
        label,
        kind: key.kind.as_str(),
        scope: scope(key),
        id: key.id.clone(),
        revision: key.revision,
        state,
        owner,
        policy,
        links,
    }
}
fn site(site: &EdgeSite) -> String {
    match site {
        EdgeSite::WorldMember { kind } => format!(
            "Observed world membership: {} (not a historical world pin)",
            kind.as_str()
        ),
        EdgeSite::Owner => "Observed ownership backlink (not an exact pin)".into(),
        EdgeSite::Source => {
            "Explicit source provenance (no implicit shared-form learnset inheritance)".into()
        }
        EdgeSite::Species => "Exact species context".into(),
        EdgeSite::DefaultForm => "Default form identity".into(),
        EdgeSite::Typing { slot } => format!("Typing slot {slot}"),
        EdgeSite::StatValue { slot } => format!("Stat value slot {slot}"),
        EdgeSite::StatRoster => "Exact Stat roster context".into(),
        EdgeSite::RosterEntry { slot } => format!("Stat roster entry {slot}"),
        EdgeSite::ChartRoster { slot } => format!("Chart roster entry {slot}"),
        EdgeSite::Mapping {
            kind,
            slot,
            endpoint,
        } => format!("Explicit {kind:?} mapping slot {slot}, {endpoint:?} endpoint"),
        EdgeSite::Attribute { slot } => format!("Move attribute slot {slot}"),
        EdgeSite::Expectation { slot } => format!("Ruleset expectation slot {slot}"),
        EdgeSite::Acquisition { slot, record_id } => format!(
            "Full learnset context · acquisition slot {slot} · acquisition ID {record_id} (not necessarily selected)"
        ),
        EdgeSite::PresetLearnset => {
            "Full pinned learnset context (distinct from selected acquisitions)".into()
        }
        EdgeSite::PresetRuleset => "Exact preset ruleset context".into(),
        EdgeSite::PresetSelection {
            slot,
            record_id,
            learnset,
        } => format!(
            "Selected acquisition · preset selection slot {slot} · acquisition ID {record_id} · learnset {} revision {}",
            learnset.id, learnset.revision
        ),
    }
}
fn common(result: &InspectionResult, prior: Option<&str>, page: usize, compare: bool) -> Common {
    let c = &result.capture;
    let notice: String = match (prior, result.fingerprint.as_deref()) {
        (Some(old), Some(new)) if old != new => "World changed — results refreshed from one new capture. Pagination reset to page 1; these are not mixed-snapshot results.",
        (Some(_), None) => "Coverage is now Partial — the previous fingerprint cannot be confirmed. Results refreshed from the inspected portion; pagination reset to page 1.",
        _ => "",
    }.into();
    let page = if notice.is_empty() { page } else { 1 };
    let report_url = format!(
        "{}?{}",
        root_url(c),
        suffix(result.fingerprint.as_deref(), compare)
    );
    Common {
        world_name: c.world.fields.display_name.clone(),
        world_description: c.world.fields.description.clone(),
        world_slug: c.world.fields.slug.clone(),
        world_state: if c.world.archived_at.is_some() {
            "Archived"
        } else {
            "Active"
        },
        world_id: c.world.id.clone(),
        revision: c.world.revision,
        contract: c.contract_version,
        complete: c.is_complete(),
        fingerprint: result.fingerprint.clone().unwrap_or_default(),
        notice,
        page,
        active: c.roots.values().filter(|r| **r == RootRole::Active).count(),
        archived: c
            .roots
            .values()
            .filter(|r| **r == RootRole::Archived)
            .count(),
        dependencies: c
            .nodes
            .keys()
            .filter(|key| {
                c.roots
                    .get(*key)
                    .is_none_or(|role| *role == RootRole::Dependency)
            })
            .count(),
        nodes: c.nodes.len(),
        edges: c.edges.len(),
        report_url,
        refresh_url: format!(
            "{}?compare={}",
            root_url(c),
            if compare { "yes" } else { "no" }
        ),
        world_url: format!("/worlds/{}", c.world.id),
        gaps: c
            .gaps
            .iter()
            .map(|g| Gap {
                kind: format!("{:?}", g.kind),
                message: g.message.clone(),
                subject: g
                    .subject
                    .iter()
                    .map(|k| node(c, k, result.fingerprint.as_deref(), compare))
                    .collect(),
            })
            .collect(),
    }
}
fn window<T>(items: impl Iterator<Item = T>, page: usize) -> Vec<T> {
    items.skip((page - 1) * PAGE_SIZE).take(PAGE_SIZE).collect()
}
fn pagination(base: &str, page: usize, total: usize) -> Vec<Link> {
    let mut links = vec![];
    if page > 1 {
        links.push(Link {
            label: "Previous page".into(),
            url: format!("{base}&page={}", page - 1),
        });
    }
    if page * PAGE_SIZE < total {
        links.push(Link {
            label: "Next page".into(),
            url: format!("{base}&page={}", page + 1),
        });
    }
    links
}
fn kinds() -> Vec<&'static str> {
    vec![
        "world",
        "species",
        "form",
        "variant",
        "type",
        "stat",
        "stat-roster",
        "move-attribute",
        "move",
        "chart",
        "ruleset",
        "learnset",
        "preset",
    ]
}
#[derive(Template)]
#[template(path = "inspection.html")]
struct ReportPage {
    common: Common,
    action: String,
    category: String,
    kind: String,
    compare: bool,
    kinds: Vec<&'static str>,
    counts: Vec<(String, usize)>,
    findings: Vec<Finding>,
    finding_total: usize,
    roots: Vec<Node>,
    root_total: usize,
    dependencies: Vec<Node>,
    dependency_total: usize,
    absences: Vec<String>,
    pagination: Vec<Link>,
}
async fn report(
    State(state): State<AppState>,
    Path(world): Path<String>,
    raw: Result<Query<ReportQuery>, QueryRejection>,
) -> Response {
    let result = async {
        let Query(raw) = raw.map_err(|_| query_error())?;
        fingerprint_input(raw.fingerprint.as_deref())?;
        let page = page_input(raw.page.as_deref())?;
        let compare = compare_input(raw.compare.as_deref())?;
        let category = raw.category.as_deref().unwrap_or("all");
        let category_filter = if category == "all" {
            None
        } else {
            Some(category.parse::<FindingCategory>().map_err(|_| {
                invalid(
                    "category",
                    "Choose all, contradiction, incomplete, context_difference or not_evaluated.",
                )
            })?)
        };
        let kind = raw.kind.as_deref().unwrap_or("all");
        let kind_filter = if kind == "all" {
            None
        } else {
            Some(kind_input(kind)?)
        };
        let result = state
            .inspection
            .inspect(AddressKey::Id(world), compare)
            .await?;
        let c = &result.capture;
        let mut common = common(&result, raw.fingerprint.as_deref(), page, compare);
        common.refresh_url = format!("{}&category={category}&kind={kind}", common.refresh_url);
        let selected = |f: &&InspectionFinding| {
            category_filter.is_none_or(|cat| f.category == cat)
                && kind_filter.is_none_or(|kind| f.subject.kind == kind)
        };
        let finding_total = result.findings.iter().filter(selected).count();
        let findings = finding_views(
            window(result.findings.iter().filter(selected), common.page),
            c,
            result.fingerprint.as_deref(),
            compare,
        );
        let roots: Vec<_> = c
            .roots
            .iter()
            .filter(|(key, role)| {
                **role != RootRole::Dependency && kind_filter.is_none_or(|k| key.kind == k)
            })
            .map(|(key, _)| key)
            .collect();
        let dependencies: Vec<_> = c
            .nodes
            .keys()
            .filter(|key| {
                c.roots
                    .get(*key)
                    .is_none_or(|role| *role == RootRole::Dependency)
                    && kind_filter.is_none_or(|k| key.kind == k)
            })
            .collect();
        let root_total = roots.len();
        let dependency_total = dependencies.len();
        let base = format!("{}&category={category}&kind={kind}", common.report_url);
        let navigation = pagination(
            &base,
            common.page,
            finding_total.max(root_total).max(dependency_total),
        );
        let to_node = |key| node(c, key, result.fingerprint.as_deref(), compare);
        Ok(render(ReportPage {
            action: root_url(c),
            category: category.into(),
            kind: kind.into(),
            compare,
            kinds: kinds(),
            counts: [
                FindingCategory::Contradiction,
                FindingCategory::Incomplete,
                FindingCategory::ContextDifference,
                FindingCategory::NotEvaluated,
            ]
            .into_iter()
            .map(|cat| {
                (
                    cat.as_str().into(),
                    result.findings.iter().filter(|f| f.category == cat).count(),
                )
            })
            .collect(),
            findings,
            finding_total,
            roots: window(roots.into_iter(), common.page)
                .into_iter()
                .map(to_node)
                .collect(),
            root_total,
            dependencies: window(dependencies.into_iter(), common.page)
                .into_iter()
                .map(to_node)
                .collect(),
            dependency_total,
            absences: c
                .absences
                .iter()
                .map(|a| {
                    format!(
                        "{}: Unconfigured{}",
                        a.kind.as_str(),
                        a.owner_id
                            .as_ref()
                            .map(|id| format!(" for content owner identity {id}"))
                            .unwrap_or_default()
                    )
                })
                .collect(),
            common,
            pagination: navigation,
        }))
    }
    .await;
    match result {
        Ok(response) => no_store(response),
        Err(error) => failure(error),
    }
}

// A small rendering adapter; diagnostics themselves remain in the domain/application.
fn finding_views(
    findings: Vec<&InspectionFinding>,
    c: &CapturedWorld,
    fingerprint: Option<&str>,
    compare: bool,
) -> Vec<Finding> {
    findings
        .into_iter()
        .map(|f| Finding {
            category: f.category.as_str(),
            code: f.code.clone(),
            subject: node(c, &f.subject, fingerprint, compare),
            site: f
                .site
                .as_ref()
                .map(site)
                .unwrap_or_else(|| "Whole object / exact context".into()),
            message: f.message.clone(),
            policy: f
                .policy
                .map(|p| format!("Validator policy {p}"))
                .unwrap_or_else(|| "Observational diagnostic; no validator policy".into()),
            comparison: if f.comparison {
                "Ad-hoc current-policy comparison; no attributed save"
            } else {
                "Saved inputs / retained saved policy where applicable"
            },
            evidence: f
                .evidence
                .iter()
                .take(PAGE_SIZE)
                .map(|key| node(c, key, fingerprint, compare))
                .collect(),
            evidence_note: if f.evidence.len() > PAGE_SIZE {
                format!("Showing {PAGE_SIZE} of {} exact evidence nodes. The remaining nodes are available in this capture's root/dependency inventories; validation used all inputs.", f.evidence.len())
            } else { String::new() },
        })
        .collect()
}
struct Edge {
    from: Node,
    targets: Vec<Node>,
    identity: String,
    kind: &'static str,
    site: String,
}
fn edge(c: &CapturedWorld, e: &InspectionEdge, fp: Option<&str>, compare: bool) -> Edge {
    Edge {
        from: node(c, &e.from, fp, compare),
        targets: match &e.to {
            DomainRef::Exact(k) => vec![node(c, k, fp, compare)],
            DomainRef::Identity(_) => vec![],
        },
        identity: match &e.to {
            DomainRef::Identity(i) => format!(
                "Identity-only {} {} (no exact revision pin)",
                i.kind.as_str(),
                i.id
            ),
            _ => String::new(),
        },
        kind: match e.kind {
            EdgeKind::Membership => "Observed membership",
            EdgeKind::Ownership => "Observed ownership",
            EdgeKind::ExactPin => "Exact pin",
            EdgeKind::Provenance => "Explicit provenance",
            EdgeKind::Selection => "Actual preset selection",
        },
        site: site(&e.site),
    }
}
struct PathView {
    root: Node,
    role: &'static str,
    edges: Vec<Edge>,
}
#[derive(Template)]
#[template(path = "inspection_where_used.html")]
struct UsagePage {
    common: Common,
    action: String,
    query: WhereUsedQuery,
    version: String,
    view: &'static str,
    scope: String,
    compare: bool,
    captured: Vec<Node>,
    captured_total: usize,
    uses: Vec<Edge>,
    paths: Vec<PathView>,
    total: usize,
    complete: bool,
    truncated: bool,
    empty_message: String,
    pagination: Vec<Link>,
}
async fn usage(
    State(state): State<AppState>,
    Path(world): Path<String>,
    raw: Result<Query<UsageQuery>, QueryRejection>,
) -> Response {
    let result = async {
        let Query(raw) = raw.map_err(|_| query_error())?;
        fingerprint_input(raw.fingerprint.as_deref())?;
        let compare = compare_input(raw.compare.as_deref())?;
        let mut query = usage_input(&raw, &world)?;
        let result = state.inspection.inspect(AddressKey::Id(world), compare).await?;
        if result.capture.card_data.records.relations.values().any(|r|r.relations.collections.as_ref().is_some_and(|c|!c.is_empty())) {
            let target = match query.target.kind {DomainKind::Form|DomainKind::Variant|DomainKind::Type|DomainKind::Move=>Some(("card",format!("&namespace={}",query.target.kind.as_str()))),_=>Some(("typed",format!("&kind={}",query.target.kind.as_str())))};
            if let Some((target,namespace))=target {
                let version=match query.versions {VersionFilter::All=>"all".into(),VersionFilter::Exact(v)=>v.to_string()};
                return Ok(Redirect::temporary(&format!("/worlds/{}/inspection/cards/where-used?target={target}{namespace}&scope={}&id={}&version={version}&view={}&page={}{}",result.capture.world.id,raw.scope,query.target.id,if query.view==UseView::DirectUses{"direct"}else{"paths"},query.page,raw.fingerprint.as_ref().map(|f|format!("&fingerprint={f}")).unwrap_or_default())).into_response());
            }
        }
        let mut common = common(&result, raw.fingerprint.as_deref(), query.page, compare);
        query.page = common.page;
        let (result, found) = state.inspection.usages(result, query.clone()).await?;
        let c = &result.capture;
        let captured_keys: Vec<_> = c.nodes.keys().filter(|k| k.identity() == query.target && match query.versions { VersionFilter::All => true, VersionFilter::Exact(v) => k.revision == v }).collect();
        let captured_total = captured_keys.len();
        let captured = window(captured_keys.into_iter(), query.page).into_iter().map(|k| node(c, k, result.fingerprint.as_deref(), compare)).collect();
        let empty_message = if captured_total == 0 {
            "Target exact revision or identity is not captured here. This is not a global or all-history search.".into()
        } else if found.total == 0 && query.view == UseView::DirectUses && found.complete {
            "Unused in this world capture: no incoming content uses. Membership and ownership are excluded. This does not mean globally unused or safe to delete.".into()
        } else if found.total == 0 && !found.complete {
            "No observed uses in the inspected portion. Partial coverage or traversal limits prevent an unused claim.".into()
        } else if found.total == 0 {
            "No representative root paths observed. Use Direct uses to inspect incoming content sites; this is not an unused claim.".into()
        } else { String::new() };
        let version = match query.versions { VersionFilter::All => "all".into(), VersionFilter::Exact(v) => v.to_string() };
        let view = if query.view == UseView::DirectUses { "direct" } else { "paths" };
        let base = format!("{}/where-used?kind={}&scope={}&id={}&version={version}&view={view}&{}", root_url(c), query.target.kind.as_str(), raw.scope, query.target.id, suffix(result.fingerprint.as_deref(), compare));
        common.refresh_url = format!("{}/where-used?kind={}&scope={}&id={}&version={version}&view={view}&compare={}", root_url(c), query.target.kind.as_str(), raw.scope, query.target.id, if compare { "yes" } else { "no" });
        let pagination = pagination(&base, query.page, found.total.min(256).max(captured_total));
        Ok(render(UsagePage { action: format!("{}/where-used", root_url(c)), common, version, view, scope: raw.scope, compare, captured, captured_total, uses: found.uses.iter().map(|u| edge(c, &u.edge, result.fingerprint.as_deref(), compare)).collect(), paths: found.paths.iter().map(|p| PathView { root: node(c, &p.root, result.fingerprint.as_deref(), compare), role: root_role(p.role), edges: p.edges.iter().map(|e| edge(c, e, result.fingerprint.as_deref(), compare)).collect() }).collect(), total: found.total, complete: found.complete, truncated: found.truncated, empty_message, pagination, query }))
    }.await;
    match result {
        Ok(response) => no_store(response),
        Err(error) => failure(error),
    }
}
