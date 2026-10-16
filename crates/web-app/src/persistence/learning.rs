//! Typed learning aggregates. Writers require the caller's locked transaction.
use super::{authoring as content, moves as core};
use crate::application::authoring::{self as author, AddressKey, Error, HistoryEntry};
use game_data::{learning::*, moves::NumericValue, stats::Decimal};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sqlx::{Row, SqliteConnection, sqlite::SqliteRow};

fn scope(world: Option<&str>) -> &str {
    world.unwrap_or("")
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Snapshot<T> {
    snapshot_version: i64,
    content: T,
}
fn parse<T: DeserializeOwned + Serialize>(text: &str) -> Result<T, Error> {
    let snapshot: Snapshot<T> = serde_json::from_str(text).map_err(|_| Error::Unavailable)?;
    let value: serde_json::Value = serde_json::from_str(text).map_err(|_| Error::Unavailable)?;
    if snapshot.snapshot_version != 1
        || value != serde_json::json!({"snapshot_version":1,"content":snapshot.content})
    {
        return Err(Error::Unavailable);
    }
    Ok(snapshot.content)
}
fn continuity(row: &SqliteRow, revision: i64) -> Result<(Option<String>, String), Error> {
    let before: Option<String> = row.try_get("before_json").map_err(content::storage_error)?;
    let after: String = row.try_get("after_json").map_err(content::storage_error)?;
    let previous: Option<i64> = row
        .try_get("previous_revision")
        .map_err(content::storage_error)?;
    let audit: bool = row
        .try_get("audit_present")
        .map_err(content::storage_error)?;
    if revision < 1
        || previous != content::previous(revision)
        || before.is_some() != (revision > 1)
        || !audit
    {
        return Err(Error::Unavailable);
    }
    Ok((before, after))
}
async fn validate_audit(
    c: &mut SqliteConnection,
    row: &SqliteRow,
    revision: i64,
    adoption: bool,
) -> Result<(), Error> {
    let id: String = row.try_get("audit_id").map_err(content::storage_error)?;
    let audit=sqlx::query("SELECT coalesce(u.display_name,a.system_key) AS actor_name,a.action,a.reason FROM audit_events a LEFT JOIN users u ON u.id=a.actor_user_id WHERE a.id=?")
        .bind(id).fetch_optional(c).await.map_err(content::storage_error)?.ok_or(Error::Unavailable)?;
    let name: Option<String> = audit
        .try_get("actor_name")
        .map_err(content::storage_error)?;
    let action: String = audit.try_get("action").map_err(content::storage_error)?;
    let reason: Option<String> = audit.try_get("reason").map_err(content::storage_error)?;
    if name
        .as_ref()
        .is_none_or(|name| name.is_empty() || name.chars().any(char::is_control))
        || !(action == if revision == 1 { "create" } else { "edit" }
            || adoption && action == "adopt")
        || game_data::authoring::validate_reason(reason.as_deref())
            .ok()
            .as_ref()
            != Some(&reason)
    {
        return Err(Error::Unavailable);
    }
    Ok(())
}
pub(crate) async fn owner(
    c: &mut SqliteConnection,
    world: Option<&str>,
    id: &str,
) -> Result<(), Error> {
    if let Some(w) = world {
        content::variant(c, w, id).await?;
    } else {
        content::form(c, id).await?;
    }
    Ok(())
}
pub(crate) async fn learnset(
    c: &mut SqliteConnection,
    world: Option<&str>,
    owner_id: &str,
) -> Result<Option<LearnsetRecord>, Error> {
    owner(c, world, owner_id).await?;
    let head: Option<(String, i64)> = sqlx::query_as(
        "SELECT id,revision FROM learnsets WHERE scope=? AND coalesce(form_id,variant_id)=?",
    )
    .bind(scope(world))
    .bind(owner_id)
    .fetch_optional(&mut *c)
    .await
    .map_err(content::storage_error)?;
    match head {
        Some((id, v)) => learnset_version(c, world, &id, v)
            .await
            .map(Some)
            .map_err(|_| Error::Unavailable),
        None => Ok(None),
    }
}
pub(crate) async fn shared_learnsets(
    c: &mut SqliteConnection,
) -> Result<Vec<LearnsetRecord>, Error> {
    let ids: Vec<(String, i64)> =
        sqlx::query_as("SELECT id,revision FROM learnsets WHERE scope='' ORDER BY form_id,id")
            .fetch_all(&mut *c)
            .await
            .map_err(content::storage_error)?;
    let mut rows = vec![];
    for (id, v) in ids {
        rows.push(
            learnset_version(c, None, &id, v)
                .await
                .map_err(|_| Error::Unavailable)?,
        );
    }
    Ok(rows)
}
pub(crate) async fn learnset_dependencies(
    c: &mut SqliteConnection,
    row: &LearnsetRecord,
) -> Result<(), Error> {
    row.validate().map_err(Error::Invalid)?;
    owner(
        c,
        row.world_id.as_deref(),
        row.form_id
            .as_deref()
            .or(row.variant_id.as_deref())
            .ok_or(Error::Unavailable)?,
    )
    .await?;
    for record in row.records.iter().flatten() {
        let exact = core::move_version(
            c,
            row.world_id.as_deref(),
            &record.move_record.id,
            record.move_record.revision,
        )
        .await?;
        if exact != record.move_record {
            return Err(Error::Unavailable);
        }
    }
    if let Some(pin) = &row.source {
        let source = Box::pin(learnset_version(c, None, &pin.learnset_id, pin.version)).await?;
        for mapping in &row.mappings {
            if !source
                .records
                .iter()
                .flatten()
                .any(|r| r.move_record.id == mapping.shared_move_id)
            {
                return Err(author::invalid(
                    "mappings",
                    "Map only moves present in the exact shared source learnset.",
                ));
            }
            let local = core::move_version(
                c,
                row.world_id.as_deref(),
                &mapping.local_move.move_id,
                mapping.local_move.version,
            )
            .await?;
            if local.source.as_ref().map(|p| &p.move_id) != Some(&mapping.shared_move_id) {
                return Err(author::invalid(
                    "mappings",
                    "The local move must have the mapped shared move as its explicit source.",
                ));
            }
        }
    }
    Ok(())
}
async fn load_learnset(
    c: &mut SqliteConnection,
    world: Option<&str>,
    id: &str,
    version: i64,
) -> Result<(LearnsetRecord, Option<String>, String), Error> {
    let r=sqlx::query("SELECT v.*,EXISTS(SELECT 1 FROM audit_events a LEFT JOIN users u ON u.id=a.actor_user_id WHERE a.id=v.audit_id AND coalesce(u.display_name,a.system_key) IS NOT NULL AND ((v.revision=1 AND a.action IN ('create','adopt')) OR (v.revision>1 AND a.action IN ('edit','adopt')))) AS audit_present FROM learnset_versions v WHERE scope=? AND learnset_id=? AND revision=?")
        .bind(scope(world)).bind(id).bind(version).fetch_optional(&mut *c).await.map_err(content::storage_error)?.ok_or(Error::NotFound)?;
    let (before, after) = continuity(&r, version)?;
    validate_audit(c, &r, version, true).await?;
    let row: LearnsetRecord = parse(&after)?;
    row.validate().map_err(|_| Error::Unavailable)?;
    let variant: Option<String> = r.try_get("variant_id").map_err(content::storage_error)?;
    let source_scope: String = r.try_get("source_scope").map_err(content::storage_error)?;
    let source_id: Option<String> = r
        .try_get("source_learnset_id")
        .map_err(content::storage_error)?;
    let source_version: Option<i64> = r
        .try_get("source_revision")
        .map_err(content::storage_error)?;
    let defined: i64 = r
        .try_get("records_defined")
        .map_err(content::storage_error)?;
    if row.id != id
        || row.revision != version
        || row.world_id.as_deref() != world
        || row.variant_id != variant
        || !source_scope.is_empty()
        || source_id.as_deref() != row.source.as_ref().map(|p| p.learnset_id.as_str())
        || source_version != row.source.as_ref().map(|p| p.version)
        || defined != i64::from(row.records.is_some())
    {
        return Err(Error::Unavailable);
    }
    let records=sqlx::query("SELECT * FROM learnset_records WHERE scope=? AND learnset_id=? AND revision=? ORDER BY slot")
        .bind(scope(world)).bind(id).bind(version).fetch_all(&mut *c).await.map_err(content::storage_error)?;
    if records.len() != row.records.as_ref().map_or(0, Vec::len) {
        return Err(Error::Unavailable);
    }
    for (i, (typed, record)) in records.iter().zip(row.records.iter().flatten()).enumerate() {
        let slot: i64 = typed.try_get("slot").map_err(content::storage_error)?;
        let record_id: String = typed.try_get("record_id").map_err(content::storage_error)?;
        let move_id: String = typed.try_get("move_id").map_err(content::storage_error)?;
        let move_version: i64 = typed
            .try_get("move_version")
            .map_err(content::storage_error)?;
        let method: Option<String> = typed.try_get("method").map_err(content::storage_error)?;
        let context: Option<String> = typed.try_get("context").map_err(content::storage_error)?;
        let condition: Option<String> =
            typed.try_get("condition").map_err(content::storage_error)?;
        if slot != ordinal(i)?
            || record_id != record.id
            || move_id != record.move_record.id
            || move_version != record.move_record.revision
            || method != record.method
            || context != record.context
            || condition != record.condition
            || numeric(typed)? != record.threshold
        {
            return Err(Error::Unavailable);
        }
    }
    let mappings=sqlx::query("SELECT * FROM learnset_move_mappings WHERE scope=? AND learnset_id=? AND revision=? ORDER BY slot")
        .bind(scope(world)).bind(id).bind(version).fetch_all(&mut *c).await.map_err(content::storage_error)?;
    if mappings.len() != row.mappings.len() {
        return Err(Error::Unavailable);
    }
    for (i, (typed, mapping)) in mappings.iter().zip(&row.mappings).enumerate() {
        let slot: i64 = typed.try_get("slot").map_err(content::storage_error)?;
        let shared_scope: String = typed
            .try_get("shared_scope")
            .map_err(content::storage_error)?;
        let shared: String = typed
            .try_get("shared_move_id")
            .map_err(content::storage_error)?;
        let shared_version: i64 = typed
            .try_get("shared_move_version")
            .map_err(content::storage_error)?;
        let local: String = typed
            .try_get("local_move_id")
            .map_err(content::storage_error)?;
        let v: i64 = typed
            .try_get("local_move_version")
            .map_err(content::storage_error)?;
        if slot != ordinal(i)?
            || !shared_scope.is_empty()
            || shared != mapping.shared_move_id
            || local != mapping.local_move.move_id
            || v != mapping.local_move.version
        {
            return Err(Error::Unavailable);
        }
        let pin = row.source.as_ref().ok_or(Error::Unavailable)?;
        let source = Box::pin(learnset_version(c, None, &pin.learnset_id, pin.version)).await?;
        if source
            .records
            .iter()
            .flatten()
            .find(|r| r.move_record.id == shared)
            .map(|r| r.move_record.revision)
            != Some(shared_version)
        {
            return Err(Error::Unavailable);
        }
    }
    learnset_dependencies(c, &row)
        .await
        .map_err(|_| Error::Unavailable)?;
    Ok((row, before, after))
}
fn numeric(row: &SqliteRow) -> Result<NumericValue, Error> {
    let state: String = row
        .try_get("threshold_state")
        .map_err(content::storage_error)?;
    let coefficient: Option<i64> = row.try_get("coefficient").map_err(content::storage_error)?;
    let scale: Option<i64> = row.try_get("scale").map_err(content::storage_error)?;
    match (state.as_str(), coefficient, scale) {
        ("unknown", None, None) => Ok(NumericValue::Unknown),
        ("not_applicable", None, None) => Ok(NumericValue::NotApplicable),
        ("known", Some(n), Some(s)) => {
            let d = Decimal::new(n, u8::try_from(s).map_err(|_| Error::Unavailable)?)
                .map_err(|_| Error::Unavailable)?;
            if d.coefficient() != n || i64::from(d.scale()) != s {
                return Err(Error::Unavailable);
            }
            Ok(NumericValue::Known(d))
        }
        _ => Err(Error::Unavailable),
    }
}
fn ordinal(i: usize) -> Result<i64, Error> {
    i64::try_from(i)
        .ok()
        .and_then(|i| i.checked_add(1))
        .ok_or(Error::Unavailable)
}
pub(crate) async fn learnset_version(
    c: &mut SqliteConnection,
    world: Option<&str>,
    id: &str,
    version: i64,
) -> Result<LearnsetRecord, Error> {
    let (row, before, _) = load_learnset(c, world, id, version).await?;
    let head = sqlx::query("SELECT * FROM learnsets WHERE scope=? AND id=?")
        .bind(scope(world))
        .bind(id)
        .fetch_optional(&mut *c)
        .await
        .map_err(content::storage_error)?
        .ok_or(Error::Unavailable)?;
    let head_world: Option<String> = head.try_get("world_id").map_err(content::storage_error)?;
    let form: Option<String> = head.try_get("form_id").map_err(content::storage_error)?;
    let variant: Option<String> = head.try_get("variant_id").map_err(content::storage_error)?;
    let revision: i64 = head.try_get("revision").map_err(content::storage_error)?;
    if head_world != row.world_id
        || form != row.form_id
        || variant != row.variant_id
        || revision < version
    {
        return Err(Error::Unavailable);
    }
    let (current, current_before) = if revision == version {
        (row.clone(), before.clone())
    } else {
        let (r, b, _) = load_learnset(c, world, id, revision)
            .await
            .map_err(|_| Error::Unavailable)?;
        (r, b)
    };
    if current.form_id != form || current.variant_id != variant {
        return Err(Error::Unavailable);
    }
    for (v, b) in [(version, before), (revision, current_before)] {
        if v > 1 {
            let (previous, _, after) = load_learnset(c, world, id, v - 1)
                .await
                .map_err(|_| Error::Unavailable)?;
            if b.as_deref() != Some(after.as_str())
                || previous.form_id != form
                || previous.variant_id != variant
            {
                return Err(Error::Unavailable);
            }
        }
    }
    Ok(row)
}
pub(crate) async fn presets(
    c: &mut SqliteConnection,
    world: &str,
    variant: &str,
) -> Result<Vec<MovesetRecord>, Error> {
    owner(c, Some(world), variant).await?;
    let ids: Vec<String> = sqlx::query_scalar(
        "SELECT id FROM moveset_presets WHERE world_id=? AND variant_id=? ORDER BY slug,id",
    )
    .bind(world)
    .bind(variant)
    .fetch_all(&mut *c)
    .await
    .map_err(content::storage_error)?;
    let mut rows = vec![];
    for id in ids {
        rows.push(preset(c, world, variant, &id).await?);
    }
    Ok(rows)
}
pub(crate) async fn preset(
    c: &mut SqliteConnection,
    world: &str,
    variant: &str,
    id: &str,
) -> Result<MovesetRecord, Error> {
    let version: i64 = sqlx::query_scalar(
        "SELECT revision FROM moveset_presets WHERE world_id=? AND variant_id=? AND id=?",
    )
    .bind(world)
    .bind(variant)
    .bind(id)
    .fetch_optional(&mut *c)
    .await
    .map_err(content::storage_error)?
    .ok_or(Error::NotFound)?;
    preset_version(c, world, variant, id, version)
        .await
        .map_err(|_| Error::Unavailable)
}
pub(crate) async fn preset_dependencies(
    c: &mut SqliteConnection,
    row: &MovesetRecord,
) -> Result<(LearnsetRecord, game_data::rulesets::RulesetRecord), Error> {
    row.validate().map_err(Error::Invalid)?;
    owner(c, Some(&row.world_id), &row.variant_id).await?;
    let learnset = learnset_version(
        c,
        Some(&row.world_id),
        &row.learnset.learnset_id,
        row.learnset.version,
    )
    .await?;
    if learnset.variant_id.as_deref() != Some(row.variant_id.as_str()) {
        return Err(author::invalid(
            "learnset",
            "Select an exact learnset belonging to this variant.",
        ));
    }
    let ruleset = core::ruleset_version(
        c,
        &row.world_id,
        &row.ruleset.ruleset_id,
        row.ruleset.version,
    )
    .await?;
    for id in &row.entries {
        if !learnset.records.iter().flatten().any(|r| &r.id == id) {
            return Err(author::invalid(
                "entries",
                "Every selected acquisition must exist in the exact pinned learnset revision.",
            ));
        }
    }
    Ok((learnset, ruleset))
}
async fn load_preset(
    c: &mut SqliteConnection,
    world: &str,
    variant: &str,
    id: &str,
    version: i64,
) -> Result<(MovesetRecord, Option<String>, String), Error> {
    let r=sqlx::query("SELECT v.*,EXISTS(SELECT 1 FROM audit_events a LEFT JOIN users u ON u.id=a.actor_user_id WHERE a.id=v.audit_id AND coalesce(u.display_name,a.system_key) IS NOT NULL AND ((v.revision=1 AND a.action='create') OR (v.revision>1 AND a.action='edit'))) AS audit_present FROM moveset_preset_versions v WHERE world_id=? AND variant_id=? AND preset_id=? AND revision=?")
        .bind(world).bind(variant).bind(id).bind(version).fetch_optional(&mut *c).await.map_err(content::storage_error)?.ok_or(Error::NotFound)?;
    let (before, after) = continuity(&r, version)?;
    validate_audit(c, &r, version, false).await?;
    let row: MovesetRecord = parse(&after)?;
    row.validate().map_err(|_| Error::Unavailable)?;
    let archived: Option<i64> = r.try_get("archived_at").map_err(content::storage_error)?;
    let learnset_id: String = r.try_get("learnset_id").map_err(content::storage_error)?;
    let learnset_version: i64 = r
        .try_get("learnset_version")
        .map_err(content::storage_error)?;
    let ruleset_id: String = r.try_get("ruleset_id").map_err(content::storage_error)?;
    let ruleset_version: i64 = r
        .try_get("ruleset_version")
        .map_err(content::storage_error)?;
    let validator: i64 = r
        .try_get("validator_version")
        .map_err(content::storage_error)?;
    if row.id != id
        || row.world_id != world
        || row.variant_id != variant
        || row.revision != version
        || row.fields != content::fields(&r)?
        || row.archived_at != archived
        || row.learnset.learnset_id != learnset_id
        || row.learnset.version != learnset_version
        || row.ruleset.ruleset_id != ruleset_id
        || row.ruleset.version != ruleset_version
        || row.validator_version != validator
    {
        return Err(Error::Unavailable);
    }
    let entries=sqlx::query("SELECT * FROM moveset_preset_entries WHERE world_id=? AND variant_id=? AND preset_id=? AND revision=? ORDER BY slot")
        .bind(world).bind(variant).bind(id).bind(version).fetch_all(&mut *c).await.map_err(content::storage_error)?;
    if entries.len() != row.entries.len() {
        return Err(Error::Unavailable);
    }
    for (i, (entry, id)) in entries.iter().zip(&row.entries).enumerate() {
        let slot: i64 = entry.try_get("slot").map_err(content::storage_error)?;
        let record: String = entry.try_get("record_id").map_err(content::storage_error)?;
        let set: String = entry
            .try_get("learnset_id")
            .map_err(content::storage_error)?;
        let v: i64 = entry
            .try_get("learnset_version")
            .map_err(content::storage_error)?;
        if slot != ordinal(i)? || &record != id || set != learnset_id || v != learnset_version {
            return Err(Error::Unavailable);
        }
    }
    preset_dependencies(c, &row)
        .await
        .map_err(|_| Error::Unavailable)?;
    Ok((row, before, after))
}
pub(crate) async fn preset_version(
    c: &mut SqliteConnection,
    world: &str,
    variant: &str,
    id: &str,
    version: i64,
) -> Result<MovesetRecord, Error> {
    let (row, before, _) = load_preset(c, world, variant, id, version).await?;
    let head: Option<(String, i64)> = sqlx::query_as(
        "SELECT slug,revision FROM moveset_presets WHERE world_id=? AND variant_id=? AND id=?",
    )
    .bind(world)
    .bind(variant)
    .bind(id)
    .fetch_optional(&mut *c)
    .await
    .map_err(content::storage_error)?;
    let (slug, revision) = head.ok_or(Error::Unavailable)?;
    if revision < version {
        return Err(Error::Unavailable);
    }
    let (current, current_before) = if revision == version {
        (row.clone(), before.clone())
    } else {
        let (r, b, _) = load_preset(c, world, variant, id, revision)
            .await
            .map_err(|_| Error::Unavailable)?;
        (r, b)
    };
    if current.fields.slug != slug {
        return Err(Error::Unavailable);
    }
    for (v, b) in [(version, before), (revision, current_before)] {
        if v > 1 {
            let (_, _, after) = load_preset(c, world, variant, id, v - 1)
                .await
                .map_err(|_| Error::Unavailable)?;
            if b.as_deref() != Some(after.as_str()) {
                return Err(Error::Unavailable);
            }
        }
    }
    Ok(row)
}
pub(crate) async fn resolve_preset(
    c: &mut SqliteConnection,
    world: &str,
    variant: &str,
    key: AddressKey,
) -> Result<MovesetRecord, Error> {
    match key {
        AddressKey::Id(id) => preset(c, world, variant, &id).await,
        AddressKey::Slug(slug) => {
            let id: String = sqlx::query_scalar(
                "SELECT id FROM moveset_presets WHERE world_id=? AND variant_id=? AND slug=?",
            )
            .bind(world)
            .bind(variant)
            .bind(slug)
            .fetch_optional(&mut *c)
            .await
            .map_err(content::storage_error)?
            .ok_or(Error::NotFound)?;
            preset(c, world, variant, &id).await
        }
    }
}
pub(crate) async fn slug_exists(
    c: &mut SqliteConnection,
    world: &str,
    variant: &str,
    slug: &str,
    except: Option<&str>,
) -> Result<bool, Error> {
    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM moveset_presets WHERE world_id=? AND variant_id=? AND slug=? AND (? IS NULL OR id<>?))")
        .bind(world).bind(variant).bind(slug).bind(except).bind(except).fetch_one(c).await.map_err(content::storage_error)
}
pub(crate) async fn put_learnset(
    c: &mut SqliteConnection,
    row: &LearnsetRecord,
    actor: Option<&str>,
    action: &str,
    reason: Option<&str>,
) -> Result<(), Error> {
    if !(action == if row.revision == 1 { "create" } else { "edit" } || action == "adopt") {
        return Err(author::invalid(
            "action",
            "Use the owning learnset's creation, edit or adoption action.",
        ));
    }
    learnset_dependencies(c, row).await?;
    let world = row.world_id.as_deref();
    let before = if row.revision > 1 {
        let old = learnset(
            c,
            world,
            row.form_id
                .as_deref()
                .or(row.variant_id.as_deref())
                .ok_or(Error::Unavailable)?,
        )
        .await?
        .ok_or(Error::NotFound)?;
        if old.id != row.id {
            return Err(Error::Unavailable);
        }
        author::expected(author::next(old.revision)?, row.revision)?;
        Some(sqlx::query_scalar::<_,String>("SELECT after_json FROM learnset_versions WHERE scope=? AND learnset_id=? AND revision=?").bind(scope(world)).bind(&row.id).bind(old.revision).fetch_one(&mut *c).await.map_err(content::storage_error)?)
    } else {
        None
    };
    let after = author::snapshot(row)?;
    let audit = content::audit(
        c,
        &content::Revision::new(actor, action, reason, before.clone(), after.clone()),
    )
    .await?;
    let sql = if row.revision == 1 {
        "INSERT INTO learnsets(scope,id,world_id,form_id,variant_id,revision) VALUES (?1,?2,?3,?4,?5,?6)"
    } else {
        "UPDATE learnsets SET revision=?6 WHERE scope=?1 AND id=?2 AND world_id IS ?3 AND form_id IS ?4 AND variant_id IS ?5 AND revision=?6-1"
    };
    let result = sqlx::query(sql)
        .bind(scope(world))
        .bind(&row.id)
        .bind(world)
        .bind(&row.form_id)
        .bind(&row.variant_id)
        .bind(row.revision)
        .execute(&mut *c)
        .await
        .map_err(content::content_write_error)?;
    if result.rows_affected() != 1 {
        return Err(Error::Conflict);
    }
    for (i, record) in row.records.iter().flatten().enumerate() {
        let (state, n, s) = match &record.threshold {
            NumericValue::Known(d) => ("known", Some(d.coefficient()), Some(i64::from(d.scale()))),
            NumericValue::Unknown => ("unknown", None, None),
            NumericValue::NotApplicable => ("not_applicable", None, None),
        };
        sqlx::query("INSERT INTO learnset_records(scope,learnset_id,revision,slot,record_id,move_id,move_version,method,context,condition,threshold_state,coefficient,scale) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?)")
            .bind(scope(world)).bind(&row.id).bind(row.revision).bind(ordinal(i)?).bind(&record.id).bind(&record.move_record.id).bind(record.move_record.revision)
            .bind(&record.method).bind(&record.context).bind(&record.condition).bind(state).bind(n).bind(s).execute(&mut *c).await.map_err(content::storage_error)?;
    }
    if let Some(pin) = &row.source {
        let source = learnset_version(c, None, &pin.learnset_id, pin.version).await?;
        for (i, mapping) in row.mappings.iter().enumerate() {
            let shared = source
                .records
                .iter()
                .flatten()
                .find(|r| r.move_record.id == mapping.shared_move_id)
                .ok_or(Error::Unavailable)?;
            sqlx::query("INSERT INTO learnset_move_mappings(scope,learnset_id,revision,slot,shared_move_id,shared_move_version,local_move_id,local_move_version) VALUES (?,?,?,?,?,?,?,?)")
                .bind(scope(world)).bind(&row.id).bind(row.revision).bind(ordinal(i)?).bind(&mapping.shared_move_id).bind(shared.move_record.revision).bind(&mapping.local_move.move_id).bind(mapping.local_move.version).execute(&mut *c).await.map_err(content::storage_error)?;
        }
    }
    sqlx::query("INSERT INTO learnset_versions(scope,learnset_id,revision,variant_id,source_learnset_id,source_revision,records_defined,audit_id,previous_revision,before_json,after_json) VALUES (?,?,?,?,?,?,?,?,?,?,?)")
        .bind(scope(world)).bind(&row.id).bind(row.revision).bind(&row.variant_id).bind(row.source.as_ref().map(|p|&p.learnset_id)).bind(row.source.as_ref().map(|p|p.version)).bind(i64::from(row.records.is_some())).bind(audit).bind(content::previous(row.revision)).bind(before).bind(after).execute(c).await.map_err(content::storage_error)?;
    Ok(())
}
pub(crate) async fn put_preset(
    c: &mut SqliteConnection,
    row: &MovesetRecord,
    actor: Option<&str>,
    reason: Option<&str>,
) -> Result<(), Error> {
    preset_dependencies(c, row).await?;
    let before = if row.revision > 1 {
        let old = preset(c, &row.world_id, &row.variant_id, &row.id).await?;
        author::expected(author::next(old.revision)?, row.revision)?;
        Some(sqlx::query_scalar::<_,String>("SELECT after_json FROM moveset_preset_versions WHERE world_id=? AND variant_id=? AND preset_id=? AND revision=?").bind(&row.world_id).bind(&row.variant_id).bind(&row.id).bind(old.revision).fetch_one(&mut *c).await.map_err(content::storage_error)?)
    } else {
        None
    };
    let after = author::snapshot(row)?;
    let action = if row.revision == 1 { "create" } else { "edit" };
    let audit = content::audit(
        c,
        &content::Revision::new(actor, action, reason, before.clone(), after.clone()),
    )
    .await?;
    let sql = if row.revision == 1 {
        "INSERT INTO moveset_presets(world_id,variant_id,id,slug,revision) VALUES (?1,?2,?3,?4,?5)"
    } else {
        "UPDATE moveset_presets SET slug=?4,revision=?5 WHERE world_id=?1 AND variant_id=?2 AND id=?3 AND revision=?5-1"
    };
    let result = sqlx::query(sql)
        .bind(&row.world_id)
        .bind(&row.variant_id)
        .bind(&row.id)
        .bind(&row.fields.slug)
        .bind(row.revision)
        .execute(&mut *c)
        .await
        .map_err(content::content_write_error)?;
    if result.rows_affected() != 1 {
        return Err(Error::Conflict);
    }
    for (i, id) in row.entries.iter().enumerate() {
        sqlx::query("INSERT INTO moveset_preset_entries(world_id,variant_id,preset_id,revision,slot,learnset_id,learnset_version,record_id) VALUES (?,?,?,?,?,?,?,?)")
            .bind(&row.world_id).bind(&row.variant_id).bind(&row.id).bind(row.revision).bind(ordinal(i)?).bind(&row.learnset.learnset_id).bind(row.learnset.version).bind(id).execute(&mut *c).await.map_err(content::storage_error)?;
    }
    sqlx::query("INSERT INTO moveset_preset_versions(world_id,variant_id,preset_id,revision,slug,display_name,description,archived_at,learnset_id,learnset_version,ruleset_id,ruleset_version,validator_version,audit_id,previous_revision,before_json,after_json) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)")
        .bind(&row.world_id).bind(&row.variant_id).bind(&row.id).bind(row.revision).bind(&row.fields.slug).bind(&row.fields.display_name).bind(&row.fields.description).bind(row.archived_at)
        .bind(&row.learnset.learnset_id).bind(row.learnset.version).bind(&row.ruleset.ruleset_id).bind(row.ruleset.version).bind(row.validator_version).bind(audit).bind(content::previous(row.revision)).bind(before).bind(after).execute(c).await.map_err(content::storage_error)?;
    Ok(())
}
fn validate_history(rows: &[HistoryEntry]) -> Result<(), Error> {
    for row in rows {
        if row.actor_name.is_empty()
            || row.actor_name.chars().any(char::is_control)
            || !matches!(row.action.as_str(), "create" | "edit" | "adopt")
            || game_data::authoring::validate_reason(row.reason.as_deref())
                .ok()
                .as_ref()
                != Some(&row.reason)
        {
            return Err(Error::Unavailable);
        }
    }
    Ok(())
}

