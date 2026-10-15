//! Immutable v3 creature seals; v1 metadata and v2 typing remain projections.
use super::{authoring::storage_error, stats};
use crate::application::authoring::{self as author, Error, HistoryKind};
use game_data::stats::{BaseStats, Decimal, RosterPin, StatValue};
use serde_json::Value;
use sqlx::{Row, SqliteConnection};

fn invalid() -> Error {
    author::invalid(
        "stats",
        "Use exact Stat definitions and the complete roster from this scope, with values satisfying their pinned constraints.",
    )
}

/// Validate historical dependencies, deliberately independent of current archive state.
pub(crate) async fn validate(
    c: &mut SqliteConnection,
    kind: HistoryKind,
    world: Option<&str>,
    state: &BaseStats,
) -> Result<(), Error> {
    state.validate().map_err(|_| invalid())?;
    let scope = match kind {
        HistoryKind::Form if world.is_none() && state.roster.is_none() => None,
        HistoryKind::Variant if world.is_some() => world,
        _ => return Err(invalid()),
    };
    let Some(values) = &state.assignments else {
        return if state.roster.is_none() {
            Ok(())
        } else {
            Err(invalid())
        };
    };
    for value in values {
        if value.definition.world_id.as_deref() != scope
            || stats::definition_version(c, scope, &value.definition.id, value.definition.revision)
                .await
                .map_err(|e| author::missing_reference(e, "stats"))?
                != value.definition
        {
            return Err(invalid());
        }
    }
    if let Some(world) = scope {
        let pin = state.roster.as_ref().ok_or_else(invalid)?;
        let roster = stats::roster_version(c, world, &pin.roster_id, pin.version)
            .await
            .map_err(|e| author::missing_reference(e, "stats"))?;
        if roster.world_id != world
            || roster.id != pin.roster_id
            || roster.revision != pin.version
            || roster.entries.len() != values.len()
            || roster
                .entries
                .iter()
                .zip(values)
                .any(|(a, b)| a != &b.definition)
        {
            return Err(invalid());
        }
    }
    Ok(())
}

fn tables(kind: HistoryKind) -> Result<(&'static str, &'static str, &'static str), Error> {
    match kind {
        HistoryKind::Form => Ok(("form_stat_snapshots", "form_stat_bindings", "form_id")),
        HistoryKind::Variant => Ok((
            "variant_stat_snapshots",
            "variant_stat_bindings",
            "variant_id",
        )),
        _ => Err(Error::Unavailable),
    }
}

pub(crate) struct StatContent {
    pub(crate) before: Option<String>,
    pub(crate) after: String,
    pub(crate) stats: BaseStats,
}

