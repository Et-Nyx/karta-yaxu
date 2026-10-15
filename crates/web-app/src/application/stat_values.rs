//! Exact base values, explicit roster reconciliation and mapped source adoption.
use super::authoring::{self as author, Authoring, Error, FormRecord, HistoryKind, VariantRecord};
use crate::{
    accounts::Accounts,
    persistence::{authoring as content, stat_content, stats as store, types as snapshots},
};
use game_data::authoring::validate_reason;
pub use game_data::stats::{BaseStats, Decimal, StatMapping, StatPin, StatRoster, StatValue};
use sqlx::{Sqlite, SqliteConnection, Transaction};
use std::collections::HashSet;

#[derive(Clone, Debug)]
pub struct FormStatsEdit {
    pub expected_revision: i64,
    pub values: Option<Vec<(StatPin, Option<Decimal>)>>,
    pub reason: Option<String>,
}

#[derive(Clone, Debug)]
pub struct VariantStatsEdit {
    pub expected_revision: i64,
    pub expected_roster_revision: i64,
    /// Partial updates keyed by canonical local Stat identity. None means Missing.
    pub values: Vec<(String, Option<Decimal>)>,
    pub reason: Option<String>,
}

#[derive(Clone, Debug)]
pub struct SourceValuesEdit {
    pub expected_revision: i64,
    pub expected_roster_revision: i64,
    pub target_form_version: i64,
    pub mapping: Vec<StatMapping>,
    pub reason: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ReconcilePreview {
    pub entry: VariantRecord,
    pub old: BaseStats,
    pub roster: StatRoster,
    pub proposed: BaseStats,
    pub removed: Vec<StatValue>,
}

#[derive(Clone, Debug)]
pub struct SourceValuesPreview {
    pub entry: VariantRecord,
    pub old_source: BaseStats,
    pub new_source: BaseStats,
    pub local: BaseStats,
    pub proposed: BaseStats,
    pub source: FormRecord,
}

#[derive(Clone, Debug)]
pub struct StatValues {
    accounts: Accounts,
    authoring: Authoring,
}

impl StatValues {
    pub fn new(accounts: Accounts) -> Self {
        Self {
            authoring: Authoring::new(accounts.clone()),
            accounts,
        }
    }

    async fn read(&self) -> Result<Transaction<'static, Sqlite>, Error> {
        self.accounts
            .pool()
            .begin()
            .await
            .map_err(content::storage_error)
    }

    pub async fn form(&self, form_id: &str) -> Result<BaseStats, Error> {
        let mut tx = self.read().await?;
        let row = content::form(&mut tx, form_id).await?;
        stats(&mut tx, HistoryKind::Form, form_id, None, row.revision).await
    }

    pub async fn variant(&self, world: &str, id: &str) -> Result<BaseStats, Error> {
        let mut tx = self.read().await?;
        let row = content::variant(&mut tx, world, id).await?;
        stats(&mut tx, HistoryKind::Variant, id, Some(world), row.revision).await
    }

