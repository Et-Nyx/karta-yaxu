//! One-connection, bounded, ordinary read-snapshot capture. SQL identifiers below
//! come only from closed domain kinds, never addresses or browser input.
use super::{authoring as content, charts, learning, moves, stats, types};
use crate::application::authoring::{AddressKey, Error, HistoryKind};
use content::SnapshotKind;
use game_data::{inspection::*, stats::BaseStats, types::TypingState};
use serde::{Serialize, de::DeserializeOwned};
use sqlx::{Connection, Row, Sqlite, SqliteConnection, pool::PoolConnection};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::time::{Instant, timeout_at};

fn key(kind: DomainKind, world: Option<&str>, id: &str, revision: i64) -> DomainNodeKey {
    DomainNodeKey {
        kind,
        world_id: world.map(str::to_owned),
        id: id.into(),
        revision,
    }
}
fn integrity(subject: &DomainNodeKey) -> CaptureGap {
    CaptureGap { kind: CaptureGapKind::Integrity, subject: Some(subject.clone()), message: "The exact record or one of its required verification inputs is missing or inconsistent.".into() }
}
fn budget(subject: Option<&DomainNodeKey>, message: &str) -> CaptureGap {
    CaptureGap {
        kind: CaptureGapKind::Budget,
        subject: subject.cloned(),
        message: message.into(),
    }
}
fn decode<T: DeserializeOwned>(value: &serde_json::Value) -> Result<T, Error> {
    serde_json::from_value(value.clone()).map_err(|_| Error::Unavailable)
}
fn json(text: &str) -> Result<serde_json::Value, Error> {
    serde_json::from_str(text).map_err(|_| Error::Unavailable)
}

struct Table {
    head: &'static str,
    versions: &'static str,
    id: &'static str,
    scope: &'static str,
    children: &'static [(&'static str, &'static str)],
    sidecars: &'static [&'static str],
}
fn table(k: &DomainNodeKey) -> Table {
    use DomainKind::*;
    let (head, versions, id, scope, children, sidecars) = match k.kind {
        World => (
            "worlds",
            "world_revisions",
            "world_id",
            "''",
            &[][..],
            &["world_content_snapshots"][..],
        ),
        Species => (
            "species",
            "species_versions",
            "species_id",
            "''",
            &[][..],
            &[][..],
        ),
        Form => (
            "forms",
            "form_versions",
            "form_id",
            "''",
            &[
                ("form_type_bindings", "form_id"),
                ("form_stat_bindings", "form_id"),
            ][..],
            &["form_content_snapshots", "form_stat_snapshots"][..],
        ),
        Variant => (
            "variants",
            "variant_revisions",
            "variant_id",
            "world_id",
            &[
                ("variant_type_bindings", "variant_id"),
                ("variant_stat_bindings", "variant_id"),
            ][..],
            &["variant_content_snapshots", "variant_stat_snapshots"][..],
        ),
        Type if k.world_id.is_some() => (
            "world_types",
            "world_type_versions",
            "type_id",
            "world_id",
            &[][..],
            &[][..],
        ),
        Type => ("types", "type_versions", "type_id", "''", &[][..], &[][..]),
        Stat => (
            "stat_definitions",
            "stat_versions",
            "stat_id",
            "scope",
            &[][..],
            &[][..],
        ),
        StatRoster => (
            "stat_rosters",
            "stat_roster_versions",
            "roster_id",
            "world_id",
            &[("stat_roster_entries", "roster_id")][..],
            &[][..],
        ),
        MoveAttribute => (
            "move_attribute_definitions",
            "move_attribute_versions",
            "attribute_id",
            "scope",
            &[][..],
            &[][..],
        ),
        Move => (
            "moves",
            "move_versions",
            "move_id",
            "scope",
            &[
                ("move_attribute_bindings", "move_id"),
                ("move_attribute_mappings", "move_id"),
            ][..],
            &[][..],
        ),
        Chart => (
            "charts",
            "chart_versions",
            "chart_id",
            "scope",
            &[
                ("chart_roster", "chart_id"),
                ("chart_cells", "chart_id"),
                ("chart_mapping", "chart_id"),
            ][..],
            &[][..],
        ),
        Ruleset => (
            "world_rulesets",
            "world_ruleset_versions",
            "ruleset_id",
            "world_id",
            &[("ruleset_attribute_expectations", "ruleset_id")][..],
            &[][..],
        ),
        Learnset => (
            "learnsets",
            "learnset_versions",
            "learnset_id",
            "scope",
            &[
                ("learnset_records", "learnset_id"),
                ("learnset_move_mappings", "learnset_id"),
            ][..],
            &[][..],
        ),
        Preset => (
            "moveset_presets",
            "moveset_preset_versions",
            "preset_id",
            "world_id",
            &[("moveset_preset_entries", "preset_id")][..],
            &[][..],
        ),
    };
    Table {
        head,
        versions,
        id,
        scope,
        children,
        sidecars,
    }
}

struct Capture {
    result: CapturedWorld,
    limits: InspectionLimits,
    deadline: Instant,
    bytes: usize,
    children: usize,
    // Preflight and checked reads have separate caches. Unverified records never
    // enter the public graph. Companions are verification inputs, not graph nodes.
    preflight: BTreeMap<DomainNodeKey, Result<InspectionPayload, CaptureGap>>,
    plans: BTreeMap<DomainNodeKey, Result<(), CaptureGap>>,
    input_refs: BTreeMap<DomainNodeKey, BTreeSet<DomainNodeKey>>,
    verified: BTreeMap<DomainNodeKey, Result<InspectionPayload, Error>>,
    legacy_companions: BTreeMap<DomainNodeKey, Result<(), Error>>,
    current: BTreeMap<DomainIdentity, DomainNodeKey>,
}

/// The first SELECT resolves the anchor. Capture owns this connection's progress
/// handler until ordinary exit, when it is removed *before* rollback. On caller
/// cancellation the borrowed-connection handler becomes inert, and SQLx queues
/// rollback on transaction drop; reuse waits for any in-flight step to finish.
/// Applications should use WorldInspection, which also discards cancelled leases.
pub async fn capture_world(
    c: &mut SqliteConnection,
    world: AddressKey,
    limits: InspectionLimits,
) -> Result<CapturedWorld, Error> {
    capture_impl(c, world, limits, || async {}).await
}

/// The deadline starts before pool acquisition in the application service. This
/// owned lease is always closed on drop, including cancellation during cleanup;
/// an installed/cancelled progress callback can never reach the authoring pool.
pub(crate) async fn capture_pooled_world(
    mut c: PoolConnection<Sqlite>,
    world: AddressKey,
    limits: InspectionLimits,
    deadline: Instant,
) -> Result<CapturedWorld, Error> {
    c.close_on_drop();
    capture_until(&mut c, world, limits, deadline, true, || async {}).await
}

/// Authorized ADR0015 active-catalog capture. The caller owns admission and must
/// recheck permission before releasing a download. Only public content leaves
/// this read transaction; package validation/serialization happen afterwards.
pub(crate) async fn capture_pooled_package(
    c: PoolConnection<Sqlite>,
    world: AddressKey,
    limits: InspectionLimits,
    deadline: Instant,
    accounts: &crate::accounts::Accounts,
    token: &str,
) -> Result<game_data::package::PackageContent, Error> {
    capture_package_until(c, world, limits, deadline, accounts, token, || async {}).await
}

// As with inspection, a generic rendezvous tests the first-read boundary without
// installing a production hook. The owned lease is discarded even if cancelled
// while waiting for the worker or removing its progress handler.
async fn capture_package_until<F, Fut>(
    mut c: PoolConnection<Sqlite>,
    world: AddressKey,
    limits: InspectionLimits,
    deadline: Instant,
    accounts: &crate::accounts::Accounts,
    token: &str,
    after_first_read: F,
) -> Result<game_data::package::PackageContent, Error>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    c.close_on_drop();
    let guard = SqliteDeadline::install(&mut c, deadline, true).await?;
    let result = async {
        let mut tx = timeout_at(deadline, c.begin())
            .await
            .map_err(|_| Error::Unavailable)?
            .map_err(content::storage_error)?;
        let result = timeout_at(deadline, async {
            // Resolving the world is deliberately the first read, before session
            // and collaborator queries, to pin permission and content together.
            let anchor = content::resolve_world(&mut tx, world).await?;
            after_first_read().await;
            let actor = accounts
                .authenticate_in_transaction(&mut tx, token)
                .await
                .map_err(|_| Error::Unavailable)?
                .ok_or(Error::Unauthorized)?;
            crate::application::authoring::Authoring::new(accounts.clone())
                .world_actor(&mut tx, &actor, &anchor, false)
                .await?;
            if anchor.archived_at.is_some() {
                return Err(Error::Forbidden);
            }
            let public = PublicWorldContent {
                id: anchor.id,
                fields: anchor.fields,
                revision: anchor.revision,
                archived_at: anchor.archived_at,
            };
            let mut capture = Capture::new(public, limits, deadline);
            capture.package(&mut tx).await
        })
        .await
        .map_err(|_| Error::Unavailable)
        .and_then(|result| result);
        // Keep the callback armed until any cancelled sqlite3_step has drained,
        // then remove it before rollback, including authorization/error exits.
        guard.remove(&mut tx).await?;
        tx.rollback().await.map_err(content::storage_error)?;
        result
    }
    .await;
    // Also covers begin failures. Removal is idempotent.
    guard.remove(&mut c).await?;
    if guard.expired() {
        return Err(Error::Unavailable);
    }
    result
}

struct DeadlineState {
    deadline: Instant,
    active: AtomicBool,
    cancelled: AtomicBool,
    interrupted: AtomicBool,
}

/// SQLx 0.9's safe locked-handle API installs a callback returning false to
/// interrupt SQLite. No connection operations occur inside that callback.
/// Progress checks are cooperative (every ~1000 VM instructions), not a hard
/// wall-clock bound: busy waits (normally bounded by this pool's 5s busy timeout),
/// filesystem stalls and non-VM Rust/SQLite work can delay completion/cleanup.
struct SqliteDeadline {
    state: Arc<DeadlineState>,
    discard_on_cancel: bool,
}
impl SqliteDeadline {
    async fn install(
        c: &mut SqliteConnection,
        deadline: Instant,
        discard_on_cancel: bool,
    ) -> Result<Self, Error> {
        let state = Arc::new(DeadlineState {
            deadline,
            active: AtomicBool::new(true),
            cancelled: AtomicBool::new(false),
            interrupted: AtomicBool::new(false),
        });
        let callback = state.clone();
        let mut handle = timeout_at(deadline, c.lock_handle())
            .await
            .map_err(|_| Error::Unavailable)?
            .map_err(content::storage_error)?;
        if Instant::now() >= deadline {
            return Err(Error::Unavailable);
        }
        handle.set_progress_handler(1000, move || {
            if !callback.active.load(Ordering::Acquire) {
                return true;
            }
            if callback.cancelled.load(Ordering::Acquire) || Instant::now() >= callback.deadline {
                callback.interrupted.store(true, Ordering::Release);
                false
            } else {
                true
            }
        });
        Ok(Self {
            state,
            discard_on_cancel,
        })
    }

