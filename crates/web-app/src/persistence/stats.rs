//! Exact stat and roster revisions. Call writers on the owning locked transaction.
use super::authoring::{self as content, storage_error};
use crate::application::authoring::{self as author, AddressKey, Error, HistoryEntry};
use game_data::stats::*;
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
fn parse<T: DeserializeOwned + Serialize>(s: &str) -> Result<T, Error> {
    let snapshot: Snapshot<T> = serde_json::from_str(s).map_err(|_| Error::Unavailable)?;
    let value: serde_json::Value = serde_json::from_str(s).map_err(|_| Error::Unavailable)?;
    if snapshot.snapshot_version != 1
        || value != serde_json::json!({"snapshot_version":1,"content":snapshot.content})
    {
        return Err(Error::Unavailable);
    }
    Ok(snapshot.content)
}
pub(crate) async fn slug_exists(
    c: &mut SqliteConnection,
    world: Option<&str>,
    slug: &str,
    except: Option<&str>,
) -> Result<bool, Error> {
    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM stat_definitions WHERE scope=?1 AND slug=?2 AND (?3 IS NULL OR id<>?3))")
        .bind(scope(world)).bind(slug).bind(except).fetch_one(c).await.map_err(storage_error)
}
pub(crate) async fn ids(
    c: &mut SqliteConnection,
    world: Option<&str>,
) -> Result<Vec<String>, Error> {
    sqlx::query_scalar("SELECT d.id FROM stat_definitions d LEFT JOIN stat_versions v ON v.scope=d.scope AND v.stat_id=d.id AND v.revision=d.revision WHERE d.scope=? ORDER BY v.display_name,d.id")
        .bind(scope(world)).fetch_all(c).await.map_err(storage_error)
}
pub(crate) async fn resolve(
    c: &mut SqliteConnection,
    world: Option<&str>,
    key: AddressKey,
) -> Result<StatDefinition, Error> {
    let (sql, value) = match key {
        AddressKey::Id(v) => ("SELECT id FROM stat_definitions WHERE scope=? AND id=?", v),
        AddressKey::Slug(v) => (
            "SELECT id FROM stat_definitions WHERE scope=? AND slug=?",
            v,
        ),
    };
    let id: String = sqlx::query_scalar(sql)
        .bind(scope(world))
        .bind(value)
        .fetch_optional(&mut *c)
        .await
        .map_err(storage_error)?
        .ok_or(Error::NotFound)?;
    definition(c, world, &id).await
}
pub(crate) async fn definition(
    c: &mut SqliteConnection,
    world: Option<&str>,
    id: &str,
) -> Result<StatDefinition, Error> {
    let revision: i64 =
        sqlx::query_scalar("SELECT revision FROM stat_definitions WHERE scope=? AND id=?")
            .bind(scope(world))
            .bind(id)
            .fetch_optional(&mut *c)
            .await
            .map_err(storage_error)?
            .ok_or(Error::NotFound)?;
    definition_version(c, world, id, revision)
        .await
        .map_err(|_| Error::Unavailable)
}
pub(crate) async fn select_definition(
    c: &mut SqliteConnection,
    world: Option<&str>,
    pin: &StatPin,
) -> Result<StatDefinition, Error> {
    pin.validate().map_err(Error::Invalid)?;
    let head = definition(c, world, &pin.stat_id).await?;
    let row = definition_version(c, world, &pin.stat_id, pin.version).await?;
    if head.archived_at.is_some() || row.archived_at.is_some() {
        return Err(author::invalid(
            "stat",
            "Choose an unarchived stat revision.",
        ));
    }
    Ok(row)
}
fn decimal(r: &SqliteRow, coefficient: &str, scale: &str) -> Result<Option<Decimal>, Error> {
    let n: Option<i64> = r.try_get(coefficient).map_err(storage_error)?;
    let s: Option<i64> = r.try_get(scale).map_err(storage_error)?;
    match (n, s) {
        (None, None) => Ok(None),
        (Some(n), Some(s)) => {
            let s = u8::try_from(s).map_err(|_| Error::Unavailable)?;
            let d = Decimal::new(n, s).map_err(|_| Error::Unavailable)?;
            if d.coefficient() != n || d.scale() != s {
                return Err(Error::Unavailable);
            }
            Ok(Some(d))
        }
        _ => Err(Error::Unavailable),
    }
}
fn continuity(r: &SqliteRow, revision: i64) -> Result<(Option<String>, String), Error> {
    let before: Option<String> = r.try_get("before_json").map_err(storage_error)?;
    let after: String = r.try_get("after_json").map_err(storage_error)?;
    let previous: Option<i64> = r.try_get("previous_revision").map_err(storage_error)?;
    let audit: bool = r.try_get("audit_present").map_err(storage_error)?;
    if revision <= 0
        || previous != content::previous(revision)
        || before.is_some() != (revision > 1)
        || !audit
    {
        return Err(Error::Unavailable);
    }
    Ok((before, after))
}
pub(crate) async fn definition_dependencies(
    c: &mut SqliteConnection,
    row: &StatDefinition,
) -> Result<(), Error> {
    if let Some(pin) = &row.source {
        Box::pin(definition_version(c, None, &pin.stat_id, pin.version)).await?;
    }
    if let Some(world) = &row.world_id {
        content::world(c, world).await?;
    }
    Ok(())
}
async fn load_definition(
    c: &mut SqliteConnection,
    world: Option<&str>,
    id: &str,
    revision: i64,
) -> Result<(StatDefinition, Option<String>, String), Error> {
    let r=sqlx::query("SELECT v.*,EXISTS(SELECT 1 FROM audit_events a WHERE a.id=v.audit_id) AS audit_present FROM stat_versions v WHERE scope=? AND stat_id=? AND revision=?")
        .bind(scope(world)).bind(id).bind(revision).fetch_optional(&mut *c).await.map_err(storage_error)?.ok_or(Error::NotFound)?;
    let (before, after) = continuity(&r, revision)?;
    let row: StatDefinition = parse(&after)?;
    row.validate().map_err(|_| Error::Unavailable)?;
    let source_scope: String = r.try_get("source_scope").map_err(storage_error)?;
    let source_id: Option<String> = r.try_get("source_stat_id").map_err(storage_error)?;
    let source_revision: Option<i64> = r.try_get("source_revision").map_err(storage_error)?;
    let archived: Option<i64> = r.try_get("archived_at").map_err(storage_error)?;
    let whole: i64 = r.try_get("whole_only").map_err(storage_error)?;
    let constraints = NumericConstraints {
        whole_only: whole == 1,
        minimum: decimal(&r, "minimum_coefficient", "minimum_scale")?,
        maximum: decimal(&r, "maximum_coefficient", "maximum_scale")?,
    };
    if row.id != id
        || row.world_id.as_deref() != world
        || row.revision != revision
        || row.fields != content::fields(&r)?
        || row.archived_at != archived
        || !source_scope.is_empty()
        || source_id.as_deref() != row.source.as_ref().map(|p| p.stat_id.as_str())
        || source_revision != row.source.as_ref().map(|p| p.version)
        || !(0..=1).contains(&whole)
        || row.constraints != constraints
    {
        return Err(Error::Unavailable);
    }
    definition_dependencies(c, &row)
        .await
        .map_err(|_| Error::Unavailable)?;
    Ok((row, before, after))
}
pub(crate) async fn definition_version(
    c: &mut SqliteConnection,
    world: Option<&str>,
    id: &str,
    version: i64,
) -> Result<StatDefinition, Error> {
    let (row, before, _) = load_definition(c, world, id, version).await?;
    let head =
        sqlx::query("SELECT world_id,slug,revision FROM stat_definitions WHERE scope=? AND id=?")
            .bind(scope(world))
            .bind(id)
            .fetch_optional(&mut *c)
            .await
            .map_err(storage_error)?
            .ok_or(Error::Unavailable)?;
    let head_world: Option<String> = head.try_get("world_id").map_err(storage_error)?;
    let slug: String = head.try_get("slug").map_err(storage_error)?;
    let revision: i64 = head.try_get("revision").map_err(storage_error)?;
    if head_world.as_deref() != world || revision < version {
        return Err(Error::Unavailable);
    }
    let (current, current_before) = if revision == version {
        (row.clone(), before.clone())
    } else {
        let (r, b, _) = load_definition(c, world, id, revision)
            .await
            .map_err(|_| Error::Unavailable)?;
        (r, b)
    };
    if current.fields.slug != slug {
        return Err(Error::Unavailable);
    }
    for (v, b) in [(version, before), (revision, current_before)] {
        if v > 1 {
            let (_, _, previous) = load_definition(c, world, id, v - 1)
                .await
                .map_err(|_| Error::Unavailable)?;
            if b.as_deref() != Some(previous.as_str()) {
                return Err(Error::Unavailable);
            }
        }
    }
    Ok(row)
}
pub(crate) async fn roster(
    c: &mut SqliteConnection,
    world: &str,
) -> Result<Option<StatRoster>, Error> {
    content::world(c, world).await?;
    let head: Option<(String, i64)> =
        sqlx::query_as("SELECT id,revision FROM stat_rosters WHERE world_id=?")
            .bind(world)
            .fetch_optional(&mut *c)
            .await
            .map_err(storage_error)?;
    match head {
        Some((id, v)) => Ok(Some(
            roster_version(c, world, &id, v)
                .await
                .map_err(|_| Error::Unavailable)?,
        )),
        None => Ok(None),
    }
}
pub(crate) async fn roster_dependencies(
    c: &mut SqliteConnection,
    row: &StatRoster,
) -> Result<(), Error> {
    content::world(c, &row.world_id).await?;
    for entry in &row.entries {
        if definition_version(c, Some(&row.world_id), &entry.id, entry.revision).await? != *entry {
            return Err(Error::Unavailable);
        }
    }
    Ok(())
}
async fn load_roster(
    c: &mut SqliteConnection,
    world: &str,
    id: &str,
    version: i64,
) -> Result<(StatRoster, Option<String>, String), Error> {
    let r=sqlx::query("SELECT v.*,EXISTS(SELECT 1 FROM audit_events a WHERE a.id=v.audit_id) AS audit_present FROM stat_roster_versions v WHERE world_id=? AND roster_id=? AND revision=?")
        .bind(world).bind(id).bind(version).fetch_optional(&mut *c).await.map_err(storage_error)?.ok_or(Error::NotFound)?;
    let (before, after) = continuity(&r, version)?;
    let row: StatRoster = parse(&after)?;
    row.validate().map_err(|_| Error::Unavailable)?;
    if row.id != id || row.world_id != world || row.revision != version {
        return Err(Error::Unavailable);
    }
    let entries=sqlx::query("SELECT slot,scope,stat_id,stat_version FROM stat_roster_entries WHERE world_id=? AND roster_id=? AND revision=? ORDER BY slot")
        .bind(world).bind(id).bind(version).fetch_all(&mut *c).await.map_err(storage_error)?;
    if entries.len() != row.entries.len() {
        return Err(Error::Unavailable);
    }
    for (i, (r, entry)) in entries.iter().zip(&row.entries).enumerate() {
        let slot: i64 = r.try_get("slot").map_err(storage_error)?;
        let scope: String = r.try_get("scope").map_err(storage_error)?;
        let stat: String = r.try_get("stat_id").map_err(storage_error)?;
        let revision: i64 = r.try_get("stat_version").map_err(storage_error)?;
        if slot != i64::try_from(i + 1).map_err(|_| Error::Unavailable)?
            || scope != world
            || stat != entry.id
            || revision != entry.revision
        {
            return Err(Error::Unavailable);
        }
    }
    roster_dependencies(c, &row)
        .await
        .map_err(|_| Error::Unavailable)?;
    Ok((row, before, after))
}
pub(crate) async fn roster_version(
    c: &mut SqliteConnection,
    world: &str,
    id: &str,
    version: i64,
) -> Result<StatRoster, Error> {
    let (row, before, _) = load_roster(c, world, id, version).await?;
    let head: Option<(String, i64)> =
        sqlx::query_as("SELECT id,revision FROM stat_rosters WHERE world_id=?")
            .bind(world)
            .fetch_optional(&mut *c)
            .await
            .map_err(storage_error)?;
    let (head_id, revision) = head.ok_or(Error::Unavailable)?;
    if head_id != id || revision < version {
        return Err(Error::Unavailable);
    }
    let current_before = if revision == version {
        before.clone()
    } else {
        load_roster(c, world, id, revision)
            .await
            .map_err(|_| Error::Unavailable)?
            .1
    };
    for (v, b) in [(version, before), (revision, current_before)] {
        if v > 1 {
            let (_, _, previous) = load_roster(c, world, id, v - 1)
                .await
                .map_err(|_| Error::Unavailable)?;
            if b.as_deref() != Some(previous.as_str()) {
                return Err(Error::Unavailable);
            }
        }
    }
    Ok(row)
}

