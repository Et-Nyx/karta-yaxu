//! Concrete Type queries and immutable full-content seals. No authorization policy.
use super::authoring::{self as content, Revision, SnapshotKind, storage_error};
use crate::application::{
    authoring::{self as author, AddressKey, Error, HistoryKind},
    types::{TypeAssignment, TypeRecord, TypingState, WorldTypeRecord},
};
use game_data::stats::BaseStats;
use game_data::types::{TypePin, validate_pins};
use serde_json::Value;
use sqlx::{Row, SqliteConnection, sqlite::SqliteRow};

fn type_row(r: SqliteRow) -> Result<TypeRecord, Error> {
    Ok(TypeRecord {
        id: content::id(&r, "id")?,
        fields: content::fields(&r)?,
        revision: content::revision(&r, "revision")?,
        archived_at: r.try_get("archived_at").map_err(storage_error)?,
    })
}
fn world_type_row(r: SqliteRow) -> Result<WorldTypeRecord, Error> {
    let source_id: Option<String> = r.try_get("source_type_id").map_err(storage_error)?;
    let source_version: Option<i64> = r.try_get("source_type_version").map_err(storage_error)?;
    let source = match (source_id, source_version) {
        (None, None) => None,
        (Some(type_id), Some(version)) if content::valid_id(&type_id) && version > 0 => {
            Some(TypePin { type_id, version })
        }
        _ => return Err(Error::Unavailable),
    };
    Ok(WorldTypeRecord {
        id: content::id(&r, "id")?,
        world_id: content::id(&r, "world_id")?,
        source,
        fields: content::fields(&r)?,
        revision: content::revision(&r, "revision")?,
        archived_at: r.try_get("archived_at").map_err(storage_error)?,
    })
}
pub(crate) async fn types(c: &mut SqliteConnection) -> Result<Vec<TypeRecord>, Error> {
    sqlx::query("SELECT * FROM types ORDER BY display_name, id")
        .fetch_all(c)
        .await
        .map_err(storage_error)?
        .into_iter()
        .map(type_row)
        .collect()
}
pub(crate) async fn type_detail(c: &mut SqliteConnection, id: &str) -> Result<TypeRecord, Error> {
    resolve_type(c, AddressKey::Id(id.into())).await
}
pub(crate) async fn resolve_type(
    c: &mut SqliteConnection,
    key: AddressKey,
) -> Result<TypeRecord, Error> {
    let (sql, value) = match key {
        AddressKey::Id(v) => ("SELECT * FROM types WHERE id=?", v),
        AddressKey::Slug(v) => ("SELECT * FROM types WHERE slug=?", v),
    };
    type_row(
        sqlx::query(sql)
            .bind(value)
            .fetch_optional(c)
            .await
            .map_err(storage_error)?
            .ok_or(Error::NotFound)?,
    )
}
pub(crate) async fn type_version(
    c: &mut SqliteConnection,
    id: &str,
    version: i64,
) -> Result<TypeRecord, Error> {
    let r =
        sqlx::query("SELECT type_id AS id, * FROM type_versions WHERE type_id=? AND revision=?")
            .bind(id)
            .bind(version)
            .fetch_optional(c)
            .await
            .map_err(storage_error)?
            .ok_or(Error::NotFound)?;
    let after: String = r.try_get("after_json").map_err(storage_error)?;
    let row = type_row(r)?;
    validate_type_snapshot(&after, HistoryKind::Type, id, None, version)?;
    if json(&after)?["content"] != serde_json::to_value(&row).map_err(|_| Error::Unavailable)? {
        return Err(Error::Unavailable);
    }
    Ok(row)
}
pub(crate) async fn world_types(
    c: &mut SqliteConnection,
    world: &str,
) -> Result<Vec<WorldTypeRecord>, Error> {
    sqlx::query("SELECT * FROM world_types WHERE world_id=? ORDER BY display_name,id")
        .bind(world)
        .fetch_all(c)
        .await
        .map_err(storage_error)?
        .into_iter()
        .map(world_type_row)
        .collect()
}
pub(crate) async fn world_type_detail(
    c: &mut SqliteConnection,
    world: &str,
    id: &str,
) -> Result<WorldTypeRecord, Error> {
    resolve_world_type(c, world, AddressKey::Id(id.into())).await
}
pub(crate) async fn resolve_world_type(
    c: &mut SqliteConnection,
    world: &str,
    key: AddressKey,
) -> Result<WorldTypeRecord, Error> {
    let (sql, value) = match key {
        AddressKey::Id(v) => ("SELECT * FROM world_types WHERE world_id=? AND id=?", v),
        AddressKey::Slug(v) => ("SELECT * FROM world_types WHERE world_id=? AND slug=?", v),
    };
    world_type_row(
        sqlx::query(sql)
            .bind(world)
            .bind(value)
            .fetch_optional(c)
            .await
            .map_err(storage_error)?
            .ok_or(Error::NotFound)?,
    )
}
pub(crate) async fn world_type_version(
    c: &mut SqliteConnection,
    world: &str,
    id: &str,
    version: i64,
) -> Result<WorldTypeRecord, Error> {
    let r = sqlx::query("SELECT type_id AS id,* FROM world_type_versions WHERE world_id=? AND type_id=? AND revision=?").bind(world).bind(id).bind(version).fetch_optional(c).await.map_err(storage_error)?.ok_or(Error::NotFound)?;
    let after: String = r.try_get("after_json").map_err(storage_error)?;
    let row = world_type_row(r)?;
    validate_type_snapshot(&after, HistoryKind::WorldType, id, Some(world), version)?;
    if json(&after)?["content"] != serde_json::to_value(&row).map_err(|_| Error::Unavailable)? {
        return Err(Error::Unavailable);
    }
    Ok(row)
}
pub(crate) async fn type_after(
    c: &mut SqliteConnection,
    id: &str,
    version: i64,
) -> Result<String, Error> {
    type_version(c, id, version).await?;
    sqlx::query_scalar("SELECT after_json FROM type_versions WHERE type_id=? AND revision=?")
        .bind(id)
        .bind(version)
        .fetch_one(c)
        .await
        .map_err(storage_error)
}
pub(crate) async fn world_type_after(
    c: &mut SqliteConnection,
    world: &str,
    id: &str,
    version: i64,
) -> Result<String, Error> {
    world_type_version(c, world, id, version).await?;
    sqlx::query_scalar(
        "SELECT after_json FROM world_type_versions WHERE world_id=? AND type_id=? AND revision=?",
    )
    .bind(world)
    .bind(id)
    .bind(version)
    .fetch_one(c)
    .await
    .map_err(storage_error)
}
pub(crate) async fn select_shared_pin(
    c: &mut SqliteConnection,
    pin: &TypePin,
) -> Result<TypeRecord, Error> {
    let current = type_detail(c, &pin.type_id)
        .await
        .map_err(|e| author::missing_reference(e, "typing"))?;
    if current.archived_at.is_some() {
        return Err(author::invalid(
            "typing",
            "Choose an active Type definition.",
        ));
    }
    type_version(c, &pin.type_id, pin.version)
        .await
        .map_err(|e| author::missing_reference(e, "typing"))
}
pub(crate) async fn select_typing(
    c: &mut SqliteConnection,
    world: Option<&str>,
    pins: Option<Vec<TypePin>>,
) -> Result<TypingState, Error> {
    validate_pins(pins.as_deref()).map_err(Error::Invalid)?;
    let Some(pins) = pins else {
        return Ok(TypingState::unmodeled());
    };
    let mut assignments = Vec::with_capacity(pins.len());
    for pin in pins {
        let fields = if let Some(world) = world {
            let current = world_type_detail(c, world, &pin.type_id)
                .await
                .map_err(|e| author::missing_reference(e, "typing"))?;
            if current.archived_at.is_some() {
                return Err(author::invalid(
                    "typing",
                    "Choose an active Type in this world.",
                ));
            }
            world_type_version(c, world, &pin.type_id, pin.version)
                .await
                .map_err(|e| author::missing_reference(e, "typing"))?
                .fields
        } else {
            select_shared_pin(c, &pin).await?.fields
        };
        assignments.push(TypeAssignment { pin, fields });
    }
    Ok(TypingState {
        assignments: Some(assignments),
    })
}
pub(crate) async fn put_type(
    c: &mut SqliteConnection,
    row: &TypeRecord,
    r: Revision<'_>,
) -> Result<(), Error> {
    let sql = if row.revision == 1 {
        "INSERT INTO types(id,slug,display_name,description,revision,archived_at) VALUES (?1,?2,?3,?4,?5,?6)"
    } else {
        "UPDATE types SET slug=?2,display_name=?3,description=?4,revision=?5,archived_at=?6 WHERE id=?1"
    };
    sqlx::query(sql)
        .bind(&row.id)
        .bind(&row.fields.slug)
        .bind(&row.fields.display_name)
        .bind(&row.fields.description)
        .bind(row.revision)
        .bind(row.archived_at)
        .execute(&mut *c)
        .await
        .map_err(content::content_write_error)?;
    let audit = content::audit(c, &r).await?;
    sqlx::query("INSERT INTO type_versions(type_id,revision,slug,display_name,description,archived_at,audit_id,previous_revision,before_json,after_json) VALUES (?,?,?,?,?,?,?,?,?,?)").bind(&row.id).bind(row.revision).bind(&row.fields.slug).bind(&row.fields.display_name).bind(&row.fields.description).bind(row.archived_at).bind(audit).bind(content::previous(row.revision)).bind(r.before).bind(r.after).execute(c).await.map_err(storage_error)?;
    Ok(())
}
pub(crate) async fn put_world_type(
    c: &mut SqliteConnection,
    row: &WorldTypeRecord,
    r: Revision<'_>,
) -> Result<(), Error> {
    let sql = if row.revision == 1 {
        "INSERT INTO world_types(id,world_id,source_type_id,source_type_version,slug,display_name,description,revision,archived_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)"
    } else {
        "UPDATE world_types SET source_type_id=?3,source_type_version=?4,slug=?5,display_name=?6,description=?7,revision=?8,archived_at=?9 WHERE id=?1 AND world_id=?2"
    };
    sqlx::query(sql)
        .bind(&row.id)
        .bind(&row.world_id)
        .bind(row.source.as_ref().map(|p| &p.type_id))
        .bind(row.source.as_ref().map(|p| p.version))
        .bind(&row.fields.slug)
        .bind(&row.fields.display_name)
        .bind(&row.fields.description)
        .bind(row.revision)
        .bind(row.archived_at)
        .execute(&mut *c)
        .await
        .map_err(content::content_write_error)?;
    let audit = content::audit(c, &r).await?;
    sqlx::query("INSERT INTO world_type_versions(world_id,type_id,revision,source_type_id,source_type_version,slug,display_name,description,archived_at,audit_id,previous_revision,before_json,after_json) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?)").bind(&row.world_id).bind(&row.id).bind(row.revision).bind(row.source.as_ref().map(|p| &p.type_id)).bind(row.source.as_ref().map(|p| p.version)).bind(&row.fields.slug).bind(&row.fields.display_name).bind(&row.fields.description).bind(row.archived_at).bind(audit).bind(content::previous(row.revision)).bind(r.before).bind(r.after).execute(c).await.map_err(storage_error)?;
    Ok(())
}
fn json(s: &str) -> Result<Value, Error> {
    serde_json::from_str(s).map_err(|_| Error::Unavailable)
}
fn encode(v: &Value) -> Result<String, Error> {
    serde_json::to_string(v).map_err(|_| Error::Unavailable)
}
fn exact_keys(v: &Value, keys: &[&str]) -> bool {
    v.as_object()
        .is_some_and(|m| m.len() == keys.len() && keys.iter().all(|k| m.contains_key(*k)))
}
pub(crate) fn validate_type_snapshot(
    s: &str,
    kind: HistoryKind,
    key: &str,
    scope: Option<&str>,
    revision: i64,
) -> Result<(), Error> {
    let v = json(s)?;
    if !exact_keys(&v, &["snapshot_version", "content"]) || v["snapshot_version"] != 1 {
        return Err(Error::Unavailable);
    }
    let value = &v["content"];
    let (id, fields, rev, valid) = if matches!(kind, HistoryKind::Type) {
        if !exact_keys(value, &["id", "fields", "revision", "archived_at"]) {
            return Err(Error::Unavailable);
        }
        let row: TypeRecord =
            serde_json::from_value(value.clone()).map_err(|_| Error::Unavailable)?;
        (row.id, row.fields, row.revision, true)
    } else {
        if !exact_keys(
            value,
            &[
                "id",
                "world_id",
                "source",
                "fields",
                "revision",
                "archived_at",
            ],
        ) {
            return Err(Error::Unavailable);
        }
        let row: WorldTypeRecord =
            serde_json::from_value(value.clone()).map_err(|_| Error::Unavailable)?;
        let valid = scope == Some(row.world_id.as_str())
            && content::valid_id(&row.world_id)
            && row
                .source
                .as_ref()
                .is_none_or(|p| content::valid_id(&p.type_id) && p.version > 0);
        (row.id, row.fields, row.revision, valid)
    };
    if !valid
        || id != key
        || !content::valid_id(&id)
        || rev != revision
        || rev <= 0
        || fields.normalized().ok().as_ref() != Some(&fields)
    {
        return Err(Error::Unavailable);
    }
    Ok(())
}

