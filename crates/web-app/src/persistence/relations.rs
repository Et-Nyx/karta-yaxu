//! ADR0018 typed storage. Callers own the read snapshot / locked write transaction.
//! Capture can enumerate `record_layouts` and `record_collection_definitions` by
//! (scope,id,revision); card evidence is keyed by (namespace,scope,card_id,revision).
use super::{authoring as content, cards, stats};
use crate::application::{
    authoring::{self as author, Error},
    cards::CardHistoryEntry,
};
use game_data::{authoring::ContentFields, cards::*, relations::*, stats::Decimal};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sqlx::{AssertSqlSafe, Row, SqliteConnection, sqlite::SqliteRow};

fn scope(w: Option<&str>) -> &str {
    w.unwrap_or("")
}
fn world(s: String) -> Option<String> {
    if s.is_empty() { None } else { Some(s) }
}
fn get<T: for<'r> sqlx::Decode<'r, sqlx::Sqlite> + sqlx::Type<sqlx::Sqlite>>(
    r: &SqliteRow,
    n: &str,
) -> Result<T, Error> {
    r.try_get(n).map_err(content::storage_error)
}
fn require(v: bool) -> Result<(), Error> {
    if v { Ok(()) } else { Err(Error::Unavailable) }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Snapshot<T> {
    snapshot_version: i64,
    content: T,
}
fn parse<T: DeserializeOwned + Serialize>(s: &str) -> Result<T, Error> {
    let x: Snapshot<T> = serde_json::from_str(s).map_err(|_| Error::Unavailable)?;
    let value: serde_json::Value = serde_json::from_str(s).map_err(|_| Error::Unavailable)?;
    require(
        x.snapshot_version == 1
            && value == serde_json::json!({"snapshot_version":1,"content":x.content}),
    )?;
    Ok(x.content)
}
#[derive(Clone, Copy)]
pub(crate) enum Kind {
    Layout,
    Collection,
}
impl Kind {
    fn tables(self) -> (&'static str, &'static str) {
        match self {
            Self::Layout => ("record_layouts", "record_layout_versions"),
            Self::Collection => (
                "record_collection_definitions",
                "record_collection_versions",
            ),
        }
    }
}
pub(crate) async fn allocate_slug(
    c: &mut SqliteConnection,
    k: Kind,
    w: Option<&str>,
    f: &mut ContentFields,
    except: Option<&str>,
) -> Result<(), Error> {
    let (h, _) = k.tables();
    let base = game_data::authoring::slug_from_name(&f.display_name);
    for n in 1..=10000 {
        let s = game_data::authoring::slug_candidate(&base, n);
        let exists: bool = sqlx::query_scalar(AssertSqlSafe(format!(
            "SELECT EXISTS(SELECT 1 FROM {h} WHERE scope=? AND slug=? AND (? IS NULL OR id<>?))"
        )))
        .bind(scope(w))
        .bind(&s)
        .bind(except)
        .bind(except)
        .fetch_one(&mut *c)
        .await
        .map_err(content::storage_error)?;
        if !exists {
            f.slug = s;
            return Ok(());
        }
    }
    Err(author::invalid(
        "display_name",
        "Cannot allocate a record address.",
    ))
}
pub(crate) async fn ids(
    c: &mut SqliteConnection,
    k: Kind,
    w: Option<&str>,
    query: &str,
    page: usize,
) -> Result<Vec<String>, Error> {
    if query.chars().count() > 100 || !(1..=10000).contains(&page) {
        return Err(author::invalid(
            "query",
            "Use at most 100 literal characters and a page from 1 to 10000.",
        ));
    }
    let (h, v) = k.tables();
    let offset = i64::try_from(
        page.saturating_sub(1)
            .checked_mul(25)
            .ok_or(Error::NotFound)?,
    )
    .map_err(|_| Error::NotFound)?;
    // instr is literal, including %, _ and backslashes. Filter before paging.
    sqlx::query_scalar(AssertSqlSafe(format!("SELECT h.id FROM {h} h LEFT JOIN {v} v ON v.scope=h.scope AND v.id=h.id AND v.revision=h.revision WHERE h.scope=? AND (v.id IS NULL OR instr(lower(v.display_name),lower(?))>0) ORDER BY h.slug,h.id LIMIT 25 OFFSET ?"))).bind(scope(w)).bind(query.trim()).bind(offset).fetch_all(c).await.map_err(content::storage_error)
}
async fn head(
    c: &mut SqliteConnection,
    k: Kind,
    w: Option<&str>,
    id: &str,
) -> Result<(i64, String), Error> {
    let (h, _) = k.tables();
    let r = sqlx::query(AssertSqlSafe(format!(
        "SELECT * FROM {h} WHERE scope=? AND id=?"
    )))
    .bind(scope(w))
    .bind(id)
    .fetch_optional(c)
    .await
    .map_err(content::storage_error)?
    .ok_or(Error::NotFound)?;
    require(
        get::<String>(&r, "scope")? == scope(w)
            && get::<String>(&r, "id")? == id
            && get::<Option<String>>(&r, "world_id")?.as_deref() == w,
    )?;
    Ok((get(&r, "revision")?, get(&r, "slug")?))
}
pub(crate) async fn layout(
    c: &mut SqliteConnection,
    w: Option<&str>,
    id: &str,
) -> Result<LayoutRecord, Error> {
    let (rev, slug) = head(c, Kind::Layout, w, id).await?;
    let out = layout_version(
        c,
        &LayoutPin {
            world_id: w.map(str::to_owned),
            layout_id: id.into(),
            version: rev,
        },
    )
    .await?;
    require(out.fields.slug == slug)?;
    Ok(out)
}
pub(crate) async fn collection(
    c: &mut SqliteConnection,
    w: Option<&str>,
    id: &str,
) -> Result<CollectionDefinition, Error> {
    let (rev, slug) = head(c, Kind::Collection, w, id).await?;
    let out = collection_version(
        c,
        &CollectionPin {
            world_id: w.map(str::to_owned),
            collection_id: id.into(),
            version: rev,
        },
    )
    .await?;
    require(out.fields.slug == slug)?;
    Ok(out)
}
async fn header(
    c: &mut SqliteConnection,
    k: Kind,
    w: Option<&str>,
    id: &str,
    rev: i64,
) -> Result<SqliteRow, Error> {
    let (_, v) = k.tables();
    let r=sqlx::query(AssertSqlSafe(format!("SELECT v.*,a.action,coalesce(u.display_name,a.system_key) AS actor FROM {v} v LEFT JOIN audit_events a ON a.id=v.audit_id LEFT JOIN users u ON u.id=a.actor_user_id WHERE v.scope=? AND v.id=? AND v.revision=?"))).bind(scope(w)).bind(id).bind(rev).fetch_optional(&mut *c).await.map_err(content::storage_error)?.ok_or(Error::NotFound)?;
    let action: String = get(&r, "action")?;
    let _: String = get(&r, "actor")?;
    require(
        rev > 0
            && get::<Option<i64>>(&r, "previous_revision")? == content::previous(rev)
            && matches!(action.as_str(), "create" | "edit" | "adopt")
            && (rev == 1 || action != "create")
            && (rev != 1 || action != "edit"),
    )?;
    let (hr, hs) = head(c, k, w, id).await?;
    require(hr >= rev && (hr != rev || hs == get::<String>(&r, "slug")?))?;
    Ok(r)
}
fn fields(r: &SqliteRow) -> Result<ContentFields, Error> {
    Ok(ContentFields {
        slug: get(r, "slug")?,
        display_name: get(r, "display_name")?,
        description: get(r, "description")?,
    })
}
fn source(r: &SqliteRow) -> Result<Option<(String, i64)>, Error> {
    match (
        get::<Option<String>>(r, "source_id")?,
        get::<Option<i64>>(r, "source_revision")?,
    ) {
        (None, None) => Ok(None),
        (Some(id), Some(v)) => Ok(Some((id, v))),
        _ => Err(Error::Unavailable),
    }
}
async fn layout_raw(
    c: &mut SqliteConnection,
    p: &LayoutPin,
) -> Result<(LayoutRecord, Option<String>, String), Error> {
    p.validate().map_err(|_| Error::Unavailable)?;
    let w = p.world_id.as_deref();
    let r = header(c, Kind::Layout, w, &p.layout_id, p.version).await?;
    let rows=sqlx::query("SELECT * FROM record_layout_columns WHERE scope=? AND layout_id=? AND revision=? ORDER BY slot").bind(scope(w)).bind(&p.layout_id).bind(p.version).fetch_all(&mut *c).await.map_err(content::storage_error)?;
    let mut columns = vec![];
    for (n, r) in rows.iter().enumerate() {
        require(get::<i64>(r, "slot")? == n as i64)?;
        let allowed=sqlx::query("SELECT slot,kind FROM record_column_reference_kinds WHERE scope=? AND layout_id=? AND revision=? AND column_slot=? ORDER BY slot").bind(scope(w)).bind(&p.layout_id).bind(p.version).bind(n as i64).fetch_all(&mut *c).await.map_err(content::storage_error)?;
        let mut kinds = vec![];
        for (i, a) in allowed.iter().enumerate() {
            require(get::<i64>(a, "slot")? == i as i64)?;
            kinds.push(ref_kind(&get::<String>(a, "kind")?)?);
        }
        let kind = match get::<String>(r, "kind")?.as_str() {
            "field" => {
                require(kinds.is_empty())?;
                ColumnKind::Field(
                    cards::field_version(
                        c,
                        w,
                        &get::<String>(r, "field_id")?,
                        get(r, "field_revision")?,
                    )
                    .await?,
                )
            }
            "reference" => {
                require(
                    get::<Option<String>>(r, "field_id")?.is_none()
                        && get::<Option<i64>>(r, "field_revision")?.is_none(),
                )?;
                ColumnKind::Reference { allowed: kinds }
            }
            _ => return Err(Error::Unavailable),
        };
        columns.push(RecordColumn {
            id: get(r, "column_id")?,
            label: get(r, "label")?,
            kind,
        });
    }
    let row = LayoutRecord {
        id: p.layout_id.clone(),
        world_id: p.world_id.clone(),
        revision: p.version,
        fields: fields(&r)?,
        archived_at: get(&r, "archived_at")?,
        source: source(&r)?.map(|(layout_id, version)| LayoutPin {
            world_id: None,
            layout_id,
            version,
        }),
        columns,
    };
    row.validate().map_err(|_| Error::Unavailable)?;
    let after: String = get(&r, "after_json")?;
    require(parse::<LayoutRecord>(&after)? == row)?;
    let before: Option<String> = get(&r, "before_json")?;
    require((p.version == 1) == before.is_none())?;
    Ok((row, before, after))
}
pub(crate) async fn layout_version(
    c: &mut SqliteConnection,
    p: &LayoutPin,
) -> Result<LayoutRecord, Error> {
    let row = layout_checked(c, p).await?;
    if let Some(s) = &row.source {
        layout_checked(c, s).await.map_err(|_| Error::Unavailable)?;
    }
    Ok(row)
}
async fn layout_checked(c: &mut SqliteConnection, p: &LayoutPin) -> Result<LayoutRecord, Error> {
    let (row, before, _) = layout_raw(c, p).await?;
    if p.version > 1 {
        let (_, _, previous) = layout_raw(
            c,
            &LayoutPin {
                version: p.version - 1,
                ..p.clone()
            },
        )
        .await
        .map_err(|_| Error::Unavailable)?;
        require(before.as_deref() == Some(previous.as_str()))?;
    }
    Ok(row)
}
async fn collection_raw(
    c: &mut SqliteConnection,
    p: &CollectionPin,
) -> Result<(CollectionDefinition, Option<String>, String), Error> {
    p.validate().map_err(|_| Error::Unavailable)?;
    let r = header(
        c,
        Kind::Collection,
        p.world_id.as_deref(),
        &p.collection_id,
        p.version,
    )
    .await?;
    let layout = layout_version(
        c,
        &LayoutPin {
            world_id: p.world_id.clone(),
            layout_id: get(&r, "layout_id")?,
            version: get(&r, "layout_revision")?,
        },
    )
    .await?;
    let row = CollectionDefinition {
        id: p.collection_id.clone(),
        world_id: p.world_id.clone(),
        revision: p.version,
        fields: fields(&r)?,
        archived_at: get(&r, "archived_at")?,
        source: source(&r)?.map(|(collection_id, version)| CollectionPin {
            world_id: None,
            collection_id,
            version,
        }),
        layout,
    };
    row.validate().map_err(|_| Error::Unavailable)?;
    let after: String = get(&r, "after_json")?;
    require(parse::<CollectionDefinition>(&after)? == row)?;
    let before: Option<String> = get(&r, "before_json")?;
    require((p.version == 1) == before.is_none())?;
    Ok((row, before, after))
}
pub(crate) async fn collection_version(
    c: &mut SqliteConnection,
    p: &CollectionPin,
) -> Result<CollectionDefinition, Error> {
    let row = collection_checked(c, p).await?;
    if let Some(s) = &row.source {
        collection_checked(c, s)
            .await
            .map_err(|_| Error::Unavailable)?;
    }
    Ok(row)
}
async fn collection_checked(
    c: &mut SqliteConnection,
    p: &CollectionPin,
) -> Result<CollectionDefinition, Error> {
    let (row, before, _) = collection_raw(c, p).await?;
    if p.version > 1 {
        let (_, _, previous) = collection_raw(
            c,
            &CollectionPin {
                version: p.version - 1,
                ..p.clone()
            },
        )
        .await
        .map_err(|_| Error::Unavailable)?;
        require(before.as_deref() == Some(previous.as_str()))?;
    }
    Ok(row)
}
pub(crate) async fn selectable_layout(
    c: &mut SqliteConnection,
    p: &LayoutPin,
) -> Result<LayoutRecord, Error> {
    let row = layout_version(c, p).await?;
    active(
        row.archived_at.is_none()
            && layout(c, p.world_id.as_deref(), &p.layout_id)
                .await?
                .archived_at
                .is_none(),
    )?;
    Ok(row)
}
pub(crate) async fn selectable_collection(
    c: &mut SqliteConnection,
    p: &CollectionPin,
) -> Result<CollectionDefinition, Error> {
    let row = collection_version(c, p).await?;
    active(
        row.archived_at.is_none()
            && collection(c, p.world_id.as_deref(), &p.collection_id)
                .await?
                .archived_at
                .is_none(),
    )?;
    Ok(row)
}
fn active(v: bool) -> Result<(), Error> {
    if v {
        Ok(())
    } else {
        Err(author::invalid(
            "relations",
            "Choose active definitions for new selections.",
        ))
    }
}
pub(crate) async fn validate_columns(
    c: &mut SqliteConnection,
    row: &LayoutRecord,
    old: Option<&LayoutRecord>,
) -> Result<(), Error> {
    row.validate().map_err(Error::Invalid)?;
    for col in &row.columns {
        if let ColumnKind::Field(d) = &col.kind {
            require(
                cards::field_version(c, d.world_id.as_deref(), &d.id, d.revision).await? == *d,
            )?;
            let retained = old.is_some_and(|o| {
                o.columns
                    .iter()
                    .any(|a| a.id == col.id && a.kind == col.kind)
            });
            if !retained {
                active(
                    d.archived_at.is_none()
                        && cards::field(c, d.world_id.as_deref(), &d.id)
                            .await?
                            .archived_at
                            .is_none(),
                )?;
            }
        }
    }
    Ok(())
}
pub(crate) async fn put_layout(
    c: &mut SqliteConnection,
    row: &LayoutRecord,
    actor: &str,
    action: &str,
    reason: Option<&str>,
) -> Result<(), Error> {
    put_layout_as(c, row, Some(actor), action, reason).await
}
pub(crate) async fn put_layout_as(
    c: &mut SqliteConnection,
    row: &LayoutRecord,
    actor: Option<&str>,
    action: &str,
    reason: Option<&str>,
) -> Result<(), Error> {
    row.validate().map_err(Error::Invalid)?;
    let before = if row.revision > 1 {
        Some(author::snapshot(
            &layout_version(
                c,
                &LayoutPin {
                    version: row.revision - 1,
                    ..row.pin()
                },
            )
            .await?,
        )?)
    } else {
        None
    };
    if let Some(p) = &row.source {
        layout_version(c, p).await?;
    }
    let after = author::snapshot(row)?;
    let audit = content::audit(
        c,
        &content::Revision::new(actor, action, reason, before.clone(), after.clone()),
    )
    .await?;
    sqlx::query("INSERT INTO record_layouts(scope,id,world_id,slug,revision) VALUES (?,?,?,?,?) ON CONFLICT(scope,id) DO UPDATE SET slug=excluded.slug,revision=excluded.revision").bind(scope(row.world_id.as_deref())).bind(&row.id).bind(&row.world_id).bind(&row.fields.slug).bind(row.revision).execute(&mut *c).await.map_err(content::content_write_error)?;
    for (i, col) in row.columns.iter().enumerate() {
        let d = if let ColumnKind::Field(d) = &col.kind {
            Some(d)
        } else {
            None
        };
        sqlx::query("INSERT INTO record_layout_columns(scope,layout_id,revision,slot,column_id,label,kind,field_id,field_revision) VALUES (?,?,?,?,?,?,?,?,?)").bind(scope(row.world_id.as_deref())).bind(&row.id).bind(row.revision).bind(i as i64).bind(&col.id).bind(&col.label).bind(if d.is_some(){"field"}else{"reference"}).bind(d.map(|d|&d.id)).bind(d.map(|d|d.revision)).execute(&mut *c).await.map_err(content::storage_error)?;
        if let ColumnKind::Reference { allowed } = &col.kind {
            for (j, k) in allowed.iter().enumerate() {
                sqlx::query("INSERT INTO record_column_reference_kinds(scope,layout_id,revision,column_slot,slot,kind) VALUES (?,?,?,?,?,?)").bind(scope(row.world_id.as_deref())).bind(&row.id).bind(row.revision).bind(i as i64).bind(j as i64).bind(kind_str(*k)).execute(&mut *c).await.map_err(content::storage_error)?;
            }
        }
    }
    sqlx::query("INSERT INTO record_layout_versions(scope,id,revision,source_id,source_revision,slug,display_name,description,archived_at,audit_id,previous_revision,before_json,after_json) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?)").bind(scope(row.world_id.as_deref())).bind(&row.id).bind(row.revision).bind(row.source.as_ref().map(|s|&s.layout_id)).bind(row.source.as_ref().map(|s|s.version)).bind(&row.fields.slug).bind(&row.fields.display_name).bind(&row.fields.description).bind(row.archived_at).bind(audit).bind(content::previous(row.revision)).bind(before).bind(after).execute(&mut *c).await.map_err(content::storage_error)?;
    require(layout_version(c, &row.pin()).await? == *row)
}
pub(crate) async fn put_collection(
    c: &mut SqliteConnection,
    row: &CollectionDefinition,
    actor: &str,
    action: &str,
    reason: Option<&str>,
) -> Result<(), Error> {
    put_collection_as(c, row, Some(actor), action, reason).await
}
pub(crate) async fn put_collection_as(
    c: &mut SqliteConnection,
    row: &CollectionDefinition,
    actor: Option<&str>,
    action: &str,
    reason: Option<&str>,
) -> Result<(), Error> {
    row.validate().map_err(Error::Invalid)?;
    require(layout_version(c, &row.layout.pin()).await? == row.layout)?;
    let before = if row.revision > 1 {
        Some(author::snapshot(
            &collection_version(
                c,
                &CollectionPin {
                    version: row.revision - 1,
                    ..row.pin()
                },
            )
            .await?,
        )?)
    } else {
        None
    };
    if let Some(p) = &row.source {
        collection_version(c, p).await?;
    }
    let after = author::snapshot(row)?;
    let audit = content::audit(
        c,
        &content::Revision::new(actor, action, reason, before.clone(), after.clone()),
    )
    .await?;
    sqlx::query("INSERT INTO record_collection_definitions(scope,id,world_id,slug,revision) VALUES (?,?,?,?,?) ON CONFLICT(scope,id) DO UPDATE SET slug=excluded.slug,revision=excluded.revision").bind(scope(row.world_id.as_deref())).bind(&row.id).bind(&row.world_id).bind(&row.fields.slug).bind(row.revision).execute(&mut *c).await.map_err(content::content_write_error)?;
    sqlx::query("INSERT INTO record_collection_versions(scope,id,revision,source_id,source_revision,slug,display_name,description,archived_at,layout_id,layout_revision,audit_id,previous_revision,before_json,after_json) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)").bind(scope(row.world_id.as_deref())).bind(&row.id).bind(row.revision).bind(row.source.as_ref().map(|s|&s.collection_id)).bind(row.source.as_ref().map(|s|s.version)).bind(&row.fields.slug).bind(&row.fields.display_name).bind(&row.fields.description).bind(row.archived_at).bind(&row.layout.id).bind(row.layout.revision).bind(audit).bind(content::previous(row.revision)).bind(before).bind(after).execute(&mut *c).await.map_err(content::storage_error)?;
    require(collection_version(c, &row.pin()).await? == *row)
}
fn kind_str(k: ReferenceKind) -> &'static str {
    match k {
        ReferenceKind::Card => "card",
        ReferenceKind::Stat => "stat",
        ReferenceKind::Field => "field",
    }
}
fn ref_kind(s: &str) -> Result<ReferenceKind, Error> {
    match s {
        "card" => Ok(ReferenceKind::Card),
        "stat" => Ok(ReferenceKind::Stat),
        "field" => Ok(ReferenceKind::Field),
        _ => Err(Error::Unavailable),
    }
}
fn namespace(s: &str) -> Result<CardNamespace, Error> {
    match s {
        "form" => Ok(CardNamespace::Form),
        "variant" => Ok(CardNamespace::Variant),
        "type" => Ok(CardNamespace::Type),
        "move" => Ok(CardNamespace::Move),
        "standalone" => Ok(CardNamespace::Standalone),
        _ => Err(Error::Unavailable),
    }
}