    pub async fn save_form(
        &self,
        token: &str,
        id: &str,
        edit: FormStatsEdit,
    ) -> Result<FormRecord, Error> {
        let mut tx = self.authoring.write().await?;
        let actor = self.authoring.actor(&mut tx, token).await?;
        author::require_admin(&actor)?;
        let mut row = content::form(&mut tx, id).await?;
        author::expected(row.revision, edit.expected_revision)?;
        let reason = validate_reason(edit.reason.as_deref()).map_err(Error::Invalid)?;
        let old = stats(&mut tx, HistoryKind::Form, id, None, row.revision).await?;
        let old_values = old.assignments.as_ref().map(|values| {
            values
                .iter()
                .map(|v| {
                    (
                        StatPin {
                            stat_id: v.definition.id.clone(),
                            version: v.definition.revision,
                        },
                        v.value,
                    )
                })
                .collect::<Vec<_>>()
        });
        if old_values != edit.values {
            let assignments = if let Some(values) = edit.values {
                let mut seen = HashSet::new();
                let mut selected = Vec::with_capacity(values.len());
                for (pin, value) in values {
                    if !seen.insert(pin.stat_id.clone()) {
                        return Err(author::invalid("values", "Select each Stat only once."));
                    }
                    let definition = store::select_definition(&mut tx, None, &pin)
                        .await
                        .map_err(|e| author::missing_reference(e, "values"))?;
                    selected.push(StatValue { definition, value });
                }
                Some(selected)
            } else {
                None
            };
            let proposed = BaseStats {
                roster: None,
                assignments,
            };
            stat_content::validate(&mut tx, HistoryKind::Form, None, &proposed).await?;
            row.revision = author::next(row.revision)?;
            content::put_form_stats(
                &mut tx,
                &row,
                content::Revision::new(
                    Some(&actor.user_id),
                    "edit",
                    reason.as_deref(),
                    None,
                    author::snapshot(&row)?,
                ),
                &proposed,
            )
            .await?;
        }
        tx.commit().await.map_err(content::storage_error)?;
        Ok(row)
    }

    pub async fn save_variant(
        &self,
        token: &str,
        world: &str,
        id: &str,
        edit: VariantStatsEdit,
    ) -> Result<VariantRecord, Error> {
        self.edit_variant(token, world, id, edit, false).await
    }

    /// Calling this method is the explicit confirmation of the reconciliation.
    /// The browser adapter is responsible for requiring its confirmation control.
    pub async fn reconcile(
        &self,
        token: &str,
        world: &str,
        id: &str,
        edit: VariantStatsEdit,
    ) -> Result<VariantRecord, Error> {
        self.edit_variant(token, world, id, edit, true).await
    }

    async fn edit_variant(
        &self,
        token: &str,
        world: &str,
        id: &str,
        edit: VariantStatsEdit,
        reconcile: bool,
    ) -> Result<VariantRecord, Error> {
        let mut tx = self.authoring.write().await?;
        let actor = self.authoring.actor(&mut tx, token).await?;
        let world_row = content::world(&mut tx, world).await?;
        self.authoring
            .world_actor(&mut tx, &actor, &world_row, false)
            .await?;
        let mut row = content::variant(&mut tx, world, id).await?;
        author::expected(row.revision, edit.expected_revision)?;
        let roster = checked_roster(&mut tx, world, edit.expected_roster_revision).await?;
        let reason = validate_reason(edit.reason.as_deref()).map_err(Error::Invalid)?;
        let old = stats(&mut tx, HistoryKind::Variant, id, Some(world), row.revision).await?;
        if !reconcile {
            require_current(&old, &roster)?;
        }
        let (proposed, _) = reconcile_values(&old, &roster, &edit.values)?;
        stat_content::validate(&mut tx, HistoryKind::Variant, Some(world), &proposed).await?;
        if old != proposed {
            row.revision = author::next(row.revision)?;
            content::put_variant_stats(
                &mut tx,
                &row,
                content::Revision::new(
                    Some(&actor.user_id),
                    "edit",
                    reason.as_deref(),
                    None,
                    author::snapshot(&row)?,
                ),
                &proposed,
            )
            .await?;
        }
        tx.commit().await.map_err(content::storage_error)?;
        Ok(row)
    }

    pub async fn preview_reconcile(
        &self,
        world: &str,
        id: &str,
        target_roster_revision: i64,
        overrides: &[(String, Option<Decimal>)],
    ) -> Result<ReconcilePreview, Error> {
        let mut tx = self.read().await?;
        let entry = content::variant(&mut tx, world, id).await?;
        let roster = checked_roster(&mut tx, world, target_roster_revision).await?;
        let old = stats(
            &mut tx,
            HistoryKind::Variant,
            id,
            Some(world),
            entry.revision,
        )
        .await?;
        let (proposed, removed) = reconcile_values(&old, &roster, overrides)?;
        stat_content::validate(&mut tx, HistoryKind::Variant, Some(world), &proposed).await?;
        Ok(ReconcilePreview {
            entry,
            old,
            roster,
            proposed,
            removed,
        })
    }

