//! Authorized frozen packages. No database transaction survives capture.
use super::authoring::{AddressKey, Authoring, Error, WorldRecord, invalid};
use crate::{accounts::Accounts, persistence};
use game_data::{
    inspection::InspectionLimits, inspection_analysis::InspectionFinding,
    record_package::RecordPackage,
};
use std::{sync::Arc, time::Duration};
use tokio::{
    sync::{OwnedSemaphorePermit, Semaphore},
    time::{Instant, timeout_at},
};

#[derive(Clone, Debug)]
pub struct WorldExport {
    accounts: Accounts,
    work: Arc<Semaphore>,
}

#[derive(Debug)]
pub struct ExportResult {
    pub package: RecordPackage,
    pub bytes: Vec<u8>,
    pub findings: Vec<InspectionFinding>,
    // Also retained inside blocking work when its awaiting request is cancelled.
    _slot: OwnedSemaphorePermit,
}

#[derive(Debug)]
pub enum Confirmation {
    Download(ExportResult),
    Changed(ExportResult),
}

impl WorldExport {
    pub fn new(accounts: Accounts) -> Self {
        Self {
            accounts,
            work: Arc::new(Semaphore::new(2)),
        }
    }

    /// Explicit export permission, currently owners and eligible collaborators.
    /// A short ordinary read transaction checks session and grants together.
    pub async fn entry(&self, token: &str, world: AddressKey) -> Result<WorldRecord, Error> {
        let mut tx = self
            .accounts
            .pool()
            .begin()
            .await
            .map_err(persistence::authoring::storage_error)?;
        let world = persistence::authoring::resolve_world(&mut tx, world).await?;
        let policy = Authoring::new(self.accounts.clone());
        let actor = policy.actor(&mut tx, token).await?;
        policy.world_actor(&mut tx, &actor, &world, false).await?;
        if world.archived_at.is_some() {
            return Err(Error::Forbidden);
        }
        tx.rollback()
            .await
            .map_err(persistence::authoring::storage_error)?;
        Ok(world)
    }

    pub async fn preview(&self, token: &str, world: AddressKey) -> Result<ExportResult, Error> {
        self.preview_with_limits(token, world, InspectionLimits::default())
            .await
    }

    pub async fn preview_with_limits(
        &self,
        token: &str,
        world: AddressKey,
        limits: InspectionLimits,
    ) -> Result<ExportResult, Error> {
        let slot = self
            .work
            .clone()
            .try_acquire_owned()
            .map_err(|_| Error::Unavailable)?;
        let deadline = Instant::now() + Duration::from_millis(limits.deadline_ms);
        let c = timeout_at(deadline, self.accounts.pool().acquire())
            .await
            .map_err(|_| Error::Unavailable)?
            .map_err(persistence::authoring::storage_error)?;
        let content = persistence::inspection::capture_pooled_record_package(
            c,
            world,
            limits,
            deadline,
            &self.accounts,
            token,
        )
        .await?;
        let result = tokio::task::spawn_blocking(move || {
            let _slot = slot;
            let package = RecordPackage::build(content).map_err(|_| invalid("package", "The selected content cannot form a supported, self-contained record package. Review its exact dependencies and retry; no download was produced."))?;
            let bytes = package.to_bytes().map_err(|_| Error::Unavailable)?;
            // Acceptance uses the real offline reader over the actual final bytes.
            let package = RecordPackage::from_bytes(&bytes).map_err(|_| Error::Unavailable)?;
            let findings = package.diagnostics();
            Ok(ExportResult { package, bytes, findings, _slot })
        }).await.map_err(|_| Error::Unavailable)??;
        // Revocation during expensive processing must not release an artifact.
        self.entry(
            token,
            AddressKey::Id(result.package.content.cards.typed.world.id.clone()),
        )
        .await?;
        Ok(result)
    }

    /// Confirmation is enforced in the use case, not only in the HTTP handler.
    pub async fn confirm(
        &self,
        token: &str,
        world: AddressKey,
        reviewed: &str,
    ) -> Result<Confirmation, Error> {
        if reviewed.len() != 64
            || !reviewed
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        {
            return Err(invalid(
                "reviewed_digest",
                "Preview this package before confirming its download.",
            ));
        }
        let result = self.preview(token, world).await?;
        if result.package.manifest.content_sha256 == reviewed {
            Ok(Confirmation::Download(result))
        } else {
            Ok(Confirmation::Changed(result))
        }
    }
}
