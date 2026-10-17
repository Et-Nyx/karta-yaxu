//! Public read-only world inspection. Capture finishes before pure consumers run.
use super::authoring::{AddressKey, Error};
use crate::{accounts::Accounts, persistence};
use game_data::inspection::{CapturedWorld, InspectionLimits};
use game_data::inspection_analysis::{
    self as analysis, InspectionFinding, WhereUsedQuery, WhereUsedResult,
};
use sha2::{Digest, Sha256};
use std::{sync::Arc, time::Duration};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio::time::{Instant, timeout_at};

#[derive(Clone, Debug)]
pub struct WorldInspection {
    accounts: Accounts,
    work: Arc<Semaphore>,
}

/// Materialized public data and its pure analysis. No transaction or lazy reader
/// survives this boundary; every view uses exactly this capture.
#[derive(Debug)]
pub struct InspectionResult {
    pub capture: CapturedWorld,
    pub findings: Vec<InspectionFinding>,
    pub fingerprint: Option<String>,
    // Retain the request's admission slot through analysis, usage traversal and
    // rendering. Moving the result into a blocking task keeps cancellation from
    // releasing capacity while that task is still running.
    _slot: OwnedSemaphorePermit,
}
impl WorldInspection {
    pub fn new(accounts: Accounts) -> Self {
        Self {
            accounts,
            work: Arc::new(Semaphore::new(2)),
        }
    }
    pub async fn capture(&self, world: AddressKey) -> Result<CapturedWorld, Error> {
        self.capture_with_limits(world, InspectionLimits::default())
            .await
    }
    pub async fn inspect(
        &self,
        world: AddressKey,
        compare_policy: bool,
    ) -> Result<InspectionResult, Error> {
        let slot = self
            .work
            .clone()
            .try_acquire_owned()
            .map_err(|_| Error::Unavailable)?;
        let capture = self.capture(world).await?;
        tokio::task::spawn_blocking(move || {
            let fingerprint = fingerprint(&capture);
            let findings = analysis::analyze_world(&capture, compare_policy);
            InspectionResult {
                capture,
                findings,
                fingerprint,
                _slot: slot,
            }
        })
        .await
        .map_err(|_| Error::Unavailable)
    }
    pub async fn usages(
        &self,
        result: InspectionResult,
        query: WhereUsedQuery,
    ) -> Result<(InspectionResult, WhereUsedResult), Error> {
        tokio::task::spawn_blocking(move || {
            let uses = where_used(&result.capture, &query);
            (result, uses)
        })
        .await
        .map_err(|_| Error::Unavailable)
    }
    pub async fn capture_with_limits(
        &self,
        world: AddressKey,
        limits: InspectionLimits,
    ) -> Result<CapturedWorld, Error> {
        let deadline = Instant::now() + Duration::from_millis(limits.deadline_ms);
        let c = timeout_at(deadline, self.accounts.pool().acquire())
            .await
            .map_err(|_| Error::Unavailable)?
            .map_err(persistence::authoring::storage_error)?;
        persistence::inspection::capture_pooled_world(c, world, limits, deadline).await
    }
}

/// Usage is derived from the caller's existing capture, never a second read.
pub fn where_used(capture: &CapturedWorld, query: &WhereUsedQuery) -> WhereUsedResult {
    analysis::where_used(capture, query)
}

/// Versioned canonical vectors, never JSON maps with non-string keys. Duplicate
/// edge occurrences remain significant; ordering of input vectors does not.
pub fn fingerprint(captured: &CapturedWorld) -> Option<String> {
    if !captured.is_complete() {
        return None;
    }
    let mut edges = captured.edges.iter().collect::<Vec<_>>();
    edges.sort();
    let mut absences = captured.absences.iter().collect::<Vec<_>>();
    absences.sort();
    let bytes = serde_json::to_vec(&(
        "karta-world-inspection-sha256-v1",
        captured.contract_version,
        &captured.world,
        captured.roots.iter().collect::<Vec<_>>(),
        captured.nodes.iter().collect::<Vec<_>>(),
        captured.heads.iter().collect::<Vec<_>>(),
        edges,
        absences,
    ))
    .ok()?;
    Some(hex::encode(Sha256::digest(bytes)))
}
