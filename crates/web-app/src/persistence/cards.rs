//! Typed card components and reusable fields. Callers own transactions and policy.
use super::{authoring as content, moves, types};
use crate::application::{
    authoring::{self as author, AddressKey, Error},
    cards::CardHistoryEntry,
};
use game_data::{
    authoring::ContentFields,
    card_fields::*,
    cards::*,
    stats::{Decimal, NumericConstraints},
};
use serde::{Serialize, de::DeserializeOwned};
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
fn require(b: bool) -> Result<(), Error> {
    if b { Ok(()) } else { Err(Error::Unavailable) }
}
fn parse<T: DeserializeOwned + Serialize>(s: &str) -> Result<T, Error> {
    let v: serde_json::Value = serde_json::from_str(s).map_err(|_| Error::Unavailable)?;
    let t: T = serde_json::from_value(v.get("content").cloned().ok_or(Error::Unavailable)?)
        .map_err(|_| Error::Unavailable)?;
    require(v == serde_json::json!({"snapshot_version":1,"content":t}))?;
    Ok(t)
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
fn decimal(r: &SqliteRow, n: &str, s: &str) -> Result<Option<Decimal>, Error> {
    match (get::<Option<i64>>(r, n)?, get::<Option<i64>>(r, s)?) {
        (None, None) => Ok(None),
        (Some(n), Some(s)) => {
            let s = u8::try_from(s).map_err(|_| Error::Unavailable)?;
            let d = Decimal::new(n, s).map_err(|_| Error::Unavailable)?;
            require(d.coefficient() == n && d.scale() == s)?;
            Ok(Some(d))
        }
        _ => Err(Error::Unavailable),
    }
}
fn boolean(r: &SqliteRow, n: &str) -> Result<bool, Error> {
    match get::<i64>(r, n)? {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(Error::Unavailable),
    }
}

pub(crate) async fn field(
    c: &mut SqliteConnection,
    w: Option<&str>,
    id: &str,
) -> Result<FieldDefinition, Error> {
    let head = sqlx::query(
        "SELECT scope,id,world_id,slug,revision FROM card_field_definitions WHERE scope=? AND id=?",
    )
    .bind(scope(w))
    .bind(id)
    .fetch_optional(&mut *c)
    .await
    .map_err(content::storage_error)?
    .ok_or(Error::NotFound)?;
    require(
        get::<String>(&head, "scope")? == scope(w)
            && get::<String>(&head, "id")? == id
            && get::<Option<String>>(&head, "world_id")?.as_deref() == w,
    )?;
    let row = field_version(c, w, id, get(&head, "revision")?).await?;
    require(get::<String>(&head, "slug")? == row.fields.slug)?;
    Ok(row)
}
pub(crate) async fn field_version(
    c: &mut SqliteConnection,
    w: Option<&str>,
    id: &str,
    rev: i64,
) -> Result<FieldDefinition, Error> {
    let row = field_version_raw(c, w, id, rev).await?;
    if let Some(p) = &row.source {
        field_version_raw(c, None, &p.field_id, p.version)
            .await
            .map_err(|_| Error::Unavailable)?;
    }
    Ok(row)
}
async fn field_version_raw(
    c: &mut SqliteConnection,
    w: Option<&str>,
    id: &str,
    rev: i64,
) -> Result<FieldDefinition, Error> {
    let (row, before, _) = field_revision(c, w, id, rev).await?;
    if rev > 1 {
        // Exactly one predecessor: verify typed header/options/audit against its
        // own snapshot before trusting its bytes as this revision's `before`.
        let (_, _, previous) = field_revision(c, w, id, rev - 1)
            .await
            .map_err(|_| Error::Unavailable)?;
        require(before.as_deref() == Some(previous.as_str()))?;
    }
    Ok(row)
}
async fn field_revision(
    c: &mut SqliteConnection,
    w: Option<&str>,
    id: &str,
    rev: i64,
) -> Result<(FieldDefinition, Option<String>, String), Error> {
    let r=sqlx::query("SELECT v.*,EXISTS(SELECT 1 FROM audit_events a LEFT JOIN users u ON u.id=a.actor_user_id WHERE a.id=v.audit_id AND coalesce(u.display_name,a.system_key) IS NOT NULL AND ((v.revision=1 AND a.action IN ('create','adopt')) OR (v.revision>1 AND a.action IN ('edit','adopt')))) AS audit_present FROM card_field_versions v WHERE scope=? AND field_id=? AND revision=?").bind(scope(w)).bind(id).bind(rev).fetch_optional(&mut *c).await.map_err(content::storage_error)?.ok_or(Error::NotFound)?;
    require(
        boolean(&r, "audit_present")?
            && get::<Option<i64>>(&r, "previous_revision")? == content::previous(rev),
    )?;
    let after: String = get(&r, "after_json")?;
    let source = match (
        get::<Option<String>>(&r, "source_id")?,
        get::<Option<i64>>(&r, "source_revision")?,
    ) {
        (None, None) => None,
        (Some(field_id), Some(version)) => Some(FieldPin {
            world_id: None,
            field_id,
            version,
        }),
        _ => return Err(Error::Unavailable),
    };
    let options=sqlx::query("SELECT slot,option_id,label FROM card_field_options WHERE scope=? AND field_id=? AND revision=? ORDER BY slot").bind(scope(w)).bind(id).bind(rev).fetch_all(&mut *c).await.map_err(content::storage_error)?;
    let mut choices = vec![];
    for (i, o) in options.iter().enumerate() {
        require(get::<i64>(o, "slot")? == i as i64)?;
        choices.push(ChoiceOption {
            id: get(o, "option_id")?,
            label: get(o, "label")?,
        });
    }
    let constraints = NumericConstraints {
        whole_only: boolean(&r, "whole_only")?,
        minimum: decimal(&r, "minimum_coefficient", "minimum_scale")?,
        maximum: decimal(&r, "maximum_coefficient", "maximum_scale")?,
    };
    let k: String = get(&r, "kind")?;
    require(k == "text" || get::<Option<i64>>(&r, "max_length")?.is_none())?;
    if k != "choice" {
        require(choices.is_empty())?;
    }
    if k != "number" {
        require(constraints == NumericConstraints::default())?;
    }
    let kind = match k.as_str() {
        "number" => FieldKind::Number(constraints),
        "text" => FieldKind::Text {
            max_length: u32::try_from(get::<i64>(&r, "max_length")?)
                .map_err(|_| Error::Unavailable)?,
        },
        "boolean" => FieldKind::Boolean,
        "choice" => FieldKind::Choice { options: choices },
        "card_reference" => FieldKind::CardReference,
        _ => return Err(Error::Unavailable),
    };
    let row = FieldDefinition {
        id: id.into(),
        world_id: w.map(str::to_owned),
        source,
        fields: ContentFields {
            slug: get(&r, "slug")?,
            display_name: get(&r, "display_name")?,
            description: get(&r, "description")?,
        },
        revision: rev,
        archived_at: get(&r, "archived_at")?,
        kind,
    };
    row.validate().map_err(|_| Error::Unavailable)?;
    require(parse::<FieldDefinition>(&after)? == row)?;
    let before: Option<String> = get(&r, "before_json")?;
    if rev == 1 {
        require(before.is_none())?;
    } else {
        require(before.is_some())?;
    }
    Ok((row, before, after))
}
pub(crate) async fn field_ids(
    c: &mut SqliteConnection,
    w: Option<&str>,
    page: usize,
) -> Result<Vec<String>, Error> {
    sqlx::query_scalar(
        "SELECT id FROM card_field_definitions WHERE scope=? ORDER BY slug,id LIMIT 25 OFFSET ?",
    )
    .bind(scope(w))
    .bind(offset(page)?)
    .fetch_all(c)
    .await
    .map_err(content::storage_error)
}
fn offset(page: usize) -> Result<i64, Error> {
    i64::try_from(
        page.saturating_sub(1)
            .checked_mul(25)
            .ok_or(Error::NotFound)?,
    )
    .map_err(|_| Error::NotFound)
}
pub(crate) async fn allocate_slug(
    c: &mut SqliteConnection,
    w: Option<&str>,
    fields: &mut ContentFields,
    except: Option<&str>,
    field: bool,
) -> Result<(), Error> {
    let table = if field {
        "card_field_definitions"
    } else {
        "standalone_cards"
    };
    let base = game_data::authoring::slug_from_name(&fields.display_name);
    for n in 1..=10000 {
        let slug = game_data::authoring::slug_candidate(&base, n);
        let exists: bool = sqlx::query_scalar(AssertSqlSafe(format!(
            "SELECT EXISTS(SELECT 1 FROM {table} WHERE scope=? AND slug=? AND (? IS NULL OR id<>?))"
        )))
        .bind(scope(w))
        .bind(&slug)
        .bind(except)
        .bind(except)
        .fetch_one(&mut *c)
        .await
        .map_err(content::storage_error)?;
        if !exists {
            fields.slug = slug;
            return Ok(());
        }
    }
    Err(author::invalid(
        "display_name",
        "Cannot allocate an address for this name.",
    ))
}
pub(crate) async fn put_field(
    c: &mut SqliteConnection,
    row: &FieldDefinition,
    actor: &str,
    action: &str,
    reason: Option<&str>,
) -> Result<(), Error> {
    row.validate().map_err(Error::Invalid)?;
    let w = row.world_id.as_deref();
    let before = if row.revision > 1 {
        Some(author::snapshot(
            &field_version(c, w, &row.id, row.revision - 1).await?,
        )?)
    } else {
        None
    };
    if let Some(p) = &row.source {
        field_version(c, None, &p.field_id, p.version).await?;
    }
    let after = author::snapshot(row)?;
    let audit = content::audit(
        c,
        &content::Revision::new(Some(actor), action, reason, before.clone(), after.clone()),
    )
    .await?;
    sqlx::query("INSERT INTO card_field_definitions(scope,id,world_id,slug,revision) VALUES (?,?,?,?,?) ON CONFLICT(scope,id) DO UPDATE SET slug=excluded.slug,revision=excluded.revision").bind(scope(w)).bind(&row.id).bind(w).bind(&row.fields.slug).bind(row.revision).execute(&mut *c).await.map_err(content::content_write_error)?;
    let (kind, constraints, max) = match &row.kind {
        FieldKind::Number(n) => ("number", n.clone(), None),
        FieldKind::Text { max_length } => (
            "text",
            NumericConstraints::default(),
            Some(i64::from(*max_length)),
        ),
        FieldKind::Boolean => ("boolean", NumericConstraints::default(), None),
        FieldKind::Choice { .. } => ("choice", NumericConstraints::default(), None),
        FieldKind::CardReference => ("card_reference", NumericConstraints::default(), None),
    };
    if let FieldKind::Choice { options } = &row.kind {
        for (i, o) in options.iter().enumerate() {
            sqlx::query("INSERT INTO card_field_options(scope,field_id,revision,slot,option_id,label) VALUES (?,?,?,?,?,?)").bind(scope(w)).bind(&row.id).bind(row.revision).bind(i as i64).bind(&o.id).bind(&o.label).execute(&mut *c).await.map_err(content::storage_error)?;
        }
    }
    sqlx::query("INSERT INTO card_field_versions(scope,field_id,revision,source_id,source_revision,slug,display_name,description,archived_at,kind,whole_only,minimum_coefficient,minimum_scale,maximum_coefficient,maximum_scale,max_length,audit_id,previous_revision,before_json,after_json) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)")
        .bind(scope(w)).bind(&row.id).bind(row.revision).bind(row.source.as_ref().map(|p|&p.field_id)).bind(row.source.as_ref().map(|p|p.version)).bind(&row.fields.slug).bind(&row.fields.display_name).bind(&row.fields.description).bind(row.archived_at).bind(kind).bind(i64::from(constraints.whole_only)).bind(constraints.minimum.map(Decimal::coefficient)).bind(constraints.minimum.map(|n|i64::from(n.scale()))).bind(constraints.maximum.map(Decimal::coefficient)).bind(constraints.maximum.map(|n|i64::from(n.scale()))).bind(max).bind(audit).bind(content::previous(row.revision)).bind(before).bind(after).execute(&mut *c).await.map_err(content::storage_error)?;
    Ok(())
}

// Native metadata remains authoritative in the original typed tables/readers.
async fn native_version(c: &mut SqliteConnection, p: &CardPin) -> Result<CardRecord, Error> {
    p.validate().map_err(|_| Error::NotFound)?;
    let i = &p.identity;
    let w = i.world_id.as_deref();
    let rev = p.version;
    let (fields, archived_at, source) = match i.namespace {
        CardNamespace::Form => {
            let r = content::form_version(c, &i.id, rev).await?;
            (r.fields, r.archived_at, None)
        }
        CardNamespace::Variant => {
            let full = types::full_content(
                c,
                crate::application::authoring::HistoryKind::Variant,
                &i.id,
                w,
                rev,
            )
            .await?;
            let r: crate::application::authoring::VariantRecord = parse(&full.projection_after)?;
            let s = r.source.map(|s| CardPin {
                identity: CardIdentity {
                    namespace: CardNamespace::Form,
                    world_id: None,
                    id: s.form_id,
                },
                version: s.version,
            });
            (r.fields, r.archived_at, s)
        }
        CardNamespace::Type => {
            if let Some(w) = w {
                let r = types::world_type_version(c, w, &i.id, rev).await?;
                let s = r.source.map(|s| CardPin {
                    identity: CardIdentity {
                        namespace: CardNamespace::Type,
                        world_id: None,
                        id: s.type_id,
                    },
                    version: s.version,
                });
                (r.fields, r.archived_at, s)
            } else {
                let r = types::type_version(c, &i.id, rev).await?;
                (r.fields, r.archived_at, None)
            }
        }
        CardNamespace::Move => {
            let r = moves::move_version(c, w, &i.id, rev).await?;
            let s = r.source.map(|s| CardPin {
                identity: CardIdentity {
                    namespace: CardNamespace::Move,
                    world_id: None,
                    id: s.move_id,
                },
                version: s.version,
            });
            (r.fields, r.archived_at, s)
        }
        CardNamespace::Standalone => {
            let r = sqlx::query(
                "SELECT * FROM standalone_card_versions WHERE scope=? AND card_id=? AND revision=?",
            )
            .bind(scope(w))
            .bind(&i.id)
            .bind(rev)
            .fetch_optional(&mut *c)
            .await
            .map_err(content::storage_error)?
            .ok_or(Error::NotFound)?;
            require(get::<Option<i64>>(&r, "previous_revision")? == content::previous(rev))?;
            let s = match (
                get::<Option<String>>(&r, "source_id")?,
                get::<Option<i64>>(&r, "source_revision")?,
            ) {
                (None, None) => None,
                (Some(id), Some(version)) => Some(CardPin {
                    identity: CardIdentity {
                        namespace: CardNamespace::Standalone,
                        world_id: None,
                        id,
                    },
                    version,
                }),
                _ => return Err(Error::Unavailable),
            };
            (
                ContentFields {
                    slug: get(&r, "slug")?,
                    display_name: get(&r, "display_name")?,
                    description: get(&r, "description")?,
                },
                get(&r, "archived_at")?,
                s,
            )
        }
    };
    Ok(CardRecord {
        identity: i.clone(),
        revision: rev,
        fields,
        archived_at,
        source,
        component: CardComponent::default(),
        legacy_unmodeled: false,
    })
}
fn native_tables(i: &CardIdentity) -> (&'static str, &'static str, &'static str, &'static str) {
    match i.namespace {
        CardNamespace::Form => ("forms", "form_versions", "form_id", "''"),
        CardNamespace::Variant => ("variants", "variant_revisions", "variant_id", "world_id"),
        CardNamespace::Type if i.world_id.is_none() => ("types", "type_versions", "type_id", "''"),
        CardNamespace::Type => ("world_types", "world_type_versions", "type_id", "world_id"),
        CardNamespace::Move => ("moves", "move_versions", "move_id", "scope"),
        CardNamespace::Standalone => (
            "standalone_cards",
            "standalone_card_versions",
            "card_id",
            "scope",
        ),
    }
}
async fn native_audit(c: &mut SqliteConnection, p: &CardPin) -> Result<String, Error> {
    let (_, v, k, s) = native_tables(&p.identity);
    sqlx::query_scalar(AssertSqlSafe(format!("SELECT v.audit_id FROM {v} v JOIN audit_events a ON a.id=v.audit_id LEFT JOIN users u ON u.id=a.actor_user_id WHERE v.{k}=? AND {s}=? AND v.revision=? AND coalesce(u.display_name,a.system_key) IS NOT NULL AND ((v.revision=1 AND a.action IN ('create','adopt') AND v.previous_revision IS NULL AND v.before_json IS NULL) OR (v.revision>1 AND a.action IN ('edit','adopt') AND v.previous_revision=v.revision-1 AND v.before_json IS NOT NULL))"))).bind(&p.identity.id).bind(scope(p.identity.world_id.as_deref())).bind(p.version).fetch_optional(c).await.map_err(content::storage_error)?.ok_or(Error::Unavailable)
}
async fn legacy(c: &mut SqliteConnection, p: &CardPin) -> Result<bool, Error> {
    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM card_legacy_boundaries WHERE namespace=? AND scope=? AND id=? AND maximum_revision>=?)").bind(p.identity.namespace.as_str()).bind(scope(p.identity.world_id.as_deref())).bind(&p.identity.id).bind(p.version).fetch_one(c).await.map_err(content::storage_error)
}
pub(crate) async fn card(c: &mut SqliteConnection, i: &CardIdentity) -> Result<CardRecord, Error> {
    i.validate().map_err(|_| Error::NotFound)?;
    let (h, _, _, s) = native_tables(i);
    let r = sqlx::query(AssertSqlSafe(format!(
        "SELECT revision,slug FROM {h} WHERE id=? AND {s}=?"
    )))
    .bind(&i.id)
    .bind(scope(i.world_id.as_deref()))
    .fetch_optional(&mut *c)
    .await
    .map_err(content::storage_error)?
    .ok_or(Error::NotFound)?;
    let out = card_version(
        c,
        &CardPin {
            identity: i.clone(),
            version: get(&r, "revision")?,
        },
    )
    .await?;
    require(out.fields.slug == get::<String>(&r, "slug")?)?;
    let current = match i.namespace {
        CardNamespace::Form => {
            let n = content::form(c, &i.id).await?;
            Some((n.fields, n.archived_at))
        }
        CardNamespace::Variant => {
            let n =
                content::variant(c, i.world_id.as_deref().ok_or(Error::NotFound)?, &i.id).await?;
            Some((n.fields, n.archived_at))
        }
        CardNamespace::Type if i.world_id.is_none() => {
            let n = types::type_detail(c, &i.id).await?;
            Some((n.fields, n.archived_at))
        }
        CardNamespace::Type => {
            let n =
                types::world_type_detail(c, i.world_id.as_deref().ok_or(Error::NotFound)?, &i.id)
                    .await?;
            Some((n.fields, n.archived_at))
        }
        _ => None,
    };
    if let Some((fields, archived)) = current {
        require(out.fields == fields && out.archived_at == archived)?;
    }
    Ok(out)
}
pub(crate) async fn resolve_card(
    c: &mut SqliteConnection,
    w: Option<&str>,
    namespace: CardNamespace,
    key: AddressKey,
) -> Result<CardRecord, Error> {
    let mut identity = CardIdentity {
        namespace,
        world_id: w.map(str::to_owned),
        id: "0".repeat(32),
    };
    identity.validate().map_err(|_| Error::NotFound)?;
    identity.id = match key {
        AddressKey::Id(id) => id,
        AddressKey::Slug(_) if namespace == CardNamespace::Form => {
            return Err(author::invalid(
                "card",
                "Form slugs require a species-qualified address; use the canonical Form ID here.",
            ));
        }
        AddressKey::Slug(slug) => {
            let (h, _, _, s) = native_tables(&identity);
            sqlx::query_scalar(AssertSqlSafe(format!(
                "SELECT id FROM {h} WHERE {s}=? AND slug=? LIMIT 1"
            )))
            .bind(scope(w))
            .bind(slug)
            .fetch_optional(&mut *c)
            .await
            .map_err(content::storage_error)?
            .ok_or(Error::NotFound)?
        }
    };
    card(c, &identity).await
}
pub(crate) async fn resolve_field(
    c: &mut SqliteConnection,
    w: Option<&str>,
    key: AddressKey,
) -> Result<FieldDefinition, Error> {
    let id = match key {
        AddressKey::Id(id) => id,
        AddressKey::Slug(slug) => sqlx::query_scalar(
            "SELECT id FROM card_field_definitions WHERE scope=? AND slug=? LIMIT 1",
        )
        .bind(scope(w))
        .bind(slug)
        .fetch_optional(&mut *c)
        .await
        .map_err(content::storage_error)?
        .ok_or(Error::NotFound)?,
    };
    field(c, w, &id).await
}
fn history_cursor(before: Option<i64>) -> Result<(), Error> {
    if before.is_some_and(|n| n <= 0) {
        return Err(author::invalid(
            "before",
            "Choose a positive revision cursor.",
        ));
    }
    Ok(())
}
pub(crate) async fn history(
    c: &mut SqliteConnection,
    i: &CardIdentity,
    before: Option<i64>,
) -> Result<Vec<CardHistoryEntry>, Error> {
    history_cursor(before)?;
    card(c, i).await?;
    let (_, v, k, s) = native_tables(i);
    // LEFT joins ensure corrupt/missing audit attribution cannot silently remove
    // a revision from the bounded page. NULL required DTO fields fail closed.
    let entries:Vec<CardHistoryEntry>=sqlx::query_as(AssertSqlSafe(format!("SELECT v.revision,coalesce(u.display_name,a.system_key) AS actor_name,a.action,a.occurred_at,a.reason FROM {v} v LEFT JOIN audit_events a ON a.id=v.audit_id LEFT JOIN users u ON u.id=a.actor_user_id WHERE v.{k}=?1 AND {s}=?2 AND (?3 IS NULL OR v.revision<?3) ORDER BY v.revision DESC LIMIT 25"))).bind(&i.id).bind(scope(i.world_id.as_deref())).bind(before).fetch_all(&mut *c).await.map_err(content::storage_error)?;
    for entry in &entries {
        card_version(
            c,
            &CardPin {
                identity: i.clone(),
                version: entry.revision,
            },
        )
        .await
        .map_err(|_| Error::Unavailable)?;
    }
    Ok(entries)
}
pub(crate) async fn field_history(
    c: &mut SqliteConnection,
    w: Option<&str>,
    id: &str,
    before: Option<i64>,
) -> Result<Vec<CardHistoryEntry>, Error> {
    history_cursor(before)?;
    field(c, w, id).await?;
    let entries:Vec<CardHistoryEntry>=sqlx::query_as("SELECT v.revision,coalesce(u.display_name,a.system_key) AS actor_name,a.action,a.occurred_at,a.reason FROM card_field_versions v LEFT JOIN audit_events a ON a.id=v.audit_id LEFT JOIN users u ON u.id=a.actor_user_id WHERE v.scope=?1 AND v.field_id=?2 AND (?3 IS NULL OR v.revision<?3) ORDER BY v.revision DESC LIMIT 25").bind(scope(w)).bind(id).bind(before).fetch_all(&mut *c).await.map_err(content::storage_error)?;
    for entry in &entries {
        field_version(c, w, id, entry.revision)
            .await
            .map_err(|_| Error::Unavailable)?;
    }
    Ok(entries)
}
pub(crate) async fn card_version(
    c: &mut SqliteConnection,
    p: &CardPin,
) -> Result<CardRecord, Error> {
    let mut row = native_version(c, p).await?;
    let audit = native_audit(c, p).await?;
    let r = sqlx::query(
        "SELECT * FROM card_components WHERE namespace=? AND scope=? AND card_id=? AND revision=?",
    )
    .bind(p.identity.namespace.as_str())
    .bind(scope(p.identity.world_id.as_deref()))
    .bind(&p.identity.id)
    .bind(p.version)
    .fetch_optional(&mut *c)
    .await
    .map_err(content::storage_error)?;
    if let Some(r) = r {
        require(!legacy(c, p).await?)?;
        require(get::<Option<String>>(&r, "audit_id")?.as_deref() == Some(&audit))?;
        row.component = read_component(c, p, &r).await?;
        let snapshot: String = get(&r, "snapshot_json")?;
        require(parse::<CardRecord>(&snapshot)? == row)?;
        verify_dependencies(c, &row.component, None).await?;
    } else {
        require(legacy(c, p).await?)?;
        row.legacy_unmodeled = true;
    }
    row.validate().map_err(|_| Error::Unavailable)?;
    if let Some(source) = &row.source {
        verify_direct_pin(c, source).await?;
    }
    verify_standalone_snapshot(c, &row).await?;
    Ok(row)
}
async fn verify_standalone_snapshot(
    c: &mut SqliteConnection,
    row: &CardRecord,
) -> Result<(), Error> {
    let p = row.pin();
    if p.identity.namespace == CardNamespace::Standalone {
        let after:String=sqlx::query_scalar("SELECT after_json FROM standalone_card_versions WHERE scope=? AND card_id=? AND revision=?").bind(scope(p.identity.world_id.as_deref())).bind(&p.identity.id).bind(p.version).fetch_one(&mut *c).await.map_err(content::storage_error)?;
        require(parse::<CardRecord>(&after)? == *row)?;
        let before:Option<String>=sqlx::query_scalar("SELECT before_json FROM standalone_card_versions WHERE scope=? AND card_id=? AND revision=?").bind(scope(p.identity.world_id.as_deref())).bind(&p.identity.id).bind(p.version).fetch_one(&mut *c).await.map_err(content::storage_error)?;
        if p.version == 1 {
            require(before.is_none())?;
        } else {
            let previous:String=sqlx::query_scalar("SELECT after_json FROM standalone_card_versions WHERE scope=? AND card_id=? AND revision=?").bind(scope(p.identity.world_id.as_deref())).bind(&p.identity.id).bind(p.version-1).fetch_one(c).await.map_err(content::storage_error)?;
            require(before.as_deref() == Some(&previous))?;
        }
    }
    Ok(())
}
async fn read_component(
    c: &mut SqliteConnection,
    p: &CardPin,
    r: &SqliteRow,
) -> Result<CardComponent, Error> {
    let i = &p.identity;
    let s = scope(i.world_id.as_deref());
    let labels=sqlx::query("SELECT slot,label FROM card_component_labels WHERE namespace=? AND scope=? AND card_id=? AND revision=? ORDER BY slot").bind(i.namespace.as_str()).bind(s).bind(&i.id).bind(p.version).fetch_all(&mut *c).await.map_err(content::storage_error)?;
    let mut ls = vec![];
    for (n, l) in labels.iter().enumerate() {
        require(get::<i64>(l, "slot")? == n as i64)?;
        ls.push(get(l, "label")?);
    }
    let rs=sqlx::query("SELECT * FROM card_component_fields WHERE namespace=? AND scope=? AND card_id=? AND revision=? ORDER BY slot").bind(i.namespace.as_str()).bind(s).bind(&i.id).bind(p.version).fetch_all(&mut *c).await.map_err(content::storage_error)?;
    let mut fs = vec![];
    for (n, f) in rs.iter().enumerate() {
        require(get::<i64>(f, "slot")? == n as i64)?;
        let definition = field_version(
            c,
            i.world_id.as_deref(),
            &get::<String>(f, "field_id")?,
            get(f, "field_revision")?,
        )
        .await?;
        let state: String = get(f, "state")?;
        let value = match state.as_str() {
            "missing" => FieldValue::Missing,
            "unknown" => FieldValue::Unknown,
            "not_applicable" => FieldValue::NotApplicable,
            "number" => {
                FieldValue::Number(decimal(f, "coefficient", "scale")?.ok_or(Error::Unavailable)?)
            }
            "text" => FieldValue::Text(get(f, "text_value")?),
            "boolean" => FieldValue::Boolean(boolean(f, "boolean_value")?),
            "choice" => FieldValue::Choice(get(f, "option_id")?),
            "card_reference" => FieldValue::CardReference(CardPin {
                identity: CardIdentity {
                    namespace: namespace(&get::<String>(f, "ref_namespace")?)?,
                    world_id: world(get(f, "ref_scope")?),
                    id: get(f, "ref_id")?,
                },
                version: get(f, "ref_revision")?,
            }),
            _ => return Err(Error::Unavailable),
        };
        fs.push(CardField { definition, value });
    }
    let fd = boolean(r, "fields_defined")?;
    let ld = boolean(r, "labels_defined")?;
    require((fd || fs.is_empty()) && (ld || ls.is_empty()))?;
    let out = CardComponent {
        fields: fd.then_some(fs),
        labels: ld.then_some(ls),
        rules_text: get(r, "rules_text")?,
    };
    out.validate(i).map_err(|_| Error::Unavailable)?;
    Ok(out)
}
/// Check direct pins only. In particular, do not recursively walk reference cycles.
async fn verify_dependencies(
    c: &mut SqliteConnection,
    component: &CardComponent,
    old: Option<&CardComponent>,
) -> Result<(), Error> {
    for f in component.fields.iter().flatten() {
        let d = field_version(
            c,
            f.definition.world_id.as_deref(),
            &f.definition.id,
            f.definition.revision,
        )
        .await?;
        require(d == f.definition)?;
        if let Some(old) = old {
            let retained = old
                .fields
                .iter()
                .flatten()
                .any(|x| x.definition.pin() == d.pin());
            if !retained
                && (d.archived_at.is_some()
                    || field(c, d.world_id.as_deref(), &d.id)
                        .await?
                        .archived_at
                        .is_some())
            {
                return Err(author::invalid(
                    "fields",
                    "Choose active field definitions for new selections.",
                ));
            }
        }
        if let FieldValue::CardReference(p) = &f.value {
            let target = verify_direct_pin(c, p).await?;
            if let Some(old) = old {
                let retained = old
                    .fields
                    .iter()
                    .flatten()
                    .any(|x| x.definition.pin() == f.definition.pin() && x.value == f.value);
                if !retained {
                    let (h, _, _, s) = native_tables(&p.identity);
                    let head: i64 = sqlx::query_scalar(AssertSqlSafe(format!(
                        "SELECT revision FROM {h} WHERE id=? AND {s}=?"
                    )))
                    .bind(&p.identity.id)
                    .bind(scope(p.identity.world_id.as_deref()))
                    .fetch_one(&mut *c)
                    .await
                    .map_err(content::storage_error)?;
                    if target.archived_at.is_some()
                        || native_version(
                            c,
                            &CardPin {
                                identity: p.identity.clone(),
                                version: head,
                            },
                        )
                        .await?
                        .archived_at
                        .is_some()
                    {
                        return Err(author::invalid(
                            "fields",
                            "Choose active cards for new reference selections.",
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}
async fn verify_direct_pin(c: &mut SqliteConnection, p: &CardPin) -> Result<CardRecord, Error> {
    let mut target = native_version(c, p).await?;
    let audit = native_audit(c, p).await?;
    let seal = sqlx::query(
        "SELECT * FROM card_components WHERE namespace=? AND scope=? AND card_id=? AND revision=?",
    )
    .bind(p.identity.namespace.as_str())
    .bind(scope(p.identity.world_id.as_deref()))
    .bind(&p.identity.id)
    .bind(p.version)
    .fetch_optional(&mut *c)
    .await
    .map_err(content::storage_error)?;
    if let Some(seal) = seal {
        require(!legacy(c, p).await?)?;
        require(get::<Option<String>>(&seal, "audit_id")?.as_deref() == Some(&audit))?;
        // Reconstruct typed labels/values and exact field definitions, but do not
        // follow the target's outgoing CardReference/source edges. Capture owns
        // that visited/budgeted traversal, including exact cycles.
        target.component = read_component(c, p, &seal).await?;
        require(parse::<CardRecord>(&get::<String>(&seal, "snapshot_json")?)? == target)?;
    } else {
        require(legacy(c, p).await?)?;
        target.legacy_unmodeled = true;
    }
    target.validate().map_err(|_| Error::Unavailable)?;
    verify_standalone_snapshot(c, &target).await?;
    Ok(target)
}
pub(crate) async fn validate_selection(
    c: &mut SqliteConnection,
    i: &CardIdentity,
    component: &CardComponent,
    old: &CardComponent,
) -> Result<(), Error> {
    component.validate(i).map_err(Error::Invalid)?;
    verify_dependencies(c, component, Some(old))
        .await
        .map_err(|e| author::missing_reference(e, "fields"))
}

/// Stage a complete typed proposal inside the caller's writer transaction.
pub(crate) async fn stage(
    c: &mut SqliteConnection,
    p: &CardPin,
    x: &CardComponent,
) -> Result<(), Error> {
    x.validate(&p.identity).map_err(Error::Invalid)?;
    let i = &p.identity;
    let s = scope(i.world_id.as_deref());
    sqlx::query("INSERT INTO card_components(namespace,scope,card_id,revision,fields_defined,rules_text,labels_defined) VALUES (?,?,?,?,?,?,?)").bind(i.namespace.as_str()).bind(s).bind(&i.id).bind(p.version).bind(i64::from(x.fields.is_some())).bind(&x.rules_text).bind(i64::from(x.labels.is_some())).execute(&mut *c).await.map_err(content::storage_error)?;
    for (n, l) in x.labels.iter().flatten().enumerate() {
        sqlx::query("INSERT INTO card_component_labels(namespace,scope,card_id,revision,slot,label) VALUES (?,?,?,?,?,?)").bind(i.namespace.as_str()).bind(s).bind(&i.id).bind(p.version).bind(n as i64).bind(l).execute(&mut *c).await.map_err(content::storage_error)?;
    }
    for (n, f) in x.fields.iter().flatten().enumerate() {
        let mut number = None;
        let mut text = None;
        let mut bool_value = None;
        let mut choice = None;
        let mut reference = None;
        let state = match &f.value {
            FieldValue::Missing => "missing",
            FieldValue::Unknown => "unknown",
            FieldValue::NotApplicable => "not_applicable",
            FieldValue::Number(d) => {
                number = Some(*d);
                "number"
            }
            FieldValue::Text(t) => {
                text = Some(t);
                "text"
            }
            FieldValue::Boolean(b) => {
                bool_value = Some(i64::from(*b));
                "boolean"
            }
            FieldValue::Choice(id) => {
                choice = Some(id);
                "choice"
            }
            FieldValue::CardReference(p) => {
                reference = Some(p);
                "card_reference"
            }
        };
        sqlx::query("INSERT INTO card_component_fields(namespace,scope,card_id,revision,slot,field_id,field_revision,state,coefficient,scale,text_value,boolean_value,option_id,ref_namespace,ref_scope,ref_id,ref_revision) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)").bind(i.namespace.as_str()).bind(s).bind(&i.id).bind(p.version).bind(n as i64).bind(&f.definition.id).bind(f.definition.revision).bind(state).bind(number.map(Decimal::coefficient)).bind(number.map(|d|i64::from(d.scale()))).bind(text).bind(bool_value).bind(choice).bind(reference.map(|p|p.identity.namespace.as_str())).bind(reference.map(|p|scope(p.identity.world_id.as_deref()))).bind(reference.map(|p|&p.identity.id)).bind(reference.map(|p|p.version)).execute(&mut *c).await.map_err(content::storage_error)?;
    }
    Ok(())
}
/// Called by every owning native writer after its version is inserted. Reuses its audit.
pub(crate) async fn seal_native(
    c: &mut SqliteConnection,
    i: CardIdentity,
    revision: i64,
) -> Result<(), Error> {
    let p = CardPin {
        identity: i,
        version: revision,
    };
    let r = sqlx::query(
        "SELECT * FROM card_components WHERE namespace=? AND scope=? AND card_id=? AND revision=?",
    )
    .bind(p.identity.namespace.as_str())
    .bind(scope(p.identity.world_id.as_deref()))
    .bind(&p.identity.id)
    .bind(revision)
    .fetch_optional(&mut *c)
    .await
    .map_err(content::storage_error)?;
    let component = if let Some(r) = r {
        require(get::<Option<String>>(&r, "audit_id")?.is_none())?;
        read_component(c, &p, &r).await?
    } else {
        let x = if revision == 1 {
            CardComponent::default()
        } else {
            card_version(
                c,
                &CardPin {
                    identity: p.identity.clone(),
                    version: revision - 1,
                },
            )
            .await?
            .component
        };
        stage(c, &p, &x).await?;
        x
    };
    verify_dependencies(c, &component, None).await?;
    let mut record = native_version(c, &p).await?;
    record.component = component;
    record.validate().map_err(|_| Error::Unavailable)?;
    if let Some(source) = &record.source {
        verify_direct_pin(c, source).await?;
    }
    let audit = native_audit(c, &p).await?;
    sqlx::query("UPDATE card_components SET audit_id=?,snapshot_json=? WHERE namespace=? AND scope=? AND card_id=? AND revision=? AND audit_id IS NULL").bind(audit).bind(author::snapshot(&record)?).bind(p.identity.namespace.as_str()).bind(scope(p.identity.world_id.as_deref())).bind(&p.identity.id).bind(revision).execute(c).await.map_err(content::storage_error)?;
    Ok(())
}
pub(crate) async fn identities(
    c: &mut SqliteConnection,
    w: Option<&str>,
    page: usize,
) -> Result<Vec<CardIdentity>, Error> {
    let rows=sqlx::query("SELECT namespace,id FROM (SELECT 'form' namespace,id,slug,'' scope FROM forms UNION ALL SELECT 'variant',id,slug,world_id FROM variants UNION ALL SELECT 'type',id,slug,'' FROM types UNION ALL SELECT 'type',id,slug,world_id FROM world_types UNION ALL SELECT 'move',id,slug,scope FROM moves UNION ALL SELECT 'standalone',id,slug,scope FROM standalone_cards) WHERE scope=? ORDER BY slug,namespace,id LIMIT 25 OFFSET ?").bind(scope(w)).bind(offset(page)?).fetch_all(c).await.map_err(content::storage_error)?;
    rows.iter()
        .map(|r| {
            Ok(CardIdentity {
                namespace: namespace(&get::<String>(r, "namespace")?)?,
                world_id: w.map(str::to_owned),
                id: get(r, "id")?,
            })
        })
        .collect()
}
pub(crate) async fn standalone_ids(
    c: &mut SqliteConnection,
    w: Option<&str>,
    page: usize,
) -> Result<Vec<String>, Error> {
    sqlx::query_scalar(
        "SELECT id FROM standalone_cards WHERE scope=? ORDER BY id LIMIT 25 OFFSET ?",
    )
    .bind(scope(w))
    .bind(offset(page)?)
    .fetch_all(c)
    .await
    .map_err(content::storage_error)
}

/// Custom-only native edits still go through the original aggregate writer.
pub(crate) async fn put_card(
    c: &mut SqliteConnection,
    row: &CardRecord,
    actor: &str,
    action: &str,
    reason: Option<&str>,
) -> Result<(), Error> {
    row.validate().map_err(Error::Invalid)?;
    stage(c, &row.pin(), &row.component).await?;
    let i = &row.identity;
    let w = i.world_id.as_deref();
    let before = if row.revision > 1 {
        Some(author::snapshot(
            &card_version(
                c,
                &CardPin {
                    identity: i.clone(),
                    version: row.revision - 1,
                },
            )
            .await?,
        )?)
    } else {
        None
    };
    match i.namespace {
        CardNamespace::Standalone => {
            let after = author::snapshot(row)?;
            let audit = content::audit(
                c,
                &content::Revision::new(Some(actor), action, reason, before.clone(), after.clone()),
            )
            .await?;
            sqlx::query("INSERT INTO standalone_cards(scope,id,world_id,slug,revision) VALUES (?,?,?,?,?) ON CONFLICT(scope,id) DO UPDATE SET slug=excluded.slug,revision=excluded.revision").bind(scope(w)).bind(&i.id).bind(w).bind(&row.fields.slug).bind(row.revision).execute(&mut *c).await.map_err(content::content_write_error)?;
            sqlx::query("INSERT INTO standalone_card_versions(scope,card_id,revision,slug,display_name,description,archived_at,source_id,source_revision,audit_id,previous_revision,before_json,after_json) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?)").bind(scope(w)).bind(&i.id).bind(row.revision).bind(&row.fields.slug).bind(&row.fields.display_name).bind(&row.fields.description).bind(row.archived_at).bind(row.source.as_ref().map(|p|&p.identity.id)).bind(row.source.as_ref().map(|p|p.version)).bind(audit).bind(content::previous(row.revision)).bind(before).bind(after).execute(&mut *c).await.map_err(content::storage_error)?;
            seal_native(c, i.clone(), row.revision).await?;
        }
        CardNamespace::Form => {
            let mut n = content::form(c, &i.id).await?;
            n.revision = row.revision;
            n.fields = row.fields.clone();
            n.archived_at = row.archived_at;
            let after = author::snapshot(&n)?;
            content::put_form(
                c,
                &n,
                content::Revision::new(Some(actor), action, reason, None, after),
            )
            .await?;
        }
        CardNamespace::Variant => {
            let mut n = content::variant(c, w.ok_or(Error::NotFound)?, &i.id).await?;
            n.revision = row.revision;
            n.fields = row.fields.clone();
            n.archived_at = row.archived_at;
            let after = author::snapshot(&n)?;
            content::put_variant(
                c,
                &n,
                content::Revision::new(Some(actor), action, reason, None, after),
            )
            .await?;
        }
        CardNamespace::Type if w.is_none() => {
            let mut n = types::type_detail(c, &i.id).await?;
            let old = types::type_after(c, &i.id, n.revision).await?;
            n.revision = row.revision;
            n.fields = row.fields.clone();
            n.archived_at = row.archived_at;
            let after = author::snapshot(&n)?;
            types::put_type(
                c,
                &n,
                content::Revision::new(Some(actor), action, reason, Some(old), after),
            )
            .await?;
        }
        CardNamespace::Type => {
            let w = w.ok_or(Error::NotFound)?;
            let mut n = types::world_type_detail(c, w, &i.id).await?;
            let old = types::world_type_after(c, w, &i.id, n.revision).await?;
            n.revision = row.revision;
            n.fields = row.fields.clone();
            n.archived_at = row.archived_at;
            let after = author::snapshot(&n)?;
            types::put_world_type(
                c,
                &n,
                content::Revision::new(Some(actor), action, reason, Some(old), after),
            )
            .await?;
        }
        CardNamespace::Move => {
            let mut n = moves::move_detail(c, w, &i.id).await?;
            n.revision = row.revision;
            n.fields = row.fields.clone();
            n.archived_at = row.archived_at;
            moves::put_move(c, &n, Some(actor), action, reason).await?;
        }
    }
    Ok(())
}