pub(crate) async fn put_definition(
    c: &mut SqliteConnection,
    row: &StatDefinition,
    actor: Option<&str>,
    action: &str,
    reason: Option<&str>,
) -> Result<(), Error> {
    row.validate().map_err(Error::Invalid)?;
    definition_dependencies(c, row).await?;
    let world = row.world_id.as_deref();
    let before = if row.revision > 1 {
        let old = definition(c, world, &row.id).await?;
        author::expected(author::next(old.revision)?, row.revision)?;
        Some(definition_after(c, world, &row.id, old.revision).await?)
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
        "INSERT INTO stat_definitions(id,scope,world_id,slug,revision) VALUES (?1,?2,?3,?4,?5)"
    } else {
        "UPDATE stat_definitions SET slug=?4,revision=?5 WHERE id=?1 AND scope=?2 AND world_id IS ?3 AND revision=?5-1"
    };
    let result = sqlx::query(sql)
        .bind(&row.id)
        .bind(scope(world))
        .bind(world)
        .bind(&row.fields.slug)
        .bind(row.revision)
        .execute(&mut *c)
        .await
        .map_err(content::content_write_error)?;
    if result.rows_affected() != 1 {
        return Err(Error::Conflict);
    }
    sqlx::query("INSERT INTO stat_versions(scope,stat_id,revision,source_stat_id,source_revision,slug,display_name,description,archived_at,whole_only,minimum_coefficient,minimum_scale,maximum_coefficient,maximum_scale,audit_id,previous_revision,before_json,after_json) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)")
        .bind(scope(world)).bind(&row.id).bind(row.revision).bind(row.source.as_ref().map(|p| &p.stat_id)).bind(row.source.as_ref().map(|p| p.version))
        .bind(&row.fields.slug).bind(&row.fields.display_name).bind(&row.fields.description).bind(row.archived_at).bind(i64::from(row.constraints.whole_only))
        .bind(row.constraints.minimum.map(Decimal::coefficient)).bind(row.constraints.minimum.map(|d| i64::from(d.scale())))
        .bind(row.constraints.maximum.map(Decimal::coefficient)).bind(row.constraints.maximum.map(|d| i64::from(d.scale())))
        .bind(audit).bind(content::previous(row.revision)).bind(before).bind(after).execute(c).await.map_err(storage_error)?;
    Ok(())
}
async fn definition_after(
    c: &mut SqliteConnection,
    world: Option<&str>,
    id: &str,
    revision: i64,
) -> Result<String, Error> {
    definition_version(c, world, id, revision).await?;
    sqlx::query_scalar(
        "SELECT after_json FROM stat_versions WHERE scope=? AND stat_id=? AND revision=?",
    )
    .bind(scope(world))
    .bind(id)
    .bind(revision)
    .fetch_one(c)
    .await
    .map_err(storage_error)
}
pub(crate) async fn put_roster(
    c: &mut SqliteConnection,
    row: &StatRoster,
    actor: Option<&str>,
    reason: Option<&str>,
) -> Result<(), Error> {
    row.validate().map_err(Error::Invalid)?;
    roster_dependencies(c, row).await?;
    let before = if row.revision > 1 {
        let old = roster(c, &row.world_id).await?.ok_or(Error::NotFound)?;
        if old.id != row.id {
            return Err(Error::Unavailable);
        }
        author::expected(author::next(old.revision)?, row.revision)?;
        Some(sqlx::query_scalar::<_,String>("SELECT after_json FROM stat_roster_versions WHERE world_id=? AND roster_id=? AND revision=?").bind(&row.world_id).bind(&row.id).bind(old.revision).fetch_one(&mut *c).await.map_err(storage_error)?)
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
        "INSERT INTO stat_rosters(world_id,id,revision) VALUES (?1,?2,?3)"
    } else {
        "UPDATE stat_rosters SET revision=?3 WHERE world_id=?1 AND id=?2 AND revision=?3-1"
    };
    let result = sqlx::query(sql)
        .bind(&row.world_id)
        .bind(&row.id)
        .bind(row.revision)
        .execute(&mut *c)
        .await
        .map_err(content::content_write_error)?;
    if result.rows_affected() != 1 {
        return Err(Error::Conflict);
    }
    for (i, entry) in row.entries.iter().enumerate() {
        sqlx::query("INSERT INTO stat_roster_entries(world_id,roster_id,revision,slot,scope,stat_id,stat_version) VALUES (?,?,?,?,?,?,?)")
            .bind(&row.world_id).bind(&row.id).bind(row.revision).bind(i64::try_from(i+1).map_err(|_| Error::Unavailable)?)
            .bind(&row.world_id).bind(&entry.id).bind(entry.revision).execute(&mut *c).await.map_err(storage_error)?;
    }
    sqlx::query("INSERT INTO stat_roster_versions(world_id,roster_id,revision,audit_id,previous_revision,before_json,after_json) VALUES (?,?,?,?,?,?,?)")
        .bind(&row.world_id).bind(&row.id).bind(row.revision).bind(audit).bind(content::previous(row.revision)).bind(before).bind(after).execute(c).await.map_err(storage_error)?;
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
pub(crate) async fn history(
    c: &mut SqliteConnection,
    world: Option<&str>,
    id: &str,
    before: Option<i64>,
    exact: Option<i64>,
) -> Result<Vec<HistoryEntry>, Error> {
    definition(c, world, id).await?;
    let rows:Vec<HistoryEntry>=sqlx::query_as("SELECT r.revision,r.previous_revision,coalesce(u.display_name,a.system_key) AS actor_name,a.action,a.occurred_at,a.reason,r.before_json,r.after_json FROM stat_versions r JOIN audit_events a ON a.id=r.audit_id LEFT JOIN users u ON u.id=a.actor_user_id WHERE r.scope=?1 AND r.stat_id=?2 AND (?3 IS NULL OR r.revision<?3) AND (?4 IS NULL OR r.revision=?4) ORDER BY r.revision DESC LIMIT 50")
        .bind(scope(world)).bind(id).bind(before).bind(exact).fetch_all(&mut *c).await.map_err(storage_error)?;
    for row in &rows {
        definition_version(c, world, id, row.revision).await?;
    }
    validate_history(&rows)?;
    Ok(rows)
}
pub(crate) async fn roster_history(
    c: &mut SqliteConnection,
    world: &str,
    before: Option<i64>,
) -> Result<Vec<HistoryEntry>, Error> {
    let Some(head) = roster(c, world).await? else {
        return Ok(vec![]);
    };
    let rows:Vec<HistoryEntry>=sqlx::query_as("SELECT r.revision,r.previous_revision,coalesce(u.display_name,a.system_key) AS actor_name,a.action,a.occurred_at,a.reason,r.before_json,r.after_json FROM stat_roster_versions r JOIN audit_events a ON a.id=r.audit_id LEFT JOIN users u ON u.id=a.actor_user_id WHERE r.world_id=?1 AND r.roster_id=?2 AND (?3 IS NULL OR r.revision<?3) ORDER BY r.revision DESC LIMIT 50")
        .bind(world).bind(&head.id).bind(before).fetch_all(&mut *c).await.map_err(storage_error)?;
    for row in &rows {
        roster_version(c, world, &head.id, row.revision).await?;
    }
    validate_history(&rows)?;
    Ok(rows)
}
pub(crate) async fn definition_fixture_actor(
    c: &mut SqliteConnection,
    id: &str,
) -> Result<bool, Error> {
    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM stat_versions r JOIN audit_events a ON a.id=r.audit_id WHERE r.stat_id=? AND r.revision=1 AND a.system_key='bundled-fixture-loader' AND a.actor_user_id IS NULL AND a.action='create' AND a.reason IS NULL)")
        .bind(id).fetch_one(c).await.map_err(storage_error)
}
pub(crate) async fn roster_fixture_actor(
    c: &mut SqliteConnection,
    id: &str,
) -> Result<bool, Error> {
    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM stat_roster_versions r JOIN audit_events a ON a.id=r.audit_id WHERE r.roster_id=? AND r.revision=1 AND a.system_key='bundled-fixture-loader' AND a.actor_user_id IS NULL AND a.action='create' AND a.reason IS NULL)")
        .bind(id).fetch_one(c).await.map_err(storage_error)
}
pub(crate) async fn identity_exists(c: &mut SqliteConnection, id: &str) -> Result<bool, Error> {
    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM stat_definitions WHERE id=?)")
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
        "SELECT EXISTS(SELECT 1 FROM stat_versions WHERE scope=? AND revision=1 AND slug=?)",
    )
    .bind(scope(world))
    .bind(slug)
    .fetch_one(c)
    .await
    .map_err(storage_error)
}