    async fn remove(&self, c: &mut SqliteConnection) -> Result<(), Error> {
        // Dropping a Tokio query future does not stop sqlite3_step. Keep the
        // deadline callback armed while lock_handle waits for the worker to
        // finish/interrupt that step. Only then remove it, so rollback cannot be
        // rejected by the expired handler. This intentionally awaits cooperative
        // worker drainage rather than hiding it behind another Tokio timeout.
        let mut handle = c.lock_handle().await.map_err(content::storage_error)?;
        handle.remove_progress_handler();
        self.state.active.store(false, Ordering::Release);
        Ok(())
    }

    fn expired(&self) -> bool {
        self.state.interrupted.load(Ordering::Acquire) || Instant::now() >= self.state.deadline
    }
}
impl Drop for SqliteDeadline {
    fn drop(&mut self) {
        if self.discard_on_cancel {
            // Only the owned close-on-drop path uses this policy. Interrupt the
            // cancelled worker at its next VM progress check; never reuse it.
            self.state.cancelled.store(true, Ordering::Release);
        } else {
            // A borrowed connection cannot be asynchronously cleaned up in Drop.
            // Leave a safe inactive callback, not an expired authoring poison.
            self.state.active.store(false, Ordering::Release);
        }
    }
}

// A generic first-read rendezvous permits a real synchronized WAL test without
// a runtime hook, mutable global, or production testing framework.
async fn capture_impl<F, Fut>(
    c: &mut SqliteConnection,
    world: AddressKey,
    limits: InspectionLimits,
    after_first_read: F,
) -> Result<CapturedWorld, Error>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    let deadline = Instant::now() + Duration::from_millis(limits.deadline_ms);
    capture_until(c, world, limits, deadline, false, after_first_read).await
}

async fn capture_until<F, Fut>(
    c: &mut SqliteConnection,
    world: AddressKey,
    limits: InspectionLimits,
    deadline: Instant,
    discard_on_cancel: bool,
    after_first_read: F,
) -> Result<CapturedWorld, Error>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    let guard = SqliteDeadline::install(c, deadline, discard_on_cancel).await?;
    let mut result =
        capture_transaction(c, world, limits, deadline, &guard, after_first_read).await;
    // Also covers begin/anchor failures. Ordinary transaction exits already
    // removed the handler before rollback; removing it twice is harmless.
    guard.remove(c).await?;
    if let Ok(captured) = &mut result
        && guard.expired()
    {
        captured.gaps.push(budget(
            Some(&captured.world_key()),
            "The capture deadline was reached; coverage is partial.",
        ));
        captured.gaps.sort();
        captured.gaps.dedup();
    }
    result
}

async fn capture_transaction<F, Fut>(
    c: &mut SqliteConnection,
    world: AddressKey,
    limits: InspectionLimits,
    deadline: Instant,
    guard: &SqliteDeadline,
    after_first_read: F,
) -> Result<CapturedWorld, Error>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    let mut tx = timeout_at(deadline, c.begin())
        .await
        .map_err(|_| Error::Unavailable)?
        .map_err(content::storage_error)?;
    let result = capture_snapshot(&mut tx, world, limits, deadline, guard, after_first_read).await;
    guard.remove(&mut tx).await?;
    tx.rollback().await.map_err(content::storage_error)?;
    result
}

async fn capture_snapshot<F, Fut>(
    c: &mut SqliteConnection,
    world: AddressKey,
    limits: InspectionLimits,
    deadline: Instant,
    guard: &SqliteDeadline,
    after_first_read: F,
) -> Result<CapturedWorld, Error>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    let anchor = timeout_at(deadline, content::resolve_world(&mut *c, world))
        .await
        .map_err(|_| Error::Unavailable)??;
    if guard.expired() {
        return Err(Error::Unavailable);
    }
    let public = PublicWorldContent {
        id: anchor.id,
        fields: anchor.fields,
        revision: anchor.revision,
        archived_at: anchor.archived_at,
    };
    let mut capture = Capture::new(public, limits, deadline);
    let anchor = capture.result.world_key();
    capture.result.nodes.insert(
        anchor.clone(),
        InspectionPayload::World(capture.result.world.clone()),
    );
    capture.result.roots.insert(
        anchor.clone(),
        if capture.result.world.archived_at.is_some() {
            RootRole::Archived
        } else {
            RootRole::Active
        },
    );
    // The anchor can still be displayed when a caller's operational budget
    // prevents verification. A corrupt anchor, when read, is unavailable.
    let run = timeout_at(deadline, async {
        after_first_read().await;
        capture.run(c).await
    })
    .await;
    match run {
        _ if guard.expired() => capture.result.gaps.push(budget(
            Some(&anchor),
            "The capture deadline was reached; coverage is partial.",
        )),
        Err(_) => capture.result.gaps.push(budget(
            Some(&anchor),
            "The capture deadline was reached; coverage is partial.",
        )),
        Ok(Err(e)) => return Err(e),
        Ok(Ok(())) => {}
    }
    capture.result.edges.sort();
    capture.result.absences.sort();
    capture.result.gaps.sort();
    capture.result.gaps.dedup();
    Ok(capture.result)
}

impl Capture {
    fn new(world: PublicWorldContent, limits: InspectionLimits, deadline: Instant) -> Self {
        Self {
            result: CapturedWorld {
                contract_version: 1,
                world,
                roots: BTreeMap::new(),
                nodes: BTreeMap::new(),
                edges: vec![],
                heads: BTreeMap::new(),
                absences: vec![],
                gaps: vec![],
            },
            limits,
            deadline,
            bytes: 0,
            children: 0,
            preflight: BTreeMap::new(),
            plans: BTreeMap::new(),
            input_refs: BTreeMap::new(),
            verified: BTreeMap::new(),
            legacy_companions: BTreeMap::new(),
            current: BTreeMap::new(),
        }
    }

    /// Selected roots and package references only: neither the full inspection
    /// inventory nor its derived graph enters this content closure.
    async fn package(
        &mut self,
        c: &mut SqliteConnection,
    ) -> Result<game_data::package::PackageContent, Error> {
        use game_data::package::{self, IdentityBinding, PackageContent, PackageRecord};
        self.limits.max_roots = self.limits.max_roots.min(package::MAX_ROOTS);
        self.limits.max_nodes = self.limits.max_nodes.min(package::MAX_RECORDS);
        self.limits.max_edges = self.limits.max_edges.min(package::MAX_OCCURRENCES);
        if self.limits.max_roots == 0 || self.limits.max_nodes == 0 {
            return Err(Error::Unavailable);
        }
        let roots = self.package_roots(c).await?;
        self.result.roots = roots
            .iter()
            .cloned()
            .map(|k| (k, RootRole::Active))
            .collect();
        let mut pending = roots.clone();
        let mut bindings = BTreeMap::new();
        let mut occurrences = roots.len();
        while let Some(k) = pending.pop_first() {
            if self.result.nodes.contains_key(&k) {
                continue;
            }
            if self.result.nodes.len() >= self.limits.max_nodes {
                return Err(Error::Unavailable);
            }
            let row = self.package_checked(c, &k).await?;
            if roots.contains(&k) && row.archived_at().is_some() {
                return Err(Error::Unavailable);
            }
            let refs = package::references(&row).map_err(|_| Error::Unavailable)?;
            occurrences = occurrences
                .checked_add(refs.exact.len())
                .and_then(|n| n.checked_add(refs.identities.len()))
                .ok_or(Error::Unavailable)?;
            if occurrences > self.limits.max_edges {
                return Err(Error::Unavailable);
            }
            pending.extend(refs.exact);
            for identity in refs.identities {
                let binding_count = bindings.len();
                if let std::collections::btree_map::Entry::Vacant(entry) = bindings.entry(identity)
                {
                    if binding_count >= self.limits.max_nodes {
                        return Err(Error::Unavailable);
                    }
                    let head = self.package_current(c, entry.key()).await?;
                    pending.insert(head.clone());
                    entry.insert(head);
                }
            }
            self.result.nodes.insert(k, row);
            // Bound queued unique keys as well as materialized records. Cycles
            // and duplicate occurrences remain legal and are counted above.
            pending.retain(|k| !self.result.nodes.contains_key(k));
            if pending.len()
                > self
                    .limits
                    .max_nodes
                    .saturating_sub(self.result.nodes.len())
            {
                return Err(Error::Unavailable);
            }
        }
        if Instant::now() >= self.deadline || !self.result.gaps.is_empty() {
            return Err(Error::Unavailable);
        }
        Ok(PackageContent {
            schema_version: package::SCHEMA_VERSION,
            root_policy: package::ROOT_POLICY.into(),
            world: self.result.world_key(),
            roots: roots.into_iter().collect(),
            records: std::mem::take(&mut self.result.nodes)
                .into_iter()
                .map(|(key, payload)| PackageRecord { key, payload })
                .collect(),
            bindings: bindings
                .into_iter()
                .map(|(identity, target)| IdentityBinding { identity, target })
                .collect(),
        })
    }

    async fn package_roots(
        &mut self,
        c: &mut SqliteConnection,
    ) -> Result<BTreeSet<DomainNodeKey>, Error> {
        let w = self.result.world.id.clone();
        let mut roots = BTreeSet::from([self.result.world_key()]);
        for kind in [
            DomainKind::Type,
            DomainKind::Stat,
            DomainKind::MoveAttribute,
            DomainKind::Move,
            DomainKind::Variant,
            DomainKind::Chart,
            DomainKind::StatRoster,
            DomainKind::Ruleset,
            DomainKind::Learnset,
            DomainKind::Preset,
        ] {
            let t = table(&key(kind, Some(&w), "", 1));
            let scope = if t.scope == "scope" {
                "(h.scope=?1 OR h.world_id=?1)"
            } else {
                "h.world_id=?1"
            };
            // Most aggregates keep archive state on the current immutable
            // version. NOT EXISTS intentionally retains missing version rows:
            // checked capture must reject them rather than silently omit roots.
            let active = match kind {
                DomainKind::Type | DomainKind::Variant => "h.archived_at IS NULL".into(),
                DomainKind::StatRoster | DomainKind::Ruleset | DomainKind::Learnset => "1=1".into(),
                _ => format!(
                    "NOT EXISTS(SELECT 1 FROM {} v WHERE v.{}=h.id AND v.{}=h.{} AND v.revision=h.revision AND v.archived_at IS NOT NULL)",
                    t.versions, t.id, t.scope, t.scope
                ),
            };
            let owner = if matches!(kind, DomainKind::Learnset | DomainKind::Preset) {
                "AND EXISTS(SELECT 1 FROM variants owner WHERE owner.world_id=?1 AND owner.id=h.variant_id AND owner.archived_at IS NULL)"
            } else {
                ""
            };
            let remaining = self.limits.max_roots.saturating_sub(roots.len());
            let sql = format!(
                "SELECT h.id,h.revision FROM {} h WHERE {scope} AND {active} {owner} ORDER BY h.id LIMIT ?2",
                t.head
            );
            let rows: Vec<(String, i64)> = sqlx::query_as(sqlx::AssertSqlSafe(sql))
                .bind(&w)
                .bind(i64::try_from(remaining.saturating_add(1)).unwrap_or(i64::MAX))
                .fetch_all(&mut *c)
                .await
                .map_err(|_| Error::Unavailable)?;
            if rows.len() > remaining
                || (matches!(
                    kind,
                    DomainKind::Chart | DomainKind::StatRoster | DomainKind::Ruleset
                ) && rows.len() > 1)
            {
                return Err(Error::Unavailable);
            }
            for (id, revision) in rows {
                if !content::valid_id(&id) || revision < 1 {
                    return Err(Error::Unavailable);
                }
                roots.insert(key(kind, Some(&w), &id, revision));
            }
        }
        Ok(roots)
    }