pub(crate) struct FullContent {
    pub(crate) before: Option<String>,
    pub(crate) after: String,
    pub(crate) projection_after: String,
    pub(crate) typing: TypingState,
    pub(crate) stats: BaseStats,
    /// Actual stored v2 bytes, or v1 for a pre-typing predecessor.
    pub(crate) typing_projection_after: String,
    typing_projection_before: Option<String>,
}
// Closed aggregate choices are the only source of SQL identifiers.
fn tables(
    kind: HistoryKind,
) -> Result<(&'static str, &'static str, &'static str, &'static str), Error> {
    match kind {
        HistoryKind::Form => Ok((
            "form_versions",
            "form_content_snapshots",
            "form_id",
            "species_id",
        )),
        HistoryKind::Variant => Ok((
            "variant_revisions",
            "variant_content_snapshots",
            "variant_id",
            "world_id",
        )),
        HistoryKind::World => Ok((
            "world_revisions",
            "world_content_snapshots",
            "world_id",
            "NULL",
        )),
        _ => Err(Error::Unavailable),
    }
}
/// Validate an authoritative after independently; do not recursively traverse all history.
async fn load_content(
    c: &mut SqliteConnection,
    kind: HistoryKind,
    id: &str,
    world: Option<&str>,
    revision: i64,
) -> Result<FullContent, Error> {
    let (legacy, sidecar, key, scope_column) = tables(kind)?;
    let filter = if matches!(kind, HistoryKind::Variant) {
        "AND (?3 IS NULL OR world_id=?3)"
    } else {
        "AND (?3 IS NULL OR ?3 IS NOT NULL)"
    };
    let extension_column = if matches!(kind, HistoryKind::World) {
        "0"
    } else {
        "stat_snapshot_version"
    };
    let sql = format!(
        "SELECT before_json,after_json,content_snapshot_version,{extension_column} AS stat_marker,{scope_column} AS scope FROM {legacy} WHERE {key}=?1 AND revision=?2 {filter}"
    );
    let r = sqlx::query(sqlx::AssertSqlSafe(sql))
        .bind(id)
        .bind(revision)
        .bind(world)
        .fetch_optional(&mut *c)
        .await
        .map_err(storage_error)?
        .ok_or(Error::NotFound)?;
    let projection_after: String = r.try_get("after_json").map_err(storage_error)?;
    let before: Option<String> = r.try_get("before_json").map_err(storage_error)?;
    let marker: i64 = r
        .try_get("content_snapshot_version")
        .map_err(storage_error)?;
    let scope: Option<String> = r.try_get("scope").map_err(storage_error)?;
    let stat_marker: i64 = r.try_get("stat_marker").map_err(storage_error)?;
    let stat_world = if matches!(kind, HistoryKind::Variant) {
        scope.as_deref()
    } else {
        None
    };
    content::validate_snapshot(
        &projection_after,
        SnapshotKind::Public(kind),
        id,
        scope.as_deref(),
        revision,
    )?;
    if let (Some(s), Some(v)) = (&before, content::previous(revision)) {
        content::validate_snapshot(s, SnapshotKind::Public(kind), id, scope.as_deref(), v)?;
    }
    if before.is_some() != (revision > 1) {
        return Err(Error::Unavailable);
    }
    // Metadata projections must agree with the immutable typed version, too.
    let projected = json(&projection_after)?;
    let typed = match kind {
        HistoryKind::Form => Some(
            serde_json::to_value(content::form_version_projection(c, id, revision).await?)
                .map_err(|_| Error::Unavailable)?,
        ),
        HistoryKind::Variant => {
            let row: author::VariantRecord = serde_json::from_value(projected["content"].clone())
                .map_err(|_| Error::Unavailable)?;
            if let Some(pin) = &row.source {
                // Finite dependency direction: variant -> exact form -> exact Types.
                // Boxing breaks the shared loader's async future type cycle, not its validation.
                Box::pin(content::form_version(c, &pin.form_id, pin.version))
                    .await
                    .map_err(|_| Error::Unavailable)?;
            }
            let head: i64 =
                sqlx::query_scalar("SELECT revision FROM variants WHERE world_id=? AND id=?")
                    .bind(&row.world_id)
                    .bind(id)
                    .fetch_optional(&mut *c)
                    .await
                    .map_err(storage_error)?
                    .ok_or(Error::Unavailable)?;
            if head == revision {
                Some(
                    serde_json::to_value(content::variant(c, &row.world_id, id).await?)
                        .map_err(|_| Error::Unavailable)?,
                )
            } else {
                None
            }
        }
        HistoryKind::World => {
            let ids: Vec<String> =
                serde_json::from_value(projected["content"]["variant_ids"].clone())
                    .map_err(|_| Error::Unavailable)?;
            validate_world_membership(c, id, &ids, false).await?;
            None
        }
        _ => None,
    };
    if typed.is_some_and(|v| v != projected["content"]) {
        return Err(Error::Unavailable);
    }
    let filter = if matches!(kind, HistoryKind::Variant) {
        "AND world_id=?3"
    } else {
        "AND (?3 IS NULL OR ?3 IS NOT NULL)"
    };
    let sql = format!("SELECT * FROM {sidecar} WHERE {key}=?1 AND revision=?2 {filter}");
    let side = sqlx::query(sqlx::AssertSqlSafe(sql))
        .bind(id)
        .bind(revision)
        .bind(scope.as_deref())
        .fetch_optional(&mut *c)
        .await
        .map_err(storage_error)?;
    if marker == 1 {
        if side.is_some() {
            return Err(Error::Unavailable);
        }
        let bindings = read_bindings(c, kind, id, scope.as_deref(), revision).await?;
        if !bindings.is_empty() {
            return Err(Error::Unavailable);
        }
        if !matches!(kind, HistoryKind::World) {
            super::stat_content::load(
                c,
                kind,
                id,
                stat_world,
                revision,
                stat_marker,
                &projection_after,
            )
            .await?;
            if stat_marker != 0 {
                return Err(Error::Unavailable);
            }
        }
        return Ok(FullContent {
            before,
            after: projection_after.clone(),
            typing_projection_after: projection_after.clone(),
            typing_projection_before: None,
            projection_after,
            typing: TypingState::unmodeled(),
            stats: BaseStats::unmodeled(),
        });
    }
    if marker != 2 {
        return Err(Error::Unavailable);
    }
    let side = side.ok_or(Error::Unavailable)?;
    let seal_marker: i64 = side.try_get("marker").map_err(storage_error)?;
    if seal_marker != 2 {
        return Err(Error::Unavailable);
    }
    let after: String = side.try_get("after_json").map_err(storage_error)?;
    let full_before: Option<String> = side.try_get("before_json").map_err(storage_error)?;
    if full_before.is_some() != (revision > 1) {
        return Err(Error::Unavailable);
    }
    let mut full = json(&after)?;
    if !exact_keys(&full, &["snapshot_version", "content"]) || full["snapshot_version"] != 2 {
        return Err(Error::Unavailable);
    }
    let object = full["content"].as_object_mut().ok_or(Error::Unavailable)?;
    let typing = if matches!(kind, HistoryKind::World) {
        let ids: Vec<String> =
            serde_json::from_value(object.remove("type_ids").ok_or(Error::Unavailable)?)
                .map_err(|_| Error::Unavailable)?;
        if ids.iter().any(|id| !content::valid_id(id)) || ids.windows(2).any(|v| v[0] >= v[1]) {
            return Err(Error::Unavailable);
        }
        validate_world_membership(c, id, &ids, true).await?;
        TypingState::unmodeled()
    } else {
        let value = object.remove("typing").ok_or(Error::Unavailable)?;
        if !exact_keys(&value, &["assignments"]) {
            return Err(Error::Unavailable);
        }
        let typing: TypingState = serde_json::from_value(value).map_err(|_| Error::Unavailable)?;
        let pins = typing.pins();
        validate_pins(pins.as_deref()).map_err(|_| Error::Unavailable)?;
        let defined: i64 = side.try_get("typing_defined").map_err(storage_error)?;
        let bindings = read_bindings(c, kind, id, scope.as_deref(), revision).await?;
        if defined != i64::from(typing.assignments.is_some())
            || typing.assignments.as_deref().unwrap_or(&[]) != bindings
        {
            return Err(Error::Unavailable);
        }
        typing
    };
    full["snapshot_version"] = 1.into();
    if full != projected {
        return Err(Error::Unavailable);
    }
    // The projection's strict v1 parser above rejects every unrecognized full field.
    let extension = if matches!(kind, HistoryKind::World) {
        None
    } else {
        super::stat_content::load(c, kind, id, stat_world, revision, stat_marker, &after).await?
    };
    let (before, authoritative, stats) = match extension {
        Some(v) => (v.before, v.after, v.stats),
        None => (full_before.clone(), after.clone(), BaseStats::unmodeled()),
    };
    Ok(FullContent {
        before,
        after: authoritative,
        projection_after,
        typing,
        stats,
        typing_projection_after: after,
        typing_projection_before: full_before,
    })
}
/// Membership snapshots retain identities, independently of current metadata/archive state
/// and of children added in later world revisions.
async fn validate_world_membership(
    c: &mut SqliteConnection,
    world: &str,
    ids: &[String],
    types: bool,
) -> Result<(), Error> {
    let sql = if types {
        "SELECT EXISTS(SELECT 1 FROM world_types WHERE world_id=? AND id=?)"
    } else {
        "SELECT EXISTS(SELECT 1 FROM variants WHERE world_id=? AND id=?)"
    };
    for id in ids {
        let exists: bool = sqlx::query_scalar(sql)
            .bind(world)
            .bind(id)
            .fetch_one(&mut *c)
            .await
            .map_err(storage_error)?;
        if !exists {
            return Err(Error::Unavailable);
        }
    }
    Ok(())
}
pub(crate) async fn full_content(
    c: &mut SqliteConnection,
    kind: HistoryKind,
    id: &str,
    world: Option<&str>,
    revision: i64,
) -> Result<FullContent, Error> {
    let full = load_content(c, kind, id, world, revision).await?;
    if revision > 1 {
        let previous = load_content(c, kind, id, world, revision - 1)
            .await
            .map_err(|_| Error::Unavailable)?;
        if full.before.as_deref() != Some(previous.after.as_str()) {
            return Err(Error::Unavailable);
        }
        if full.typing_projection_after != full.projection_after
            && full.typing_projection_before.as_deref()
                != Some(previous.typing_projection_after.as_str())
        {
            return Err(Error::Unavailable);
        }
        let (legacy, _, key, _) = tables(kind)?;
        let s = format!("SELECT before_json FROM {legacy} WHERE {key}=? AND revision=?");
        let before: String = sqlx::query_scalar(sqlx::AssertSqlSafe(s))
            .bind(id)
            .bind(revision)
            .fetch_one(c)
            .await
            .map_err(storage_error)?;
        if before != previous.projection_after {
            return Err(Error::Unavailable);
        }
    }
    Ok(full)
}
async fn read_bindings(
    c: &mut SqliteConnection,
    kind: HistoryKind,
    id: &str,
    world: Option<&str>,
    revision: i64,
) -> Result<Vec<TypeAssignment>, Error> {
    let sql = match kind {
        HistoryKind::Form => {
            "SELECT slot,type_id,type_version FROM form_type_bindings WHERE form_id=?1 AND revision=?2 AND (?3 IS NULL OR ?3 IS NOT NULL) ORDER BY slot"
        }
        HistoryKind::Variant => {
            "SELECT slot,type_id,type_version FROM variant_type_bindings WHERE variant_id=?1 AND revision=?2 AND world_id=?3 ORDER BY slot"
        }
        HistoryKind::World => return Ok(Vec::new()),
        _ => return Err(Error::Unavailable),
    };
    let rows = sqlx::query(sql)
        .bind(id)
        .bind(revision)
        .bind(world)
        .fetch_all(&mut *c)
        .await
        .map_err(storage_error)?;
    if rows.len() > 32 {
        return Err(Error::Unavailable);
    }
    let mut assignments = Vec::with_capacity(rows.len());
    for (i, r) in rows.into_iter().enumerate() {
        let slot: i64 = r.try_get("slot").map_err(storage_error)?;
        if slot != (i + 1) as i64 {
            return Err(Error::Unavailable);
        }
        let pin = TypePin {
            type_id: content::id(&r, "type_id")?,
            version: content::revision(&r, "type_version")?,
        };
        let fields = if matches!(kind, HistoryKind::Form) {
            type_version(c, &pin.type_id, pin.version)
                .await
                .map_err(|_| Error::Unavailable)?
                .fields
        } else {
            world_type_version(
                c,
                world.ok_or(Error::Unavailable)?,
                &pin.type_id,
                pin.version,
            )
            .await
            .map_err(|_| Error::Unavailable)?
            .fields
        };
        assignments.push(TypeAssignment { pin, fields });
    }
    validate_pins(Some(
        &assignments
            .iter()
            .map(|a| a.pin.clone())
            .collect::<Vec<_>>(),
    ))
    .map_err(|_| Error::Unavailable)?;
    Ok(assignments)
}
pub(crate) async fn form_typing(
    c: &mut SqliteConnection,
    id: &str,
    revision: i64,
) -> Result<TypingState, Error> {
    Ok(full_content(c, HistoryKind::Form, id, None, revision)
        .await?
        .typing)
}
pub(crate) async fn variant_typing(
    c: &mut SqliteConnection,
    world: &str,
    id: &str,
    revision: i64,
) -> Result<TypingState, Error> {
    Ok(
        full_content(c, HistoryKind::Variant, id, Some(world), revision)
            .await?
            .typing,
    )
}
pub(crate) struct ContentSeal {
    pub(crate) projection_before: Option<String>,
    before: Option<String>,
    after: String,
    typing: TypingState,
    stats: Option<super::stat_content::StatContent>,
}
#[allow(clippy::too_many_arguments)]
pub(crate) async fn prepare_content(
    c: &mut SqliteConnection,
    kind: HistoryKind,
    id: &str,
    world: Option<&str>,
    revision: i64,
    projection: &str,
    typing: Option<&TypingState>,
    stats: Option<&BaseStats>,
) -> Result<ContentSeal, Error> {
    let previous = if revision > 1 {
        Some(full_content(c, kind, id, world, revision - 1).await?)
    } else {
        None
    };
    let typing = typing.cloned().unwrap_or_else(|| {
        previous
            .as_ref()
            .map(|v| v.typing.clone())
            .unwrap_or_else(TypingState::unmodeled)
    });
    let mut after = json(projection)?;
    after["snapshot_version"] = 2.into();
    if matches!(kind, HistoryKind::World) {
        let ids: Vec<String> =
            sqlx::query_scalar("SELECT id FROM world_types WHERE world_id=? ORDER BY id")
                .bind(id)
                .fetch_all(&mut *c)
                .await
                .map_err(storage_error)?;
        after["content"]["type_ids"] = serde_json::to_value(ids).map_err(|_| Error::Unavailable)?;
    } else {
        after["content"]["typing"] =
            serde_json::to_value(&typing).map_err(|_| Error::Unavailable)?;
    }
    let stat_seal = if matches!(kind, HistoryKind::World) {
        None
    } else {
        let stats = stats.cloned().unwrap_or_else(|| {
            previous
                .as_ref()
                .map(|p| p.stats.clone())
                .unwrap_or_default()
        });
        super::stat_content::validate(c, kind, world, &stats).await?;
        let mut v3 = after.clone();
        v3["snapshot_version"] = 3.into();
        v3["content"]["stats"] = serde_json::to_value(&stats).map_err(|_| Error::Unavailable)?;
        Some(super::stat_content::StatContent {
            before: previous.as_ref().map(|p| p.after.clone()),
            after: encode(&v3)?,
            stats,
        })
    };
    Ok(ContentSeal {
        projection_before: previous.as_ref().map(|p| p.projection_after.clone()),
        before: previous.map(|p| p.typing_projection_after),
        after: encode(&after)?,
        typing,
        stats: stat_seal,
    })
}
pub(crate) async fn seal_content(
    c: &mut SqliteConnection,
    kind: HistoryKind,
    id: &str,
    world: Option<&str>,
    revision: i64,
    seal: &ContentSeal,
) -> Result<(), Error> {
    for (slot, assignment) in seal
        .typing
        .assignments
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .enumerate()
    {
        let sql = if matches!(kind, HistoryKind::Form) {
            "INSERT INTO form_type_bindings(form_id,revision,slot,type_id,type_version) VALUES (?1,?2,?3,?4,?5)"
        } else {
            "INSERT INTO variant_type_bindings(variant_id,revision,slot,type_id,type_version,world_id) VALUES (?1,?2,?3,?4,?5,?6)"
        };
        let query = sqlx::query(sql)
            .bind(id)
            .bind(revision)
            .bind((slot + 1) as i64)
            .bind(&assignment.pin.type_id)
            .bind(assignment.pin.version);
        let query = if matches!(kind, HistoryKind::Form) {
            query
        } else {
            query.bind(world)
        };
        query.execute(&mut *c).await.map_err(storage_error)?;
    }
    let sql = match kind {
        HistoryKind::Form => {
            "INSERT INTO form_content_snapshots(form_id,revision,before_json,after_json,typing_defined) VALUES (?1,?2,?3,?4,?5)"
        }
        HistoryKind::Variant => {
            "INSERT INTO variant_content_snapshots(variant_id,revision,before_json,after_json,typing_defined,world_id) VALUES (?1,?2,?3,?4,?5,?6)"
        }
        HistoryKind::World => {
            "INSERT INTO world_content_snapshots(world_id,revision,before_json,after_json) VALUES (?1,?2,?3,?4)"
        }
        _ => return Err(Error::Unavailable),
    };
    let query = sqlx::query(sql)
        .bind(id)
        .bind(revision)
        .bind(&seal.before)
        .bind(&seal.after);
    let query = if matches!(kind, HistoryKind::World) {
        query
    } else {
        query.bind(seal.typing.assignments.is_some())
    };
    let query = if matches!(kind, HistoryKind::Variant) {
        query.bind(world)
    } else {
        query
    };
    query.execute(&mut *c).await.map_err(storage_error)?;
    if let Some(stats) = &seal.stats {
        super::stat_content::seal(c, kind, id, world, revision, stats).await?;
    }
    Ok(())
}

