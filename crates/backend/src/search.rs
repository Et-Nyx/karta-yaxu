use axum::extract::{Query, State};
use axum::response::IntoResponse;
use askama::Template;
use serde::Deserialize;
use sqlx::SqlitePool;
use crate::auth;
use crate::error::render;
use crate::filters::to_slug;
use tower_sessions::Session;

#[derive(Deserialize, Default)]
pub struct SearchParams {
    pub q: Option<String>,
    pub entity_type: Option<String>,
    pub type_name: Option<String>,
    pub category: Option<String>,
    pub stat: Option<String>,
}

#[derive(Debug, Clone)]
struct SearchResult {
    entity_type: String,
    name: String,
    url: String,
    snippet: String,
    badge: Option<String>,
    badge_color: Option<String>,
    icon: Option<String>,
    type1: Option<String>,
    type1_color: Option<String>,
    type2: Option<String>,
    type2_color: Option<String>,
    entity_label: String,
    type_name_lower: Option<String>,
    category_lower: Option<String>,
    stats: Vec<String>,
    // Move-specific stats
    power: Option<i64>,
    accuracy: Option<i64>,
    pp: Option<i64>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct FacetOption {
    value: String,
    label: String,
    count: usize,
    color: Option<String>,
    url: String,
    is_active: bool,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct FacetGroup {
    key: String,
    label: String,
    options: Vec<FacetOption>,
}

#[derive(Template)]
#[template(path = "search/results.html")]
struct SearchResultsTemplate {
    query: String,
    results: Vec<SearchResult>,
    facets: Vec<FacetGroup>,
    clear_url: Option<String>,
    result_count: usize,
    user_role: Option<String>,
}

fn build_url(q: &str, entity_type: Option<&str>, type_name: Option<&str>, category: Option<&str>, stat: Option<&str>) -> String {
    let mut params = vec![format!("q={}", urlencoding::encode(q))];
    if let Some(v) = entity_type { params.push(format!("entity_type={}", v)); }
    if let Some(v) = type_name { params.push(format!("type_name={}", v)); }
    if let Some(v) = category { params.push(format!("category={}", v)); }
    if let Some(v) = stat { params.push(format!("stat={}", v)); }
    format!("/search?{}", params.join("&"))
}

fn matches_text(q_lower: &str, fields: &[&str]) -> bool {
    if q_lower.is_empty() { return true; }
    fields.iter().any(|f| f.to_lowercase().contains(q_lower))
}

fn matches_facets(r: &SearchResult, aet: Option<&str>, at: Option<&str>, ac: Option<&str>, ast: Option<&str>) -> bool {
    if let Some(et) = aet { if et != r.entity_type { return false; } }
    if let Some(t) = at {
        let t_lower = t.to_lowercase();
        let found = r.type_name_lower.as_deref() == Some(&t_lower)
            || r.type1.as_deref().map(|s| s.to_lowercase()) == Some(t_lower.clone())
            || r.type2.as_deref().map(|s| s.to_lowercase()) == Some(t_lower.clone())
            || r.entity_type == "type" && r.name.to_lowercase() == t_lower;
        if !found { return false; }
    }
    if let Some(c) = ac {
        if r.category_lower.as_deref() != Some(c.to_lowercase().as_str()) { return false; }
    }
    if let Some(s) = ast {
        if !r.stats.iter().any(|st| st.to_lowercase() == s.to_lowercase()) { return false; }
    }
    true
}

pub async fn search_page(session: Session, State(pool): State<SqlitePool>, Query(params): Query<SearchParams>) -> impl IntoResponse {
    let user_role = auth::get_user_role(&session).await;
    let q = params.q.clone().unwrap_or_default();
    let q_lower = q.to_lowercase();
    let aet = params.entity_type.as_deref();
    let at = params.type_name.as_deref();
    let ac = params.category.as_deref();
    let ast = params.stat.as_deref();

    let mut base: Vec<SearchResult> = Vec::new();

    let pokemon = sqlx::query_as::<_, (i64, String, String, String, String, i64)>(
        "SELECT p.id, p.name, p.pokedex_entry, p.sprite, t.hexcolor, p.dex_number FROM pokemon p JOIN types t ON p.type1_id = t.id ORDER BY p.dex_number, p.id"
    ).fetch_all(&pool).await.unwrap_or_default();

    let all_types = sqlx::query_as::<_, (i64, String, String)>(
        "SELECT id, name, hexcolor FROM types WHERE id > 0 ORDER BY id"
    ).fetch_all(&pool).await.unwrap_or_default();

    let type_map: std::collections::HashMap<i64, (String, String)> = all_types.iter()
        .map(|(id, name, color)| (*id, (name.clone(), color.clone())))
        .collect();

    let pokemon_t2 = sqlx::query_as::<_, (i64, Option<i64>)>(
        "SELECT id, type2_id FROM pokemon"
    ).fetch_all(&pool).await.unwrap_or_default();
    let p2_map: std::collections::HashMap<i64, Option<i64>> = pokemon_t2.into_iter().collect();

    for (id, name, entry, sprite, color, dex_number) in &pokemon {
        if !matches_text(&q_lower, &[name, entry]) { continue; }
        let t2_info = p2_map.get(id).and_then(|t2_id| t2_id.and_then(|tid| type_map.get(&tid).cloned()));
        // Look up type1 name from type_map using the color (hexcolor from the join)
        let type1_info = all_types.iter().find(|(_, _, c)| c == color).map(|(_, n, c)| (n.clone(), c.clone()));
        base.push(SearchResult {
            entity_type: "pokemon".to_string(),
            name: name.clone(),
            url: format!("/wiki/pokemon/{}", to_slug(name)),
            snippet: truncate(entry, 120),
            badge: Some(format!("#{}", dex_number)),
            badge_color: Some(color.clone()),
            icon: Some(format!("/static/sprites/pokemon/{}?v=1", sprite)),
            type1: type1_info.as_ref().map(|(n, _)| n.clone()),
            type1_color: Some(color.clone()),
            type2: t2_info.as_ref().map(|(n, _)| n.clone()),
            type2_color: t2_info.as_ref().map(|(_, c)| c.clone()),
            entity_label: "Pokémon".to_string(),
            type_name_lower: type1_info.as_ref().map(|(n, _)| n.to_lowercase()),
            category_lower: None,
            stats: Vec::new(),
            power: None,
            accuracy: None,
            pp: None,
        });
    }

    let moves_data = sqlx::query_as::<_, (i64, String, String, String, String, String, Option<i64>, Option<i64>, Option<i64>)>(
        "SELECT m.id, m.name, t.name as type_name, t.hexcolor, m.description, m.category, m.power, m.accuracy, m.pp FROM moves m JOIN types t ON m.type_id = t.id ORDER BY m.name"
    ).fetch_all(&pool).await.unwrap_or_default();

    for (_id, name, type_name, color, desc, category, power, accuracy, pp) in &moves_data {
        if !matches_text(&q_lower, &[name, desc]) { continue; }
        base.push(SearchResult {
            entity_type: "move".to_string(),
            name: name.clone(),
            url: format!("/wiki/moves/{}", to_slug(name)),
            snippet: truncate(desc, 120),
            badge: Some(category.clone()),
            badge_color: Some(color.clone()),
            icon: Some(format!("/static/icons/moves/{}.png?v=1", to_slug(category))),
            type1: Some(type_name.clone()),
            type1_color: Some(color.clone()),
            type2: None,
            type2_color: None,
            entity_label: "Move".to_string(),
            type_name_lower: Some(color.to_lowercase()),
            category_lower: Some(category.to_lowercase()),
            stats: Vec::new(),
            power: *power,
            accuracy: *accuracy,
            pp: *pp,
        });
    }

    let abilities = sqlx::query_as::<_, (String, String)>(
        "SELECT name, description FROM abilities ORDER BY name"
    ).fetch_all(&pool).await.unwrap_or_default();

    for (name, desc) in &abilities {
        if !matches_text(&q_lower, &[name, desc]) { continue; }
        base.push(SearchResult {
            entity_type: "ability".to_string(),
            name: name.clone(),
            url: format!("/wiki/abilities/{}", to_slug(name)),
            snippet: truncate(desc, 120),
            badge: None, badge_color: None, icon: Some("/static/icons/items/ability_patch.png?v=1".to_string()),
            type1: None, type1_color: None, type2: None, type2_color: None,
            entity_label: "Ability".to_string(),
            type_name_lower: None,
            category_lower: None,
            stats: Vec::new(),
            power: None,
            accuracy: None,
            pp: None,
        });
    }

    for (_type_id, type_name, type_color) in &all_types {
        if !matches_text(&q_lower, &[type_name]) { continue; }
        base.push(SearchResult {
            entity_type: "type".to_string(),
            name: type_name.clone(),
            url: format!("/wiki/types/{}", to_slug(type_name)),
            snippet: String::new(),
            badge: None, badge_color: Some(type_color.clone()),
            icon: Some(format!("/static/icons/types/{}.png?v=1", to_slug(type_name))),
            type1: Some(type_name.clone()), type1_color: Some(type_color.clone()),
            type2: None, type2_color: None,
            entity_label: "Type".to_string(),
            type_name_lower: Some(type_name.to_lowercase()),
            category_lower: None,
            stats: Vec::new(),
            power: None,
            accuracy: None,
            pp: None,
        });
    }

    let natures = sqlx::query_as::<_, (String, Option<String>, Option<String>)>(
        "SELECT name, increased_stat, decreased_stat FROM natures ORDER BY name"
    ).fetch_all(&pool).await.unwrap_or_default();

    for (name, inc, dec) in &natures {
        let searchable = vec![Some(name.clone()), inc.clone(), dec.clone()].into_iter().flatten().collect::<Vec<_>>();
        let refs: Vec<&str> = searchable.iter().map(|s| s.as_str()).collect();
        if !matches_text(&q_lower, &refs) { continue; }
        let mut stats = Vec::new();
        if let Some(s) = inc { stats.push(s.to_lowercase()); }
        if let Some(s) = dec { stats.push(s.to_lowercase()); }
        let mint_icon = match inc.as_deref().map(|s| s.to_lowercase()).as_deref() {
            Some("spa") => "modest_mint",
            Some("spe") => "timid_mint",
            Some("spd") => "calm_mint",
            Some("def") => "bold_mint",
            Some("atk") => "lonely_mint",
            _ => "modest_mint",
        };
        let nature_icon = format!("/static/icons/items/{}.png?v=1", mint_icon);
        base.push(SearchResult {
            entity_type: "nature".to_string(),
            name: name.clone(),
            url: "/wiki/mechanics/nature".to_string(),
            snippet: format!("+10% {} / -10% {}", inc.as_deref().unwrap_or("—"), dec.as_deref().unwrap_or("—")),
            badge: None, badge_color: None, icon: Some(nature_icon),
            type1: None, type1_color: None, type2: None, type2_color: None,
            entity_label: "Nature".to_string(),
            type_name_lower: None,
            category_lower: None,
            stats,
            power: None,
            accuracy: None,
            pp: None,
        });
    }

    let statuses = sqlx::query_as::<_, (String, String)>(
        "SELECT name, description FROM status_conditions ORDER BY name"
    ).fetch_all(&pool).await.unwrap_or_default();

    for (name, desc) in &statuses {
        if !matches_text(&q_lower, &[name, desc]) { continue; }
        base.push(SearchResult {
            entity_type: "status".to_string(),
            name: name.clone(),
            url: "/wiki/mechanics/status".to_string(),
            snippet: truncate(desc, 120),
            badge: None, badge_color: None, icon: Some("/static/icons/items/burn_heal.png?v=1".to_string()),
            type1: None, type1_color: None, type2: None, type2_color: None,
            entity_label: "Status".to_string(),
            type_name_lower: None,
            category_lower: None,
            stats: Vec::new(),
            power: None,
            accuracy: None,
            pp: None,
        });
    }

    // Compute facet counts from base (query-matched, unfiltered) set
    let mut entity_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut type_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut cat_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut stat_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    for r in &base {
        *entity_counts.entry(r.entity_type.clone()).or_insert(0) += 1;
        if let Some(ref tn) = r.type_name_lower {
            *type_counts.entry(tn.clone()).or_insert(0) += 1;
        }
        if r.entity_type == "type" {
            *type_counts.entry(r.name.to_lowercase()).or_insert(0) += 1;
        }
        if let Some(ref cat) = r.category_lower {
            *cat_counts.entry(cat.clone()).or_insert(0) += 1;
        }
        for st in &r.stats {
            *stat_counts.entry(st.clone()).or_insert(0) += 1;
        }
    }

    // Apply facet filters
    let results: Vec<SearchResult> = base.into_iter()
        .filter(|r| matches_facets(r, aet, at, ac, ast))
        .take(100)
        .collect();

    let result_count = results.len();

    // Build entity facet
    let entity_labels = vec![
        ("pokemon", "Pokémon"),
        ("move", "Moves"),
        ("ability", "Abilities"),
        ("type", "Types"),
        ("nature", "Natures"),
        ("status", "Status"),
    ];
    let entity_opts: Vec<FacetOption> = entity_labels.iter().map(|(key, label)| {
        let count = entity_counts.get(*key).copied().unwrap_or(0);
        FacetOption {
            value: key.to_string(),
            label: label.to_string(),
            count,
            color: None,
            url: build_url(&q, Some(key), at, ac, ast),
            is_active: aet == Some(key),
        }
    }).filter(|opt| opt.count > 0 || opt.is_active)
    .collect();

    // Build type facets
    let type_opts: Vec<FacetOption> = all_types.iter().map(|(_id, tn, tc)| {
        let key = tn.to_lowercase();
        let count = type_counts.get(&key).copied().unwrap_or(0);
        FacetOption {
            value: key.clone(),
            label: tn.clone(),
            count,
            color: Some(tc.clone()),
            url: build_url(&q, aet, Some(&key), ac, ast),
            is_active: at == Some(&key),
        }
    }).filter(|opt| opt.count > 0 || opt.is_active)
    .collect();

    // Build category facets
    let cat_labels = vec![("physical", "Physical"), ("special", "Special"), ("status", "Status")];
    let cat_opts: Vec<FacetOption> = cat_labels.iter().map(|(key, label)| {
        let count = cat_counts.get(*key).copied().unwrap_or(0);
        FacetOption {
            value: key.to_string(),
            label: label.to_string(),
            count,
            color: None,
            url: build_url(&q, aet, at, Some(key), ast),
            is_active: ac == Some(key),
        }
    }).filter(|opt| opt.count > 0 || opt.is_active)
    .collect();

    // Build stat facets
    let stat_labels = vec![("hp", "HP"), ("atk", "ATK"), ("def", "DEF"), ("spa", "SPA"), ("spd", "SPD"), ("spe", "SPE")];
    let stat_opts: Vec<FacetOption> = stat_labels.iter().map(|(key, label)| {
        let count = stat_counts.get(*key).copied().unwrap_or(0);
        FacetOption {
            value: key.to_string(),
            label: label.to_string(),
            count,
            color: None,
            url: build_url(&q, aet, at, ac, Some(key)),
            is_active: ast == Some(key),
        }
    }).filter(|opt| opt.count > 0 || opt.is_active)
    .collect();

    let clear_url = if aet.is_some() || at.is_some() || ac.is_some() || ast.is_some() {
        Some(build_url(&q, None, None, None, None))
    } else {
        None
    };

    let all_facets = vec![
        FacetGroup { key: "entity_type".to_string(), label: "Categoria".to_string(), options: entity_opts },
        FacetGroup { key: "type_name".to_string(), label: "Tipo".to_string(), options: type_opts },
        FacetGroup { key: "category".to_string(), label: "Categoria (Moves)".to_string(), options: cat_opts },
        FacetGroup { key: "stat".to_string(), label: "Stat (Natures)".to_string(), options: stat_opts },
    ];

    render(SearchResultsTemplate {
        query: q,
        results,
        facets: all_facets,
        clear_url,
        result_count,
        user_role,
    }).into_response()
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        let end = s.char_indices().take_while(|(i, _)| *i < max_len).last().map(|(i, c)| i + c.len_utf8()).unwrap_or(max_len);
        format!("{}...", &s[..end.min(s.len())])
    }
}