    async fn package_current(
        &mut self,
        c: &mut SqliteConnection,
        identity: &DomainIdentity,
    ) -> Result<DomainNodeKey, Error> {
        if let Some(head) = self.current.get(identity) {
            return Ok(head.clone());
        }
        let mut head = key(identity.kind, identity.world_id.as_deref(), &identity.id, 1);
        let t = table(&head);
        head.revision = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT revision FROM {} WHERE id=?1 AND {}=?2",
            t.head, t.scope
        )))
        .bind(&head.id)
        .bind(head.world_id.as_deref().unwrap_or(""))
        .fetch_optional(&mut *c)
        .await
        .map_err(|_| Error::Unavailable)?
        .ok_or(Error::Unavailable)?;
        // Resolve the mutable head, never MAX(version). Its full checked read
        // (including companions) is cached internally; only the bound key enters
        // the package closure, not any unrelated current verification input.
        self.package_checked(c, &head).await?;
        Ok(head)
    }

    async fn package_checked(
        &mut self,
        c: &mut SqliteConnection,
        k: &DomainNodeKey,
    ) -> Result<InspectionPayload, Error> {
        self.prepare(c, k).await.map_err(|_| Error::Unavailable)?;
        let row = self.checked(c, k).await.map_err(|_| Error::Unavailable)?;
        let head = self
            .current
            .get(&k.identity())
            .cloned()
            .ok_or(Error::Unavailable)?;
        let current = self
            .checked(c, &head)
            .await
            .map_err(|_| Error::Unavailable)?;
        if game_data::package::payload_key(&row) != *k
            || game_data::package::payload_key(&current) != head
            || !self.result.gaps.is_empty()
            || Instant::now() >= self.deadline
        {
            return Err(Error::Unavailable);
        }
        Ok(row)
    }

    async fn run(&mut self, c: &mut SqliteConnection) -> Result<(), Error> {
        let world = self.result.world_key();
        if self.limits.max_roots == 0 || self.limits.max_nodes == 0 {
            self.result.gaps.push(budget(
                Some(&world),
                "The world anchor exceeds the root or exact-node budget.",
            ));
            return Ok(());
        }
        let roots = self.inventory(c).await?;
        let mut pending = BTreeSet::from([world.clone()]);
        pending.extend(roots.iter().cloned());
        while let Some(k) = pending.pop_first() {
            if let Err(gap) = self.prepare(c, &k).await {
                if Instant::now() >= self.deadline {
                    self.result
                        .gaps
                        .push(budget(Some(&k), "The capture deadline was reached."));
                    break;
                }
                if k == world && gap.kind == CaptureGapKind::Integrity {
                    return Err(Error::Unavailable);
                }
                self.result.gaps.push(gap);
                continue;
            }
            // Only explicit graph references enter the closure. Current-head and
            // predecessor companions prepared above never become roots/nodes.
            let row = self.checked(c, &k).await;
            if Instant::now() >= self.deadline {
                self.result
                    .gaps
                    .push(budget(Some(&k), "The capture deadline was reached."));
                break;
            }
            let row = match row {
                Ok(row) => row,
                Err(_) if k == world => return Err(Error::Unavailable),
                Err(_) => {
                    self.result.gaps.push(integrity(&k));
                    continue;
                }
            };
            if self.result.nodes.len() >= self.limits.max_nodes
                && !self.result.nodes.contains_key(&k)
            {
                self.result
                    .gaps
                    .push(budget(Some(&k), "The exact-node budget was reached."));
                continue;
            }
            let head_key = self
                .current
                .get(&k.identity())
                .cloned()
                .ok_or(Error::Unavailable)?;
            let head = self.checked(c, &head_key).await;
            if Instant::now() >= self.deadline {
                self.result
                    .gaps
                    .push(budget(Some(&head_key), "The capture deadline was reached."));
                break;
            }
            match head {
                Ok(head) => {
                    self.result.heads.insert(
                        k.identity(),
                        HeadObservation {
                            revision: head_key.revision,
                            archived_at: head.archived_at(),
                        },
                    );
                }
                Err(_) if k == world => return Err(Error::Unavailable),
                Err(_) => {
                    self.result.gaps.push(integrity(&head_key));
                }
            }
            if roots.contains(&k) || k == world {
                self.result.roots.insert(
                    k.clone(),
                    if row.archived_at().is_some() {
                        RootRole::Archived
                    } else {
                        RootRole::Active
                    },
                );
            }
            let edges = basic_edges(&k, &row);
            self.result.nodes.insert(k.clone(), row);
            for edge in edges {
                if self.add_edge(edge.clone())
                    && let DomainRef::Exact(to) = edge.to
                    && !self.result.nodes.contains_key(&to)
                    && !self.verified.get(&to).is_some_and(Result::is_err)
                {
                    pending.insert(to);
                }
            }
        }
        self.derived_edges();
        if Instant::now() >= self.deadline {
            self.result
                .gaps
                .push(budget(Some(&world), "The capture deadline was reached."));
        }
        Ok(())
    }

    fn add_edge(&mut self, edge: InspectionEdge) -> bool {
        if self.result.edges.len() >= self.limits.max_edges {
            self.result.gaps.push(budget(
                Some(&edge.from),
                "The edge-occurrence budget was reached.",
            ));
            false
        } else {
            self.result.edges.push(edge);
            true
        }
    }

    async fn inventory(
        &mut self,
        c: &mut SqliteConnection,
    ) -> Result<BTreeSet<DomainNodeKey>, Error> {
        let w = self.result.world.id.clone();
        let world = self.result.world_key();
        let mut roots = BTreeSet::new();
        // Direct head inventories deliberately do not inner-join owners/versions:
        // missing parents and missing immutable rows must remain observable.
        for kind in [
            DomainKind::Type,
            DomainKind::Stat,
            DomainKind::MoveAttribute,
            DomainKind::Move,
            DomainKind::Variant,
            DomainKind::Chart,
            DomainKind::StatRoster,
            DomainKind::Ruleset,
            DomainKind::Learnset,
            DomainKind::Preset,
        ] {
            let t = table(&key(kind, Some(&w), "", 1));
            let remaining = self.limits.max_roots.saturating_sub(1 + roots.len());
            let filter = if t.scope == "scope" {
                "(scope=?1 OR world_id=?1)"
            } else {
                "world_id=?1"
            };
            let sql = format!(
                "SELECT id,revision FROM {} WHERE {filter} ORDER BY id LIMIT ?2",
                t.head
            );
            let rows: Result<Vec<(String, i64)>, _> = sqlx::query_as(sqlx::AssertSqlSafe(sql))
                .bind(&w)
                .bind(i64::try_from(remaining.saturating_add(1)).unwrap_or(i64::MAX))
                .fetch_all(&mut *c)
                .await;
            let rows = match rows {
                Ok(rows) => rows,
                Err(_) => {
                    if Instant::now() >= self.deadline {
                        self.result
                            .gaps
                            .push(budget(Some(&world), "The capture deadline was reached."));
                        return Ok(roots);
                    }
                    self.result.gaps.push(CaptureGap {
                        kind: CaptureGapKind::Inventory,
                        subject: Some(world.clone()),
                        message: format!(
                            "The current {} inventory could not be read.",
                            kind.as_str()
                        ),
                    });
                    continue;
                }
            };
            if rows.len() > remaining {
                self.result.gaps.push(budget(
                    Some(&world),
                    &format!(
                        "The current-root budget prevents a complete {} inventory.",
                        kind.as_str()
                    ),
                ));
                // Do not present a truncated fetch as a complete inventory.
                continue;
            }
            if matches!(
                kind,
                DomainKind::Chart | DomainKind::StatRoster | DomainKind::Ruleset
            ) {
                if rows.is_empty() {
                    self.result.absences.push(InventoryAbsence {
                        kind,
                        owner_id: None,
                    });
                }
                if rows.len() > 1 {
                    self.result.gaps.push(CaptureGap {
                        kind: CaptureGapKind::Inventory,
                        subject: Some(world.clone()),
                        message: format!("More than one {} singleton was observed.", kind.as_str()),
                    });
                }
            }
            for (id, revision) in rows {
                let k = key(kind, Some(&w), &id, revision);
                if !content::valid_id(&id) || revision < 1 {
                    self.result.gaps.push(integrity(&k));
                    continue;
                }
                roots.insert(k.clone());
                self.add_edge(InspectionEdge {
                    from: world.clone(),
                    to: DomainRef::Exact(k),
                    kind: EdgeKind::Membership,
                    site: EdgeSite::WorldMember { kind },
                });
            }
        }
        let variants = roots
            .iter()
            .filter(|k| k.kind == DomainKind::Variant)
            .cloned()
            .collect::<Vec<_>>();
        for v in variants {
            for (kind, head) in [
                (DomainKind::Learnset, "learnsets"),
                (DomainKind::Preset, "moveset_presets"),
            ] {
                let count: Result<i64, _> = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                    "SELECT count(*) FROM {head} WHERE world_id=? AND variant_id=?"
                )))
                .bind(&w)
                .bind(&v.id)
                .fetch_one(&mut *c)
                .await;
                let count = match count {
                    Ok(count) => count,
                    Err(_) => {
                        if Instant::now() >= self.deadline {
                            self.result
                                .gaps
                                .push(budget(Some(&v), "The capture deadline was reached."));
                            return Ok(roots);
                        }
                        self.result.gaps.push(CaptureGap {
                            kind: CaptureGapKind::Inventory,
                            subject: Some(v.clone()),
                            message: format!(
                                "This variant's {} inventory could not be read.",
                                kind.as_str()
                            ),
                        });
                        continue;
                    }
                };
                if count == 0 {
                    self.result.absences.push(InventoryAbsence {
                        kind,
                        owner_id: Some(v.id.clone()),
                    });
                }
            }
        }
        // Validate current-world membership against the authoritative supported
        // world snapshot separately, after bounded inventory/preflight.
        Ok(roots)
    }

    // Preflight every known requested/head/predecessor snapshot and its finite
    // explicit DAG before entering readers that recursively verify dependencies.
    // This is accounting of known inputs, not a hard SQL/CPU meter: checked
    // readers may revisit nested dependencies internally.
    async fn prepare(
        &mut self,
        c: &mut SqliteConnection,
        requested: &DomainNodeKey,
    ) -> Result<(), CaptureGap> {
        if let Some(result) = self.plans.get(requested) {
            return result.clone();
        }
        let result = self.prepare_plan(c, requested).await;
        self.plans.insert(requested.clone(), result.clone());
        result
    }

    async fn prepare_plan(
        &mut self,
        c: &mut SqliteConnection,
        requested: &DomainNodeKey,
    ) -> Result<(), CaptureGap> {
        let mut pending = BTreeSet::from([(requested.clone(), true)]);
        let mut visited = BTreeSet::new();
        while let Some((k, companions)) = pending.pop_first() {
            if !visited.insert((k.clone(), companions)) {
                continue;
            }
            if Instant::now() >= self.deadline {
                return Err(budget(Some(&k), "The capture deadline was reached."));
            }
            if !self.preflight.contains_key(&k) && self.preflight.len() >= self.limits.max_nodes {
                let gap = budget(
                    Some(&k),
                    "The bounded verification-input node budget was reached.",
                );
                self.preflight.insert(k.clone(), Err(gap.clone()));
                return Err(gap);
            }
            let result = match self.preflight.get(&k) {
                Some(Ok(row)) => Ok((
                    row.clone(),
                    self.current
                        .get(&k.identity())
                        .cloned()
                        .ok_or_else(|| integrity(&k))?,
                )),
                Some(Err(gap)) => Err(gap.clone()),
                None => self.prepare_one(c, &k).await,
            };
            match result {
                Ok((row, head)) => {
                    self.current.insert(k.identity(), head.clone());
                    if companions {
                        pending.insert((head.clone(), false));
                        for mut prev in [k.clone(), head] {
                            if prev.revision > 1 {
                                prev.revision -= 1;
                                pending.insert((prev, false));
                            }
                        }
                    }
                    // Predecessors of the requested and current revision only;
                    // do not recursively traverse all historical predecessors.
                    for edge in basic_edges(&k, &row) {
                        if let DomainRef::Exact(to) = edge.to {
                            pending.insert((to, true));
                        }
                    }
                    for to in self.input_refs.get(&k).into_iter().flatten() {
                        pending.insert((to.clone(), true));
                    }
                    self.preflight.insert(k, Ok(row));
                }
                Err(gap) => {
                    self.preflight.insert(k, Err(gap.clone()));
                    return Err(gap);
                }
            }
        }
        Ok(())
    }

    async fn prepare_one(
        &mut self,
        c: &mut SqliteConnection,
        k: &DomainNodeKey,
    ) -> Result<(InspectionPayload, DomainNodeKey), CaptureGap> {
        self.preflight_one(c, k).await.map_err(|error| {
            if Instant::now() >= self.deadline {
                budget(Some(k), "The capture deadline was reached.")
            } else {
                match error {
                    PreflightError::Integrity => integrity(k),
                    PreflightError::Budget(message) => budget(Some(k), message),
                }
            }
        })
    }

    async fn preflight_one(
        &mut self,
        c: &mut SqliteConnection,
        k: &DomainNodeKey,
    ) -> Result<(InspectionPayload, DomainNodeKey), PreflightError> {
        if !content::valid_id(&k.id)
            || k.revision < 1
            || k.world_id.as_ref().is_some_and(|w| !content::valid_id(w))
        {
            return Err(PreflightError::Integrity);
        }
        let t = table(k);
        let scope = k.world_id.as_deref().unwrap_or("");
        let head: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT revision FROM {} WHERE id=?1 AND {}=?3",
            t.head, t.scope
        )))
        .bind(&k.id)
        .bind(k.revision)
        .bind(scope)
        .fetch_optional(&mut *c)
        .await
        .map_err(|_| PreflightError::Integrity)?
        .ok_or(PreflightError::Integrity)?;
        if head < k.revision {
            return Err(PreflightError::Integrity);
        }
        let mut current = k.clone();
        current.revision = head;
        let mut projections = vec![];
        for name in std::iter::once(t.versions).chain(t.sidecars.iter().copied()) {
            let sql = format!(
                "SELECT length(CAST(before_json AS BLOB)),length(CAST(after_json AS BLOB)) FROM {name} WHERE {}=?1 AND revision=?2 AND {}=?3 LIMIT 2",
                t.id, t.scope
            );
            let sizes: Vec<(Option<i64>, i64)> = sqlx::query_as(sqlx::AssertSqlSafe(sql))
                .bind(&k.id)
                .bind(k.revision)
                .bind(scope)
                .fetch_all(&mut *c)
                .await
                .map_err(|_| PreflightError::Integrity)?;
            if sizes.len() > 1 || (name == t.versions && sizes.is_empty()) {
                return Err(PreflightError::Integrity);
            }
            for (before, after) in sizes {
                for size in before.into_iter().chain([after]) {
                    let size = usize::try_from(size).map_err(|_| PreflightError::Integrity)?;
                    if size > self.limits.max_snapshot_bytes {
                        return Err(PreflightError::Budget(
                            "A known snapshot exceeds the per-snapshot byte budget.",
                        ));
                    }
                    self.bytes = self.bytes.checked_add(size).ok_or(PreflightError::Budget(
                        "The accounted snapshot byte budget was reached.",
                    ))?;
                    if self.bytes > self.limits.max_total_snapshot_bytes {
                        return Err(PreflightError::Budget(
                            "The accounted snapshot byte budget was reached.",
                        ));
                    }
                }
                let sql = format!(
                    "SELECT after_json FROM {name} WHERE {}=?1 AND revision=?2 AND {}=?3",
                    t.id, t.scope
                );
                let text: String = sqlx::query_scalar(sqlx::AssertSqlSafe(sql))
                    .bind(&k.id)
                    .bind(k.revision)
                    .bind(scope)
                    .fetch_one(&mut *c)
                    .await
                    .map_err(|_| PreflightError::Integrity)?;
                let version =
                    json(&text).map_err(|_| PreflightError::Integrity)?["snapshot_version"]
                        .as_i64()
                        .ok_or(PreflightError::Integrity)?;
                let row = raw_payload(k, &text).map_err(|_| PreflightError::Integrity)?;
                projections.push((version, row));
            }
        }
        for (child, id) in t.children {
            let sql = format!(
                "SELECT count(*) FROM {child} WHERE {id}=?1 AND revision=?2 AND {}=?3",
                t.scope
            );
            let count: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(sql))
                .bind(&k.id)
                .bind(k.revision)
                .bind(scope)
                .fetch_one(&mut *c)
                .await
                .map_err(|_| PreflightError::Integrity)?;
            self.children = self
                .children
                .checked_add(usize::try_from(count).map_err(|_| PreflightError::Integrity)?)
                .ok_or(PreflightError::Budget(
                    "The bounded verification-input child count was reached.",
                ))?;
            if self.children > self.limits.max_edges {
                return Err(PreflightError::Budget(
                    "A known child collection exceeds the bounded verification-input count.",
                ));
            }
            // Some legacy readers resolve typed child pins before comparing the
            // projection. Preflight those concrete pins too, so corruption cannot
            // smuggle a large, unaccounted dependency into a checked read.
            let ref_spec = match *child {
                "form_type_bindings" => Some((DomainKind::Type, "type_id", "type_version", "''")),
                "variant_type_bindings" | "chart_roster" => Some((
                    DomainKind::Type,
                    "type_id",
                    "type_version",
                    "coalesce(world_id,'')",
                )),
                "form_stat_bindings" | "variant_stat_bindings" | "stat_roster_entries" => {
                    Some((DomainKind::Stat, "stat_id", "stat_version", "scope"))
                }
                "move_attribute_bindings" => Some((
                    DomainKind::MoveAttribute,
                    "attribute_id",
                    "attribute_version",
                    "scope",
                )),
                "ruleset_attribute_expectations" => Some((
                    DomainKind::MoveAttribute,
                    "attribute_id",
                    "attribute_version",
                    "world_id",
                )),
                "learnset_records" => Some((DomainKind::Move, "move_id", "move_version", "scope")),
                _ => None,
            };
            if let Some((kind, ref_id, ref_version, ref_scope)) = ref_spec {
                let sql = format!(
                    "SELECT {ref_id},{ref_version},{ref_scope} FROM {child} WHERE {id}=?1 AND revision=?2 AND {}=?3 LIMIT ?4",
                    t.scope
                );
                let refs: Vec<(String, i64, String)> = sqlx::query_as(sqlx::AssertSqlSafe(sql))
                    .bind(&k.id)
                    .bind(k.revision)
                    .bind(scope)
                    .bind(count)
                    .fetch_all(&mut *c)
                    .await
                    .map_err(|_| PreflightError::Integrity)?;
                for (id, version, world) in refs {
                    self.input_refs.entry(k.clone()).or_default().insert(key(
                        kind,
                        (!world.is_empty()).then_some(world.as_str()),
                        &id,
                        version,
                    ));
                }
            }
        }
        let row = projections
            .last()
            .ok_or(PreflightError::Integrity)?
            .1
            .clone();
        for (version, projection) in projections {
            let mut expected = row.clone();
            match &mut expected {
                InspectionPayload::Form(r) => {
                    if version < 3 {
                        r.stats = BaseStats::unmodeled();
                    }
                    if version < 2 {
                        r.typing = TypingState::unmodeled();
                    }
                }
                InspectionPayload::Variant(r) => {
                    if version < 3 {
                        r.stats = BaseStats::unmodeled();
                    }
                    if version < 2 {
                        r.typing = TypingState::unmodeled();
                    }
                }
                _ => {}
            }
            if projection != expected {
                return Err(PreflightError::Integrity);
            }
        }
        Ok((row, current))
    }

    async fn checked(
        &mut self,
        c: &mut SqliteConnection,
        k: &DomainNodeKey,
    ) -> Result<InspectionPayload, Error> {
        if let Some(row) = self.verified.get(k) {
            return row.clone();
        }
        // A dependency/companion rejected by preflight is never read unbounded.
        if self.preflight.get(k).is_none_or(Result::is_err) {
            return Err(Error::Unavailable);
        }
        let row = read_checked(c, k).await;
        if row.is_ok()
            && k.revision > 1
            && matches!(
                k.kind,
                DomainKind::World
                    | DomainKind::Species
                    | DomainKind::Form
                    | DomainKind::Variant
                    | DomainKind::Type
            )
        {
            let mut previous = k.clone();
            previous.revision -= 1;
            let verified = match self.legacy_companions.get(&previous) {
                Some(result) => result.clone(),
                None => {
                    let result = if self.preflight.get(&previous).is_some_and(Result::is_ok) {
                        verify_legacy_companion(c, &previous).await
                    } else {
                        Err(Error::Unavailable)
                    };
                    self.legacy_companions
                        .insert(previous.clone(), result.clone());
                    result
                }
            };
            if verified.is_err() {
                self.result.gaps.push(if Instant::now() >= self.deadline {
                    budget(Some(&previous), "The capture deadline was reached.")
                } else {
                    integrity(&previous)
                });
            }
        }
        self.verified.insert(k.clone(), row.clone());
        row
    }

    fn derived_edges(&mut self) {
        let mut edges = vec![];
        for (k, row) in &self.result.nodes {
            let exact =
                |kind, world: Option<&str>, id: &str, revision, site, edge_kind| InspectionEdge {
                    from: k.clone(),
                    to: DomainRef::Exact(key(kind, world, id, revision)),
                    kind: edge_kind,
                    site,
                };
            match row {
                InspectionPayload::Learnset(r) => {
                    if let Some(v) = &r.variant_id {
                        self_owner_edge(&self.result, k, v, &mut edges);
                    }
                    if let Some(pin) = &r.source {
                        let source_key =
                            key(DomainKind::Learnset, None, &pin.learnset_id, pin.version);
                        if let Some(InspectionPayload::Learnset(source)) =
                            self.result.nodes.get(&source_key)
                        {
                            for (i, m) in r.mappings.iter().enumerate() {
                                if let Some(record) = source
                                    .records
                                    .iter()
                                    .flatten()
                                    .find(|a| a.move_record.id == m.shared_move_id)
                                {
                                    edges.push(exact(
                                        DomainKind::Move,
                                        None,
                                        &record.move_record.id,
                                        record.move_record.revision,
                                        mapping_site(
                                            MappingKind::LearnsetMove,
                                            i,
                                            MappingEndpoint::Source,
                                        ),
                                        EdgeKind::ExactPin,
                                    ));
                                }
                            }
                        }
                    }
                }
                InspectionPayload::Preset(r) => {
                    self_owner_edge(&self.result, k, &r.variant_id, &mut edges);
                    let lk = key(
                        DomainKind::Learnset,
                        Some(&r.world_id),
                        &r.learnset.learnset_id,
                        r.learnset.version,
                    );
                    if let Some(InspectionPayload::Learnset(l)) = self.result.nodes.get(&lk) {
                        for (i, id) in r.entries.iter().enumerate() {
                            if let Some(record) = l.records.iter().flatten().find(|a| &a.id == id) {
                                edges.push(exact(
                                    DomainKind::Move,
                                    Some(&r.world_id),
                                    &record.move_record.id,
                                    record.move_record.revision,
                                    EdgeSite::PresetSelection {
                                        slot: i + 1,
                                        record_id: id.clone(),
                                        learnset: lk.clone(),
                                    },
                                    EdgeKind::Selection,
                                ));
                            }
                        }
                    }
                }
                InspectionPayload::Move(r) => {
                    if let Some(pin) = &r.source
                        && let Some(InspectionPayload::Move(source)) = self.result.nodes.get(&key(
                            DomainKind::Move,
                            None,
                            &pin.move_id,
                            pin.version,
                        ))
                    {
                        for (i, m) in r.mappings.iter().enumerate() {
                            if let Some(a) = source
                                .attributes
                                .iter()
                                .flatten()
                                .find(|a| a.definition.id == m.shared_attribute_id)
                            {
                                edges.push(exact(
                                    DomainKind::MoveAttribute,
                                    None,
                                    &a.definition.id,
                                    a.definition.revision,
                                    mapping_site(
                                        MappingKind::MoveAttribute,
                                        i,
                                        MappingEndpoint::Source,
                                    ),
                                    EdgeKind::ExactPin,
                                ));
                            }
                        }
                    }
                }
                InspectionPayload::Chart(r) => {
                    if let Some(pin) = &r.source
                        && let Some(InspectionPayload::Chart(source)) = self.result.nodes.get(&key(
                            DomainKind::Chart,
                            None,
                            &pin.chart_id,
                            pin.version,
                        ))
                    {
                        for (i, m) in r.mapping.iter().enumerate() {
                            if let Some(a) = source
                                .roster
                                .iter()
                                .find(|a| a.pin.type_id == m.source_type_id)
                            {
                                edges.push(exact(
                                    DomainKind::Type,
                                    None,
                                    &a.pin.type_id,
                                    a.pin.version,
                                    mapping_site(
                                        MappingKind::ChartType,
                                        i,
                                        MappingEndpoint::Source,
                                    ),
                                    EdgeKind::ExactPin,
                                ));
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        for edge in edges {
            self.add_edge(edge);
        }
    }
}

enum PreflightError {
    Integrity,
    Budget(&'static str),
}
fn mapping_site(kind: MappingKind, index: usize, endpoint: MappingEndpoint) -> EdgeSite {
    EdgeSite::Mapping {
        kind,
        slot: index + 1,
        endpoint,
    }
}
fn self_owner_edge(
    result: &CapturedWorld,
    child: &DomainNodeKey,
    id: &str,
    edges: &mut Vec<InspectionEdge>,
) {
    // Only current owner inventories imply ownership observations. A historical
    // learnset dependency does not become a current variant child.
    if result.roots.contains_key(child)
        && let Some(owner) = result
            .roots
            .keys()
            .find(|k| k.kind == DomainKind::Variant && k.id == id && k.world_id == child.world_id)
    {
        edges.push(InspectionEdge {
            from: owner.clone(),
            to: DomainRef::Exact(child.clone()),
            kind: EdgeKind::Ownership,
            site: EdgeSite::Owner,
        });
    }
}

fn basic_edges(k: &DomainNodeKey, row: &InspectionPayload) -> Vec<InspectionEdge> {
    use DomainKind as K;
    use InspectionPayload as P;
    let mut edges = vec![];
    let mut add = |kind, world: Option<&str>, id: &str, revision, site, relation| {
        edges.push(InspectionEdge {
            from: k.clone(),
            to: DomainRef::Exact(key(kind, world, id, revision)),
            kind: relation,
            site,
        })
    };
    let w = k.world_id.as_deref();
    let (typing, base_stats) = match row {
        P::Form(r) => {
            add(
                K::Species,
                None,
                &r.species_id,
                r.species_version,
                EdgeSite::Species,
                EdgeKind::ExactPin,
            );
            (Some(&r.typing), Some(&r.stats))
        }
        P::Variant(r) => {
            if let Some(p) = &r.source {
                add(
                    K::Form,
                    None,
                    &p.form_id,
                    p.version,
                    EdgeSite::Source,
                    EdgeKind::Provenance,
                );
            }
            (Some(&r.typing), Some(&r.stats))
        }
        _ => (None, None),
    };
    if let Some(typing) = typing {
        for (i, a) in typing.assignments.iter().flatten().enumerate() {
            add(
                K::Type,
                w,
                &a.pin.type_id,
                a.pin.version,
                EdgeSite::Typing { slot: i + 1 },
                EdgeKind::ExactPin,
            );
        }
    }
    if let Some(stats) = base_stats {
        if let Some(p) = &stats.roster {
            add(
                K::StatRoster,
                w,
                &p.roster_id,
                p.version,
                EdgeSite::StatRoster,
                EdgeKind::ExactPin,
            );
        }
        for (i, a) in stats.assignments.iter().flatten().enumerate() {
            add(
                K::Stat,
                w,
                &a.definition.id,
                a.definition.revision,
                EdgeSite::StatValue { slot: i + 1 },
                EdgeKind::ExactPin,
            );
        }
    }
    match row {
        P::WorldType(r) => {
            if let Some(p) = &r.source {
                add(
                    K::Type,
                    None,
                    &p.type_id,
                    p.version,
                    EdgeSite::Source,
                    EdgeKind::Provenance,
                );
            }
        }
        P::Stat(r) => {
            if let Some(p) = &r.source {
                add(
                    K::Stat,
                    None,
                    &p.stat_id,
                    p.version,
                    EdgeSite::Source,
                    EdgeKind::Provenance,
                );
            }
        }
        P::MoveAttribute(r) => {
            if let Some(p) = &r.source {
                add(
                    K::MoveAttribute,
                    None,
                    &p.attribute_id,
                    p.version,
                    EdgeSite::Source,
                    EdgeKind::Provenance,
                );
            }
        }
        P::StatRoster(r) => {
            for (i, a) in r.entries.iter().enumerate() {
                add(
                    K::Stat,
                    w,
                    &a.id,
                    a.revision,
                    EdgeSite::RosterEntry { slot: i + 1 },
                    EdgeKind::ExactPin,
                );
            }
        }
        P::Move(r) => {
            if let Some(p) = &r.source {
                add(
                    K::Move,
                    None,
                    &p.move_id,
                    p.version,
                    EdgeSite::Source,
                    EdgeKind::Provenance,
                );
            }
            for (i, a) in r.attributes.iter().flatten().enumerate() {
                add(
                    K::MoveAttribute,
                    w,
                    &a.definition.id,
                    a.definition.revision,
                    EdgeSite::Attribute { slot: i + 1 },
                    EdgeKind::ExactPin,
                );
            }
            for (i, m) in r.mappings.iter().enumerate() {
                add(
                    K::MoveAttribute,
                    w,
                    &m.local_attribute.attribute_id,
                    m.local_attribute.version,
                    mapping_site(MappingKind::MoveAttribute, i, MappingEndpoint::Local),
                    EdgeKind::ExactPin,
                );
            }
        }
        P::Chart(r) => {
            if let Some(p) = &r.source {
                add(
                    K::Chart,
                    None,
                    &p.chart_id,
                    p.version,
                    EdgeSite::Source,
                    EdgeKind::Provenance,
                );
            }
            for (i, a) in r.roster.iter().enumerate() {
                add(
                    K::Type,
                    w,
                    &a.pin.type_id,
                    a.pin.version,
                    EdgeSite::ChartRoster { slot: i + 1 },
                    EdgeKind::ExactPin,
                );
            }
            for (i, m) in r.mapping.iter().enumerate() {
                if let Some(a) = r.roster.iter().find(|a| a.pin.type_id == m.local_type_id) {
                    add(
                        K::Type,
                        w,
                        &a.pin.type_id,
                        a.pin.version,
                        mapping_site(MappingKind::ChartType, i, MappingEndpoint::Local),
                        EdgeKind::ExactPin,
                    );
                }
            }
        }
        P::Ruleset(r) => {
            if let Some(p) = &r.stat_roster {
                add(
                    K::StatRoster,
                    w,
                    &p.roster_id,
                    p.version,
                    EdgeSite::StatRoster,
                    EdgeKind::ExactPin,
                );
            }
            if let Some(p) = &r.chart {
                add(
                    K::Chart,
                    w,
                    &p.chart_id,
                    p.version,
                    EdgeSite::Source,
                    EdgeKind::ExactPin,
                );
            }
            for (i, a) in r.expectations.iter().flatten().enumerate() {
                add(
                    K::MoveAttribute,
                    w,
                    &a.definition.id,
                    a.definition.revision,
                    EdgeSite::Expectation { slot: i + 1 },
                    EdgeKind::ExactPin,
                );
            }
        }
        P::Learnset(r) => {
            if let Some(p) = &r.source {
                add(
                    K::Learnset,
                    None,
                    &p.learnset_id,
                    p.version,
                    EdgeSite::Source,
                    EdgeKind::Provenance,
                );
            }
            for (i, a) in r.records.iter().flatten().enumerate() {
                add(
                    K::Move,
                    w,
                    &a.move_record.id,
                    a.move_record.revision,
                    EdgeSite::Acquisition {
                        slot: i + 1,
                        record_id: a.id.clone(),
                    },
                    EdgeKind::ExactPin,
                );
            }
            for (i, m) in r.mappings.iter().enumerate() {
                add(
                    K::Move,
                    w,
                    &m.local_move.move_id,
                    m.local_move.version,
                    mapping_site(MappingKind::LearnsetMove, i, MappingEndpoint::Local),
                    EdgeKind::ExactPin,
                );
            }
        }
        P::Preset(r) => {
            add(
                K::Learnset,
                w,
                &r.learnset.learnset_id,
                r.learnset.version,
                EdgeSite::PresetLearnset,
                EdgeKind::ExactPin,
            );
            add(
                K::Ruleset,
                w,
                &r.ruleset.ruleset_id,
                r.ruleset.version,
                EdgeSite::PresetRuleset,
                EdgeKind::ExactPin,
            );
        }
        _ => {}
    }
    let backlink = match row {
        P::Species(r) => Some((
            K::Form,
            None,
            r.default_form_id.as_str(),
            EdgeSite::DefaultForm,
        )),
        P::Learnset(r) => r
            .form_id
            .as_deref()
            .map(|id| (K::Form, None, id, EdgeSite::Owner))
            .or_else(|| {
                r.variant_id
                    .as_deref()
                    .map(|id| (K::Variant, w, id, EdgeSite::Owner))
            }),
        P::Preset(r) => Some((K::Variant, w, r.variant_id.as_str(), EdgeSite::Owner)),
        _ => None,
    };
    if let Some((kind, world, id, site)) = backlink {
        edges.push(InspectionEdge {
            from: k.clone(),
            to: DomainRef::Identity(DomainIdentity {
                kind,
                world_id: world.map(str::to_owned),
                id: id.into(),
            }),
            kind: EdgeKind::Ownership,
            site,
        });
    }
    edges
}

// These values are private preflight plans only. Checked readers are still the
// authority; this decoder never substitutes malformed data with Unknown.
fn raw_payload(k: &DomainNodeKey, text: &str) -> Result<InspectionPayload, Error> {
    use DomainKind as K;
    use InspectionPayload as P;
    let v = json(text)?;
    let mut body = v.get("content").cloned().ok_or(Error::Unavailable)?;
    let version = v["snapshot_version"].as_i64().ok_or(Error::Unavailable)?;
    if !(1..=3).contains(&version) {
        return Err(Error::Unavailable);
    }
    let row = match k.kind {
        K::World => {
            let b = body.as_object_mut().ok_or(Error::Unavailable)?;
            b.remove("variant_ids");
            b.remove("type_ids");
            P::World(decode(&body)?)
        }
        K::Species => P::Species(decode(&body)?),
        K::Form => {
            if version == 1 {
                body["typing"] = serde_json::to_value(TypingState::unmodeled())
                    .map_err(|_| Error::Unavailable)?;
            }
            if version < 3 {
                body["stats"] =
                    serde_json::to_value(BaseStats::unmodeled()).map_err(|_| Error::Unavailable)?;
            }
            P::Form(decode(&body)?)
        }
        K::Variant => {
            if version == 1 {
                body["typing"] = serde_json::to_value(TypingState::unmodeled())
                    .map_err(|_| Error::Unavailable)?;
            }
            if version < 3 {
                body["stats"] =
                    serde_json::to_value(BaseStats::unmodeled()).map_err(|_| Error::Unavailable)?;
            }
            P::Variant(decode(&body)?)
        }
        K::Type if k.world_id.is_some() => P::WorldType(decode(&body)?),
        K::Type => P::Type(decode(&body)?),
        K::Stat => {
            let r: game_data::stats::StatDefinition = decode(&body)?;
            r.validate().map_err(|_| Error::Unavailable)?;
            P::Stat(r)
        }
        K::StatRoster => {
            let r: game_data::stats::StatRoster = decode(&body)?;
            r.validate().map_err(|_| Error::Unavailable)?;
            P::StatRoster(r)
        }
        K::MoveAttribute => {
            let r: game_data::moves::MoveAttributeDefinition = decode(&body)?;
            r.validate().map_err(|_| Error::Unavailable)?;
            P::MoveAttribute(r)
        }
        K::Move => {
            let r: game_data::moves::MoveRecord = decode(&body)?;
            r.validate().map_err(|_| Error::Unavailable)?;
            P::Move(r)
        }
        K::Chart => {
            let r: game_data::charts::ChartRecord = decode(&body)?;
            r.validate().map_err(|_| Error::Unavailable)?;
            P::Chart(r)
        }
        K::Ruleset => {
            let r: game_data::rulesets::RulesetRecord = decode(&body)?;
            r.validate().map_err(|_| Error::Unavailable)?;
            P::Ruleset(r)
        }
        K::Learnset => {
            let r: game_data::learning::LearnsetRecord = decode(&body)?;
            r.validate().map_err(|_| Error::Unavailable)?;
            P::Learnset(r)
        }
        K::Preset => {
            let r: game_data::learning::MovesetRecord = decode(&body)?;
            r.validate().map_err(|_| Error::Unavailable)?;
            P::Preset(r)
        }
    };
    // Scope agreement must precede checked-reader recursion. In particular a
    // corrupt shared snapshot must not masquerade as a local object with a
    // recursive shared-source pin and defeat the readers' finite DAG direction.
    let identity = match &row {
        P::World(r) => key(K::World, None, &r.id, r.revision),
        P::Species(r) => key(K::Species, None, &r.id, r.revision),
        P::Form(r) => key(K::Form, None, &r.id, r.revision),
        P::Variant(r) => key(K::Variant, Some(&r.world_id), &r.id, r.revision),
        P::Type(r) => key(K::Type, None, &r.id, r.revision),
        P::WorldType(r) => key(K::Type, Some(&r.world_id), &r.id, r.revision),
        P::Stat(r) => key(K::Stat, r.world_id.as_deref(), &r.id, r.revision),
        P::StatRoster(r) => key(K::StatRoster, Some(&r.world_id), &r.id, r.revision),
        P::MoveAttribute(r) => key(K::MoveAttribute, r.world_id.as_deref(), &r.id, r.revision),
        P::Move(r) => key(K::Move, r.world_id.as_deref(), &r.id, r.revision),
        P::Chart(r) => key(K::Chart, r.world_id.as_deref(), &r.id, r.revision),
        P::Ruleset(r) => key(K::Ruleset, Some(&r.world_id), &r.id, r.revision),
        P::Learnset(r) => key(K::Learnset, r.world_id.as_deref(), &r.id, r.revision),
        P::Preset(r) => key(K::Preset, Some(&r.world_id), &r.id, r.revision),
    };
    if identity != *k
        || row
            .fields()
            .is_some_and(|f| f.normalized().ok().as_ref() != Some(f))
    {
        return Err(Error::Unavailable);
    }
    Ok(row)
}

async fn read_checked(
    c: &mut SqliteConnection,
    k: &DomainNodeKey,
) -> Result<InspectionPayload, Error> {
    use DomainKind as K;
    use InspectionPayload as P;
    let w = k.world_id.as_deref();
    let world = || w.ok_or(Error::Unavailable);
    match k.kind {
        K::World | K::Species | K::Form | K::Variant | K::Type => legacy_checked(c, k).await,
        K::Stat => Ok(P::Stat(
            stats::definition_version(c, w, &k.id, k.revision).await?,
        )),
        K::StatRoster => Ok(P::StatRoster(
            stats::roster_version(c, world()?, &k.id, k.revision).await?,
        )),
        K::MoveAttribute => Ok(P::MoveAttribute(
            moves::attribute_version(c, w, &k.id, k.revision).await?,
        )),
        K::Move => Ok(P::Move(moves::move_version(c, w, &k.id, k.revision).await?)),
        K::Chart => Ok(P::Chart(charts::version(c, w, &k.id, k.revision).await?)),
        K::Ruleset => Ok(P::Ruleset(
            moves::ruleset_version(c, world()?, &k.id, k.revision).await?,
        )),
        K::Learnset => Ok(P::Learnset(
            learning::learnset_version(c, w, &k.id, k.revision).await?,
        )),
        K::Preset => {
            let owner: String = sqlx::query_scalar(
                "SELECT variant_id FROM moveset_presets WHERE world_id=? AND id=?",
            )
            .bind(world()?)
            .bind(&k.id)
            .fetch_optional(&mut *c)
            .await
            .map_err(content::storage_error)?
            .ok_or(Error::Unavailable)?;
            Ok(P::Preset(
                learning::preset_version(c, world()?, &owner, &k.id, k.revision).await?,
            ))
        }
    }
}

/// Narrow adapters add the checks that older metadata readers do not promise.
/// They deliberately compare a historical snapshot only with its typed exact
/// projection; only current snapshots are compared with mutable head metadata.
async fn legacy_checked(
    c: &mut SqliteConnection,
    k: &DomainNodeKey,
) -> Result<InspectionPayload, Error> {
    use DomainKind as K;
    use InspectionPayload as P;
    let t = table(k);
    let scope = k.world_id.as_deref().unwrap_or("");
    let head: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT revision FROM {} WHERE id=?1 AND {}=?3",
        t.head, t.scope
    )))
    .bind(&k.id)
    .bind(k.revision)
    .bind(scope)
    .fetch_optional(&mut *c)
    .await
    .map_err(content::storage_error)?
    .ok_or(Error::Unavailable)?;
    if head < k.revision {
        return Err(Error::Unavailable);
    }
    let row = match k.kind {
        K::World => {
            let full = types::full_content(c, HistoryKind::World, &k.id, None, k.revision).await?;
            let value = json(&full.after)?;
            let projection = json(&full.projection_after)?;
            verify_legacy_snapshot(c, k, HistoryKind::World, None, &projection["content"]).await?;
            let p = raw_payload(k, &full.after)?;
            if head == k.revision {
                let current = content::world(c, &k.id).await?;
                let expected = PublicWorldContent {
                    id: current.id,
                    fields: current.fields,
                    revision: current.revision,
                    archived_at: current.archived_at,
                };
                if p != P::World(expected) {
                    return Err(Error::Unavailable);
                }
                for (field, table) in [("variant_ids", "variants"), ("type_ids", "world_types")] {
                    if field == "type_ids" && value["snapshot_version"] == 1 {
                        continue;
                    }
                    let ids: Vec<String> = decode(&value["content"][field])?;
                    let count: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                        "SELECT count(*) FROM {table} WHERE world_id=?"
                    )))
                    .bind(&k.id)
                    .fetch_one(&mut *c)
                    .await
                    .map_err(content::storage_error)?;
                    // full_content already verifies every listed member exists;
                    // exact cardinality proves there are no omitted members.
                    if usize::try_from(count).ok() != Some(ids.len()) {
                        return Err(Error::Unavailable);
                    }
                }
            }
            p
        }
        K::Species => {
            let r = content::species_version(c, &k.id, k.revision).await?;
            let p = PublicSpeciesContent {
                id: r.id.clone(),
                fields: r.fields.clone(),
                revision: r.revision,
                default_form_id: r.default_form_id.clone(),
                archived_at: r.archived_at,
            };
            verify_legacy_snapshot(c, k, HistoryKind::Species, Some(&r.default_form_id), &p)
                .await?;
            if head == k.revision && content::species(c, &k.id).await? != r {
                return Err(Error::Unavailable);
            }
            let (parent, is_default): (String, bool) =
                sqlx::query_as("SELECT species_id,is_default FROM forms WHERE id=?")
                    .bind(&r.default_form_id)
                    .fetch_optional(&mut *c)
                    .await
                    .map_err(content::storage_error)?
                    .ok_or(Error::Unavailable)?;
            if parent != k.id || !is_default {
                return Err(Error::Unavailable);
            }
            P::Species(p)
        }
        K::Form => {
            let r = content::form_version(c, &k.id, k.revision).await?;
            let current = content::form(c, &k.id).await?;
            if current.species_id != r.species_id || head == k.revision && current != r {
                return Err(Error::Unavailable);
            }
            let parent = content::species_version(c, &r.species_id, r.species_version).await?;
            if r.is_default != (parent.default_form_id == r.id) {
                return Err(Error::Unavailable);
            }
            let full = types::full_content(c, HistoryKind::Form, &k.id, None, k.revision).await?;
            verify_legacy_snapshot(c, k, HistoryKind::Form, Some(&r.species_id), &r).await?;
            P::Form(PublicFormContent {
                id: r.id,
                species_id: r.species_id,
                species_version: r.species_version,
                fields: r.fields,
                revision: r.revision,
                is_default: r.is_default,
                archived_at: r.archived_at,
                typing: full.typing,
                stats: full.stats,
            })
        }
        K::Variant => {
            let w = k.world_id.as_deref().ok_or(Error::Unavailable)?;
            let full =
                types::full_content(c, HistoryKind::Variant, &k.id, Some(w), k.revision).await?;
            let value = json(&full.projection_after)?;
            let r: crate::application::authoring::VariantRecord = decode(&value["content"])?;
            if r.world_id != w {
                return Err(Error::Unavailable);
            }
            let current = content::variant(c, w, &k.id).await?;
            if head == k.revision && current != r {
                return Err(Error::Unavailable);
            }
            verify_legacy_snapshot(c, k, HistoryKind::Variant, Some(w), &r).await?;
            P::Variant(PublicVariantContent {
                id: r.id,
                world_id: r.world_id,
                source: r.source.map(|p| InspectionFormPin {
                    form_id: p.form_id,
                    version: p.version,
                }),
                fields: r.fields,
                revision: r.revision,
                archived_at: r.archived_at,
                typing: full.typing,
                stats: full.stats,
            })
        }
        K::Type if k.world_id.is_some() => {
            let w = k.world_id.as_deref().ok_or(Error::Unavailable)?;
            let r = types::world_type_version(c, w, &k.id, k.revision).await?;
            verify_legacy_snapshot(c, k, HistoryKind::WorldType, Some(w), &r).await?;
            if head == k.revision && types::world_type_detail(c, w, &k.id).await? != r {
                return Err(Error::Unavailable);
            }
            P::WorldType(r)
        }
        K::Type => {
            let r = types::type_version(c, &k.id, k.revision).await?;
            verify_legacy_snapshot(c, k, HistoryKind::Type, None, &r).await?;
            if head == k.revision && types::type_detail(c, &k.id).await? != r {
                return Err(Error::Unavailable);
            }
            P::Type(r)
        }
        _ => return Err(Error::Unavailable),
    };
    Ok(row)
}

