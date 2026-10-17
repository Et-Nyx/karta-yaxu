//! Typed move aggregates. Writers require the caller's locked transaction.
use super::{
    authoring::{self as content, storage_error},
    charts, stats,
};
use crate::application::authoring::{self as author, AddressKey, Error, HistoryEntry};
use game_data::{authoring::ContentFields, moves::*, rulesets::*};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sqlx::{AssertSqlSafe, Row, SqliteConnection, sqlite::SqliteRow};

#[derive(Clone, Copy)]
pub(crate) enum Kind {
    Attribute,
    Move,
    Ruleset,
}
impl Kind {
    fn head(self) -> &'static str {
        match self {
            Self::Attribute => "move_attribute_definitions",
            Self::Move => "moves",
            Self::Ruleset => "world_rulesets",
        }
    }
    fn versions(self) -> &'static str {
        match self {
            Self::Attribute => "move_attribute_versions",
            Self::Move => "move_versions",
            Self::Ruleset => "world_ruleset_versions",
        }
    }
    fn key(self) -> &'static str {
        match self {
            Self::Attribute => "attribute_id",
            Self::Move => "move_id",
            Self::Ruleset => "ruleset_id",
        }
    }
    fn scope(self) -> &'static str {
        match self {
            Self::Ruleset => "world_id",
            _ => "scope",
        }
    }
}
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
    let x: Snapshot<T> = serde_json::from_str(s).map_err(|_| Error::Unavailable)?;
    let v: serde_json::Value = serde_json::from_str(s).map_err(|_| Error::Unavailable)?;
    if x.snapshot_version != 1 || v != serde_json::json!({"snapshot_version":1,"content":x.content})
    {
        return Err(Error::Unavailable);
    }
    Ok(x.content)
}
fn get<T: for<'r> sqlx::Decode<'r, sqlx::Sqlite> + sqlx::Type<sqlx::Sqlite>>(
    r: &SqliteRow,
    name: &str,
) -> Result<T, Error> {
    r.try_get(name).map_err(storage_error)
}
fn decimal(r: &SqliteRow, n: &str, s: &str) -> Result<Option<Decimal>, Error> {
    match (get::<Option<i64>>(r, n)?, get::<Option<i64>>(r, s)?) {
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
fn boolean(r: &SqliteRow, name: &str) -> Result<bool, Error> {
    match get::<i64>(r, name)? {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(Error::Unavailable),
    }
}
fn constraints(r: &SqliteRow) -> Result<NumericConstraints, Error> {
    Ok(NumericConstraints {
        whole_only: boolean(r, "whole_only")?,
        minimum: decimal(r, "minimum_coefficient", "minimum_scale")?,
        maximum: decimal(r, "maximum_coefficient", "maximum_scale")?,
    })
}
fn require(ok: bool) -> Result<(), Error> {
    if ok { Ok(()) } else { Err(Error::Unavailable) }
}
pub(crate) async fn ids(
    c: &mut SqliteConnection,
    kind: Kind,
    world: Option<&str>,
) -> Result<Vec<String>, Error> {
    // Interpolated identifiers in this module come exclusively from Kind or literal
    // column choices. Every caller-supplied value is a bound parameter.
    sqlx::query_scalar(AssertSqlSafe(format!("SELECT h.id FROM {} h LEFT JOIN {} v ON v.{}=h.{} AND v.{}=h.id AND v.revision=h.revision WHERE h.{}=? ORDER BY v.display_name,h.id",kind.head(),kind.versions(),kind.scope(),kind.scope(),kind.key(),kind.scope())))
        .bind(scope(world)).fetch_all(c).await.map_err(storage_error)
}
pub(crate) async fn slug_exists(
    c: &mut SqliteConnection,
    kind: Kind,
    world: Option<&str>,
    slug: &str,
    except: Option<&str>,
) -> Result<bool, Error> {
    sqlx::query_scalar(AssertSqlSafe(format!(
        "SELECT EXISTS(SELECT 1 FROM {} WHERE {}=?1 AND slug=?2 AND (?3 IS NULL OR id<>?3))",
        kind.head(),
        kind.scope()
    )))
    .bind(scope(world))
    .bind(slug)
    .bind(except)
    .fetch_one(c)
    .await
    .map_err(storage_error)
}
pub(crate) async fn resolve_id(
    c: &mut SqliteConnection,
    kind: Kind,
    world: Option<&str>,
    key: AddressKey,
) -> Result<String, Error> {
    let (column, value) = match key {
        AddressKey::Id(v) => ("id", v),
        AddressKey::Slug(v) => ("slug", v),
    };
    sqlx::query_scalar(AssertSqlSafe(format!(
        "SELECT id FROM {} WHERE {}=? AND {}=?",
        kind.head(),
        kind.scope(),
        column
    )))
    .bind(scope(world))
    .bind(value)
    .fetch_optional(c)
    .await
    .map_err(storage_error)?
    .ok_or(Error::NotFound)
}
async fn head(
    c: &mut SqliteConnection,
    kind: Kind,
    world: Option<&str>,
    id: &str,
) -> Result<SqliteRow, Error> {
    sqlx::query(AssertSqlSafe(format!(
        "SELECT * FROM {} WHERE {}=? AND id=?",
        kind.head(),
        kind.scope()
    )))
    .bind(scope(world))
    .bind(id)
    .fetch_optional(c)
    .await
    .map_err(storage_error)?
    .ok_or(Error::NotFound)
}
enum Record {
    Attribute(MoveAttributeDefinition),
    Move(MoveRecord),
    Ruleset(RulesetRecord),
}
impl Record {
    fn identity(&self) -> (&str, Option<&str>, i64, &ContentFields) {
        match self {
            Self::Attribute(r) => (&r.id, r.world_id.as_deref(), r.revision, &r.fields),
            Self::Move(r) => (&r.id, r.world_id.as_deref(), r.revision, &r.fields),
            Self::Ruleset(r) => (&r.id, Some(&r.world_id), r.revision, &r.fields),
        }
    }
}
async fn raw(
    c: &mut SqliteConnection,
    kind: Kind,
    world: Option<&str>,
    id: &str,
    v: i64,
) -> Result<(Record, Option<String>, String), Error> {
    let r=sqlx::query(AssertSqlSafe(format!("SELECT v.*,coalesce(u.display_name,a.system_key) AS actor_name,a.action,a.reason,a.occurred_at,a.actor_user_id,a.system_key FROM {} v JOIN audit_events a ON a.id=v.audit_id LEFT JOIN users u ON u.id=a.actor_user_id WHERE v.{}=? AND v.{}=? AND v.revision=?",kind.versions(),kind.scope(),kind.key())))
        .bind(scope(world)).bind(id).bind(v).fetch_optional(&mut *c).await.map_err(storage_error)?.ok_or(Error::NotFound)?;
    let before: Option<String> = get(&r, "before_json")?;
    let after: String = get(&r, "after_json")?;
    let previous: Option<i64> = get(&r, "previous_revision")?;
    let action: String = get(&r, "action")?;
    let name: String = get(&r, "actor_name")?;
    let reason: Option<String> = get(&r, "reason")?;
    let actor: Option<String> = get(&r, "actor_user_id")?;
    let system: Option<String> = get(&r, "system_key")?;
    require(
        v > 0
            && previous == content::previous(v)
            && before.is_some() == (v > 1)
            && !name.is_empty()
            && !name.chars().any(char::is_control)
            && if v == 1 {
                action == "create"
            } else {
                matches!(action.as_str(), "edit" | "adopt")
            }
            && (actor.is_some() != system.is_some())
            && system
                .as_deref()
                .is_none_or(|s| s == "bundled-fixture-loader")
            && game_data::authoring::validate_reason(reason.as_deref())
                .ok()
                .as_ref()
                == Some(&reason),
    )?;
    let record = match kind {
        Kind::Attribute => {
            let a: MoveAttributeDefinition = parse(&after)?;
            a.validate().map_err(|_| Error::Unavailable)?;
            require(
                a.constraints == constraints(&r)?
                    && a.unit == get::<String>(&r, "unit")?
                    && a.archived_at == get::<Option<i64>>(&r, "archived_at")?
                    && get::<String>(&r, "source_scope")?.is_empty()
                    && a.source.as_ref().map(|p| p.attribute_id.as_str())
                        == get::<Option<String>>(&r, "source_attribute_id")?.as_deref()
                    && a.source.as_ref().map(|p| p.version)
                        == get::<Option<i64>>(&r, "source_revision")?,
            )?;
            if let Some(p) = &a.source {
                Box::pin(attribute_version(c, None, &p.attribute_id, p.version))
                    .await
                    .map_err(|_| Error::Unavailable)?;
            }
            Record::Attribute(a)
        }
        Kind::Move => {
            let m: MoveRecord = parse(&after)?;
            m.validate().map_err(|_| Error::Unavailable)?;
            require(
                m.archived_at == get::<Option<i64>>(&r, "archived_at")?
                    && m.attributes.is_some() == boolean(&r, "attributes_defined")?
                    && get::<String>(&r, "source_scope")?.is_empty()
                    && m.source.as_ref().map(|p| p.move_id.as_str())
                        == get::<Option<String>>(&r, "source_move_id")?.as_deref()
                    && m.source.as_ref().map(|p| p.version)
                        == get::<Option<i64>>(&r, "source_revision")?,
            )?;
            let rows=sqlx::query("SELECT * FROM move_attribute_bindings WHERE scope=? AND move_id=? AND revision=? ORDER BY slot").bind(scope(world)).bind(id).bind(v).fetch_all(&mut *c).await.map_err(storage_error)?;
            require(rows.len() == m.attributes.as_ref().map_or(0, Vec::len))?;
            for (i, (b, a)) in rows.iter().zip(m.attributes.iter().flatten()).enumerate() {
                let state: String = get(b, "state")?;
                let value = match (state.as_str(), decimal(b, "coefficient", "scale")?) {
                    ("known", Some(d)) => NumericValue::Known(d),
                    ("unknown", None) => NumericValue::Unknown,
                    ("not_applicable", None) => NumericValue::NotApplicable,
                    _ => return Err(Error::Unavailable),
                };
                require(
                    get::<i64>(b, "slot")? == slot(i)?
                        && get::<String>(b, "attribute_id")? == a.definition.id
                        && get::<i64>(b, "attribute_version")? == a.definition.revision
                        && value == a.value,
                )?;
            }
            let mappings=sqlx::query("SELECT * FROM move_attribute_mappings WHERE scope=? AND move_id=? AND revision=? ORDER BY slot").bind(scope(world)).bind(id).bind(v).fetch_all(&mut *c).await.map_err(storage_error)?;
            require(mappings.len() == m.mappings.len())?;
            for (i, (b, a)) in mappings.iter().zip(&m.mappings).enumerate() {
                require(
                    get::<i64>(b, "slot")? == slot(i)?
                        && get::<String>(b, "shared_scope")?.is_empty()
                        && get::<String>(b, "shared_attribute_id")? == a.shared_attribute_id
                        && get::<String>(b, "local_attribute_id")?
                            == a.local_attribute.attribute_id
                        && get::<i64>(b, "local_attribute_version")? == a.local_attribute.version,
                )?;
            }
            move_dependencies(c, &m)
                .await
                .map_err(|_| Error::Unavailable)?;
            Record::Move(m)
        }
        Kind::Ruleset => {
            let a: RulesetRecord = parse(&after)?;
            a.validate().map_err(|_| Error::Unavailable)?;
            require(
                a.expectations.is_some() == boolean(&r, "expectations_defined")?
                    && a.stat_roster.as_ref().map(|p| p.roster_id.as_str())
                        == get::<Option<String>>(&r, "roster_id")?.as_deref()
                    && a.stat_roster.as_ref().map(|p| p.version)
                        == get::<Option<i64>>(&r, "roster_version")?
                    && a.chart.as_ref().map(|p| p.chart_id.as_str())
                        == get::<Option<String>>(&r, "chart_id")?.as_deref()
                    && a.chart.as_ref().map(|p| p.version)
                        == get::<Option<i64>>(&r, "chart_version")?
                    && a.max_moves == get::<Option<i64>>(&r, "max_moves")?
                    && policy(a.repeat_policy) == get::<String>(&r, "repeat_policy")?
                    && a.notes == get::<String>(&r, "notes")?,
            )?;
            let rows=sqlx::query("SELECT * FROM ruleset_attribute_expectations WHERE world_id=? AND ruleset_id=? AND revision=? ORDER BY slot").bind(scope(world)).bind(id).bind(v).fetch_all(&mut *c).await.map_err(storage_error)?;
            require(rows.len() == a.expectations.as_ref().map_or(0, Vec::len))?;
            for (i, (b, e)) in rows.iter().zip(a.expectations.iter().flatten()).enumerate() {
                require(
                    get::<i64>(b, "slot")? == slot(i)?
                        && get::<String>(b, "attribute_id")? == e.definition.id
                        && get::<i64>(b, "attribute_version")? == e.definition.revision
                        && boolean(b, "required")? == e.required
                        && boolean(b, "allow_not_applicable")? == e.allow_not_applicable
                        && constraints(b)? == e.constraints,
                )?;
            }
            ruleset_dependencies(c, &a)
                .await
                .map_err(|_| Error::Unavailable)?;
            Record::Ruleset(a)
        }
    };
    let (rid, rworld, revision, fields) = record.identity();
    require(rid == id && rworld == world && revision == v && *fields == content::fields(&r)?)?;
    if let Some(w) = world {
        content::world(c, w).await.map_err(|_| Error::Unavailable)?;
    }
    Ok((record, before, after))
}
async fn version(
    c: &mut SqliteConnection,
    kind: Kind,
    world: Option<&str>,
    id: &str,
    v: i64,
) -> Result<Record, Error> {
    let (row, before, _) = Box::pin(raw(c, kind, world, id, v)).await?;
    let h = head(c, kind, world, id)
        .await
        .map_err(|_| Error::Unavailable)?;
    let rev: i64 = get(&h, "revision")?;
    require(rev >= v && get::<Option<String>>(&h, "world_id")?.as_deref() == world)?;
    let (current, cb) = if rev == v {
        (None, before.clone())
    } else {
        let (r, b, _) = Box::pin(raw(c, kind, world, id, rev))
            .await
            .map_err(|_| Error::Unavailable)?;
        (Some(r), b)
    };
    if !matches!(kind, Kind::Ruleset) {
        require(get::<String>(&h, "slug")? == current.as_ref().unwrap_or(&row).identity().3.slug)?;
    }
    for (n, b) in [(v, before), (rev, cb)] {
        if n > 1 {
            let (_, _, prev) = Box::pin(raw(c, kind, world, id, n - 1))
                .await
                .map_err(|_| Error::Unavailable)?;
            require(b.as_deref() == Some(&prev))?;
        }
    }
    Ok(row)
}
pub(crate) async fn attribute(
    c: &mut SqliteConnection,
    world: Option<&str>,
    id: &str,
) -> Result<MoveAttributeDefinition, Error> {
    let v = get(&head(c, Kind::Attribute, world, id).await?, "revision")?;
    attribute_version(c, world, id, v)
        .await
        .map_err(|_| Error::Unavailable)
}
pub(crate) async fn attribute_version(
    c: &mut SqliteConnection,
    world: Option<&str>,
    id: &str,
    v: i64,
) -> Result<MoveAttributeDefinition, Error> {
    match Box::pin(version(c, Kind::Attribute, world, id, v)).await? {
        Record::Attribute(r) => Ok(r),
        _ => Err(Error::Unavailable),
    }
}
pub(crate) async fn move_detail(
    c: &mut SqliteConnection,
    world: Option<&str>,
    id: &str,
) -> Result<MoveRecord, Error> {
    let v = get(&head(c, Kind::Move, world, id).await?, "revision")?;
    move_version(c, world, id, v)
        .await
        .map_err(|_| Error::Unavailable)
}
pub(crate) async fn move_version(
    c: &mut SqliteConnection,
    world: Option<&str>,
    id: &str,
    v: i64,
) -> Result<MoveRecord, Error> {
    match Box::pin(version(c, Kind::Move, world, id, v)).await? {
        Record::Move(r) => Ok(r),
        _ => Err(Error::Unavailable),
    }
}
pub(crate) async fn ruleset(
    c: &mut SqliteConnection,
    world: &str,
) -> Result<Option<RulesetRecord>, Error> {
    content::world(c, world).await?;
    let h: Option<(String, i64)> =
        sqlx::query_as("SELECT id,revision FROM world_rulesets WHERE world_id=?")
            .bind(world)
            .fetch_optional(&mut *c)
            .await
            .map_err(storage_error)?;
    match h {
        None => Ok(None),
        Some((id, v)) => Ok(Some(
            ruleset_version(c, world, &id, v)
                .await
                .map_err(|_| Error::Unavailable)?,
        )),
    }
}
pub(crate) async fn ruleset_version(
    c: &mut SqliteConnection,
    world: &str,
    id: &str,
    v: i64,
) -> Result<RulesetRecord, Error> {
    match Box::pin(version(c, Kind::Ruleset, Some(world), id, v)).await? {
        Record::Ruleset(r) => Ok(r),
        _ => Err(Error::Unavailable),
    }
}
pub(crate) async fn select_attribute(
    c: &mut SqliteConnection,
    world: Option<&str>,
    pin: &AttributePin,
) -> Result<MoveAttributeDefinition, Error> {
    pin.validate().map_err(Error::Invalid)?;
    let h = attribute(c, world, &pin.attribute_id).await?;
    let r = attribute_version(c, world, &pin.attribute_id, pin.version).await?;
    if h.archived_at.is_some() || r.archived_at.is_some() {
        return Err(author::invalid(
            "attribute",
            "Choose an unarchived attribute revision.",
        ));
    }
    Ok(r)
}
pub(crate) async fn select_move(
    c: &mut SqliteConnection,
    world: Option<&str>,
    pin: &MovePin,
) -> Result<MoveRecord, Error> {
    pin.validate().map_err(Error::Invalid)?;
    let h = move_detail(c, world, &pin.move_id).await?;
    let r = move_version(c, world, &pin.move_id, pin.version).await?;
    if h.archived_at.is_some() || r.archived_at.is_some() {
        return Err(author::invalid(
            "move",
            "Choose an unarchived move revision.",
        ));
    }
    Ok(r)
}
pub(crate) async fn attribute_dependencies(
    c: &mut SqliteConnection,
    row: &MoveAttributeDefinition,
) -> Result<(), Error> {
    if let Some(w) = &row.world_id {
        content::world(c, w).await?;
    }
    if let Some(p) = &row.source {
        Box::pin(attribute_version(c, None, &p.attribute_id, p.version)).await?;
    }
    Ok(())
}
pub(crate) async fn move_dependencies(
    c: &mut SqliteConnection,
    row: &MoveRecord,
) -> Result<(), Error> {
    if let Some(w) = &row.world_id {
        content::world(c, w).await?;
    }
    if let Some(p) = &row.source {
        Box::pin(move_version(c, None, &p.move_id, p.version)).await?;
    }
    for a in row.attributes.iter().flatten() {
        require(
            attribute_version(
                c,
                row.world_id.as_deref(),
                &a.definition.id,
                a.definition.revision,
            )
            .await?
                == a.definition,
        )?;
    }
    for m in &row.mappings {
        let a = attribute_version(
            c,
            row.world_id.as_deref(),
            &m.local_attribute.attribute_id,
            m.local_attribute.version,
        )
        .await?;
        require(
            a.source
                .as_ref()
                .is_some_and(|p| p.attribute_id == m.shared_attribute_id),
        )?;
    }
    Ok(())
}
pub(crate) async fn ruleset_dependencies(
    c: &mut SqliteConnection,
    row: &RulesetRecord,
) -> Result<(), Error> {
    content::world(c, &row.world_id).await?;
    for e in row.expectations.iter().flatten() {
        require(
            attribute_version(
                c,
                Some(&row.world_id),
                &e.definition.id,
                e.definition.revision,
            )
            .await?
                == e.definition,
        )?;
    }
    if let Some(p) = &row.stat_roster {
        stats::roster_version(c, &row.world_id, &p.roster_id, p.version).await?;
    }
    if let Some(p) = &row.chart {
        charts::version(c, Some(&row.world_id), &p.chart_id, p.version).await?;
    }
    Ok(())
}
fn slot(i: usize) -> Result<i64, Error> {
    i64::try_from(i)
        .ok()
        .and_then(|n| n.checked_add(1))
        .ok_or(Error::Unavailable)
}
fn policy(p: RepeatPolicy) -> &'static str {
    match p {
        RepeatPolicy::Unspecified => "unspecified",
        RepeatPolicy::Allow => "allow",
        RepeatPolicy::Forbid => "forbid",
    }
}
async fn before(
    c: &mut SqliteConnection,
    kind: Kind,
    world: Option<&str>,
    id: &str,
    revision: i64,
) -> Result<Option<String>, Error> {
    if revision == 1 {
        return Ok(None);
    }
    let h = head(c, kind, world, id).await?;
    let current: i64 = get(&h, "revision")?;
    author::expected(author::next(current)?, revision)?;
    version(c, kind, world, id, current).await?;
    sqlx::query_scalar(AssertSqlSafe(format!(
        "SELECT after_json FROM {} WHERE {}=? AND {}=? AND revision=?",
        kind.versions(),
        kind.scope(),
        kind.key()
    )))
    .bind(scope(world))
    .bind(id)
    .bind(current)
    .fetch_one(c)
    .await
    .map_err(storage_error)
    .map(Some)
}
async fn write_head(
    c: &mut SqliteConnection,
    kind: Kind,
    world: Option<&str>,
    id: &str,
    revision: i64,
    slug: &str,
) -> Result<(), Error> {
    let sql = match (kind, revision == 1) {
        (Kind::Ruleset, true) => {
            "INSERT INTO world_rulesets(id,world_id,revision) VALUES (?1,?2,?3)".into()
        }
        (Kind::Ruleset, false) => {
            "UPDATE world_rulesets SET revision=?3 WHERE id=?1 AND world_id=?2 AND revision=?3-1"
                .into()
        }
        (_, true) => format!(
            "INSERT INTO {}(id,scope,revision,slug,world_id) VALUES (?1,?2,?3,?4,?5)",
            kind.head()
        ),
        (_, false) => format!(
            "UPDATE {} SET revision=?3,slug=?4 WHERE id=?1 AND scope=?2 AND revision=?3-1 AND world_id IS ?5",
            kind.head()
        ),
    };
    let q = sqlx::query(AssertSqlSafe(sql))
        .bind(id)
        .bind(scope(world))
        .bind(revision);
    let result = if matches!(kind, Kind::Ruleset) {
        q.execute(c).await
    } else {
        q.bind(slug).bind(world).execute(c).await
    }
    .map_err(content::content_write_error)?;
    if result.rows_affected() != 1 {
        return Err(Error::Conflict);
    }
    Ok(())
}
pub(crate) async fn put_attribute(
    c: &mut SqliteConnection,
    row: &MoveAttributeDefinition,
    actor: Option<&str>,
    action: &str,
    reason: Option<&str>,
) -> Result<(), Error> {
    row.validate().map_err(Error::Invalid)?;
    attribute_dependencies(c, row).await?;
    let world = row.world_id.as_deref();
    let before = before(c, Kind::Attribute, world, &row.id, row.revision).await?;
    let after = author::snapshot(row)?;
    let audit = content::audit(
        c,
        &content::Revision::new(actor, action, reason, before.clone(), after.clone()),
    )
    .await?;
    write_head(
        c,
        Kind::Attribute,
        world,
        &row.id,
        row.revision,
        &row.fields.slug,
    )
    .await?;
    sqlx::query("INSERT INTO move_attribute_versions(scope,attribute_id,revision,source_attribute_id,source_revision,slug,display_name,description,unit,archived_at,whole_only,minimum_coefficient,minimum_scale,maximum_coefficient,maximum_scale,audit_id,previous_revision,before_json,after_json) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)")
        .bind(scope(world)).bind(&row.id).bind(row.revision).bind(row.source.as_ref().map(|p|&p.attribute_id)).bind(row.source.as_ref().map(|p|p.version))
        .bind(&row.fields.slug).bind(&row.fields.display_name).bind(&row.fields.description).bind(&row.unit).bind(row.archived_at).bind(i64::from(row.constraints.whole_only))
        .bind(row.constraints.minimum.map(Decimal::coefficient)).bind(row.constraints.minimum.map(|d|i64::from(d.scale()))).bind(row.constraints.maximum.map(Decimal::coefficient)).bind(row.constraints.maximum.map(|d|i64::from(d.scale())))
        .bind(audit).bind(content::previous(row.revision)).bind(before).bind(after).execute(c).await.map_err(storage_error)?;
    Ok(())
}
pub(crate) async fn put_move(
    c: &mut SqliteConnection,
    row: &MoveRecord,
    actor: Option<&str>,
    action: &str,
    reason: Option<&str>,
) -> Result<(), Error> {
    row.validate().map_err(Error::Invalid)?;
    move_dependencies(c, row).await?;
    let world = row.world_id.as_deref();
    let before = before(c, Kind::Move, world, &row.id, row.revision).await?;
    let after = author::snapshot(row)?;
    let audit = content::audit(
        c,
        &content::Revision::new(actor, action, reason, before.clone(), after.clone()),
    )
    .await?;
    write_head(
        c,
        Kind::Move,
        world,
        &row.id,
        row.revision,
        &row.fields.slug,
    )
    .await?;
    for (i, a) in row.attributes.iter().flatten().enumerate() {
        let (state, value) = match a.value {
            NumericValue::Known(d) => ("known", Some(d)),
            NumericValue::Unknown => ("unknown", None),
            NumericValue::NotApplicable => ("not_applicable", None),
        };
        sqlx::query("INSERT INTO move_attribute_bindings(scope,move_id,revision,slot,attribute_id,attribute_version,state,coefficient,scale) VALUES (?,?,?,?,?,?,?,?,?)")
            .bind(scope(world)).bind(&row.id).bind(row.revision).bind(slot(i)?).bind(&a.definition.id).bind(a.definition.revision).bind(state).bind(value.map(Decimal::coefficient)).bind(value.map(|d|i64::from(d.scale()))).execute(&mut *c).await.map_err(storage_error)?;
    }
    for (i, m) in row.mappings.iter().enumerate() {
        sqlx::query("INSERT INTO move_attribute_mappings(scope,move_id,revision,slot,shared_attribute_id,local_attribute_id,local_attribute_version) VALUES (?,?,?,?,?,?,?)")
            .bind(scope(world)).bind(&row.id).bind(row.revision).bind(slot(i)?).bind(&m.shared_attribute_id).bind(&m.local_attribute.attribute_id).bind(m.local_attribute.version).execute(&mut *c).await.map_err(storage_error)?;
    }
    sqlx::query("INSERT INTO move_versions(scope,move_id,revision,source_move_id,source_revision,slug,display_name,description,archived_at,attributes_defined,audit_id,previous_revision,before_json,after_json) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?)")
        .bind(scope(world)).bind(&row.id).bind(row.revision).bind(row.source.as_ref().map(|p|&p.move_id)).bind(row.source.as_ref().map(|p|p.version))
        .bind(&row.fields.slug).bind(&row.fields.display_name).bind(&row.fields.description).bind(row.archived_at).bind(i64::from(row.attributes.is_some())).bind(audit).bind(content::previous(row.revision)).bind(before).bind(after).execute(&mut *c).await.map_err(storage_error)?;
    super::cards::seal_native(
        c,
        game_data::cards::CardIdentity {
            namespace: game_data::cards::CardNamespace::Move,
            world_id: row.world_id.clone(),
            id: row.id.clone(),
        },
        row.revision,
    )
    .await?;
    Ok(())
}
pub(crate) async fn put_ruleset(
    c: &mut SqliteConnection,
    row: &RulesetRecord,
    actor: Option<&str>,
    reason: Option<&str>,
) -> Result<(), Error> {
    row.validate().map_err(Error::Invalid)?;
    ruleset_dependencies(c, row).await?;
    let before = before(c, Kind::Ruleset, Some(&row.world_id), &row.id, row.revision).await?;
    let after = author::snapshot(row)?;
    let audit = content::audit(
        c,
        &content::Revision::new(
            actor,
            if row.revision == 1 { "create" } else { "edit" },
            reason,
            before.clone(),
            after.clone(),
        ),
    )
    .await?;
    write_head(
        c,
        Kind::Ruleset,
        Some(&row.world_id),
        &row.id,
        row.revision,
        &row.fields.slug,
    )
    .await?;
    for (i, e) in row.expectations.iter().flatten().enumerate() {
        sqlx::query("INSERT INTO ruleset_attribute_expectations(world_id,ruleset_id,revision,slot,attribute_id,attribute_version,required,allow_not_applicable,whole_only,minimum_coefficient,minimum_scale,maximum_coefficient,maximum_scale) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?)")
            .bind(&row.world_id).bind(&row.id).bind(row.revision).bind(slot(i)?).bind(&e.definition.id).bind(e.definition.revision).bind(i64::from(e.required)).bind(i64::from(e.allow_not_applicable)).bind(i64::from(e.constraints.whole_only))
            .bind(e.constraints.minimum.map(Decimal::coefficient)).bind(e.constraints.minimum.map(|d|i64::from(d.scale()))).bind(e.constraints.maximum.map(Decimal::coefficient)).bind(e.constraints.maximum.map(|d|i64::from(d.scale()))).execute(&mut *c).await.map_err(storage_error)?;
    }
    sqlx::query("INSERT INTO world_ruleset_versions(world_id,ruleset_id,revision,slug,display_name,description,expectations_defined,roster_id,roster_version,chart_id,chart_version,max_moves,repeat_policy,notes,audit_id,previous_revision,before_json,after_json) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)")
        .bind(&row.world_id).bind(&row.id).bind(row.revision).bind(&row.fields.slug).bind(&row.fields.display_name).bind(&row.fields.description).bind(i64::from(row.expectations.is_some()))
        .bind(row.stat_roster.as_ref().map(|p|&p.roster_id)).bind(row.stat_roster.as_ref().map(|p|p.version)).bind(row.chart.as_ref().map(|p|&p.chart_id)).bind(row.chart.as_ref().map(|p|p.version))
        .bind(row.max_moves).bind(policy(row.repeat_policy)).bind(&row.notes).bind(audit).bind(content::previous(row.revision)).bind(before).bind(after).execute(c).await.map_err(storage_error)?;
    Ok(())
}
pub(crate) async fn history(
    c: &mut SqliteConnection,
    kind: Kind,
    world: Option<&str>,
    id: &str,
    before: Option<i64>,
    exact: Option<i64>,
) -> Result<Vec<HistoryEntry>, Error> {
    let h = head(c, kind, world, id).await?;
    version(c, kind, world, id, get(&h, "revision")?).await?;
    let rows: Vec<HistoryEntry>=sqlx::query_as(AssertSqlSafe(format!("SELECT r.revision,r.previous_revision,coalesce(u.display_name,a.system_key) AS actor_name,a.action,a.occurred_at,a.reason,r.before_json,r.after_json FROM {} r JOIN audit_events a ON a.id=r.audit_id LEFT JOIN users u ON u.id=a.actor_user_id WHERE r.{}=?1 AND r.{}=?2 AND (?3 IS NULL OR r.revision<?3) AND (?4 IS NULL OR r.revision=?4) ORDER BY r.revision DESC LIMIT 50",kind.versions(),kind.scope(),kind.key())))
        .bind(scope(world)).bind(id).bind(before).bind(exact).fetch_all(&mut *c).await.map_err(storage_error)?;
    for r in &rows {
        version(c, kind, world, id, r.revision).await?;
    }
    Ok(rows)
}
/// Creation baseline verification for complete-fixture callers. Caller owns transaction.
pub(crate) async fn fixture_baseline(
    c: &mut SqliteConnection,
    kind: Kind,
    world: Option<&str>,
    id: &str,
    expected: &str,
) -> Result<bool, Error> {
    let exists: bool = sqlx::query_scalar(AssertSqlSafe(format!(
        "SELECT EXISTS(SELECT 1 FROM {} WHERE id=?)",
        kind.head()
    )))
    .bind(id)
    .fetch_one(&mut *c)
    .await
    .map_err(storage_error)?;
    if exists {
        version(c, kind, world, id, 1).await?;
        let valid: bool=sqlx::query_scalar(AssertSqlSafe(format!("SELECT EXISTS(SELECT 1 FROM {} v JOIN audit_events a ON a.id=v.audit_id WHERE v.{}=? AND v.{}=? AND v.revision=1 AND v.after_json=? AND a.actor_user_id IS NULL AND a.system_key='bundled-fixture-loader' AND a.action='create' AND a.reason IS NULL)",kind.versions(),kind.scope(),kind.key())))
            .bind(scope(world)).bind(id).bind(expected).fetch_one(&mut *c).await.map_err(storage_error)?;
        require(valid)?;
        return Ok(true);
    }
    Ok(false)
}
pub(crate) async fn check_baseline_slug(
    c: &mut SqliteConnection,
    kind: Kind,
    world: Option<&str>,
    slug: &str,
) -> Result<(), Error> {
    let exists: bool = sqlx::query_scalar(AssertSqlSafe(format!(
        "SELECT EXISTS(SELECT 1 FROM {} WHERE {}=? AND revision=1 AND slug=?)",
        kind.versions(),
        kind.scope()
    )))
    .bind(scope(world))
    .bind(slug)
    .fetch_one(c)
    .await
    .map_err(storage_error)?;
    if exists {
        return Err(author::invalid(
            "fixtures",
            "The creation baseline address collides with existing content.",
        ));
    }
    Ok(())
}