/// Discriminator dispatch is fail-closed, including unsolicited legacy extensions.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn load(
    c: &mut SqliteConnection,
    kind: HistoryKind,
    id: &str,
    world: Option<&str>,
    revision: i64,
    marker: i64,
    projection: &str,
) -> Result<Option<StatContent>, Error> {
    let (sidecar, bindings, key) = tables(kind)?;
    let filter = if matches!(kind, HistoryKind::Variant) {
        "AND world_id=?3"
    } else {
        "AND (?3 IS NULL OR ?3 IS NOT NULL)"
    };
    let sql = format!("SELECT * FROM {sidecar} WHERE {key}=?1 AND revision=?2 {filter}");
    let side = sqlx::query(sqlx::AssertSqlSafe(sql))
        .bind(id)
        .bind(revision)
        .bind(world)
        .fetch_optional(&mut *c)
        .await
        .map_err(storage_error)?;
    let sql =
        format!("SELECT * FROM {bindings} WHERE {key}=?1 AND revision=?2 {filter} ORDER BY slot");
    let rows = sqlx::query(sqlx::AssertSqlSafe(sql))
        .bind(id)
        .bind(revision)
        .bind(world)
        .fetch_all(&mut *c)
        .await
        .map_err(storage_error)?;
    if marker == 0 {
        return if side.is_none() && rows.is_empty() {
            Ok(None)
        } else {
            Err(Error::Unavailable)
        };
    }
    if marker != 3 {
        return Err(Error::Unavailable);
    }
    let side = side.ok_or(Error::Unavailable)?;
    if side.try_get::<i64, _>("marker").map_err(storage_error)? != 3 {
        return Err(Error::Unavailable);
    }
    let before: Option<String> = side.try_get("before_json").map_err(storage_error)?;
    let after: String = side.try_get("after_json").map_err(storage_error)?;
    if before.is_some() != (revision > 1) {
        return Err(Error::Unavailable);
    }
    let mut v: Value = serde_json::from_str(&after).map_err(|_| Error::Unavailable)?;
    if v.as_object()
        .is_none_or(|m| m.len() != 2 || !m.contains_key("content"))
        || v["snapshot_version"] != 3
    {
        return Err(Error::Unavailable);
    }
    let raw = v["content"]
        .as_object_mut()
        .ok_or(Error::Unavailable)?
        .remove("stats")
        .ok_or(Error::Unavailable)?;
    let state: BaseStats = serde_json::from_value(raw.clone()).map_err(|_| Error::Unavailable)?;
    // Decimal deserialization may normalize author input; stored snapshots must already
    // be canonical, with all nullable fields present and no unknown fields anywhere.
    if serde_json::to_value(&state).map_err(|_| Error::Unavailable)? != raw {
        return Err(Error::Unavailable);
    }
    v["snapshot_version"] = 2.into();
    if v != serde_json::from_str::<Value>(projection).map_err(|_| Error::Unavailable)? {
        return Err(Error::Unavailable);
    }
    let defined: i64 = side.try_get("stats_defined").map_err(storage_error)?;
    if defined != i64::from(state.assignments.is_some()) {
        return Err(Error::Unavailable);
    }
    let roster = if matches!(kind, HistoryKind::Variant) {
        match (
            side.try_get::<Option<String>, _>("roster_id")
                .map_err(storage_error)?,
            side.try_get::<Option<i64>, _>("roster_version")
                .map_err(storage_error)?,
        ) {
            (None, None) => None,
            (Some(roster_id), Some(version)) => Some(RosterPin { roster_id, version }),
            _ => return Err(Error::Unavailable),
        }
    } else {
        None
    };
    if roster != state.roster {
        return Err(Error::Unavailable);
    }
    let mut values = Vec::with_capacity(rows.len());
    for (index, row) in rows.into_iter().enumerate() {
        if row.try_get::<i64, _>("slot").map_err(storage_error)? != (index + 1) as i64
            || row.try_get::<String, _>("scope").map_err(storage_error)? != world.unwrap_or("")
        {
            return Err(Error::Unavailable);
        }
        let stat: String = row.try_get("stat_id").map_err(storage_error)?;
        let version: i64 = row.try_get("stat_version").map_err(storage_error)?;
        let definition = stats::definition_version(c, world, &stat, version)
            .await
            .map_err(|_| Error::Unavailable)?;
        let value = match (
            row.try_get::<Option<i64>, _>("coefficient")
                .map_err(storage_error)?,
            row.try_get::<Option<i64>, _>("scale")
                .map_err(storage_error)?,
        ) {
            (None, None) => None,
            (Some(coefficient), Some(scale)) => {
                let scale = u8::try_from(scale).map_err(|_| Error::Unavailable)?;
                let decimal = Decimal::new(coefficient, scale).map_err(|_| Error::Unavailable)?;
                if decimal.coefficient() != coefficient || decimal.scale() != scale {
                    return Err(Error::Unavailable);
                }
                Some(decimal)
            }
            _ => return Err(Error::Unavailable),
        };
        values.push(StatValue { definition, value });
    }
    if state.assignments.as_deref().unwrap_or(&[]) != values {
        return Err(Error::Unavailable);
    }
    validate(c, kind, world, &state)
        .await
        .map_err(|_| Error::Unavailable)?;
    Ok(Some(StatContent {
        before,
        after,
        stats: state,
    }))
}

pub(crate) async fn seal(
    c: &mut SqliteConnection,
    kind: HistoryKind,
    id: &str,
    world: Option<&str>,
    revision: i64,
    content: &StatContent,
) -> Result<(), Error> {
    let (sidecar, bindings, key) = tables(kind)?;
    for (index, value) in content
        .stats
        .assignments
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .enumerate()
    {
        let sql = if world.is_some() {
            format!(
                "INSERT INTO {bindings}({key},revision,slot,scope,stat_id,stat_version,coefficient,scale,world_id) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?4)"
            )
        } else {
            format!(
                "INSERT INTO {bindings}({key},revision,slot,scope,stat_id,stat_version,coefficient,scale) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)"
            )
        };
        sqlx::query(sqlx::AssertSqlSafe(sql))
            .bind(id)
            .bind(revision)
            .bind((index + 1) as i64)
            .bind(world.unwrap_or(""))
            .bind(&value.definition.id)
            .bind(value.definition.revision)
            .bind(value.value.map(Decimal::coefficient))
            .bind(value.value.map(|v| i64::from(v.scale())))
            .execute(&mut *c)
            .await
            .map_err(storage_error)?;
    }
    let sql = if world.is_some() {
        format!(
            "INSERT INTO {sidecar}({key},revision,stats_defined,before_json,after_json,world_id,roster_id,roster_version) VALUES (?,?,?,?,?,?,?,?)"
        )
    } else {
        format!(
            "INSERT INTO {sidecar}({key},revision,stats_defined,before_json,after_json) VALUES (?,?,?,?,?)"
        )
    };
    let query = sqlx::query(sqlx::AssertSqlSafe(sql))
        .bind(id)
        .bind(revision)
        .bind(content.stats.assignments.is_some())
        .bind(&content.before)
        .bind(&content.after);
    let query = if world.is_some() {
        query
            .bind(world)
            .bind(content.stats.roster.as_ref().map(|p| &p.roster_id))
            .bind(content.stats.roster.as_ref().map(|p| p.version))
    } else {
        query
    };
    query.execute(c).await.map_err(storage_error)?;
    Ok(())
}