async fn verify_legacy_snapshot(
    c: &mut SqliteConnection,
    k: &DomainNodeKey,
    kind: HistoryKind,
    parent: Option<&str>,
    expected: &impl Serialize,
) -> Result<(), Error> {
    let t = table(k);
    let sql = format!(
        "SELECT v.before_json,v.after_json,v.previous_revision,EXISTS(SELECT 1 FROM audit_events a WHERE a.id=v.audit_id) AS audit_present FROM {} v WHERE {}=?1 AND revision=?2 AND {}=?3",
        t.versions, t.id, t.scope
    );
    let r = sqlx::query(sqlx::AssertSqlSafe(sql))
        .bind(&k.id)
        .bind(k.revision)
        .bind(k.world_id.as_deref().unwrap_or(""))
        .fetch_optional(&mut *c)
        .await
        .map_err(content::storage_error)?
        .ok_or(Error::Unavailable)?;
    let after: String = r.try_get("after_json").map_err(content::storage_error)?;
    let before: Option<String> = r.try_get("before_json").map_err(content::storage_error)?;
    let previous: Option<i64> = r
        .try_get("previous_revision")
        .map_err(content::storage_error)?;
    let audit: bool = r.try_get("audit_present").map_err(content::storage_error)?;
    if !audit || previous != content::previous(k.revision) || before.is_some() != (k.revision > 1) {
        return Err(Error::Unavailable);
    }
    content::validate_snapshot(
        &after,
        SnapshotKind::Public(kind),
        &k.id,
        parent,
        k.revision,
    )?;
    if json(&after)?["content"] != serde_json::to_value(expected).map_err(|_| Error::Unavailable)? {
        return Err(Error::Unavailable);
    }
    if let Some(before) = before {
        content::validate_snapshot(
            &before,
            SnapshotKind::Public(kind),
            &k.id,
            parent,
            k.revision - 1,
        )?;
        let sql = format!(
            "SELECT after_json FROM {} WHERE {}=?1 AND revision=?2 AND {}=?3",
            t.versions, t.id, t.scope
        );
        let prior: String = sqlx::query_scalar(sqlx::AssertSqlSafe(sql))
            .bind(&k.id)
            .bind(k.revision - 1)
            .bind(k.world_id.as_deref().unwrap_or(""))
            .fetch_optional(&mut *c)
            .await
            .map_err(content::storage_error)?
            .ok_or(Error::Unavailable)?;
        if before != prior {
            return Err(Error::Unavailable);
        }
    }
    Ok(())
}