#[derive(Clone, Debug, serde::Deserialize, PartialEq, Eq, sqlx::FromRow)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReferenceSource {
    pub(crate) endpoint_kind: String,
    pub(crate) external_id: i64,
    pub(crate) source_url: String,
    pub(crate) retrieved_at: String,
    pub(crate) transformation_version: i64,
}
impl ReferenceSource {
    pub(crate) fn valid(&self, expected_kind: &str) -> bool {
        self.endpoint_kind == expected_kind
            && self.external_id > 0
            && self.transformation_version == 1
            && self.source_url
                == format!(
                    "https://pokeapi.co/api/v2/{}/{}/",
                    self.endpoint_kind, self.external_id
                )
            && valid_utc_seconds(&self.retrieved_at)
    }
}

/// Gregorian UTC seconds for reference retrievals, with four-digit AD years.
/// Leap seconds are not represented; accepted years are 0001 through 9999.
fn valid_utc_seconds(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 20 {
        return false;
    }
    for (index, byte) in bytes.iter().enumerate() {
        let valid = match index {
            4 | 7 => *byte == b'-',
            10 => *byte == b'T',
            13 | 16 => *byte == b':',
            19 => *byte == b'Z',
            _ => byte.is_ascii_digit(),
        };
        if !valid {
            return false;
        }
    }
    let number = |range: std::ops::Range<usize>| {
        bytes[range]
            .iter()
            .fold(0_u32, |n, digit| n * 10 + u32::from(digit - b'0'))
    };
    let year = number(0..4);
    let month = number(5..7);
    let day = number(8..10);
    let leap_year =
        year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap_year => 29,
        2 => 28,
        _ => return false,
    };
    year > 0
        && (1..=days).contains(&day)
        && number(11..13) < 24
        && number(14..16) < 60
        && number(17..19) < 60
}
pub(crate) async fn put_reference_source(
    c: &mut SqliteConnection,
    kind: HistoryKind,
    id: &str,
    source: &ReferenceSource,
) -> Result<(), Error> {
    let (table, key) = source_table(kind)?;
    let sql = format!(
        "INSERT INTO {table}({key},revision,endpoint_kind,external_id,source_url,retrieved_at,transformation_version) VALUES (?,1,?,?,?,?,?)"
    );
    sqlx::query(sqlx::AssertSqlSafe(sql))
        .bind(id)
        .bind(&source.endpoint_kind)
        .bind(source.external_id)
        .bind(&source.source_url)
        .bind(&source.retrieved_at)
        .bind(source.transformation_version)
        .execute(c)
        .await
        .map_err(storage_error)?;
    Ok(())
}
fn source_table(kind: HistoryKind) -> Result<(&'static str, &'static str), Error> {
    match kind {
        HistoryKind::Type => Ok(("type_version_sources", "type_id")),
        HistoryKind::Species => Ok(("species_version_sources", "species_id")),
        HistoryKind::Form => Ok(("form_version_sources", "form_id")),
        _ => Err(Error::Unavailable),
    }
}
pub(crate) async fn reference_source_matches(
    c: &mut SqliteConnection,
    kind: HistoryKind,
    id: &str,
    expected: &ReferenceSource,
) -> Result<bool, Error> {
    let (table, key) = source_table(kind)?;
    let sql = format!(
        "SELECT endpoint_kind,external_id,source_url,retrieved_at,transformation_version FROM {table} WHERE {key}=? AND revision=1"
    );
    let source: Option<ReferenceSource> = sqlx::query_as(sqlx::AssertSqlSafe(sql))
        .bind(id)
        .fetch_optional(c)
        .await
        .map_err(storage_error)?;
    Ok(source.as_ref() == Some(expected))
}
pub(crate) async fn fixture_actor(
    c: &mut SqliteConnection,
    kind: HistoryKind,
    id: &str,
) -> Result<bool, Error> {
    let (table, key) = match kind {
        HistoryKind::Type => ("type_versions", "type_id"),
        HistoryKind::Species => ("species_versions", "species_id"),
        HistoryKind::Form => ("form_versions", "form_id"),
        HistoryKind::World => ("world_revisions", "world_id"),
        HistoryKind::Variant => ("variant_revisions", "variant_id"),
        HistoryKind::WorldType => ("world_type_versions", "type_id"),
    };
    let sql = format!(
        "SELECT EXISTS(SELECT 1 FROM {table} r JOIN audit_events a ON a.id=r.audit_id WHERE r.{key}=? AND r.revision=1 AND a.system_key='bundled-fixture-loader' AND a.action='create' AND a.actor_user_id IS NULL)"
    );
    sqlx::query_scalar(sqlx::AssertSqlSafe(sql))
        .bind(id)
        .fetch_one(c)
        .await
        .map_err(storage_error)
}
pub(crate) async fn identity_exists(c: &mut SqliteConnection, id: &str) -> Result<bool, Error> {
    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM types WHERE id=?1 UNION ALL SELECT 1 FROM species WHERE id=?1 UNION ALL SELECT 1 FROM forms WHERE id=?1 UNION ALL SELECT 1 FROM worlds WHERE id=?1 UNION ALL SELECT 1 FROM world_types WHERE id=?1 UNION ALL SELECT 1 FROM variants WHERE id=?1)").bind(id).fetch_one(c).await.map_err(storage_error)
}
pub(crate) async fn initial_fixture_slug_exists(
    c: &mut SqliteConnection,
    kind: HistoryKind,
    slug: &str,
) -> Result<bool, Error> {
    let sql = match kind {
        HistoryKind::Type => {
            "SELECT EXISTS(SELECT 1 FROM type_versions WHERE revision=1 AND slug=?)"
        }
        HistoryKind::World => {
            "SELECT EXISTS(SELECT 1 FROM world_revisions WHERE revision=1 AND json_extract(after_json,'$.content.fields.slug')=?)"
        }
        _ => return Err(Error::Unavailable),
    };
    sqlx::query_scalar(sql)
        .bind(slug)
        .fetch_one(c)
        .await
        .map_err(storage_error)
}
pub(crate) async fn fixture_access(
    c: &mut SqliteConnection,
    world: &str,
    owner: &str,
) -> Result<bool, Error> {
    let r = sqlx::query("SELECT r.after_json,a.system_key,a.action FROM world_access_revisions r JOIN audit_events a ON a.id=r.audit_id WHERE world_id=? AND revision=1").bind(world).fetch_optional(c).await.map_err(storage_error)?;
    let Some(r) = r else {
        return Ok(false);
    };
    let after: String = r.try_get("after_json").map_err(storage_error)?;
    let system: Option<String> = r.try_get("system_key").map_err(storage_error)?;
    let action: String = r.try_get("action").map_err(storage_error)?;
    content::validate_snapshot(&after, SnapshotKind::Access, world, None, 1)?;
    Ok(system.as_deref() == Some("bundled-fixture-loader")
        && action == "own"
        && json(&after)?
            == serde_json::json!({"snapshot_version":1,"content":{"world_id":world,"owner_user_id":owner,"access_revision":1,"collaborator_ids":[]}}))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(timestamp: &str) -> ReferenceSource {
        ReferenceSource {
            endpoint_kind: "type".into(),
            external_id: 10,
            source_url: "https://pokeapi.co/api/v2/type/10/".into(),
            retrieved_at: timestamp.into(),
            transformation_version: 1,
        }
    }

    #[test]
    fn reference_retrieval_requires_exact_ascii_utc_seconds() {
        let good = "2026-10-15T02:11:12Z";
        assert!(source(good).valid("type"));
        // Every digit and delimiter position is independently checked, including T/Z.
        for index in 0..good.len() {
            let mut bad = good.as_bytes().to_vec();
            bad[index] = b'x';
            let bad = String::from_utf8(bad).unwrap();
            assert!(!source(&bad).valid("type"), "position {index}: {bad}");
        }
        for bad in [
            "",
            "2026-10-15",
            "2026-10-15T02:11:12",
            "2026-10-15T02:11:12.0Z",
            "2026-10-15T02:11:12+00:00",
            "2026-10-15t02:11:12Z",
            "2026-10-15T02:11:12z",
            "２０２６-10-15T02:11:12Z",
            "2026-10-15T02:11:12Z ",
        ] {
            assert!(!source(bad).valid("type"), "{bad}");
        }
    }

    #[test]
    fn reference_retrieval_checks_gregorian_calendar_and_clock_ranges() {
        for good in [
            "0001-01-01T00:00:00Z",
            "9999-12-31T23:59:59Z",
            "1900-02-28T23:59:59Z",
            "2000-02-29T00:00:00Z",
            "2024-02-29T12:30:59Z",
            "2100-02-28T00:00:00Z",
            "2400-02-29T00:00:00Z",
            "2026-04-30T00:00:00Z",
            "2026-01-31T00:00:00Z",
        ] {
            assert!(source(good).valid("type"), "{good}");
        }
        for bad in [
            "0000-01-01T00:00:00Z",
            "2026-00-01T00:00:00Z",
            "2026-13-01T00:00:00Z",
            "2026-01-00T00:00:00Z",
            "2026-01-32T00:00:00Z",
            "2026-04-31T00:00:00Z",
            "2026-06-31T00:00:00Z",
            "2026-09-31T00:00:00Z",
            "2026-11-31T00:00:00Z",
            "1900-02-29T00:00:00Z",
            "2000-02-30T00:00:00Z",
            "2026-02-29T00:00:00Z",
            "2100-02-29T00:00:00Z",
            "2026-01-01T24:00:00Z",
            "2026-01-01T00:60:00Z",
            "2026-01-01T00:00:60Z",
            "2026-99-99T99:99:99Z",
        ] {
            assert!(!source(bad).valid("type"), "{bad}");
        }
    }
}
