//! Exact chart revisions and typed, immutable relationship seals.
use super::{
    authoring::{self as content, storage_error},
    types,
};
use crate::application::authoring::{self as author, AddressKey, Error, HistoryEntry};
use game_data::{
    charts::*,
    types::{TypeAssignment, TypePin},
};
use serde::Deserialize;
use sqlx::{Row, SqliteConnection};

fn scope(world: Option<&str>) -> &str {
    world.unwrap_or("")
}

pub(crate) async fn slug_exists(
    c: &mut SqliteConnection,
    world: Option<&str>,
    slug: &str,
    except: Option<&str>,
) -> Result<bool, Error> {
    sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM charts WHERE scope=? AND slug=? AND (?3 IS NULL OR id<>?3))",
    )
    .bind(scope(world))
    .bind(slug)
    .bind(except)
    .fetch_one(c)
    .await
    .map_err(storage_error)
}
pub(crate) async fn ids(
    c: &mut SqliteConnection,
    world: Option<&str>,
) -> Result<Vec<String>, Error> {
    sqlx::query_scalar("SELECT c.id FROM charts c LEFT JOIN chart_versions v ON v.scope=c.scope AND v.chart_id=c.id AND v.revision=c.revision WHERE c.scope=? ORDER BY v.display_name,c.id")
        .bind(scope(world)).fetch_all(c).await.map_err(storage_error)
}
pub(crate) async fn resolve(
    c: &mut SqliteConnection,
    world: Option<&str>,
    key: AddressKey,
) -> Result<ChartRecord, Error> {
    let (sql, value) = match key {
        AddressKey::Id(v) => (
            "SELECT id,scope,world_id,slug,revision FROM charts WHERE scope=? AND id=?",
            v,
        ),
        AddressKey::Slug(v) => (
            "SELECT id,scope,world_id,slug,revision FROM charts WHERE scope=? AND slug=?",
            v,
        ),
    };
    let r = sqlx::query(sql)
        .bind(scope(world))
        .bind(value)
        .fetch_optional(&mut *c)
        .await
        .map_err(storage_error)?
        .ok_or(Error::NotFound)?;
    let id: String = r.try_get("id").map_err(storage_error)?;
    let revision: i64 = r.try_get("revision").map_err(storage_error)?;
    let row = version(c, world, &id, revision)
        .await
        .map_err(|_| Error::Unavailable)?;
    let head_world: Option<String> = r.try_get("world_id").map_err(storage_error)?;
    let slug: String = r.try_get("slug").map_err(storage_error)?;
    if head_world != row.world_id || slug != row.fields.slug {
        return Err(Error::Unavailable);
    }
    Ok(row)
}
pub(crate) async fn detail(
    c: &mut SqliteConnection,
    world: Option<&str>,
    id: &str,
) -> Result<ChartRecord, Error> {
    resolve(c, world, AddressKey::Id(id.into())).await
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Snapshot {
    snapshot_version: i64,
    content: ChartRecord,
}
fn parse(s: &str, world: Option<&str>, id: &str, revision: i64) -> Result<ChartRecord, Error> {
    let snapshot: Snapshot = serde_json::from_str(s).map_err(|_| Error::Unavailable)?;
    let row = snapshot.content;
    row.validate().map_err(|_| Error::Unavailable)?;
    let v: serde_json::Value = serde_json::from_str(s).map_err(|_| Error::Unavailable)?;
    if snapshot.snapshot_version != 1
        || row.id != id
        || row.world_id.as_deref() != world
        || row.revision != revision
        || v != serde_json::json!({"snapshot_version":1,"content":row})
    {
        return Err(Error::Unavailable);
    }
    Ok(row)
}

/// Historical pins deliberately ignore current metadata/archive state.
pub(crate) async fn roster(
    c: &mut SqliteConnection,
    world: Option<&str>,
    pins: &[TypePin],
) -> Result<Vec<TypeAssignment>, Error> {
    let mut assignments = Vec::with_capacity(pins.len().min(MAX_CHART_TYPES));
    if pins.len() > MAX_CHART_TYPES {
        return Err(author::invalid("roster", "Choose at most 32 Types."));
    }
    let mut ids = std::collections::HashSet::new();
    for pin in pins {
        if !game_data::types::valid_id(&pin.type_id)
            || pin.version <= 0
            || !ids.insert(&pin.type_id)
        {
            return Err(author::invalid(
                "roster",
                "Choose distinct exact positive Type versions.",
            ));
        }
        let fields = if let Some(world) = world {
            let local = types::world_type_version(c, world, &pin.type_id, pin.version).await?;
            if let Some(source) = &local.source {
                types::type_version(c, &source.type_id, source.version).await?;
            }
            local.fields
        } else {
            types::type_version(c, &pin.type_id, pin.version)
                .await?
                .fields
        };
        assignments.push(TypeAssignment {
            pin: pin.clone(),
            fields,
        });
    }
    Ok(assignments)
}

pub(crate) async fn dependencies(c: &mut SqliteConnection, row: &ChartRecord) -> Result<(), Error> {
    let pins: Vec<_> = row.roster.iter().map(|a| a.pin.clone()).collect();
    if roster(c, row.world_id.as_deref(), &pins).await? != row.roster {
        return Err(Error::Unavailable);
    }
    if let Some(pin) = &row.source {
        // Finite direction: a local chart depends only on a shared chart, which
        // cannot itself have a source. Boxing breaks the async type cycle.
        let source = Box::pin(version(c, None, &pin.chart_id, pin.version)).await?;
        for m in &row.mapping {
            if !source
                .roster
                .iter()
                .any(|a| a.pin.type_id == m.source_type_id)
            {
                return Err(author::invalid(
                    "mapping",
                    "Choose a Type in the selected source roster.",
                ));
            }
            let local = row
                .roster
                .iter()
                .find(|a| a.pin.type_id == m.local_type_id)
                .ok_or_else(|| author::invalid("mapping", "Choose a local roster Type."))?;
            let t = types::world_type_version(
                c,
                row.world_id.as_deref().ok_or(Error::Unavailable)?,
                &local.pin.type_id,
                local.pin.version,
            )
            .await?;
            if t.source
                .as_ref()
                .is_none_or(|p| p.type_id != m.source_type_id)
            {
                return Err(author::invalid(
                    "mapping",
                    "The local Type must pin this shared Type identity; metadata versions may differ.",
                ));
            }
        }
    }
    Ok(())
}

async fn load(
    c: &mut SqliteConnection,
    world: Option<&str>,
    id: &str,
    revision: i64,
) -> Result<(ChartRecord, Option<String>, String), Error> {
    let r = sqlx::query("SELECT v.*, EXISTS(SELECT 1 FROM audit_events a WHERE a.id=v.audit_id) AS audit_present FROM chart_versions v WHERE scope=? AND chart_id=? AND revision=?")
        .bind(scope(world))
        .bind(id)
        .bind(revision)
        .fetch_optional(&mut *c)
        .await
        .map_err(storage_error)?
        .ok_or(Error::NotFound)?;
    let after: String = r.try_get("after_json").map_err(storage_error)?;
    let before: Option<String> = r.try_get("before_json").map_err(storage_error)?;
    let row = parse(&after, world, id, revision)?;
    let previous: Option<i64> = r.try_get("previous_revision").map_err(storage_error)?;
    let source_id: Option<String> = r.try_get("source_chart_id").map_err(storage_error)?;
    let source_revision: Option<i64> = r.try_get("source_revision").map_err(storage_error)?;
    let source_scope: String = r.try_get("source_scope").map_err(storage_error)?;
    let audit_present: bool = r.try_get("audit_present").map_err(storage_error)?;
    let fields = content::fields(&r)?;
    let archived: Option<i64> = r.try_get("archived_at").map_err(storage_error)?;
    if previous != content::previous(revision)
        || before.is_some() != (revision > 1)
        || row.fields != fields
        || row.archived_at != archived
        || source_id.as_deref() != row.source.as_ref().map(|p| p.chart_id.as_str())
        || source_revision != row.source.as_ref().map(|p| p.version)
        || !source_scope.is_empty()
        || !audit_present
    {
        return Err(Error::Unavailable);
    }
    if let Some(before) = &before {
        parse(before, world, id, revision - 1)?;
    }
    let rows = sqlx::query(
        "SELECT * FROM chart_roster WHERE scope=? AND chart_id=? AND revision=? ORDER BY slot",
    )
    .bind(scope(world))
    .bind(id)
    .bind(revision)
    .fetch_all(&mut *c)
    .await
    .map_err(storage_error)?;
    if rows.len() != row.roster.len() {
        return Err(Error::Unavailable);
    }
    for (i, (r, a)) in rows.iter().zip(&row.roster).enumerate() {
        let slot: i64 = r.try_get("slot").map_err(storage_error)?;
        let type_id: String = r.try_get("type_id").map_err(storage_error)?;
        let version: i64 = r.try_get("type_version").map_err(storage_error)?;
        let shared: Option<String> = r.try_get("shared_type_id").map_err(storage_error)?;
        let local: Option<String> = r.try_get("world_id").map_err(storage_error)?;
        if slot != (i + 1) as i64
            || type_id != a.pin.type_id
            || version != a.pin.version
            || local.as_deref() != world
            || shared.as_deref()
                != if world.is_none() {
                    Some(a.pin.type_id.as_str())
                } else {
                    None
                }
        {
            return Err(Error::Unavailable);
        }
    }
    let rows = sqlx::query("SELECT attacker_type_id,defender_type_id,numerator,denominator FROM chart_cells WHERE scope=? AND chart_id=? AND revision=? ORDER BY attacker_type_id,defender_type_id")
        .bind(scope(world)).bind(id).bind(revision).fetch_all(&mut *c).await.map_err(storage_error)?;
    if rows.len() != row.cells.len() {
        return Err(Error::Unavailable);
    }
    for (r, cell) in rows.iter().zip(&row.cells) {
        let attacker: String = r.try_get("attacker_type_id").map_err(storage_error)?;
        let defender: String = r.try_get("defender_type_id").map_err(storage_error)?;
        let n: i64 = r.try_get("numerator").map_err(storage_error)?;
        let d: i64 = r.try_get("denominator").map_err(storage_error)?;
        if attacker != cell.attacker_type_id
            || defender != cell.defender_type_id
            || n != i64::from(cell.factor.numerator)
            || d != i64::from(cell.factor.denominator)
        {
            return Err(Error::Unavailable);
        }
    }
    let rows = sqlx::query("SELECT * FROM chart_mapping WHERE scope=? AND chart_id=? AND revision=? ORDER BY source_type_id")
        .bind(scope(world)).bind(id).bind(revision).fetch_all(&mut *c).await.map_err(storage_error)?;
    if rows.len() != row.mapping.len() {
        return Err(Error::Unavailable);
    }
    for (r, m) in rows.iter().zip(&row.mapping) {
        let source: String = r.try_get("source_type_id").map_err(storage_error)?;
        let local: String = r.try_get("local_type_id").map_err(storage_error)?;
        let chart: String = r.try_get("source_chart_id").map_err(storage_error)?;
        let version: i64 = r.try_get("source_revision").map_err(storage_error)?;
        let source_scope: String = r.try_get("source_scope").map_err(storage_error)?;
        if source != m.source_type_id
            || local != m.local_type_id
            || !source_scope.is_empty()
            || row.source.as_ref()
                != Some(&ChartPin {
                    chart_id: chart,
                    version,
                })
        {
            return Err(Error::Unavailable);
        }
    }
    dependencies(c, &row)
        .await
        .map_err(|_| Error::Unavailable)?;
    Ok((row, before, after))
}
pub(crate) async fn version(
    c: &mut SqliteConnection,
    world: Option<&str>,
    id: &str,
    revision: i64,
) -> Result<ChartRecord, Error> {
    let (row, before, _) = load(c, world, id, revision).await?;
    let head = sqlx::query("SELECT world_id,slug,revision FROM charts WHERE scope=? AND id=?")
        .bind(scope(world))
        .bind(id)
        .fetch_optional(&mut *c)
        .await
        .map_err(storage_error)?
        .ok_or(Error::Unavailable)?;
    let head_world: Option<String> = head.try_get("world_id").map_err(storage_error)?;
    let head_revision: i64 = head.try_get("revision").map_err(storage_error)?;
    let head_slug: String = head.try_get("slug").map_err(storage_error)?;
    // Historical labels may differ after a legitimate rename; only the current
    // version must agree with the mutable head. Source reads use this path too.
    if head_world.as_deref() != world
        || head_revision < revision
        || (head_revision == revision && head_slug != row.fields.slug)
    {
        return Err(Error::Unavailable);
    }
    if revision > 1 {
        let (_, _, after) = load(c, world, id, revision - 1)
            .await
            .map_err(|_| Error::Unavailable)?;
        if before.as_deref() != Some(after.as_str()) {
            return Err(Error::Unavailable);
        }
    }
    Ok(row)
}
pub(crate) async fn after(
    c: &mut SqliteConnection,
    world: Option<&str>,
    id: &str,
    revision: i64,
) -> Result<String, Error> {
    version(c, world, id, revision).await?;
    sqlx::query_scalar(
        "SELECT after_json FROM chart_versions WHERE scope=? AND chart_id=? AND revision=?",
    )
    .bind(scope(world))
    .bind(id)
    .bind(revision)
    .fetch_one(c)
    .await
    .map_err(storage_error)
}

pub(crate) async fn put(
    c: &mut SqliteConnection,
    row: &ChartRecord,
    actor: Option<&str>,
    action: &str,
    reason: Option<&str>,
) -> Result<(), Error> {
    row.validate().map_err(Error::Invalid)?;
    dependencies(c, row).await?;
    let world = row.world_id.as_deref();
    let before = if row.revision > 1 {
        Some(after(c, world, &row.id, row.revision - 1).await?)
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
        "INSERT INTO charts(id,scope,world_id,slug,revision) VALUES (?1,?2,?3,?4,?5)"
    } else {
        "UPDATE charts SET slug=?4,revision=?5 WHERE id=?1 AND scope=?2 AND world_id IS ?3"
    };
    sqlx::query(sql)
        .bind(&row.id)
        .bind(scope(world))
        .bind(world)
        .bind(&row.fields.slug)
        .bind(row.revision)
        .execute(&mut *c)
        .await
        .map_err(content::content_write_error)?;
    for (i, a) in row.roster.iter().enumerate() {
        sqlx::query("INSERT INTO chart_roster(scope,chart_id,revision,slot,type_id,type_version,shared_type_id,world_id) VALUES (?,?,?,?,?,?,?,?)")
            .bind(scope(world)).bind(&row.id).bind(row.revision).bind((i+1) as i64).bind(&a.pin.type_id).bind(a.pin.version)
            .bind(if world.is_none(){Some(&a.pin.type_id)}else{None}).bind(world).execute(&mut *c).await.map_err(storage_error)?;
    }
    for cell in &row.cells {
        sqlx::query("INSERT INTO chart_cells(scope,chart_id,revision,attacker_type_id,defender_type_id,numerator,denominator) VALUES (?,?,?,?,?,?,?)")
            .bind(scope(world)).bind(&row.id).bind(row.revision).bind(&cell.attacker_type_id).bind(&cell.defender_type_id).bind(i64::from(cell.factor.numerator)).bind(i64::from(cell.factor.denominator)).execute(&mut *c).await.map_err(storage_error)?;
    }
    for m in &row.mapping {
        let pin = row.source.as_ref().ok_or(Error::Unavailable)?;
        sqlx::query("INSERT INTO chart_mapping(scope,chart_id,revision,source_chart_id,source_revision,source_type_id,local_type_id) VALUES (?,?,?,?,?,?,?)")
            .bind(scope(world)).bind(&row.id).bind(row.revision).bind(&pin.chart_id).bind(pin.version).bind(&m.source_type_id).bind(&m.local_type_id).execute(&mut *c).await.map_err(storage_error)?;
    }
    sqlx::query("INSERT INTO chart_versions(scope,chart_id,revision,source_chart_id,source_revision,slug,display_name,description,archived_at,audit_id,previous_revision,before_json,after_json) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?)")
        .bind(scope(world)).bind(&row.id).bind(row.revision).bind(row.source.as_ref().map(|p|&p.chart_id)).bind(row.source.as_ref().map(|p|p.version))
        .bind(&row.fields.slug).bind(&row.fields.display_name).bind(&row.fields.description).bind(row.archived_at).bind(audit).bind(content::previous(row.revision)).bind(before).bind(after)
        .execute(c).await.map_err(storage_error)?;
    Ok(())
}
pub(crate) async fn history(
    c: &mut SqliteConnection,
    world: Option<&str>,
    id: &str,
    before: Option<i64>,
    exact: Option<i64>,
) -> Result<Vec<HistoryEntry>, Error> {
    detail(c, world, id).await?;
    let rows: Vec<HistoryEntry> = sqlx::query_as("SELECT r.revision,r.previous_revision,coalesce(u.display_name,a.system_key) AS actor_name,a.action,a.occurred_at,a.reason,r.before_json,r.after_json FROM chart_versions r JOIN audit_events a ON a.id=r.audit_id LEFT JOIN users u ON u.id=a.actor_user_id WHERE r.scope=? AND r.chart_id=? AND (?3 IS NULL OR r.revision<?3) AND (?4 IS NULL OR r.revision=?4) ORDER BY r.revision DESC LIMIT 50")
        .bind(scope(world)).bind(id).bind(before).bind(exact).fetch_all(&mut *c).await.map_err(storage_error)?;
    for r in &rows {
        version(c, world, id, r.revision).await?;
        if r.actor_name.is_empty()
            || r.actor_name.chars().any(char::is_control)
            || !matches!(r.action.as_str(), "create" | "edit" | "adopt")
            || game_data::authoring::validate_reason(r.reason.as_deref())
                .ok()
                .as_ref()
                != Some(&r.reason)
        {
            return Err(Error::Unavailable);
        }
    }
    Ok(rows)
}
pub(crate) async fn fixture_actor(c: &mut SqliteConnection, id: &str) -> Result<bool, Error> {
    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM chart_versions r JOIN audit_events a ON a.id=r.audit_id WHERE r.chart_id=? AND r.revision=1 AND a.system_key='bundled-fixture-loader' AND a.actor_user_id IS NULL AND a.action='create' AND a.reason IS NULL)")
        .bind(id).fetch_one(c).await.map_err(storage_error)
}
pub(crate) async fn identity_exists(c: &mut SqliteConnection, id: &str) -> Result<bool, Error> {
    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM charts WHERE id=?)")
        .bind(id)
        .fetch_one(c)
        .await
        .map_err(storage_error)
}
pub(crate) async fn baseline_slug_exists(
    c: &mut SqliteConnection,
    world: Option<&str>,
    slug: &str,
) -> Result<bool, Error> {
    sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM chart_versions WHERE scope=? AND revision=1 AND slug=?)",
    )
    .bind(scope(world))
    .bind(slug)
    .fetch_one(c)
    .await
    .map_err(storage_error)
}
