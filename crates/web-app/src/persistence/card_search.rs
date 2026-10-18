//! Typed metadata predicates run before LIMIT; selected rows use checked readers.
use super::{authoring as content, cards as store};
use crate::application::{authoring::Error, cards::*};
use sqlx::{Row, SqliteConnection};

fn page<T>(mut items: Vec<T>, page: usize) -> SearchPage<T> {
    let has_more = items.len() > 25;
    items.truncate(25);
    SearchPage {
        items,
        has_more,
        page,
    }
}

pub(crate) async fn cards(
    c: &mut SqliteConnection,
    world: Option<&str>,
    q: &CardSearch,
) -> Result<SearchPage<CardSummary>, Error> {
    // LEFT joins deliberately retain broken version heads. NULL metadata bypasses
    // predicates so the checked reader fails closed instead of hiding corruption.
    let rows = sqlx::query(
        "WITH catalog AS (
          SELECT 'form' namespace,'' scope,id,revision,display_name,archived_at FROM forms
          UNION ALL SELECT 'variant',world_id,id,revision,display_name,archived_at FROM variants
          UNION ALL SELECT 'type','',id,revision,display_name,archived_at FROM types
          UNION ALL SELECT 'type',world_id,id,revision,display_name,archived_at FROM world_types
          UNION ALL SELECT 'move',h.scope,h.id,h.revision,v.display_name,v.archived_at
            FROM moves h LEFT JOIN move_versions v ON v.scope=h.scope AND v.move_id=h.id AND v.revision=h.revision
          UNION ALL SELECT 'standalone',h.scope,h.id,h.revision,v.display_name,v.archived_at
            FROM standalone_cards h LEFT JOIN standalone_card_versions v ON v.scope=h.scope AND v.card_id=h.id AND v.revision=h.revision
        ) SELECT namespace,id FROM catalog h
        WHERE scope=?1 AND (?2 IS NULL OR namespace=?2)
          AND (display_name IS NULL OR (
            instr(lower(display_name),lower(?3))>0
            AND (?4='all' OR (?4='active' AND archived_at IS NULL) OR (?4='archived' AND archived_at IS NOT NULL))
            AND (?5='' OR EXISTS(SELECT 1 FROM card_component_labels l
              WHERE l.namespace=h.namespace AND l.scope=h.scope AND l.card_id=h.id AND l.revision=h.revision AND l.label=?5))))
        ORDER BY scope,namespace,display_name,id LIMIT 26 OFFSET ?6"
    ).bind(world.unwrap_or("")).bind(q.namespace.map(CardNamespace::as_str)).bind(&q.query)
        .bind(q.archive.as_str()).bind(&q.label).bind(((q.page-1)*25) as i64)
        .fetch_all(&mut *c).await.map_err(content::storage_error)?;
    let mut items = Vec::with_capacity(rows.len());
    for r in rows {
        let namespace: String = r.try_get("namespace").map_err(content::storage_error)?;
        let namespace = match namespace.as_str() {
            "form" => CardNamespace::Form,
            "variant" => CardNamespace::Variant,
            "type" => CardNamespace::Type,
            "move" => CardNamespace::Move,
            "standalone" => CardNamespace::Standalone,
            _ => return Err(Error::Unavailable),
        };
        let row = store::card(
            c,
            &CardIdentity {
                namespace,
                world_id: world.map(str::to_owned),
                id: r.try_get("id").map_err(content::storage_error)?,
            },
        )
        .await
        .map_err(|_| Error::Unavailable)?;
        let archived = row.archived_at;
        items.push(card_summary(row, archived));
    }
    Ok(page(items, q.page))
}

pub(crate) async fn fields(
    c: &mut SqliteConnection,
    world: Option<&str>,
    source: Option<&str>,
    q: &FieldSearch,
) -> Result<SearchPage<FieldSummary>, Error> {
    let ids: Vec<String> = sqlx::query_scalar(
        "SELECT h.id FROM card_field_definitions h LEFT JOIN card_field_versions v
          ON v.scope=h.scope AND v.field_id=h.id AND v.revision=h.revision
         WHERE h.scope=?1 AND (v.field_id IS NULL OR (
           (?2 IS NULL OR v.source_id=?2) AND instr(lower(v.display_name),lower(?3))>0
           AND (?4 IS NULL OR v.kind=?4)
           AND (?5='all' OR (?5='active' AND v.archived_at IS NULL) OR (?5='archived' AND v.archived_at IS NOT NULL))))
         ORDER BY h.scope,v.display_name,h.id LIMIT 26 OFFSET ?6"
    ).bind(world.unwrap_or("")).bind(source).bind(&q.query).bind(q.kind.map(FieldKindTag::as_str))
        .bind(q.archive.as_str()).bind(((q.page-1)*25) as i64)
        .fetch_all(&mut *c).await.map_err(content::storage_error)?;
    let mut items = Vec::with_capacity(ids.len());
    for id in ids {
        let row = store::field(c, world, &id)
            .await
            .map_err(|_| Error::Unavailable)?;
        let archived = row.archived_at;
        items.push(field_summary(row, archived));
    }
    Ok(page(items, q.page))
}

pub(crate) async fn options(
    c: &mut SqliteConnection,
    pin: &FieldPin,
    query: &str,
    number: usize,
) -> Result<SearchPage<ChoiceOption>, Error> {
    let rows = sqlx::query(
        "SELECT option_id,label FROM card_field_options WHERE scope=?1 AND field_id=?2 AND revision=?3
         AND instr(lower(label),lower(?4))>0 ORDER BY slot LIMIT 26 OFFSET ?5"
    ).bind(pin.world_id.as_deref().unwrap_or("")).bind(&pin.field_id).bind(pin.version)
        .bind(query).bind(((number-1)*25) as i64).fetch_all(c).await.map_err(content::storage_error)?;
    let items = rows
        .into_iter()
        .map(|r| {
            Ok(ChoiceOption {
                id: r.try_get("option_id").map_err(content::storage_error)?,
                label: r.try_get("label").map_err(content::storage_error)?,
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;
    Ok(page(items, number))
}