    pub async fn preview_source(
        &self,
        world: &str,
        id: &str,
        target_form_version: i64,
        mapping: &[StatMapping],
    ) -> Result<SourceValuesPreview, Error> {
        let mut tx = self.read().await?;
        let entry = content::variant(&mut tx, world, id).await?;
        let roster = current_roster(&mut tx, world).await?;
        source_preview(&mut tx, world, entry, &roster, target_form_version, mapping).await
    }

    pub async fn apply_source(
        &self,
        token: &str,
        world: &str,
        id: &str,
        edit: SourceValuesEdit,
    ) -> Result<VariantRecord, Error> {
        let mut tx = self.authoring.write().await?;
        let actor = self.authoring.actor(&mut tx, token).await?;
        let world_row = content::world(&mut tx, world).await?;
        self.authoring
            .world_actor(&mut tx, &actor, &world_row, false)
            .await?;
        let mut row = content::variant(&mut tx, world, id).await?;
        author::expected(row.revision, edit.expected_revision)?;
        let roster = checked_roster(&mut tx, world, edit.expected_roster_revision).await?;
        let reason = validate_reason(edit.reason.as_deref()).map_err(Error::Invalid)?;
        let preview = source_preview(
            &mut tx,
            world,
            row.clone(),
            &roster,
            edit.target_form_version,
            &edit.mapping,
        )
        .await?;
        let pin = row.source.as_mut().ok_or_else(no_source)?;
        if pin.version != edit.target_form_version || preview.local != preview.proposed {
            pin.version = edit.target_form_version;
            row.revision = author::next(row.revision)?;
            content::put_variant_stats(
                &mut tx,
                &row,
                content::Revision::new(
                    Some(&actor.user_id),
                    "adopt",
                    reason.as_deref(),
                    None,
                    author::snapshot(&row)?,
                ),
                &preview.proposed,
            )
            .await?;
        }
        tx.commit().await.map_err(content::storage_error)?;
        Ok(row)
    }
}

async fn stats(
    c: &mut SqliteConnection,
    kind: HistoryKind,
    id: &str,
    world: Option<&str>,
    revision: i64,
) -> Result<BaseStats, Error> {
    Ok(snapshots::full_content(c, kind, id, world, revision)
        .await?
        .stats)
}

async fn current_roster(c: &mut SqliteConnection, world: &str) -> Result<StatRoster, Error> {
    store::roster(c, world).await?.ok_or_else(|| {
        author::invalid(
            "roster",
            "Configure a world Stat roster before authoring values.",
        )
    })
}

async fn checked_roster(
    c: &mut SqliteConnection,
    world: &str,
    expected_revision: i64,
) -> Result<StatRoster, Error> {
    let roster = store::roster(c, world).await?;
    author::expected(roster.as_ref().map_or(0, |r| r.revision), expected_revision)?;
    roster.ok_or_else(|| {
        author::invalid(
            "roster",
            "Configure a world Stat roster before authoring values.",
        )
    })
}

fn require_current(old: &BaseStats, roster: &StatRoster) -> Result<(), Error> {
    if old.roster.as_ref() != Some(&roster.pin()) {
        return Err(author::invalid(
            "roster",
            "Review and confirm roster reconciliation before editing these values.",
        ));
    }
    Ok(())
}

