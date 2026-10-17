//! Pure diagnostics and bounded usage queries over one public, world-scoped capture.
//! No reads, mutations, clocks, inferred relationships, or combat-readiness claims.
use crate::{
    charts::ChartRecord,
    inspection::{
        CapturedWorld, DomainIdentity, DomainKind, DomainNodeKey, DomainRef, EdgeKind, EdgeSite,
        InspectionEdge, InspectionPayload, RootRole,
    },
    learning::{MovesetRecord, validate_preset},
    moves::NumericValue,
    rulesets::{
        CURRENT_VALIDATOR_VERSION, RulesetRecord, ValidationReport, validate_move,
        validate_ruleset_with_policy,
    },
    stats::BaseStats,
    types::TypingState,
};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    str::FromStr,
    sync::Arc,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FindingCategory {
    Contradiction,
    Incomplete,
    ContextDifference,
    NotEvaluated,
}
impl FindingCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Contradiction => "contradiction",
            Self::Incomplete => "incomplete",
            Self::ContextDifference => "context_difference",
            Self::NotEvaluated => "not_evaluated",
        }
    }
}
impl FromStr for FindingCategory {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "contradiction" => Ok(Self::Contradiction),
            "incomplete" => Ok(Self::Incomplete),
            "context_difference" => Ok(Self::ContextDifference),
            "not_evaluated" => Ok(Self::NotEvaluated),
            _ => Err("Unknown inspection finding category.".into()),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct InspectionFinding {
    pub category: FindingCategory,
    pub code: String,
    pub subject: DomainNodeKey,
    pub site: Option<EdgeSite>,
    pub message: String,
    /// Immutable, canonical exact-key evidence, shared across a report's leaf
    /// findings without truncating inputs or copying the full slice per message.
    pub evidence: Arc<[DomainNodeKey]>,
    pub policy: Option<i64>,
    pub comparison: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VersionFilter {
    All,
    Exact(i64),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UseView {
    DirectUses,
    RootPaths,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WhereUsedQuery {
    pub target: DomainIdentity,
    pub versions: VersionFilter,
    pub view: UseView,
    pub page: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Usage {
    pub edge: InspectionEdge,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UsagePath {
    pub root: DomainNodeKey,
    pub role: RootRole,
    pub edges: Vec<InspectionEdge>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WhereUsedResult {
    pub uses: Vec<Usage>,
    pub paths: Vec<UsagePath>,
    /// Observed results before the result cap and pagination, not a global count.
    pub total: usize,
    pub truncated: bool,
    pub complete: bool,
}
const PAGE_SIZE: usize = 25;
const MAX_WORK: usize = 65_536;
const MAX_DEPTH: usize = 32;
const MAX_RESULTS: usize = 256;

fn in_scope(capture: &CapturedWorld, key: &DomainNodeKey) -> bool {
    identity_in_scope(capture, &key.identity())
}
fn identity_in_scope(capture: &CapturedWorld, identity: &DomainIdentity) -> bool {
    identity
        .world_id
        .as_ref()
        .is_none_or(|id| id == &capture.world.id)
        && (identity.kind != DomainKind::World || identity.id == capture.world.id)
}
fn content_use(kind: EdgeKind) -> bool {
    !matches!(kind, EdgeKind::Ownership | EdgeKind::Membership)
}
fn scoped_edges(capture: &CapturedWorld) -> Vec<&InspectionEdge> {
    let mut edges: Vec<_> = capture
        .edges
        .iter()
        .filter(|edge| {
            in_scope(capture, &edge.from)
                && match &edge.to {
                    DomainRef::Exact(key) => in_scope(capture, key),
                    DomainRef::Identity(identity) => identity_in_scope(capture, identity),
                }
        })
        .collect();
    edges.sort();
    // Sites include occurrence slots: only identical duplicate observations collapse.
    edges.dedup();
    edges
}
fn matches_target(key: &DomainNodeKey, query: &WhereUsedQuery) -> bool {
    key.identity() == query.target
        && match query.versions {
            VersionFilter::All => true,
            VersionFilter::Exact(version) => key.revision == version,
        }
}
fn page<T>(items: Vec<T>, page: usize) -> Vec<T> {
    let offset = page.max(1).saturating_sub(1).saturating_mul(PAGE_SIZE);
    items
        .into_iter()
        .take(MAX_RESULTS)
        .skip(offset)
        .take(PAGE_SIZE)
        .collect()
}
/// Search only the supplied world's captured closure. Identity backlinks are visible
/// in All direct results as `DomainRef::Identity`; they never become exact pins or paths.
pub fn where_used(capture: &CapturedWorld, query: &WhereUsedQuery) -> WhereUsedResult {
    let mut result = WhereUsedResult {
        uses: vec![],
        paths: vec![],
        total: 0,
        truncated: false,
        complete: capture.is_complete(),
    };
    if !identity_in_scope(capture, &query.target) {
        return result;
    }
    let edges = scoped_edges(capture);
    if query.view == UseView::DirectUses {
        let uses: Vec<_> = edges
            .into_iter()
            .filter(|edge| {
                content_use(edge.kind)
                    && match &edge.to {
                        DomainRef::Exact(key) => matches_target(key, query),
                        DomainRef::Identity(identity) => {
                            query.versions == VersionFilter::All && identity == &query.target
                        }
                    }
            })
            .map(|edge| Usage { edge: edge.clone() })
            .collect();
        result.total = uses.len();
        result.truncated = uses.len() > MAX_RESULTS;
        result.uses = page(uses, query.page);
    } else {
        root_paths(capture, query, &edges, &mut result);
    }
    result.complete &= !result.truncated;
    result
}
// One multi-source reverse BFS. The boolean records a content edge in the suffix,
// so a membership shortcut cannot hide an actual consumer path. Each state has one
// successor: no per-root graph searches or combinatorial path enumeration.
fn root_paths(
    capture: &CapturedWorld,
    query: &WhereUsedQuery,
    edges: &[&InspectionEdge],
    result: &mut WhereUsedResult,
) {
    type State = (DomainNodeKey, bool);
    let mut incoming: BTreeMap<&DomainNodeKey, Vec<&InspectionEdge>> = BTreeMap::new();
    let mut targets = BTreeSet::new();
    for edge in edges {
        if let DomainRef::Exact(key) = &edge.to {
            incoming.entry(key).or_default().push(edge);
            if matches_target(key, query) {
                targets.insert(key.clone());
            }
        }
    }
    let mut depths: BTreeMap<State, usize> = BTreeMap::new();
    let mut next: BTreeMap<State, (&InspectionEdge, State)> = BTreeMap::new();
    let mut queue = VecDeque::new();
    for target in targets {
        let state = (target, false);
        depths.insert(state.clone(), 0);
        queue.push_back(state);
    }
    let mut work = 0;
    'search: while let Some(state) = queue.pop_front() {
        let depth = depths[&state];
        for edge in incoming.get(&state.0).into_iter().flatten() {
            if work == MAX_WORK {
                result.truncated = true;
                break 'search;
            }
            work += 1;
            let previous = (edge.from.clone(), state.1 || content_use(edge.kind));
            if depths.contains_key(&previous) {
                continue;
            }
            if depth == MAX_DEPTH {
                result.truncated = true;
                continue;
            }
            depths.insert(previous.clone(), depth + 1);
            next.insert(previous.clone(), (edge, state.clone()));
            queue.push_back(previous);
        }
    }
    let mut paths = vec![];
    for (root, role) in &capture.roots {
        if *role == RootRole::Dependency || !in_scope(capture, root) {
            continue;
        }
        let mut state = (root.clone(), true);
        if !depths.contains_key(&state) {
            continue;
        }
        result.total += 1;
        if paths.len() == MAX_RESULTS {
            result.truncated = true;
            continue;
        }
        let mut path = vec![];
        while let Some((edge, successor)) = next.get(&state) {
            path.push((*edge).clone());
            state = successor.clone();
        }
        paths.push(UsagePath {
            root: root.clone(),
            role: *role,
            edges: path,
        });
    }
    result.paths = page(paths, query.page);
}

fn finding(
    findings: &mut Vec<InspectionFinding>,
    category: FindingCategory,
    code: &str,
    subject: &DomainNodeKey,
    site: Option<EdgeSite>,
    message: impl Into<String>,
    evidence: Vec<DomainNodeKey>,
) {
    finding_with_evidence(
        findings,
        category,
        code,
        subject,
        site,
        message,
        canonical_evidence(evidence),
    );
}
fn canonical_evidence(mut evidence: Vec<DomainNodeKey>) -> Arc<[DomainNodeKey]> {
    evidence.sort();
    evidence.dedup();
    evidence.into()
}
fn finding_with_evidence(
    findings: &mut Vec<InspectionFinding>,
    category: FindingCategory,
    code: &str,
    subject: &DomainNodeKey,
    site: Option<EdgeSite>,
    message: impl Into<String>,
    evidence: Arc<[DomainNodeKey]>,
) {
    findings.push(InspectionFinding {
        category,
        code: code.into(),
        subject: subject.clone(),
        site,
        message: message.into(),
        evidence,
        policy: None,
        comparison: false,
    });
}
fn node(kind: DomainKind, scope: Option<String>, id: &str, revision: i64) -> DomainNodeKey {
    DomainNodeKey {
        kind,
        world_id: scope,
        id: id.into(),
        revision,
    }
}
fn current(
    capture: &CapturedWorld,
    kind: DomainKind,
) -> Option<(&DomainNodeKey, &InspectionPayload)> {
    capture.roots.iter().find_map(|(key, role)| {
        (key.kind == kind && *role != RootRole::Dependency && in_scope(capture, key))
            .then(|| capture.nodes.get_key_value(key))
            .flatten()
    })
}
fn report_kind(kind: &str) -> Option<DomainKind> {
    Some(match kind {
        "move" => DomainKind::Move,
        "move_attribute" => DomainKind::MoveAttribute,
        "ruleset" => DomainKind::Ruleset,
        "learnset" => DomainKind::Learnset,
        "preset" => DomainKind::Preset,
        "stat_roster" => DomainKind::StatRoster,
        "chart" => DomainKind::Chart,
        _ => return None,
    })
}
fn report_findings(
    capture: &CapturedWorld,
    findings: &mut Vec<InspectionFinding>,
    subject: &DomainNodeKey,
    code: &str,
    report: ValidationReport,
    comparison: bool,
) {
    // ValidationInput is structured validator output. Messages remain opaque leaves.
    let evidence = canonical_evidence(
        report
            .inputs
            .iter()
            .flat_map(|input| {
                report_kind(&input.kind).into_iter().flat_map(move |kind| {
                    [subject.world_id.clone(), None]
                        .into_iter()
                        .map(move |scope| node(kind, scope, &input.id, input.version))
                })
            })
            .filter(|key| capture.nodes.contains_key(key) && in_scope(capture, key))
            .collect(),
    );
    for (category, messages) in [
        (FindingCategory::Contradiction, report.contradictions),
        (FindingCategory::Incomplete, report.incomplete),
        (FindingCategory::NotEvaluated, report.not_evaluated),
    ] {
        for message in messages {
            finding_with_evidence(
                findings,
                category,
                code,
                subject,
                None,
                message,
                Arc::clone(&evidence),
            );
            let last = findings.last_mut().expect("just appended");
            last.policy = Some(report.validator_version);
            last.comparison = comparison;
        }
    }
}
/// Analyze current active/archived roots using trusted captured payloads. Capture gaps
/// are coverage, not game findings. Older nodes remain exact context, never new heads.
pub fn analyze_world(capture: &CapturedWorld, compare_policy: bool) -> Vec<InspectionFinding> {
    let mut findings = vec![];
    let rules = current(capture, DomainKind::Ruleset).and_then(|(key, payload)| {
        if let InspectionPayload::Ruleset(rules) = payload {
            Some((key, rules))
        } else {
            None
        }
    });
    let chart = current(capture, DomainKind::Chart).and_then(|(key, payload)| {
        if let InspectionPayload::Chart(chart) = payload {
            Some((key, chart))
        } else {
            None
        }
    });
    for (key, role) in &capture.roots {
        if *role == RootRole::Dependency || !in_scope(capture, key) {
            continue;
        }
        let Some(payload) = capture.nodes.get(key) else {
            continue;
        };
        match payload {
            InspectionPayload::Ruleset(rules) => {
                report_findings(
                    capture,
                    &mut findings,
                    key,
                    "validator.ruleset",
                    validate_ruleset_with_policy(rules, CURRENT_VALIDATOR_VERSION),
                    false,
                );
                rules_context(capture, &mut findings, key, rules);
            }
            InspectionPayload::Move(m) => {
                if let Some((_, rules)) = rules {
                    report_findings(
                        capture,
                        &mut findings,
                        key,
                        "validator.move",
                        validate_move(m, rules),
                        false,
                    );
                } else {
                    numeric_state(
                        &mut findings,
                        key,
                        m.attributes
                            .as_ref()
                            .map(|a| a.iter().map(|a| &a.value).collect()),
                    );
                }
            }
            InspectionPayload::Preset(preset) => {
                analyze_preset(capture, &mut findings, key, preset, compare_policy)
            }
            InspectionPayload::Form(form) => creature_state(
                capture,
                &mut findings,
                key,
                &form.typing,
                &form.stats,
                rules,
                chart,
            ),
            InspectionPayload::Variant(variant) => creature_state(
                capture,
                &mut findings,
                key,
                &variant.typing,
                &variant.stats,
                rules,
                chart,
            ),
            InspectionPayload::Chart(chart) => chart_state(&mut findings, key, chart),
            InspectionPayload::Learnset(learnset) => {
                if learnset.records.is_none() {
                    finding(
                        &mut findings,
                        FindingCategory::Incomplete,
                        "learnset.unmodeled",
                        key,
                        None,
                        "Acquisitions are unmodeled, distinct from a defined empty learnset.",
                        vec![],
                    );
                }
                for (index, record) in learnset.records.iter().flatten().enumerate() {
                    if record.method.is_none()
                        || record.context.is_none()
                        || record.condition.is_none()
                        || record.threshold == NumericValue::Unknown
                    {
                        let evidence = node(
                            DomainKind::Move,
                            record.move_record.world_id.clone(),
                            &record.move_record.id,
                            record.move_record.revision,
                        );
                        finding(
                            &mut findings,
                            FindingCategory::Incomplete,
                            "acquisition.descriptive_unknown",
                            key,
                            Some(EdgeSite::Acquisition {
                                slot: index + 1,
                                record_id: record.id.clone(),
                            }),
                            "Acquisition has Unknown descriptive inputs; no level or eligibility meaning is inferred.",
                            captured_evidence(capture, [evidence]),
                        );
                    }
                }
            }
            _ => {}
        }
    }
    inventory_findings(capture, &mut findings);
    dependency_findings(capture, &mut findings);
    finding(
        &mut findings,
        FindingCategory::NotEvaluated,
        "mechanics.inert",
        &capture.world_key(),
        None,
        "Effects, formulas, derived stats, multi-Type composition, acquisition eligibility and combat readiness are not evaluated.",
        vec![],
    );
    findings.sort();
    findings.dedup();
    findings
}
fn captured_evidence<const N: usize>(
    capture: &CapturedWorld,
    keys: [DomainNodeKey; N],
) -> Vec<DomainNodeKey> {
    keys.into_iter()
        .filter(|key| capture.nodes.contains_key(key) && in_scope(capture, key))
        .collect()
}
fn numeric_state(
    findings: &mut Vec<InspectionFinding>,
    key: &DomainNodeKey,
    values: Option<Vec<&NumericValue>>,
) {
    match values {
        None => finding(
            findings,
            FindingCategory::Incomplete,
            "move.attributes_unmodeled",
            key,
            None,
            "Move attributes are unmodeled, distinct from a defined empty collection.",
            vec![],
        ),
        Some(values) => {
            for (index, value) in values.into_iter().enumerate() {
                if *value == NumericValue::Unknown {
                    finding(
                        findings,
                        FindingCategory::Incomplete,
                        "move.attribute_unknown",
                        key,
                        Some(EdgeSite::Attribute { slot: index + 1 }),
                        "Attribute value is Unknown, not zero or NotApplicable.",
                        vec![],
                    );
                }
            }
        }
    }
}
fn analyze_preset(
    capture: &CapturedWorld,
    findings: &mut Vec<InspectionFinding>,
    key: &DomainNodeKey,
    preset: &MovesetRecord,
    compare: bool,
) {
    let learnset_key = node(
        DomainKind::Learnset,
        Some(preset.world_id.clone()),
        &preset.learnset.learnset_id,
        preset.learnset.version,
    );
    let rules_key = node(
        DomainKind::Ruleset,
        Some(preset.world_id.clone()),
        &preset.ruleset.ruleset_id,
        preset.ruleset.version,
    );
    let (Some(InspectionPayload::Learnset(learnset)), Some(InspectionPayload::Ruleset(rules))) = (
        capture.nodes.get(&learnset_key),
        capture.nodes.get(&rules_key),
    ) else {
        // Missing trusted payloads are capture gaps, not Unknown game inputs.
        return;
    };
    if learnset.world_id.as_ref() != Some(&preset.world_id)
        || learnset.variant_id.as_ref() != Some(&preset.variant_id)
        || rules.world_id != preset.world_id
    {
        return;
    }
    report_findings(
        capture,
        findings,
        key,
        "validator.preset.saved",
        validate_preset(preset, learnset, rules),
        false,
    );
    if compare {
        let mut comparison = preset.clone();
        comparison.validator_version = CURRENT_VALIDATOR_VERSION;
        report_findings(
            capture,
            findings,
            key,
            "validator.preset.comparison",
            validate_preset(&comparison, learnset, rules),
            true,
        );
    }
    // Location-aware checks use typed selections, never validator wording.
    let mut selected = BTreeSet::new();
    for (index, id) in preset.entries.iter().enumerate() {
        let site = Some(EdgeSite::PresetSelection {
            slot: index + 1,
            record_id: id.clone(),
            learnset: learnset_key.clone(),
        });
        if let Some(record) = learnset
            .records
            .iter()
            .flatten()
            .find(|record| &record.id == id)
            && !selected.insert(&record.move_record.id)
            && rules.repeat_policy == crate::rulesets::RepeatPolicy::Forbid
            && matches!(preset.validator_version, 1 | 2)
        {
            finding(
                findings,
                FindingCategory::Contradiction,
                "preset.selection_repeat",
                key,
                site,
                "This selection repeats a canonical Move identity under the exact pinned forbid-repeat ruleset.",
                captured_evidence(
                    capture,
                    [
                        learnset_key.clone(),
                        rules_key.clone(),
                        node(
                            DomainKind::Move,
                            record.move_record.world_id.clone(),
                            &record.move_record.id,
                            record.move_record.revision,
                        ),
                    ],
                ),
            );
            findings.last_mut().expect("just appended").policy = Some(preset.validator_version);
        }
    }
}
fn creature_state(
    capture: &CapturedWorld,
    findings: &mut Vec<InspectionFinding>,
    key: &DomainNodeKey,
    typing: &TypingState,
    stats: &BaseStats,
    rules: Option<(&DomainNodeKey, &RulesetRecord)>,
    chart: Option<(&DomainNodeKey, &ChartRecord)>,
) {
    if typing.assignments.is_none() {
        finding(
            findings,
            FindingCategory::Incomplete,
            "typing.unmodeled",
            key,
            None,
            "Typing is unmodeled, distinct from a defined empty typing.",
            vec![],
        );
    }
    if stats.assignments.is_none() {
        finding(
            findings,
            FindingCategory::Incomplete,
            "stats.unmodeled",
            key,
            None,
            "Base stats are unmodeled, distinct from defined empty assignments.",
            vec![],
        );
    }
    for (index, assignment) in stats.assignments.iter().flatten().enumerate() {
        if assignment.value.is_none() {
            finding(
                findings,
                FindingCategory::Incomplete,
                "stats.missing",
                key,
                Some(EdgeSite::StatValue { slot: index + 1 }),
                "Stat value is Missing, not zero.",
                captured_evidence(
                    capture,
                    [node(
                        DomainKind::Stat,
                        assignment.definition.world_id.clone(),
                        &assignment.definition.id,
                        assignment.definition.revision,
                    )],
                ),
            );
        }
    }
    let pinned = stats.roster.as_ref().map(|pin| {
        node(
            DomainKind::StatRoster,
            key.world_id.clone(),
            &pin.roster_id,
            pin.version,
        )
    });
    if key.world_id.is_some() {
        if let Some((current_key, _)) = current(capture, DomainKind::StatRoster)
            && pinned.as_ref() != Some(current_key)
        {
            finding(
                findings,
                FindingCategory::ContextDifference,
                "stats.current_roster_context",
                key,
                Some(EdgeSite::StatRoster),
                "Saved stat context differs from the current world roster; values were not retargeted or reconciled.",
                pinned
                    .iter()
                    .chain(std::iter::once(current_key))
                    .filter(|k| capture.nodes.contains_key(*k))
                    .cloned()
                    .collect(),
            );
        }
        if let Some((rules_key, rules)) = rules
            && let Some(expected) = &rules.stat_roster
        {
            let expected = node(
                DomainKind::StatRoster,
                Some(rules.world_id.clone()),
                &expected.roster_id,
                expected.version,
            );
            if pinned.as_ref() != Some(&expected) {
                let mut evidence = captured_evidence(capture, [rules_key.clone(), expected]);
                evidence.extend(
                    pinned
                        .iter()
                        .filter(|k| capture.nodes.contains_key(*k))
                        .cloned(),
                );
                finding(
                    findings,
                    FindingCategory::ContextDifference,
                    "stats.ruleset_roster_context",
                    key,
                    Some(EdgeSite::StatRoster),
                    "Saved stat context differs from the ruleset's optional exact roster context; no automatic reconciliation was performed.",
                    evidence,
                );
            }
        }
    }
    if let Some((chart_key, chart)) = chart {
        typing_chart(
            capture,
            findings,
            key,
            typing,
            chart_key,
            chart,
            "typing.current_chart_context",
        );
    }
    if key.world_id.is_some()
        && let Some((_, rules)) = rules
        && let Some(pin) = &rules.chart
    {
        let chart_key = node(
            DomainKind::Chart,
            Some(rules.world_id.clone()),
            &pin.chart_id,
            pin.version,
        );
        if let Some(InspectionPayload::Chart(chart)) = capture.nodes.get(&chart_key) {
            typing_chart(
                capture,
                findings,
                key,
                typing,
                &chart_key,
                chart,
                "typing.ruleset_chart_context",
            );
        }
    }
}
fn typing_chart(
    capture: &CapturedWorld,
    findings: &mut Vec<InspectionFinding>,
    key: &DomainNodeKey,
    typing: &TypingState,
    chart_key: &DomainNodeKey,
    chart: &ChartRecord,
    code: &str,
) {
    for (index, assignment) in typing.assignments.iter().flatten().enumerate() {
        // Canonical Type identity includes scope; metadata revision is intentionally absent.
        if key.world_id != chart.world_id
            || !chart
                .roster
                .iter()
                .any(|entry| entry.pin.type_id == assignment.pin.type_id)
        {
            finding(
                findings,
                FindingCategory::ContextDifference,
                code,
                key,
                Some(EdgeSite::Typing { slot: index + 1 }),
                "Assigned Type is outside this chart's scoped roster (not an Unknown pair); no multi-Type effectiveness is inferred.",
                captured_evidence(
                    capture,
                    [
                        chart_key.clone(),
                        node(
                            DomainKind::Type,
                            key.world_id.clone(),
                            &assignment.pin.type_id,
                            assignment.pin.version,
                        ),
                    ],
                ),
            );
        }
    }
}
fn chart_state(findings: &mut Vec<InspectionFinding>, key: &DomainNodeKey, chart: &ChartRecord) {
    let unknown = chart
        .roster
        .len()
        .saturating_mul(chart.roster.len())
        .saturating_sub(chart.cells.len());
    if unknown > 0 {
        finding(
            findings,
            FindingCategory::Incomplete,
            "chart.unknown_pairs",
            key,
            None,
            format!(
                "{unknown} directed roster pairs are Unknown; explicit neutral and zero factors remain Known. Outside-roster Types are separate."
            ),
            vec![],
        );
    }
}
fn rules_context(
    capture: &CapturedWorld,
    findings: &mut Vec<InspectionFinding>,
    key: &DomainNodeKey,
    rules: &RulesetRecord,
) {
    if let Some(pin) = &rules.stat_roster
        && let Some((current_key, _)) = current(capture, DomainKind::StatRoster)
    {
        let pinned = node(
            DomainKind::StatRoster,
            Some(rules.world_id.clone()),
            &pin.roster_id,
            pin.version,
        );
        if &pinned != current_key {
            finding(
                findings,
                FindingCategory::ContextDifference,
                "ruleset.current_roster_context",
                key,
                Some(EdgeSite::StatRoster),
                "The optional exact ruleset roster differs from the current world roster; the saved pin remains unchanged.",
                captured_evidence(capture, [pinned, current_key.clone()]),
            );
        }
    }
}
fn inventory_findings(capture: &CapturedWorld, findings: &mut Vec<InspectionFinding>) {
    for absence in &capture.absences {
        let subject = absence
            .owner_id
            .as_ref()
            .and_then(|id| {
                capture.roots.keys().find(|key| {
                    key.id == *id && in_scope(capture, key) && capture.nodes.contains_key(*key)
                })
            })
            .cloned()
            .unwrap_or_else(|| capture.world_key());
        finding(
            findings,
            FindingCategory::NotEvaluated,
            &format!(
                "inventory.unconfigured.{}",
                match absence.kind {
                    DomainKind::Chart => "chart",
                    DomainKind::StatRoster => "stat_roster",
                    DomainKind::Ruleset => "ruleset",
                    DomainKind::Learnset => "learnset",
                    DomainKind::Preset => "preset",
                    _ => "content",
                }
            ),
            &subject,
            None,
            format!(
                "{:?} is Unconfigured in this owner inventory; no combat requirement is inferred.",
                absence.kind
            ),
            vec![],
        );
    }
}
fn dependency_findings(capture: &CapturedWorld, findings: &mut Vec<InspectionFinding>) {
    let edges = scoped_edges(capture);
    let mut used = BTreeSet::new();
    for edge in edges {
        if !content_use(edge.kind) {
            continue;
        }
        match &edge.to {
            DomainRef::Identity(identity) => {
                used.insert(identity.clone());
            }
            DomainRef::Exact(target) => {
                used.insert(target.identity());
                if !capture.nodes.contains_key(&edge.from) || !capture.nodes.contains_key(target) {
                    continue;
                }
                if let Some(head) = capture.heads.get(&target.identity()) {
                    if head.revision > target.revision {
                        finding(
                            findings,
                            FindingCategory::ContextDifference,
                            "dependency.older_pin",
                            &edge.from,
                            Some(edge.site.clone()),
                            format!(
                                "Exact revision {} is retained; observed head is revision {}. This is informational, not automatic invalidity or adoption.",
                                target.revision, head.revision
                            ),
                            vec![target.clone()],
                        );
                    }
                    if head.archived_at.is_some() {
                        finding(
                            findings,
                            FindingCategory::ContextDifference,
                            "dependency.archived",
                            &edge.from,
                            Some(edge.site.clone()),
                            "This exact dependency has an archived current head; retention is not automatically invalid.",
                            vec![target.clone()],
                        );
                    }
                }
            }
        }
    }
    for (key, role) in &capture.roots {
        if *role == RootRole::Dependency
            || key.kind == DomainKind::World
            || !in_scope(capture, key)
            || !capture.nodes.contains_key(key)
            || used.contains(&key.identity())
        {
            continue;
        }
        let (code, message) = if capture.is_complete() {
            (
                "usage.unused_in_capture",
                "Unused in this capture: no incoming content uses in this captured world's closure. Membership and ownership do not count; this is not a global unused or safe-to-delete claim.",
            )
        } else {
            (
                "usage.no_observed_uses",
                "No incoming content uses observed in the inspected portion. Coverage is partial; this is not an unused or safe-to-delete claim.",
            )
        };
        finding(
            findings,
            FindingCategory::NotEvaluated,
            code,
            key,
            None,
            message,
            vec![],
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        authoring::ContentFields,
        charts::{ChartCell, Factor},
        inspection::{
            CaptureGap, CaptureGapKind, HeadObservation, InventoryAbsence, PublicVariantContent,
            PublicWorldContent,
        },
        learning::{LearningRecord, LearnsetRecord},
        moves::{MoveAttributeDefinition, MoveRecord},
        rulesets::{AttributeExpectation, RepeatPolicy},
        stats::{NumericConstraints, StatDefinition, StatRoster, StatValue},
        types::{TypeAssignment, TypePin},
    };

    fn id(n: usize) -> String {
        format!("{n:032x}")
    }
    fn fields() -> ContentFields {
        ContentFields {
            slug: "sample".into(),
            display_name: "Sample".into(),
            description: "Inert test content".into(),
        }
    }
    fn key(kind: DomainKind, n: usize, revision: i64) -> DomainNodeKey {
        node(kind, Some(id(1)), &id(n), revision)
    }
    fn capture() -> CapturedWorld {
        let world = PublicWorldContent {
            id: id(1),
            fields: fields(),
            revision: 1,
            archived_at: None,
        };
        let world_key = node(DomainKind::World, None, &world.id, 1);
        CapturedWorld {
            contract_version: 1,
            world: world.clone(),
            roots: BTreeMap::from([(world_key.clone(), RootRole::Active)]),
            nodes: BTreeMap::from([(world_key, InspectionPayload::World(world))]),
            edges: vec![],
            heads: BTreeMap::new(),
            absences: vec![],
            gaps: vec![],
            card_data: Default::default(),
        }
    }
    fn edge(
        from: &DomainNodeKey,
        to: &DomainNodeKey,
        kind: EdgeKind,
        site: EdgeSite,
    ) -> InspectionEdge {
        InspectionEdge {
            from: from.clone(),
            to: DomainRef::Exact(to.clone()),
            kind,
            site,
        }
    }
    fn query(target: &DomainNodeKey, view: UseView) -> WhereUsedQuery {
        WhereUsedQuery {
            target: target.identity(),
            versions: VersionFilter::All,
            view,
            page: 1,
        }
    }
    fn root(
        capture: &mut CapturedWorld,
        key: &DomainNodeKey,
        payload: InspectionPayload,
        role: RootRole,
    ) {
        capture.roots.insert(key.clone(), role);
        capture.nodes.insert(key.clone(), payload);
    }
    fn variant(n: usize, stats: BaseStats, typing: TypingState) -> InspectionPayload {
        InspectionPayload::Variant(PublicVariantContent {
            id: id(n),
            world_id: id(1),
            source: None,
            fields: fields(),
            revision: 1,
            archived_at: None,
            stats,
            typing,
        })
    }
    fn move_record(n: usize) -> MoveRecord {
        MoveRecord {
            id: id(n),
            world_id: Some(id(1)),
            source: None,
            fields: fields(),
            revision: 1,
            archived_at: None,
            attributes: Some(vec![]),
            mappings: vec![],
        }
    }
    fn rules() -> RulesetRecord {
        RulesetRecord {
            id: id(20),
            world_id: id(1),
            revision: 1,
            fields: fields(),
            expectations: Some(vec![]),
            stat_roster: None,
            chart: None,
            max_moves: None,
            repeat_policy: RepeatPolicy::Unspecified,
            notes: String::new(),
        }
    }
    fn learnset() -> LearnsetRecord {
        LearnsetRecord {
            id: id(21),
            world_id: Some(id(1)),
            form_id: None,
            variant_id: Some(id(22)),
            source: None,
            revision: 1,
            records: Some(vec![]),
            mappings: vec![],
        }
    }
    fn preset(rules: &RulesetRecord, learnset: &LearnsetRecord) -> MovesetRecord {
        MovesetRecord {
            id: id(23),
            world_id: id(1),
            variant_id: id(22),
            fields: fields(),
            revision: 1,
            archived_at: None,
            learnset: learnset.pin(),
            ruleset: rules.pin(),
            entries: vec![],
            validator_version: 1,
        }
    }
    fn preset_capture(
        rules: RulesetRecord,
        learnset: LearnsetRecord,
        preset: MovesetRecord,
    ) -> CapturedWorld {
        let mut c = capture();
        root(
            &mut c,
            &key(DomainKind::Preset, 23, 1),
            InspectionPayload::Preset(preset.clone()),
            RootRole::Active,
        );
        let rkey = key(DomainKind::Ruleset, 20, rules.revision);
        let lkey = key(DomainKind::Learnset, 21, learnset.revision);
        c.nodes
            .insert(rkey.clone(), InspectionPayload::Ruleset(rules));
        c.nodes
            .insert(lkey.clone(), InspectionPayload::Learnset(learnset));
        c.edges = vec![
            edge(
                &key(DomainKind::Preset, 23, 1),
                &rkey,
                EdgeKind::ExactPin,
                EdgeSite::PresetRuleset,
            ),
            edge(
                &key(DomainKind::Preset, 23, 1),
                &lkey,
                EdgeKind::ExactPin,
                EdgeSite::PresetLearnset,
            ),
        ];
        c
    }

    #[test]
    fn direct_sites_preserve_repeated_acquisitions_selections_and_exact_old_versions() {
        let mut c = capture();
        let m1 = key(DomainKind::Move, 2, 1);
        let m2 = key(DomainKind::Move, 2, 2);
        let l = key(DomainKind::Learnset, 3, 1);
        let p = key(DomainKind::Preset, 4, 1);
        let expected = vec![
            edge(
                &l,
                &m1,
                EdgeKind::ExactPin,
                EdgeSite::Acquisition {
                    slot: 1,
                    record_id: id(5),
                },
            ),
            edge(
                &l,
                &m1,
                EdgeKind::ExactPin,
                EdgeSite::Acquisition {
                    slot: 2,
                    record_id: id(6),
                },
            ),
            edge(
                &p,
                &m1,
                EdgeKind::Selection,
                EdgeSite::PresetSelection {
                    slot: 1,
                    record_id: id(5),
                    learnset: l.clone(),
                },
            ),
            edge(
                &p,
                &m1,
                EdgeKind::Selection,
                EdgeSite::PresetSelection {
                    slot: 2,
                    record_id: id(5),
                    learnset: l.clone(),
                },
            ),
        ];
        c.edges = expected.clone();
        c.edges.push(edge(
            &l,
            &m2,
            EdgeKind::ExactPin,
            EdgeSite::Acquisition {
                slot: 3,
                record_id: id(7),
            },
        ));
        c.edges.push(edge(
            &c.world_key(),
            &m2,
            EdgeKind::Membership,
            EdgeSite::WorldMember {
                kind: DomainKind::Move,
            },
        ));
        let mut q = query(&m1, UseView::DirectUses);
        q.versions = VersionFilter::Exact(1);
        let result = where_used(&c, &q);
        assert_eq!(
            result.uses,
            expected
                .into_iter()
                .map(|edge| Usage { edge })
                .collect::<Vec<_>>()
        );
        assert_eq!(result.total, 4);
        assert!(result.complete);
        q.versions = VersionFilter::Exact(2);
        assert_eq!(where_used(&c, &q).total, 1);
        q.versions = VersionFilter::Exact(3);
        assert_eq!(where_used(&c, &q).total, 0);
        q.versions = VersionFilter::All;
        assert_eq!(where_used(&c, &q).total, 5);
        c.edges.reverse();
        assert_eq!(where_used(&c, &q).total, 5);
    }

    #[test]
    fn identity_backlinks_are_all_only_and_never_traversed_and_other_worlds_excluded() {
        let mut c = capture();
        let target = key(DomainKind::Move, 2, 1);
        let consumer = key(DomainKind::Learnset, 3, 1);
        c.roots.insert(consumer.clone(), RootRole::Active);
        let backlink = InspectionEdge {
            from: consumer.clone(),
            to: DomainRef::Identity(target.identity()),
            kind: EdgeKind::Provenance,
            site: EdgeSite::Source,
        };
        let mut foreign = consumer.clone();
        foreign.world_id = Some(id(99));
        c.edges = vec![
            backlink.clone(),
            edge(&foreign, &target, EdgeKind::ExactPin, EdgeSite::Source),
        ];
        let mut q = query(&target, UseView::DirectUses);
        assert_eq!(where_used(&c, &q).uses, vec![Usage { edge: backlink }]);
        q.versions = VersionFilter::Exact(1);
        assert_eq!(where_used(&c, &q).total, 0);
        q.versions = VersionFilter::All;
        q.view = UseView::RootPaths;
        assert_eq!(where_used(&c, &q).total, 0);
        q.target.world_id = Some(id(99));
        assert!(where_used(&c, &q).paths.is_empty());
    }

    #[test]
    fn reverse_bfs_chooses_one_shortest_stable_path_through_cycles_and_structural_roots() {
        let mut c = capture();
        let target = key(DomainKind::Move, 2, 1);
        let a = key(DomainKind::Learnset, 3, 1);
        let b = key(DomainKind::Preset, 4, 1);
        c.roots.insert(a.clone(), RootRole::Archived);
        c.roots.insert(b.clone(), RootRole::Active);
        let use_a = edge(
            &a,
            &target,
            EdgeKind::ExactPin,
            EdgeSite::Acquisition {
                slot: 1,
                record_id: id(5),
            },
        );
        let world_a = edge(
            &c.world_key(),
            &a,
            EdgeKind::Membership,
            EdgeSite::WorldMember {
                kind: DomainKind::Learnset,
            },
        );
        let b_a = edge(&b, &a, EdgeKind::ExactPin, EdgeSite::PresetLearnset);
        c.edges = vec![
            use_a.clone(),
            world_a.clone(),
            b_a.clone(),
            edge(&a, &b, EdgeKind::ExactPin, EdgeSite::Source),
            edge(
                &c.world_key(),
                &target,
                EdgeKind::Membership,
                EdgeSite::WorldMember {
                    kind: DomainKind::Move,
                },
            ),
        ];
        let q = query(&target, UseView::RootPaths);
        let expected = WhereUsedResult {
            uses: vec![],
            paths: vec![
                UsagePath {
                    root: c.world_key(),
                    role: RootRole::Active,
                    edges: vec![world_a, use_a.clone()],
                },
                UsagePath {
                    root: a,
                    role: RootRole::Archived,
                    edges: vec![use_a.clone()],
                },
                UsagePath {
                    root: b,
                    role: RootRole::Active,
                    edges: vec![b_a, use_a],
                },
            ],
            total: 3,
            truncated: false,
            complete: true,
        };
        assert_eq!(where_used(&c, &q), expected);
        c.edges.reverse();
        assert_eq!(where_used(&c, &q), expected);
    }

    #[test]
    fn ownership_and_membership_alone_are_not_uses_or_usage_paths() {
        let mut c = capture();
        let target = key(DomainKind::Variant, 2, 1);
        root(
            &mut c,
            &target,
            variant(2, BaseStats::unmodeled(), TypingState::unmodeled()),
            RootRole::Archived,
        );
        c.edges = vec![
            edge(
                &c.world_key(),
                &target,
                EdgeKind::Membership,
                EdgeSite::WorldMember {
                    kind: DomainKind::Variant,
                },
            ),
            edge(
                &c.world_key(),
                &target,
                EdgeKind::Ownership,
                EdgeSite::Owner,
            ),
        ];
        for view in [UseView::DirectUses, UseView::RootPaths] {
            assert_eq!(where_used(&c, &query(&target, view)).total, 0);
        }
        let f = analyze_world(&c, false);
        assert!(
            f.iter()
                .any(|f| f.subject == target && f.code == "usage.unused_in_capture")
        );
        c.gaps.push(CaptureGap {
            kind: CaptureGapKind::Inventory,
            subject: None,
            message: "Unobserved remainder".into(),
        });
        let f = analyze_world(&c, false);
        assert!(!f.iter().any(|f| f.code == "usage.unused_in_capture"));
        assert!(
            f.iter()
                .any(|f| f.subject == target && f.code == "usage.no_observed_uses")
        );
        assert!(!where_used(&c, &query(&target, UseView::DirectUses)).complete);
    }

    #[test]
    fn pagination_is_25_capped_at_256_and_overflow_safe() {
        let mut c = capture();
        let target = key(DomainKind::Move, 2, 1);
        let from = key(DomainKind::Learnset, 3, 1);
        for slot in 1..=300 {
            c.edges.push(edge(
                &from,
                &target,
                EdgeKind::ExactPin,
                EdgeSite::Acquisition {
                    slot,
                    record_id: id(slot),
                },
            ));
        }
        let mut q = query(&target, UseView::DirectUses);
        let first = where_used(&c, &q);
        assert_eq!(first.total, 300);
        assert_eq!(first.uses.len(), 25);
        assert!(first.truncated);
        assert!(!first.complete);
        q.page = 11;
        assert_eq!(where_used(&c, &q).uses.len(), 6);
        q.page = usize::MAX;
        assert!(where_used(&c, &q).uses.is_empty());
        q.page = 0;
        assert_eq!(where_used(&c, &q), first);
    }

    #[test]
    fn depth_and_result_limits_are_explicit_for_root_paths() {
        let mut c = capture();
        let target = key(DomainKind::Move, 2, 1);
        let mut previous = target.clone();
        for n in 3..=36 {
            let next = key(DomainKind::Move, n, 1);
            c.edges.push(edge(
                &next,
                &previous,
                EdgeKind::Provenance,
                EdgeSite::Source,
            ));
            previous = next;
        }
        c.roots.insert(previous, RootRole::Active);
        let result = where_used(&c, &query(&target, UseView::RootPaths));
        assert!(result.paths.is_empty());
        assert!(result.truncated);
        assert!(!result.complete);
        c.edges.clear();
        c.roots.clear();
        for n in 3..303 {
            let consumer = key(DomainKind::Learnset, n, 1);
            c.roots.insert(consumer.clone(), RootRole::Active);
            c.edges.push(edge(
                &consumer,
                &target,
                EdgeKind::ExactPin,
                EdgeSite::Source,
            ));
        }
        let result = where_used(&c, &query(&target, UseView::RootPaths));
        assert_eq!(result.total, 300);
        assert_eq!(result.paths.len(), 25);
        assert!(result.truncated);
        assert!(!result.complete);
    }

    #[test]
    fn traversal_work_budget_is_explicit_even_with_only_two_visited_nodes() {
        let mut c = capture();
        let target = key(DomainKind::Move, 2, 1);
        let consumer = key(DomainKind::Learnset, 3, 1);
        c.roots.insert(consumer.clone(), RootRole::Active);
        for slot in 1..=MAX_WORK + 1 {
            c.edges.push(edge(
                &consumer,
                &target,
                EdgeKind::ExactPin,
                EdgeSite::Acquisition {
                    slot,
                    record_id: id(4),
                },
            ));
        }
        let result = where_used(&c, &query(&target, UseView::RootPaths));
        assert_eq!(result.total, 1);
        assert!(result.truncated);
        assert!(!result.complete);
    }

    #[test]
    fn stats_missing_zero_unmodeled_and_empty_are_distinct_and_archived_roots_are_analyzed() {
        let mut c = capture();
        let stat = StatDefinition {
            id: id(10),
            world_id: Some(id(1)),
            source: None,
            fields: fields(),
            revision: 1,
            archived_at: None,
            constraints: NumericConstraints::default(),
        };
        c.nodes.insert(
            key(DomainKind::Stat, 10, 1),
            InspectionPayload::Stat(stat.clone()),
        );
        for (n, stats, role) in [
            (2, BaseStats::unmodeled(), RootRole::Active),
            (
                3,
                BaseStats {
                    roster: None,
                    assignments: Some(vec![]),
                },
                RootRole::Active,
            ),
            (
                4,
                BaseStats {
                    roster: None,
                    assignments: Some(vec![StatValue {
                        definition: stat.clone(),
                        value: None,
                    }]),
                },
                RootRole::Archived,
            ),
            (
                5,
                BaseStats {
                    roster: None,
                    assignments: Some(vec![StatValue {
                        definition: stat,
                        value: Some("0".parse().unwrap()),
                    }]),
                },
                RootRole::Active,
            ),
        ] {
            root(
                &mut c,
                &key(DomainKind::Variant, n, 1),
                variant(
                    n,
                    stats,
                    TypingState {
                        assignments: Some(vec![]),
                    },
                ),
                role,
            );
        }
        let findings = analyze_world(&c, false);
        let stats: Vec<_> = findings
            .iter()
            .filter(|f| f.code.starts_with("stats."))
            .collect();
        assert_eq!(stats.len(), 2);
        assert!(stats.iter().any(|f| f.subject == key(DomainKind::Variant, 2, 1) && f.code == "stats.unmodeled"));
        assert!(
            stats
                .iter()
                .any(|f| f.subject == key(DomainKind::Variant, 4, 1)
                    && f.code == "stats.missing"
                    && f.site == Some(EdgeSite::StatValue { slot: 1 })
                    && f.evidence.as_ref() == [key(DomainKind::Stat, 10, 1)])
        );
    }

    #[test]
    fn chart_uses_scoped_canonical_type_not_metadata_version_and_unknown_is_not_neutral_or_zero() {
        let mut c = capture();
        let a = TypeAssignment {
            pin: TypePin {
                type_id: id(10),
                version: 1,
            },
            fields: fields(),
        };
        let b = TypeAssignment {
            pin: TypePin {
                type_id: id(11),
                version: 1,
            },
            fields: fields(),
        };
        let chart = ChartRecord {
            id: id(12),
            world_id: Some(id(1)),
            fields: fields(),
            revision: 1,
            source: None,
            roster: vec![a.clone(), b],
            cells: vec![
                ChartCell {
                    attacker_type_id: id(10),
                    defender_type_id: id(10),
                    factor: Factor::new(1, 1).unwrap(),
                },
                ChartCell {
                    attacker_type_id: id(10),
                    defender_type_id: id(11),
                    factor: Factor::new(0, 1).unwrap(),
                },
            ],
            mapping: vec![],
            archived_at: None,
        };
        root(
            &mut c,
            &key(DomainKind::Chart, 12, 1),
            InspectionPayload::Chart(chart),
            RootRole::Active,
        );
        let mut new_a = a;
        new_a.pin.version = 2;
        root(
            &mut c,
            &key(DomainKind::Variant, 2, 1),
            variant(
                2,
                BaseStats::unmodeled(),
                TypingState {
                    assignments: Some(vec![new_a]),
                },
            ),
            RootRole::Active,
        );
        root(
            &mut c,
            &key(DomainKind::Variant, 3, 1),
            variant(
                3,
                BaseStats::unmodeled(),
                TypingState {
                    assignments: Some(vec![TypeAssignment {
                        pin: TypePin {
                            type_id: id(13),
                            version: 1,
                        },
                        fields: fields(),
                    }]),
                },
            ),
            RootRole::Active,
        );
        let findings = analyze_world(&c, false);
        assert!(
            !findings
                .iter()
                .any(|f| f.subject == key(DomainKind::Variant, 2, 1)
                    && f.code.starts_with("typing."))
        );
        assert!(
            findings
                .iter()
                .any(|f| f.code == "chart.unknown_pairs" && f.message.starts_with("2 directed"))
        );
        assert!(
            findings
                .iter()
                .any(|f| f.subject == key(DomainKind::Variant, 3, 1)
                    && f.code == "typing.current_chart_context"
                    && f.category == FindingCategory::ContextDifference)
        );
        // Identical raw IDs in another scope are not compatible.
        let chart_key = key(DomainKind::Chart, 12, 1);
        if let InspectionPayload::Chart(chart) = c.nodes.get_mut(&chart_key).unwrap() {
            chart.world_id = None;
        }
        assert!(
            analyze_world(&c, false)
                .iter()
                .any(|f| f.subject == key(DomainKind::Variant, 2, 1)
                    && f.code == "typing.current_chart_context")
        );
    }

    #[test]
    fn saved_policy_one_messages_are_preserved_and_comparison_is_explicit_and_uses_exact_old_rules()
    {
        let old_rules = rules();
        let set = learnset();
        let p = preset(&old_rules, &set);
        let expected = validate_preset(&p, &set, &old_rules);
        let mut c = preset_capture(old_rules.clone(), set, p.clone());
        let mut current_rules = old_rules;
        current_rules.revision = 2;
        current_rules.max_moves = Some(1);
        current_rules.repeat_policy = RepeatPolicy::Forbid;
        root(
            &mut c,
            &key(DomainKind::Ruleset, 20, 2),
            InspectionPayload::Ruleset(current_rules),
            RootRole::Active,
        );
        let findings = analyze_world(&c, true);
        let saved: Vec<_> = findings
            .iter()
            .filter(|f| f.code == "validator.preset.saved")
            .collect();
        for (category, messages) in [
            (FindingCategory::Contradiction, expected.contradictions),
            (FindingCategory::Incomplete, expected.incomplete),
            (FindingCategory::NotEvaluated, expected.not_evaluated),
        ] {
            let mut actual: Vec<_> = saved
                .iter()
                .filter(|f| f.category == category)
                .map(|f| f.message.clone())
                .collect();
            actual.sort();
            let mut expected = messages;
            expected.sort();
            assert_eq!(actual, expected);
        }
        assert!(saved.iter().all(|f| f.policy == Some(1)
            && !f.comparison
            && f.evidence.contains(&key(DomainKind::Ruleset, 20, 1))
            && !f.evidence.contains(&key(DomainKind::Ruleset, 20, 2))));
        let comparison: Vec<_> = findings.iter().filter(|f| f.comparison).collect();
        assert!(!comparison.is_empty());
        assert!(comparison.iter().all(|f| f.policy == Some(2)
            && f.code == "validator.preset.comparison"
            && f.evidence.contains(&key(DomainKind::Ruleset, 20, 1))));
        assert!(
            !comparison
                .iter()
                .any(|f| f.message == "The ruleset has no moveset maximum.")
        );
        assert!(
            comparison
                .iter()
                .any(|f| f.category == FindingCategory::NotEvaluated
                    && f.message == "The ruleset repeat policy is unspecified.")
        );
        assert_eq!(
            c.nodes[&key(DomainKind::Preset, 23, 1)],
            InspectionPayload::Preset(p)
        );
        let saved_without: Vec<_> = analyze_world(&c, false)
            .into_iter()
            .filter(|f| f.code == "validator.preset.saved")
            .collect();
        assert_eq!(
            saved_without,
            saved.into_iter().cloned().collect::<Vec<_>>()
        );
    }

    #[test]
    fn policy_two_configuration_and_current_move_validators_run_without_presets_or_a_four_move_default()
     {
        let mut c = capture();
        let mut r = rules();
        let attribute = MoveAttributeDefinition {
            id: id(30),
            world_id: Some(id(1)),
            source: None,
            fields: fields(),
            unit: String::new(),
            revision: 1,
            archived_at: None,
            constraints: NumericConstraints {
                minimum: Some("0".parse().unwrap()),
                maximum: Some("1".parse().unwrap()),
                whole_only: false,
            },
        };
        r.expectations = Some(vec![AttributeExpectation {
            definition: attribute,
            required: true,
            allow_not_applicable: false,
            constraints: NumericConstraints {
                minimum: Some("2".parse().unwrap()),
                ..NumericConstraints::default()
            },
        }]);
        root(
            &mut c,
            &key(DomainKind::Ruleset, 20, 1),
            InspectionPayload::Ruleset(r),
            RootRole::Active,
        );
        root(
            &mut c,
            &key(DomainKind::Move, 31, 1),
            InspectionPayload::Move(move_record(31)),
            RootRole::Archived,
        );
        c.absences.push(InventoryAbsence {
            kind: DomainKind::Preset,
            owner_id: Some(id(22)),
        });
        let findings = analyze_world(&c, false);
        assert!(findings.iter().any(|f| f.code == "validator.ruleset"
            && f.category == FindingCategory::Contradiction
            && f.policy == Some(2)));
        assert!(findings.iter().any(|f| f.code == "validator.move"
            && f.category == FindingCategory::Contradiction
            && f.policy == Some(2)));
        assert!(
            findings
                .iter()
                .any(|f| f.code == "inventory.unconfigured.preset")
        );
        assert!(!findings.iter().any(|f| f.message.contains("maximum")));
    }

    #[test]
    fn unsupported_policy_and_missing_trusted_context_do_not_run_saved_current_checks() {
        let r = rules();
        let l = learnset();
        let mut p = preset(&r, &l);
        p.validator_version = 99;
        let mut c = preset_capture(r, l, p);
        let findings = analyze_world(&c, false);
        let saved: Vec<_> = findings
            .iter()
            .filter(|f| f.code == "validator.preset.saved")
            .collect();
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0].category, FindingCategory::NotEvaluated);
        assert_eq!(saved[0].policy, Some(99));
        c.nodes.remove(&key(DomainKind::Learnset, 21, 1));
        c.gaps.push(CaptureGap {
            kind: CaptureGapKind::Integrity,
            subject: Some(key(DomainKind::Learnset, 21, 1)),
            message: "Not trusted".into(),
        });
        assert!(
            !analyze_world(&c, true)
                .iter()
                .any(|f| f.subject.kind == DomainKind::Preset && f.code.starts_with("validator."))
        );
    }

    #[test]
    fn older_archived_pins_keep_occurrence_sites_and_are_informational() {
        let mut c = capture();
        let target = key(DomainKind::Move, 2, 1);
        c.nodes
            .insert(target.clone(), InspectionPayload::Move(move_record(2)));
        let mut set = learnset();
        set.records = Some(
            (1..=2)
                .map(|n| LearningRecord {
                    id: id(n + 40),
                    move_record: move_record(2),
                    method: None,
                    context: None,
                    condition: None,
                    threshold: NumericValue::Unknown,
                })
                .collect(),
        );
        let lkey = key(DomainKind::Learnset, 21, 1);
        root(
            &mut c,
            &lkey,
            InspectionPayload::Learnset(set),
            RootRole::Active,
        );
        c.heads.insert(
            target.identity(),
            HeadObservation {
                revision: 2,
                archived_at: Some(1),
            },
        );
        for slot in 1..=2 {
            c.edges.push(edge(
                &lkey,
                &target,
                EdgeKind::ExactPin,
                EdgeSite::Acquisition {
                    slot,
                    record_id: id(slot + 40),
                },
            ));
        }
        let findings = analyze_world(&c, false);
        let pins: Vec<_> = findings
            .iter()
            .filter(|f| f.code == "dependency.older_pin")
            .collect();
        assert_eq!(pins.len(), 2);
        assert_ne!(pins[0].site, pins[1].site);
        assert!(
            findings
                .iter()
                .filter(|f| f.code.starts_with("dependency."))
                .all(|f| f.category == FindingCategory::ContextDifference
                    && f.evidence.as_ref() == [target.clone()])
        );
        assert_eq!(findings, analyze_world(&c, false));
        c.edges.reverse();
        assert_eq!(findings, analyze_world(&c, false));
    }

    #[test]
    fn roster_context_differences_are_not_corruption_or_retargeting() {
        let mut c = capture();
        let old = StatRoster {
            id: id(10),
            world_id: id(1),
            revision: 1,
            entries: vec![],
        };
        let mut new = old.clone();
        new.revision = 2;
        c.nodes.insert(
            key(DomainKind::StatRoster, 10, 1),
            InspectionPayload::StatRoster(old.clone()),
        );
        root(
            &mut c,
            &key(DomainKind::StatRoster, 10, 2),
            InspectionPayload::StatRoster(new),
            RootRole::Active,
        );
        let mut r = rules();
        r.stat_roster = Some(old.pin());
        root(
            &mut c,
            &key(DomainKind::Ruleset, 20, 1),
            InspectionPayload::Ruleset(r),
            RootRole::Active,
        );
        root(
            &mut c,
            &key(DomainKind::Variant, 2, 1),
            variant(
                2,
                BaseStats {
                    roster: Some(old.pin()),
                    assignments: Some(vec![]),
                },
                TypingState {
                    assignments: Some(vec![]),
                },
            ),
            RootRole::Active,
        );
        let findings = analyze_world(&c, false);
        assert!(
            findings
                .iter()
                .any(|f| f.code == "stats.current_roster_context"
                    && f.category == FindingCategory::ContextDifference)
        );
        assert!(
            !findings
                .iter()
                .any(|f| f.code == "stats.ruleset_roster_context")
        );
        assert!(
            !findings
                .iter()
                .any(|f| f.category == FindingCategory::Contradiction)
        );
    }

    #[test]
    fn category_parser_is_closed_and_round_trips() {
        for c in [
            FindingCategory::Contradiction,
            FindingCategory::Incomplete,
            FindingCategory::ContextDifference,
            FindingCategory::NotEvaluated,
        ] {
            assert_eq!(c.as_str().parse(), Ok(c));
        }
        assert!("ready".parse::<FindingCategory>().is_err());
    }

    #[test]
    fn saved_selected_moves_use_old_learnset_and_rules_not_current_singletons() {
        let mut r = rules();
        r.repeat_policy = RepeatPolicy::Allow;
        let mut l = learnset();
        l.records = Some(vec![LearningRecord {
            id: id(41),
            move_record: move_record(30),
            method: Some("Inert".into()),
            context: Some("Inert".into()),
            condition: Some("Inert".into()),
            threshold: NumericValue::NotApplicable,
        }]);
        let mut p = preset(&r, &l);
        p.validator_version = 2;
        p.entries = vec![id(41), id(41), id(41)];
        let mut c = preset_capture(r.clone(), l.clone(), p.clone());
        c.nodes.insert(
            key(DomainKind::Move, 30, 1),
            InspectionPayload::Move(move_record(30)),
        );
        r.revision = 2;
        r.max_moves = Some(1);
        r.repeat_policy = RepeatPolicy::Forbid;
        l.revision = 2;
        l.records = Some(vec![]);
        root(
            &mut c,
            &key(DomainKind::Ruleset, 20, 2),
            InspectionPayload::Ruleset(r),
            RootRole::Active,
        );
        root(
            &mut c,
            &key(DomainKind::Learnset, 21, 2),
            InspectionPayload::Learnset(l),
            RootRole::Active,
        );
        let f = analyze_world(&c, true);
        assert!(!f.iter().any(|f| f.subject.kind == DomainKind::Preset
            && f.category == FindingCategory::Contradiction));
        assert!(f.iter().filter(|f| f.subject.kind == DomainKind::Preset && f.code.starts_with("validator.")).all(|f|
            f.evidence.contains(&key(DomainKind::Learnset, 21, 1)) && f.evidence.contains(&key(DomainKind::Ruleset, 20, 1))
                && !f.evidence.contains(&key(DomainKind::Learnset, 21, 2)) && !f.evidence.contains(&key(DomainKind::Ruleset, 20, 2))));
        // Changing only the trusted old policy to forbid yields a finding at each
        // repeated position, not a single identity-deduplicated occurrence.
        if let Some(InspectionPayload::Ruleset(r)) =
            c.nodes.get_mut(&key(DomainKind::Ruleset, 20, 1))
        {
            r.repeat_policy = RepeatPolicy::Forbid;
        }
        let f = analyze_world(&c, false);
        let sites: Vec<_> = f
            .iter()
            .filter(|f| f.code == "preset.selection_repeat")
            .map(|f| f.site.clone())
            .collect();
        assert_eq!(
            sites,
            vec![
                Some(EdgeSite::PresetSelection {
                    slot: 2,
                    record_id: id(41),
                    learnset: key(DomainKind::Learnset, 21, 1)
                }),
                Some(EdgeSite::PresetSelection {
                    slot: 3,
                    record_id: id(41),
                    learnset: key(DomainKind::Learnset, 21, 1)
                }),
            ]
        );
    }

    #[test]
    fn evidence_canonicalization_and_finding_order_compare_contents_not_arc_identity() {
        let subject = key(DomainKind::Preset, 23, 1);
        let a = key(DomainKind::Move, 30, 1);
        let b = key(DomainKind::Move, 31, 1);
        let mut findings = vec![];
        for evidence in [
            vec![b.clone(), a.clone(), b.clone()],
            vec![a.clone(), b.clone()],
        ] {
            finding(
                &mut findings,
                FindingCategory::Incomplete,
                "test.evidence",
                &subject,
                None,
                "An unchanged leaf message",
                evidence,
            );
        }
        assert_eq!(findings[0].evidence.as_ref(), [a, b]);
        assert!(!Arc::ptr_eq(&findings[0].evidence, &findings[1].evidence));
        assert_eq!(findings[0], findings[1]);
        assert_eq!(findings[0].cmp(&findings[1]), std::cmp::Ordering::Equal);
        findings.sort();
        findings.dedup();
        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn large_preset_reports_share_complete_evidence_without_losing_messages_or_occurrences() {
        const MOVE_COUNT: usize = 512;
        const ACQUISITION_COUNT: usize = MOVE_COUNT * 2;
        let mut c = capture();
        let mut r = rules();
        r.repeat_policy = RepeatPolicy::Forbid;
        let mut l = learnset();
        let lkey = key(DomainKind::Learnset, 21, 1);
        let mut records = vec![];
        let mut move_keys = vec![];
        for index in 0..MOVE_COUNT {
            let m = move_record(1000 + index);
            let mkey = key(DomainKind::Move, 1000 + index, 1);
            c.nodes
                .insert(mkey.clone(), InspectionPayload::Move(m.clone()));
            move_keys.push(mkey.clone());
            // Two distinct acquisitions of each exact Move; neither may disappear.
            for occurrence in 0..2 {
                let slot = index * 2 + occurrence + 1;
                let record_id = id(10_000 + slot);
                records.push(LearningRecord {
                    id: record_id.clone(),
                    move_record: m.clone(),
                    method: None,
                    context: None,
                    condition: None,
                    threshold: NumericValue::Unknown,
                });
                c.edges.push(edge(
                    &lkey,
                    &mkey,
                    EdgeKind::ExactPin,
                    EdgeSite::Acquisition { slot, record_id },
                ));
            }
        }
        l.records = Some(records);
        root(
            &mut c,
            &lkey,
            InspectionPayload::Learnset(l.clone()),
            RootRole::Active,
        );
        c.nodes.insert(
            key(DomainKind::Ruleset, 20, 1),
            InspectionPayload::Ruleset(r.clone()),
        );
        let mut presets = vec![];
        for preset_id in 23..26 {
            let mut p = preset(&r, &l);
            p.id = id(preset_id);
            p.entries = l
                .records
                .as_ref()
                .unwrap()
                .iter()
                .map(|record| record.id.clone())
                .collect();
            // Preserve repeated selections of the same acquisition as distinct slots.
            p.entries.extend([id(10_001), id(10_001)]);
            let pkey = key(DomainKind::Preset, preset_id, 1);
            root(
                &mut c,
                &pkey,
                InspectionPayload::Preset(p.clone()),
                RootRole::Active,
            );
            c.edges.push(edge(
                &pkey,
                &lkey,
                EdgeKind::ExactPin,
                EdgeSite::PresetLearnset,
            ));
            c.edges.push(edge(
                &pkey,
                &key(DomainKind::Ruleset, 20, 1),
                EdgeKind::ExactPin,
                EdgeSite::PresetRuleset,
            ));
            for (index, record_id) in p.entries.iter().enumerate() {
                let mkey = if index < ACQUISITION_COUNT {
                    &move_keys[index / 2]
                } else {
                    &move_keys[0]
                };
                c.edges.push(edge(
                    &pkey,
                    mkey,
                    EdgeKind::Selection,
                    EdgeSite::PresetSelection {
                        slot: index + 1,
                        record_id: record_id.clone(),
                        learnset: lkey.clone(),
                    },
                ));
            }
            presets.push((pkey, p));
        }
        let findings = analyze_world(&c, true);
        let acquisitions: Vec<_> = findings
            .iter()
            .filter(|f| f.code == "acquisition.descriptive_unknown")
            .collect();
        assert_eq!(acquisitions.len(), ACQUISITION_COUNT);
        for (index, f) in acquisitions.iter().enumerate() {
            assert_eq!(
                f.site,
                Some(EdgeSite::Acquisition {
                    slot: index + 1,
                    record_id: id(10_001 + index)
                })
            );
        }
        for (pkey, p) in &presets {
            let mut expected_evidence = move_keys.clone();
            expected_evidence.extend([key(DomainKind::Ruleset, 20, 1), lkey.clone(), pkey.clone()]);
            expected_evidence.sort();
            for comparison in [false, true] {
                let leaves: Vec<_> = findings
                    .iter()
                    .filter(|f| {
                        f.subject == *pkey
                            && f.code.starts_with("validator.preset.")
                            && f.comparison == comparison
                    })
                    .collect();
                assert!(leaves.len() > ACQUISITION_COUNT);
                for leaf in &leaves {
                    assert_eq!(leaf.evidence.as_ref(), expected_evidence.as_slice());
                    assert!(
                        Arc::ptr_eq(&leaf.evidence, &leaves[0].evidence),
                        "Every report leaf must share one canonical evidence allocation"
                    );
                }
                let mut evaluated = p.clone();
                if comparison {
                    evaluated.validator_version = CURRENT_VALIDATOR_VERSION;
                }
                let expected = validate_preset(&evaluated, &l, &r);
                for (category, mut expected_messages) in [
                    (FindingCategory::Contradiction, expected.contradictions),
                    (FindingCategory::Incomplete, expected.incomplete),
                    (FindingCategory::NotEvaluated, expected.not_evaluated),
                ] {
                    let mut actual: Vec<_> = leaves
                        .iter()
                        .filter(|f| f.category == category)
                        .map(|f| f.message.clone())
                        .collect();
                    actual.sort();
                    expected_messages.sort();
                    assert_eq!(
                        actual, expected_messages,
                        "No validator leaf may be dropped"
                    );
                }
            }
            let repeat_sites: Vec<_> = findings
                .iter()
                .filter(|f| f.subject == *pkey && f.code == "preset.selection_repeat")
                .map(|f| f.site.clone())
                .collect();
            let expected_repeat_sites: Vec<_> = (2..=ACQUISITION_COUNT)
                .step_by(2)
                .chain([ACQUISITION_COUNT + 1, ACQUISITION_COUNT + 2])
                .map(|slot| {
                    Some(EdgeSite::PresetSelection {
                        slot,
                        record_id: if slot <= ACQUISITION_COUNT {
                            id(10_000 + slot)
                        } else {
                            id(10_001)
                        },
                        learnset: lkey.clone(),
                    })
                })
                .collect();
            assert_eq!(repeat_sites, expected_repeat_sites);
        }
        let usage = where_used(&c, &query(&move_keys[0], UseView::DirectUses));
        assert_eq!(usage.total, 14); // 2 acquisitions plus 4 selections in each preset.
        assert_eq!(usage.uses.len(), 14);
        assert!(usage.complete);
        assert_eq!(
            usage
                .uses
                .iter()
                .filter(|u| u.edge.kind == EdgeKind::Selection)
                .count(),
            12
        );
    }
}