async fn relation_header(
    c: &mut SqliteConnection,
    p: &CardPin,
) -> Result<Option<SqliteRow>, Error> {
    sqlx::query("SELECT * FROM card_relation_versions WHERE namespace=? AND scope=? AND card_id=? AND revision=?").bind(p.identity.namespace.as_str()).bind(scope(p.identity.world_id.as_deref())).bind(&p.identity.id).bind(p.version).fetch_optional(c).await.map_err(content::storage_error)
}
async fn legacy(c: &mut SqliteConnection, p: &CardPin) -> Result<bool, Error> {
    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM card_relation_legacy_boundaries WHERE namespace=? AND scope=? AND id=? AND maximum_revision>=?)").bind(p.identity.namespace.as_str()).bind(scope(p.identity.world_id.as_deref())).bind(&p.identity.id).bind(p.version).fetch_one(c).await.map_err(content::storage_error)
}
async fn read_relations(
    c: &mut SqliteConnection,
    p: &CardPin,
    r: &SqliteRow,
) -> Result<CardRelations, Error> {
    let i = &p.identity;
    let s = scope(i.world_id.as_deref());
    let rows=sqlx::query("SELECT * FROM card_relation_collections WHERE namespace=? AND scope=? AND card_id=? AND revision=? ORDER BY slot").bind(i.namespace.as_str()).bind(s).bind(&i.id).bind(p.version).fetch_all(&mut *c).await.map_err(content::storage_error)?;
    let mut collections = vec![];
    for (n, r) in rows.iter().enumerate() {
        require(get::<i64>(r, "slot")? == n as i64)?;
        let definition = collection_version(
            c,
            &CollectionPin {
                world_id: i.world_id.clone(),
                collection_id: get(r, "collection_id")?,
                version: get(r, "collection_revision")?,
            },
        )
        .await?;
        let rows=sqlx::query("SELECT slot,entry_id FROM card_relation_entries WHERE namespace=? AND scope=? AND card_id=? AND revision=? AND collection_slot=? ORDER BY slot").bind(i.namespace.as_str()).bind(s).bind(&i.id).bind(p.version).bind(n as i64).fetch_all(&mut *c).await.map_err(content::storage_error)?;
        let mut entries = vec![];
        for (j, e) in rows.iter().enumerate() {
            require(get::<i64>(e, "slot")? == j as i64)?;
            let rows=sqlx::query("SELECT * FROM card_relation_cells WHERE namespace=? AND scope=? AND card_id=? AND revision=? AND collection_slot=? AND entry_slot=? ORDER BY slot").bind(i.namespace.as_str()).bind(s).bind(&i.id).bind(p.version).bind(n as i64).bind(j as i64).fetch_all(&mut *c).await.map_err(content::storage_error)?;
            let mut cells = vec![];
            for (k, cell) in rows.iter().enumerate() {
                require(get::<i64>(cell, "slot")? == k as i64)?;
                cells.push(RecordCell {
                    column_id: get(cell, "column_id")?,
                    value: read_value(cell)?,
                });
            }
            entries.push(RecordEntry {
                id: get(e, "entry_id")?,
                cells,
            });
        }
        let state: String = get(r, "state")?;
        if state != "entries" {
            require(entries.is_empty())?;
        }
        let value = match state.as_str() {
            "missing" => CollectionValue::Missing,
            "unknown" => CollectionValue::Unknown,
            "not_applicable" => CollectionValue::NotApplicable,
            "entries" => CollectionValue::Entries(entries),
            _ => return Err(Error::Unavailable),
        };
        collections.push(RecordCollection { definition, value });
    }
    let defined: i64 = get(r, "collections_defined")?;
    require(defined == 0 || defined == 1)?;
    require(defined == 1 || collections.is_empty())?;
    let out = CardRelations {
        collections: (defined == 1).then_some(collections),
    };
    out.validate(i).map_err(|_| Error::Unavailable)?;
    Ok(out)
}
fn read_value(r: &SqliteRow) -> Result<RecordValue, Error> {
    let state: String = get(r, "state")?;
    // Validate the complete typed row, including unused nullable columns.
    for (col, used) in [
        ("coefficient", state == "number"),
        ("scale", state == "number"),
        ("boolean_value", state == "boolean"),
        ("ref_revision", state == "reference"),
    ] {
        require(get::<Option<i64>>(r, col)?.is_some() == used)?;
    }
    for (col, used) in [
        ("text_value", state == "text"),
        ("option_id", state == "choice"),
        ("ref_kind", state == "reference"),
        ("ref_scope", state == "reference"),
        ("ref_id", state == "reference"),
    ] {
        require(get::<Option<String>>(r, col)?.is_some() == used)?;
    }
    let rk: Option<String> = get(r, "ref_kind")?;
    require(
        get::<Option<String>>(r, "ref_namespace")?.is_some() == (rk.as_deref() == Some("card")),
    )?;
    Ok(match state.as_str() {
        "missing" => RecordValue::Missing,
        "unknown" => RecordValue::Unknown,
        "not_applicable" => RecordValue::NotApplicable,
        "number" => {
            let n: i64 = get(r, "coefficient")?;
            let s = u8::try_from(get::<i64>(r, "scale")?).map_err(|_| Error::Unavailable)?;
            let d = Decimal::new(n, s).map_err(|_| Error::Unavailable)?;
            require(d.coefficient() == n && d.scale() == s)?;
            RecordValue::Number(d)
        }
        "text" => RecordValue::Text(get(r, "text_value")?),
        "boolean" => {
            let b: i64 = get(r, "boolean_value")?;
            require(b == 0 || b == 1)?;
            RecordValue::Boolean(b == 1)
        }
        "choice" => RecordValue::Choice(get(r, "option_id")?),
        "reference" => {
            let w = world(get(r, "ref_scope")?);
            let id = get(r, "ref_id")?;
            let version = get(r, "ref_revision")?;
            RecordValue::Reference(match ref_kind(rk.as_deref().ok_or(Error::Unavailable)?)? {
                ReferenceKind::Card => DefinitionPin::Card(CardPin {
                    identity: CardIdentity {
                        namespace: namespace(&get::<String>(r, "ref_namespace")?)?,
                        world_id: w,
                        id,
                    },
                    version,
                }),
                ReferenceKind::Stat => DefinitionPin::Stat {
                    world_id: w,
                    stat_id: id,
                    version,
                },
                ReferenceKind::Field => DefinitionPin::Field(game_data::card_fields::FieldPin {
                    world_id: w,
                    field_id: id,
                    version,
                }),
            })
        }
        _ => return Err(Error::Unavailable),
    })
}
/// Shallow same-event proof. Never calls a card reader or follows outgoing refs.
pub(crate) async fn verify_seal(
    c: &mut SqliteConnection,
    p: &CardPin,
    audit: &str,
) -> Result<CardRelationRecord, Error> {
    let out = checked_seal(c, p, audit).await?;
    if p.version > 1 && !out.legacy_unmodeled {
        let prev = CardPin {
            version: p.version - 1,
            ..p.clone()
        };
        let before: Option<String> = get(
            &relation_header(c, p).await?.ok_or(Error::Unavailable)?,
            "before_json",
        )?;
        let old = if legacy(c, &prev).await? {
            CardRelationRecord {
                card: prev,
                relations: CardRelations::default(),
                legacy_unmodeled: true,
            }
        } else {
            let audit = cards::native_audit(c, &prev).await?;
            checked_seal(c, &prev, &audit).await?
        };
        require(before.as_deref() == Some(author::snapshot(&old)?.as_str()))?;
    }
    Ok(out)
}
async fn checked_seal(
    c: &mut SqliteConnection,
    p: &CardPin,
    audit: &str,
) -> Result<CardRelationRecord, Error> {
    p.validate().map_err(|_| Error::Unavailable)?;
    let r = relation_header(c, p).await?;
    let is_legacy = legacy(c, p).await?;
    let out = if let Some(r) = r {
        require(
            !is_legacy
                && get::<Option<String>>(&r, "audit_id")?.as_deref() == Some(audit)
                && get::<Option<i64>>(&r, "previous_revision")? == content::previous(p.version),
        )?;
        require((p.version == 1) == get::<Option<String>>(&r, "before_json")?.is_none())?;
        let out = CardRelationRecord {
            card: p.clone(),
            relations: read_relations(c, p, &r).await?,
            legacy_unmodeled: false,
        };
        require(parse::<CardRelationRecord>(&get::<String>(&r, "snapshot_json")?)? == out)?;
        out
    } else {
        require(is_legacy)?;
        CardRelationRecord {
            card: p.clone(),
            relations: CardRelations::default(),
            legacy_unmodeled: true,
        }
    };
    out.validate().map_err(|_| Error::Unavailable)?;
    Ok(out)
}
pub(crate) async fn card_version(
    c: &mut SqliteConnection,
    p: &CardPin,
) -> Result<CardRelationRecord, Error> {
    cards::card_version(c, p).await?;
    let audit = cards::native_audit(c, p).await?;
    let out = verify_seal(c, p, &audit).await?;
    verify_dependencies(c, &out.relations, None).await?;
    Ok(out)
}
pub(crate) async fn stage(
    c: &mut SqliteConnection,
    p: &CardPin,
    x: &CardRelations,
) -> Result<(), Error> {
    p.validate().map_err(Error::Invalid)?;
    x.validate(&p.identity).map_err(Error::Invalid)?;
    let i = &p.identity;
    let s = scope(i.world_id.as_deref());
    sqlx::query("INSERT INTO card_relation_versions(namespace,scope,card_id,revision,collections_defined,previous_revision) VALUES (?,?,?,?,?,?)").bind(i.namespace.as_str()).bind(s).bind(&i.id).bind(p.version).bind(i64::from(x.collections.is_some())).bind(content::previous(p.version)).execute(&mut *c).await.map_err(content::storage_error)?;
    for (n, col) in x.collections.iter().flatten().enumerate() {
        let state = match col.value {
            CollectionValue::Missing => "missing",
            CollectionValue::Unknown => "unknown",
            CollectionValue::NotApplicable => "not_applicable",
            CollectionValue::Entries(_) => "entries",
        };
        sqlx::query("INSERT INTO card_relation_collections(namespace,scope,card_id,revision,slot,collection_id,collection_revision,state) VALUES (?,?,?,?,?,?,?,?)").bind(i.namespace.as_str()).bind(s).bind(&i.id).bind(p.version).bind(n as i64).bind(&col.definition.id).bind(col.definition.revision).bind(state).execute(&mut *c).await.map_err(content::storage_error)?;
        if let CollectionValue::Entries(entries) = &col.value {
            for (j, e) in entries.iter().enumerate() {
                sqlx::query("INSERT INTO card_relation_entries(namespace,scope,card_id,revision,collection_slot,slot,entry_id) VALUES (?,?,?,?,?,?,?)").bind(i.namespace.as_str()).bind(s).bind(&i.id).bind(p.version).bind(n as i64).bind(j as i64).bind(&e.id).execute(&mut *c).await.map_err(content::storage_error)?;
                for (k, cell) in e.cells.iter().enumerate() {
                    let mut number = None;
                    let mut text = None;
                    let mut boolean = None;
                    let mut choice = None;
                    let mut reference = None;
                    let state = match &cell.value {
                        RecordValue::Missing => "missing",
                        RecordValue::Unknown => "unknown",
                        RecordValue::NotApplicable => "not_applicable",
                        RecordValue::Number(d) => {
                            number = Some(*d);
                            "number"
                        }
                        RecordValue::Text(t) => {
                            text = Some(t);
                            "text"
                        }
                        RecordValue::Boolean(b) => {
                            boolean = Some(i64::from(*b));
                            "boolean"
                        }
                        RecordValue::Choice(o) => {
                            choice = Some(o);
                            "choice"
                        }
                        RecordValue::Reference(p) => {
                            reference = Some(p);
                            "reference"
                        }
                    };
                    let (rn, rid, rv) = match reference {
                        Some(DefinitionPin::Card(p)) => (
                            Some(p.identity.namespace.as_str()),
                            Some(&p.identity.id),
                            Some(p.version),
                        ),
                        Some(DefinitionPin::Stat {
                            stat_id, version, ..
                        }) => (None, Some(stat_id), Some(*version)),
                        Some(DefinitionPin::Field(p)) => (None, Some(&p.field_id), Some(p.version)),
                        None => (None, None, None),
                    };
                    sqlx::query("INSERT INTO card_relation_cells(namespace,scope,card_id,revision,collection_slot,entry_slot,slot,column_id,state,coefficient,scale,text_value,boolean_value,option_id,ref_kind,ref_namespace,ref_scope,ref_id,ref_revision) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)").bind(i.namespace.as_str()).bind(s).bind(&i.id).bind(p.version).bind(n as i64).bind(j as i64).bind(k as i64).bind(&cell.column_id).bind(state).bind(number.map(Decimal::coefficient)).bind(number.map(|d|i64::from(d.scale()))).bind(text).bind(boolean).bind(choice).bind(reference.map(|r|kind_str(r.kind()))).bind(rn).bind(reference.map(|r|scope(r.world_id()))).bind(rid).bind(rv).execute(&mut *c).await.map_err(content::storage_error)?;
                }
            }
        }
    }
    Ok(())
}
pub(crate) async fn seal_native(
    c: &mut SqliteConnection,
    p: &CardPin,
    audit_id: &str,
) -> Result<(), Error> {
    let previous = if p.version > 1 {
        Some(
            card_version(
                c,
                &CardPin {
                    version: p.version - 1,
                    ..p.clone()
                },
            )
            .await?,
        )
    } else {
        None
    };
    let relations = if let Some(r) = relation_header(c, p).await? {
        require(get::<Option<String>>(&r, "audit_id")?.is_none())?;
        read_relations(c, p, &r).await?
    } else {
        let x = previous
            .as_ref()
            .map(|p| p.relations.clone())
            .unwrap_or_default();
        stage(c, p, &x).await?;
        x
    };
    let out = CardRelationRecord {
        card: p.clone(),
        relations,
        legacy_unmodeled: false,
    };
    let before = previous.as_ref().map(author::snapshot).transpose()?;
    let n=sqlx::query("UPDATE card_relation_versions SET audit_id=?,before_json=?,snapshot_json=? WHERE namespace=? AND scope=? AND card_id=? AND revision=? AND audit_id IS NULL").bind(audit_id).bind(before).bind(author::snapshot(&out)?).bind(p.identity.namespace.as_str()).bind(scope(p.identity.world_id.as_deref())).bind(&p.identity.id).bind(p.version).execute(&mut *c).await.map_err(content::storage_error)?.rows_affected();
    require(n == 1)?;
    verify_seal(c, p, audit_id).await?;
    // Seal the same-transaction evidence before checking outgoing targets so an
    // explicit exact self-reference can resolve its own complete companion.
    // Any failure still rolls back the owning native revision and audit.
    verify_dependencies(c, &out.relations, None).await?;
    // Both companions must be completely sealed before the owning writer can
    // commit, including when an unexpected trigger silently suppresses a write.
    cards::verify_direct_pin(c, p).await?;
    Ok(())
}
async fn target(c: &mut SqliteConnection, p: &DefinitionPin, select: bool) -> Result<(), Error> {
    match p {
        DefinitionPin::Card(p) => {
            let row = cards::verify_direct_pin(c, p).await?;
            if select {
                active(
                    row.archived_at.is_none()
                        && cards::card(c, &p.identity).await?.archived_at.is_none(),
                )?;
            }
        }
        DefinitionPin::Field(p) => {
            let row =
                cards::field_version(c, p.world_id.as_deref(), &p.field_id, p.version).await?;
            if select {
                active(
                    row.archived_at.is_none()
                        && cards::field(c, p.world_id.as_deref(), &p.field_id)
                            .await?
                            .archived_at
                            .is_none(),
                )?;
            }
        }
        DefinitionPin::Stat {
            world_id,
            stat_id,
            version,
        } => {
            let row = stats::definition_version(c, world_id.as_deref(), stat_id, *version).await?;
            if select {
                active(
                    row.archived_at.is_none()
                        && stats::definition(c, world_id.as_deref(), stat_id)
                            .await?
                            .archived_at
                            .is_none(),
                )?;
            }
        }
    }
    Ok(())
}
async fn verify_dependencies(
    c: &mut SqliteConnection,
    x: &CardRelations,
    old: Option<&CardRelations>,
) -> Result<(), Error> {
    for col in x.collections.iter().flatten() {
        let d = collection_version(c, &col.definition.pin()).await?;
        require(d == col.definition)?;
        let old_col = old
            .and_then(|o| o.collections.as_ref())
            .and_then(|v| v.iter().find(|o| o.definition.pin() == d.pin()));
        if old.is_some() && old_col.is_none() {
            selectable_collection(c, &d.pin()).await?;
            selectable_layout(c, &d.layout.pin()).await?;
        }
        if let CollectionValue::Entries(entries) = &col.value {
            for e in entries {
                for cell in &e.cells {
                    if let RecordValue::Reference(p) = &cell.value {
                        let retained = old_col.is_some_and(|o| {
                            if let CollectionValue::Entries(es) = &o.value {
                                es.iter()
                                    .any(|oe| oe.id == e.id && oe.cells.iter().any(|oc| oc == cell))
                            } else {
                                false
                            }
                        });
                        target(c, p, old.is_some() && !retained).await?;
                    }
                }
            }
        }
    }
    Ok(())
}
pub(crate) async fn validate_selection(
    c: &mut SqliteConnection,
    i: &CardIdentity,
    x: &CardRelations,
    old: &CardRelations,
) -> Result<(), Error> {
    x.validate(i).map_err(Error::Invalid)?;
    verify_dependencies(c, x, Some(old))
        .await
        .map_err(|e| author::missing_reference(e, "relations"))
}
pub(crate) async fn history(
    c: &mut SqliteConnection,
    k: Kind,
    w: Option<&str>,
    id: &str,
    before: Option<i64>,
) -> Result<Vec<CardHistoryEntry>, Error> {
    if before.is_some_and(|b| b <= 0) {
        return Err(author::invalid(
            "before",
            "Choose a positive revision cursor.",
        ));
    }
    head(c, k, w, id).await?;
    let (_, v) = k.tables();
    let out:Vec<CardHistoryEntry>=sqlx::query_as(AssertSqlSafe(format!("SELECT v.revision,coalesce(u.display_name,a.system_key) AS actor_name,a.action,a.occurred_at,a.reason FROM {v} v LEFT JOIN audit_events a ON a.id=v.audit_id LEFT JOIN users u ON u.id=a.actor_user_id WHERE v.scope=?1 AND v.id=?2 AND (?3 IS NULL OR v.revision<?3) ORDER BY v.revision DESC LIMIT 25"))).bind(scope(w)).bind(id).bind(before).fetch_all(&mut *c).await.map_err(content::storage_error)?;
    for e in &out {
        match k {
            Kind::Layout => {
                layout_version(
                    c,
                    &LayoutPin {
                        world_id: w.map(str::to_owned),
                        layout_id: id.into(),
                        version: e.revision,
                    },
                )
                .await?;
            }
            Kind::Collection => {
                collection_version(
                    c,
                    &CollectionPin {
                        world_id: w.map(str::to_owned),
                        collection_id: id.into(),
                        version: e.revision,
                    },
                )
                .await?;
            }
        }
    }
    Ok(out)
}