/// Retain by immutable identity, replacing definition context only on explicit reconciliation.
fn reconcile_values(
    old: &BaseStats,
    roster: &StatRoster,
    overrides: &[(String, Option<Decimal>)],
) -> Result<(BaseStats, Vec<StatValue>), Error> {
    let mut seen = HashSet::new();
    for (id, _) in overrides {
        if !seen.insert(id.as_str()) || !roster.entries.iter().any(|d| &d.id == id) {
            return Err(author::invalid(
                "values",
                "Update each selected roster Stat at most once; use its canonical identity.",
            ));
        }
    }
    let old_values = old.assignments.as_deref().unwrap_or_default();
    let assignments = roster
        .entries
        .iter()
        .map(|definition| {
            let value =
                if let Some((_, value)) = overrides.iter().find(|(id, _)| id == &definition.id) {
                    *value
                } else {
                    old_values
                        .iter()
                        .find(|v| v.definition.id == definition.id)
                        .and_then(|v| v.value)
                };
            StatValue {
                definition: definition.clone(),
                value,
            }
        })
        .collect();
    let removed = old_values
        .iter()
        .filter(|v| !roster.entries.iter().any(|d| d.id == v.definition.id))
        .cloned()
        .collect();
    Ok((
        BaseStats {
            roster: Some(roster.pin()),
            assignments: Some(assignments),
        },
        removed,
    ))
}

fn no_source() -> Error {
    author::invalid(
        "source",
        "World-exclusive entries have no shared form values to adopt.",
    )
}

async fn source_preview(
    c: &mut SqliteConnection,
    world: &str,
    entry: VariantRecord,
    roster: &StatRoster,
    target: i64,
    mapping: &[StatMapping],
) -> Result<SourceValuesPreview, Error> {
    let pin = entry.source.as_ref().ok_or_else(no_source)?;
    if target < pin.version {
        return Err(author::invalid(
            "target_form_version",
            "Choose the pinned source form version or a newer version of that same form.",
        ));
    }
    let local = stats(
        c,
        HistoryKind::Variant,
        &entry.id,
        Some(world),
        entry.revision,
    )
    .await?;
    // Initial modeling may be explicitly requested through a mapping. An existing
    // older context must be reconciled separately, even if every mapping is empty.
    if local.assignments.is_some() {
        require_current(&local, roster)?;
    }
    let source = content::form_version(c, &pin.form_id, target)
        .await
        .map_err(|e| author::missing_reference(e, "target_form_version"))?;
    let old_source = stats(c, HistoryKind::Form, &pin.form_id, None, pin.version).await?;
    let new_source = stats(c, HistoryKind::Form, &pin.form_id, None, target).await?;
    let mut proposed = if local.assignments.is_none() && !mapping.is_empty() {
        reconcile_values(&local, roster, &[])?.0
    } else {
        local.clone()
    };
    let mut shared_ids = HashSet::new();
    let mut local_ids = HashSet::new();
    for map in mapping {
        if !shared_ids.insert(&map.shared_stat_id) || !local_ids.insert(&map.local_stat_id) {
            return Err(author::invalid(
                "mapping",
                "Map each shared Stat and each local Stat at most once.",
            ));
        }
        let definition = roster
            .entries
            .iter()
            .find(|d| d.id == map.local_stat_id)
            .ok_or_else(|| {
                author::invalid("mapping", "Select a local Stat in the current roster.")
            })?;
        if definition.source.as_ref().map(|p| &p.stat_id) != Some(&map.shared_stat_id) {
            return Err(author::invalid(
                "mapping",
                "The exact local definition must pin the same shared Stat identity.",
            ));
        }
        // A missing assignment in an unmodeled/incomplete source is Missing too,
        // but fabricated identities must never be accepted as mappings.
        store::definition(c, None, &map.shared_stat_id)
            .await
            .map_err(|e| author::missing_reference(e, "mapping"))?;
        let value = new_source.assignments.as_ref().and_then(|values| {
            values
                .iter()
                .find(|v| v.definition.id == map.shared_stat_id)
                .and_then(|v| v.value)
        });
        if value.is_some() || map.clear_missing {
            let destination = proposed
                .assignments
                .as_mut()
                .and_then(|values| {
                    values
                        .iter_mut()
                        .find(|v| v.definition.id == map.local_stat_id)
                })
                .ok_or(Error::Unavailable)?;
            destination.value = value;
        }
    }
    stat_content::validate(c, HistoryKind::Variant, Some(world), &proposed).await?;
    Ok(SourceValuesPreview {
        entry,
        old_source,
        new_source,
        local,
        proposed,
        source,
    })
}