/// Trusted complete-fixture creation baselines. Validate the immutable first
/// revision, never the edited head, and reject cross-scope identity collisions.
pub(crate) async fn learnset_fixture_baseline(
    c: &mut SqliteConnection,
    row: &LearnsetRecord,
) -> Result<bool, Error> {
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM learnsets WHERE id=?)")
        .bind(&row.id)
        .fetch_one(&mut *c)
        .await
        .map_err(content::storage_error)?;
    if !exists {
        return Ok(false);
    }
    learnset_version(c, row.world_id.as_deref(), &row.id, 1)
        .await
        .map_err(|_| Error::Unavailable)?;
    let valid: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM learnset_versions v JOIN audit_events a ON a.id=v.audit_id WHERE v.scope=? AND v.learnset_id=? AND v.revision=1 AND v.after_json=? AND a.actor_user_id IS NULL AND a.system_key='bundled-fixture-loader' AND a.action='create' AND a.reason IS NULL)")
        .bind(scope(row.world_id.as_deref())).bind(&row.id).bind(author::snapshot(row)?).fetch_one(c).await.map_err(content::storage_error)?;
    if !valid {
        return Err(Error::Unavailable);
    }
    Ok(true)
}
pub(crate) async fn preset_fixture_baseline(
    c: &mut SqliteConnection,
    row: &MovesetRecord,
) -> Result<bool, Error> {
    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM moveset_presets WHERE id=?)")
            .bind(&row.id)
            .fetch_one(&mut *c)
            .await
            .map_err(content::storage_error)?;
    if !exists {
        return Ok(false);
    }
    preset_version(c, &row.world_id, &row.variant_id, &row.id, 1)
        .await
        .map_err(|_| Error::Unavailable)?;
    let valid: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM moveset_preset_versions v JOIN audit_events a ON a.id=v.audit_id WHERE v.world_id=? AND v.variant_id=? AND v.preset_id=? AND v.revision=1 AND v.after_json=? AND a.actor_user_id IS NULL AND a.system_key='bundled-fixture-loader' AND a.action='create' AND a.reason IS NULL)")
        .bind(&row.world_id).bind(&row.variant_id).bind(&row.id).bind(author::snapshot(row)?).fetch_one(c).await.map_err(content::storage_error)?;
    if !valid {
        return Err(Error::Unavailable);
    }
    Ok(true)
}
pub(crate) async fn learnset_history(
    c: &mut SqliteConnection,
    world: Option<&str>,
    owner_id: &str,
    before: Option<i64>,
) -> Result<Vec<HistoryEntry>, Error> {
    let Some(head) = learnset(c, world, owner_id).await? else {
        return Ok(vec![]);
    };
    let rows:Vec<HistoryEntry>=sqlx::query_as("SELECT v.revision,v.previous_revision,coalesce(u.display_name,a.system_key) AS actor_name,a.action,a.occurred_at,a.reason,v.before_json,v.after_json FROM learnset_versions v JOIN audit_events a ON a.id=v.audit_id LEFT JOIN users u ON u.id=a.actor_user_id WHERE v.scope=? AND v.learnset_id=? AND (? IS NULL OR v.revision<?) ORDER BY v.revision DESC LIMIT 50")
        .bind(scope(world)).bind(&head.id).bind(before).bind(before).fetch_all(&mut *c).await.map_err(content::storage_error)?;
    for row in &rows {
        learnset_version(c, world, &head.id, row.revision).await?;
    }
    validate_history(&rows)?;
    Ok(rows)
}
pub(crate) async fn preset_history(
    c: &mut SqliteConnection,
    world: &str,
    variant: &str,
    id: &str,
    before: Option<i64>,
) -> Result<Vec<HistoryEntry>, Error> {
    preset(c, world, variant, id).await?;
    let rows:Vec<HistoryEntry>=sqlx::query_as("SELECT v.revision,v.previous_revision,coalesce(u.display_name,a.system_key) AS actor_name,a.action,a.occurred_at,a.reason,v.before_json,v.after_json FROM moveset_preset_versions v JOIN audit_events a ON a.id=v.audit_id LEFT JOIN users u ON u.id=a.actor_user_id WHERE v.world_id=? AND v.variant_id=? AND v.preset_id=? AND (? IS NULL OR v.revision<?) ORDER BY v.revision DESC LIMIT 50")
        .bind(world).bind(variant).bind(id).bind(before).bind(before).fetch_all(&mut *c).await.map_err(content::storage_error)?;
    for row in &rows {
        preset_version(c, world, variant, id, row.revision).await?;
    }
    validate_history(&rows)?;
    Ok(rows)
}