/// Verify this already-preflighted companion's *own* typed projection and
/// required revision header. Deliberately no predecessor lookup or full_content
/// call: the requested/current reader checked continuity to these bytes, and
/// following this companion's predecessor would enumerate history recursively.
async fn verify_legacy_companion(c: &mut SqliteConnection, k: &DomainNodeKey) -> Result<(), Error> {
    use DomainKind as K;
    let (kind, parent, expected) = match k.kind {
        K::Species => {
            let row = content::species_version(c, &k.id, k.revision).await?;
            (
                HistoryKind::Species,
                Some(row.default_form_id.clone()),
                serde_json::to_value(row),
            )
        }
        K::Form => {
            let row = content::form_version_projection(c, &k.id, k.revision).await?;
            (
                HistoryKind::Form,
                Some(row.species_id.clone()),
                serde_json::to_value(row),
            )
        }
        K::Type if k.world_id.is_some() => {
            let row = types::world_type_version(
                c,
                k.world_id.as_deref().ok_or(Error::Unavailable)?,
                &k.id,
                k.revision,
            )
            .await?;
            (
                HistoryKind::WorldType,
                k.world_id.clone(),
                serde_json::to_value(row),
            )
        }
        K::Type => {
            let row = types::type_version(c, &k.id, k.revision).await?;
            (HistoryKind::Type, None, serde_json::to_value(row))
        }
        K::World | K::Variant => {
            // These revision tables have no separate immutable metadata columns;
            // full_content on the requested revision checks their v1/v2/v3 seals.
            let t = table(k);
            let text: String = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                "SELECT after_json FROM {} WHERE {}=?1 AND revision=?2 AND {}=?3",
                t.versions, t.id, t.scope
            )))
            .bind(&k.id)
            .bind(k.revision)
            .bind(k.world_id.as_deref().unwrap_or(""))
            .fetch_one(&mut *c)
            .await
            .map_err(content::storage_error)?;
            let kind = if k.kind == K::World {
                HistoryKind::World
            } else {
                HistoryKind::Variant
            };
            (
                kind,
                k.world_id.clone(),
                Ok(json(&text)?["content"].clone()),
            )
        }
        _ => return Err(Error::Unavailable),
    };
    let expected = expected.map_err(|_| Error::Unavailable)?;
    let t = table(k);
    let r = sqlx::query(sqlx::AssertSqlSafe(format!("SELECT before_json,after_json,previous_revision,EXISTS(SELECT 1 FROM audit_events a WHERE a.id=v.audit_id) AS audit_present FROM {} v WHERE {}=?1 AND revision=?2 AND {}=?3", t.versions, t.id, t.scope)))
        .bind(&k.id).bind(k.revision).bind(k.world_id.as_deref().unwrap_or(""))
        .fetch_one(&mut *c).await.map_err(content::storage_error)?;
    let before: Option<String> = r.try_get("before_json").map_err(content::storage_error)?;
    let after: String = r.try_get("after_json").map_err(content::storage_error)?;
    let previous: Option<i64> = r
        .try_get("previous_revision")
        .map_err(content::storage_error)?;
    let audit: bool = r.try_get("audit_present").map_err(content::storage_error)?;
    if !audit || previous != content::previous(k.revision) || before.is_some() != (k.revision > 1) {
        return Err(Error::Unavailable);
    }
    content::validate_snapshot(
        &after,
        SnapshotKind::Public(kind),
        &k.id,
        parent.as_deref(),
        k.revision,
    )?;
    if json(&after)?["content"] != expected {
        return Err(Error::Unavailable);
    }
    if let Some(before) = before {
        content::validate_snapshot(
            &before,
            SnapshotKind::Public(kind),
            &k.id,
            parent.as_deref(),
            k.revision - 1,
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        accounts::Accounts,
        application::{
            authoring::Authoring,
            learning::{Learning, LearningRecordInput, LearnsetEdit, MovesetEdit},
            moves::{Moves, NewMove, RulesetEdit},
        },
        auth::{Role, Subject},
    };
    use game_data::{
        authoring::{ContentFields, slug_from_name},
        moves::NumericValue,
        rulesets::RepeatPolicy,
    };

    fn fields(name: &str) -> ContentFields {
        ContentFields {
            slug: slug_from_name(name),
            display_name: name.into(),
            description: "Synchronized WAL inspection test.".into(),
        }
    }

    async fn package_user(accounts: &Accounts, name: &str, role: Role) -> (String, String) {
        accounts.invite("test", name, role, None).await.unwrap();
        let user = accounts
            .sign_in(Subject {
                provider: "test".into(),
                provider_subject: name.into(),
                display_name: name.into(),
            })
            .await
            .unwrap();
        let (token, _) = accounts.start_session(&user.user_id).await.unwrap();
        (user.user_id, token)
    }

    #[tokio::test]
    async fn package_first_read_pins_content_session_and_collaborator_permission() {
        let dir = tempfile::tempdir().unwrap();
        let pool = super::super::connect_for_migration(&format!(
            "sqlite://{}/package-race.db",
            dir.path().display()
        ))
        .await
        .unwrap();
        super::super::migrate(&pool).await.unwrap();
        let accounts = Accounts::new(pool.clone(), 3600).unwrap();
        let (_, owner) = package_user(&accounts, "owner", Role::Administrator).await;
        let (designer_id, designer) = package_user(&accounts, "designer", Role::Editor).await;
        let (_, outsider) = package_user(&accounts, "outsider", Role::Administrator).await;
        let author = Authoring::new(accounts.clone());
        let world = author
            .create_world(&owner, fields("Package race"), None)
            .await
            .unwrap();
        author
            .grant_collaborator(&owner, &world.id, 1, &designer_id, None)
            .await
            .unwrap();
        let deadline = || Instant::now() + Duration::from_secs(10);
        let baseline = capture_pooled_package(
            pool.acquire().await.unwrap(),
            AddressKey::Id(world.id.clone()),
            InspectionLimits::default(),
            deadline(),
            &accounts,
            &designer,
        )
        .await
        .unwrap();
        assert_eq!(baseline.roots, vec![baseline.world.clone()]);
        assert_eq!(
            capture_pooled_package(
                pool.acquire().await.unwrap(),
                AddressKey::Id(world.id.clone()),
                InspectionLimits::default(),
                deadline(),
                &accounts,
                &outsider,
            )
            .await,
            Err(Error::Forbidden),
            "an unrelated administrator has no export bypass"
        );
        let (started, first_read) = tokio::sync::oneshot::channel();
        let (committed, writer_done) = tokio::sync::oneshot::channel();
        let mut lease = pool.acquire().await.unwrap();
        sqlx::query("PRAGMA query_only=ON")
            .execute(&mut *lease)
            .await
            .unwrap();
        let reader = capture_package_until(
            lease,
            AddressKey::Id(world.id.clone()),
            InspectionLimits::default(),
            deadline(),
            &accounts,
            &designer,
            || async move {
                started.send(()).unwrap();
                writer_done.await.unwrap();
            },
        );
        let writer = async {
            first_read.await.unwrap();
            author
                .revoke_collaborator(&owner, &world.id, 2, &designer_id, None)
                .await
                .unwrap();
            Moves::new(accounts.clone())
                .create(
                    &owner,
                    Some(&world.id),
                    NewMove {
                        fields: fields("Committed after anchor"),
                        source: None,
                        attributes: None,
                        reason: None,
                    },
                )
                .await
                .unwrap();
            // Both eligibility and the collaborator query must still see the
            // snapshot established by the world read, not either new state.
            accounts.sign_out(&designer).await.unwrap();
            committed.send(()).unwrap();
        };
        let (captured, ()) = tokio::join!(reader, writer);
        assert_eq!(captured.unwrap(), baseline);
        assert_eq!(
            capture_pooled_package(
                pool.acquire().await.unwrap(),
                AddressKey::Id(world.id.clone()),
                InspectionLimits::default(),
                deadline(),
                &accounts,
                &designer,
            )
            .await,
            Err(Error::Unauthorized)
        );
        let (new_designer, _) = accounts.start_session(&designer_id).await.unwrap();
        assert_eq!(
            capture_pooled_package(
                pool.acquire().await.unwrap(),
                AddressKey::Id(world.id.clone()),
                InspectionLimits::default(),
                deadline(),
                &accounts,
                &new_designer,
            )
            .await,
            Err(Error::Forbidden)
        );
        let after = capture_pooled_package(
            pool.acquire().await.unwrap(),
            AddressKey::Id(world.id.clone()),
            InspectionLimits::default(),
            deadline(),
            &accounts,
            &owner,
        )
        .await
        .unwrap();
        assert_eq!(
            after.world, baseline.world,
            "independent Move does not revise the world"
        );
        assert_eq!(after.roots.len(), baseline.roots.len() + 1);
        game_data::package::Package::build(after).unwrap();
        let tiny = InspectionLimits {
            max_roots: 1,
            ..InspectionLimits::default()
        };
        assert_eq!(
            capture_pooled_package(
                pool.acquire().await.unwrap(),
                AddressKey::Id(world.id),
                tiny,
                deadline(),
                &accounts,
                &owner,
            )
            .await,
            Err(Error::Unavailable),
            "a budget gap is never a diagnostic draft or truncated package"
        );
    }

    #[tokio::test]
    async fn cancelled_package_capture_discards_lease_and_does_not_poison_next_reader() {
        let dir = tempfile::tempdir().unwrap();
        let url = format!("sqlite://{}/package-cancel.db", dir.path().display());
        let setup = super::super::connect_for_migration(&url).await.unwrap();
        super::super::migrate(&setup).await.unwrap();
        setup.close().await;
        // A single slot makes reuse/discard observable: TEMP state would survive
        // if the cancelled capture's connection escaped back into the pool.
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await
            .unwrap();
        let accounts = Accounts::new(pool.clone(), 3600).unwrap();
        let (_, token) = package_user(&accounts, "owner", Role::Administrator).await;
        let world = Authoring::new(accounts.clone())
            .create_world(&token, fields("Cancelled package"), None)
            .await
            .unwrap();
        let mut c = pool.acquire().await.unwrap();
        sqlx::query("CREATE TEMP TABLE cancelled_package_lease(id INTEGER)")
            .execute(&mut *c)
            .await
            .unwrap();
        let (started, first_read) = tokio::sync::oneshot::channel();
        let mut capture = Box::pin(capture_package_until(
            c,
            AddressKey::Id(world.id.clone()),
            InspectionLimits::default(),
            Instant::now() + Duration::from_secs(10),
            &accounts,
            &token,
            || async move {
                started.send(()).unwrap();
                std::future::pending::<()>().await;
            },
        ));
        tokio::select! {
            result = &mut capture => panic!("capture unexpectedly finished: {result:?}"),
            result = first_read => result.unwrap(),
        }
        // Drop the owned future itself, not just a Pin<&mut _> wrapper.
        drop(capture);
        let mut next = tokio::time::timeout(Duration::from_secs(5), pool.acquire())
            .await
            .unwrap()
            .unwrap();
        assert!(!next.is_in_transaction());
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM sqlite_temp_master WHERE name='cancelled_package_lease'"
            )
            .fetch_one(&mut *next)
            .await
            .unwrap(),
            0,
            "the cancelled owned connection was closed"
        );
        assert_eq!(sqlx::query_scalar::<_, i64>("WITH RECURSIVE n(x) AS (VALUES(0) UNION ALL SELECT x+1 FROM n WHERE x<10000) SELECT max(x) FROM n").fetch_one(&mut *next).await.unwrap(), 10000);
        drop(next);
        let captured = capture_pooled_package(
            pool.acquire().await.unwrap(),
            AddressKey::Id(world.id),
            InspectionLimits::default(),
            Instant::now() + Duration::from_secs(10),
            &accounts,
            &token,
        )
        .await
        .unwrap();
        assert_eq!(captured.roots, vec![captured.world.clone()]);
    }

    #[tokio::test]
    async fn sqlite_vm_reports_interrupt_and_handler_is_removed_before_rollback() {
        let mut c = SqliteConnection::connect("sqlite::memory:").await.unwrap();
        let guard =
            SqliteDeadline::install(&mut c, Instant::now() + Duration::from_millis(30), false)
                .await
                .unwrap();
        let mut tx = c.begin().await.unwrap();
        // Await SQLite's actual error, without a capture Tokio timeout dropping
        // the query. This proves the worker VM, rather than just its receiver,
        // was cooperatively interrupted by the installed progress callback.
        let error = tokio::time::timeout(Duration::from_secs(5), sqlx::query_scalar::<_, i64>("WITH RECURSIVE n(x) AS (VALUES(0) UNION ALL SELECT x+1 FROM n WHERE x<1000000000) SELECT max(x) FROM n").fetch_one(&mut *tx)).await.unwrap().unwrap_err();
        assert_eq!(
            error.as_database_error().and_then(|e| e.code()).as_deref(),
            Some("9")
        );
        assert!(guard.state.interrupted.load(Ordering::Acquire));
        guard.remove(&mut tx).await.unwrap();
        assert_eq!(
            Arc::strong_count(&guard.state),
            1,
            "SQLite no longer owns the callback"
        );
        tx.rollback().await.unwrap();
        assert!(!c.is_in_transaction());
        assert_eq!(sqlx::query_scalar::<_, i64>("WITH RECURSIVE n(x) AS (VALUES(0) UNION ALL SELECT x+1 FROM n WHERE x<10000) SELECT max(x) FROM n").fetch_one(&mut c).await.unwrap(), 10000);
        c.close().await.unwrap();
    }

    #[tokio::test]
    async fn first_read_pins_independent_inventory_before_wal_writer_commits() {
        let dir = tempfile::tempdir().unwrap();
        let pool = super::super::connect_for_migration(&format!(
            "sqlite://{}/race.db",
            dir.path().display()
        ))
        .await
        .unwrap();
        super::super::migrate(&pool).await.unwrap();
        let accounts = Accounts::new(pool.clone(), 3600).unwrap();
        accounts
            .invite("test", "owner", Role::Administrator, None)
            .await
            .unwrap();
        let user = accounts
            .sign_in(Subject {
                provider: "test".into(),
                provider_subject: "owner".into(),
                display_name: "Owner".into(),
            })
            .await
            .unwrap();
        let (token, _) = accounts.start_session(&user.user_id).await.unwrap();
        let author = Authoring::new(accounts.clone());
        let world = author
            .create_world(&token, fields("Race"), None)
            .await
            .unwrap();
        let variant = author
            .create_variant(&token, &world.id, 1, None, fields("Owner"), None)
            .await
            .unwrap();
        let service = crate::application::inspection::WorldInspection::new(accounts.clone());
        let baseline = service
            .capture(AddressKey::Id(world.id.clone()))
            .await
            .unwrap();
        let (started, first_read) = tokio::sync::oneshot::channel();
        let (committed, writer_done) = tokio::sync::oneshot::channel();
        let mut c = pool.acquire().await.unwrap();
        sqlx::query("PRAGMA query_only=ON")
            .execute(&mut *c)
            .await
            .unwrap();
        let reader = capture_impl(
            &mut c,
            AddressKey::Id(world.id.clone()),
            InspectionLimits::default(),
            || async move {
                started.send(()).unwrap();
                writer_done.await.unwrap();
            },
        );
        let writer = async {
            first_read.await.unwrap();
            let moves = Moves::new(accounts.clone());
            let m = moves
                .create(
                    &token,
                    Some(&world.id),
                    NewMove {
                        fields: fields("Independent move"),
                        source: None,
                        attributes: None,
                        reason: None,
                    },
                )
                .await
                .unwrap();
            let ruleset = moves
                .save_ruleset(
                    &token,
                    &world.id,
                    RulesetEdit {
                        expected_revision: 0,
                        fields: fields("Independent ruleset"),
                        expectations: None,
                        stat_roster: None,
                        chart: None,
                        max_moves: None,
                        repeat_policy: RepeatPolicy::Unspecified,
                        notes: String::new(),
                        reason: None,
                    },
                )
                .await
                .unwrap();
            let learning = Learning::new(accounts.clone());
            let set = learning
                .save_learnset(
                    &token,
                    Some(&world.id),
                    &variant.id,
                    LearnsetEdit {
                        expected_revision: 0,
                        records: Some(vec![LearningRecordInput {
                            id: String::new(),
                            move_pin: m.pin(),
                            method: None,
                            context: None,
                            condition: None,
                            threshold: NumericValue::Unknown,
                        }]),
                        reason: None,
                    },
                )
                .await
                .unwrap();
            learning
                .create_preset(
                    &token,
                    &world.id,
                    &variant.id,
                    MovesetEdit {
                        expected_revision: 0,
                        fields: fields("Independent preset"),
                        learnset: set.pin(),
                        ruleset: ruleset.pin(),
                        entries: vec![set.records.as_ref().unwrap()[0].id.clone()],
                        reason: None,
                    },
                )
                .await
                .unwrap();
            committed.send(()).unwrap();
        };
        let (captured, ()) = tokio::join!(reader, writer);
        sqlx::query("PRAGMA query_only=OFF")
            .execute(&mut *c)
            .await
            .unwrap();
        let captured = captured.unwrap();
        assert!(captured.is_complete(), "{:?}", captured.gaps);
        assert_eq!(captured, baseline);
        let after = service.capture(AddressKey::Id(world.id)).await.unwrap();
        assert!(after.is_complete(), "{:?}", after.gaps);
        assert_eq!(after.world, baseline.world);
        assert_eq!(after.roots.len(), baseline.roots.len() + 4);
        for kind in [
            DomainKind::Move,
            DomainKind::Ruleset,
            DomainKind::Learnset,
            DomainKind::Preset,
        ] {
            assert!(!baseline.roots.keys().any(|k| k.kind == kind));
            assert!(after.roots.keys().any(|k| k.kind == kind));
        }
        assert_eq!(
            after.nodes[&key(DomainKind::Variant, Some(&variant.world_id), &variant.id, 1)],
            baseline.nodes[&key(DomainKind::Variant, Some(&variant.world_id), &variant.id, 1)]
        );
        assert_ne!(
            crate::application::inspection::fingerprint(&after),
            crate::application::inspection::fingerprint(&baseline)
        );
    }
}
