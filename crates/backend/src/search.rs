use axum::extract::{Query, State};
use axum::response::IntoResponse;
use axum::Json;
use askama::Template;
use serde::{Deserialize, Serialize};
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
    pub type_name_2: Option<String>,
    pub category: Option<String>,
    pub stat: Option<String>,
    pub fully_evolved: Option<String>,
    pub only_base_form: Option<String>,
    pub sort: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default)]
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
    category_lower: Option<String>,
    stats: Vec<String>,
    power: Option<i64>,
    accuracy: Option<i64>,
    pp: Option<i64>,
    score: f64,
    dex_number: Option<i64>,
    is_fully_evolved: bool,
    is_base_form: bool,
    type1_lower: Option<String>,
    type2_lower: Option<String>,
}

#[derive(Debug, Clone)]
struct TypeOption {
    value: String,
    label: String,
    count: usize,
    color: Option<String>,
    is_active: bool,
    is_active_2: bool,
}

#[derive(Debug, Clone)]
struct SimpleOption {
    value: String,
    label: String,
    count: usize,
    is_active: bool,
}

#[derive(Template)]
#[template(path = "search/results.html")]
#[allow(dead_code)]
struct SearchResultsTemplate {
    query: String,
    results: Vec<SearchResult>,
    selected_entity: String,
    pokemon_count: usize,
    move_count: usize,
    ability_count: usize,
    type_count: usize,
    nature_count: usize,
    status_count: usize,
    type_options: Vec<TypeOption>,
    category_options: Vec<SimpleOption>,
    stat_options: Vec<SimpleOption>,
    selected_type: Option<String>,
    selected_type_2: Option<String>,
    selected_category: Option<String>,
    selected_stat: Option<String>,
    selected_fully_evolved: bool,
    selected_only_base_form: bool,
    selected_sort: String,
    fully_evolved_count: usize,
    base_form_count: usize,
    result_count: usize,
    user_role: Option<String>,
    error: Option<String>,
}

#[derive(Serialize)]
pub struct SearchSuggestion {
    pub name: String,
    pub entity_type: String,
    pub url: String,
    pub icon: Option<String>,
}

#[derive(Deserialize)]
pub struct SuggestParams {
    pub q: Option<String>,
}

fn score(q_lower: &str, name: &str, type_fields: &[&str], desc_fields: &[&str]) -> Option<f64> {
    if q_lower.is_empty() { return Some(0.0); }

    let name_lower = name.to_lowercase();

    if name_lower == q_lower { return Some(10.0); }
    if name_lower.starts_with(q_lower) { return Some(8.0); }
    if name_lower.contains(q_lower) { return Some(6.0); }

    // Check if query matches a dex number pattern like "25" or "#25"
    for field in type_fields {
        if !field.is_empty() && field.to_lowercase().contains(q_lower) {
            return Some(4.0);
        }
    }

    for field in desc_fields {
        if !field.is_empty() && field.to_lowercase().contains(q_lower) {
            return Some(2.0);
        }
    }

    None
}

fn matches_type(r: &SearchResult, t1: Option<&str>, t2: Option<&str>) -> bool {
    match (t1, t2) {
        (None, None) => true,
        (Some(t), None) | (None, Some(t)) => {
            let t_lower = t.to_lowercase();
            r.type1_lower.as_deref() == Some(&t_lower)
                || r.type2_lower.as_deref() == Some(&t_lower)
                || (r.entity_type == "type" && r.name.to_lowercase() == t_lower)
        }
        (Some(t1_val), Some(t2_val)) => {
            let t1_lower = t1_val.to_lowercase();
            let t2_lower = t2_val.to_lowercase();
            // Pokemon must have BOTH types (order doesn't matter)
            let has_t1 = r.type1_lower.as_deref() == Some(&t1_lower)
                || r.type2_lower.as_deref() == Some(&t1_lower);
            let has_t2 = r.type1_lower.as_deref() == Some(&t2_lower)
                || r.type2_lower.as_deref() == Some(&t2_lower);
            has_t1 && has_t2
        }
    }
}

pub async fn search_page(session: Session, State(pool): State<SqlitePool>, Query(params): Query<SearchParams>) -> Result<impl IntoResponse, crate::error::AppError> {
    let user_role = auth::get_user_role(&session).await;
    let q = params.q.clone().unwrap_or_default();
    let q_lower = q.to_lowercase();
    let entity = params.entity_type.clone().unwrap_or_else(|| "pokemon".to_string());
    let et = entity.as_str();
    let t1 = params.type_name.as_deref();
    let t2 = params.type_name_2.as_deref();
    let cat = params.category.as_deref();
    let st = params.stat.as_deref();
    let fe = params.fully_evolved.as_deref() == Some("true");
    let obf = params.only_base_form.as_deref() == Some("true");
    let sort = params.sort.clone().unwrap_or_else(|| "relevance".to_string());
    let error_msg = params.error.clone();

    let evolving_from_ids: std::collections::HashSet<i64> = sqlx::query_scalar::<_, i64>(
        "SELECT DISTINCT from_id FROM evolutions"
    ).fetch_all(&pool).await?.into_iter().collect();

    let form_ids: std::collections::HashSet<i64> = sqlx::query_scalar::<_, i64>(
        "SELECT DISTINCT form_id FROM pokemon_forms"
    ).fetch_all(&pool).await?.into_iter().collect();

    let mut all_results: Vec<SearchResult> = Vec::new();

    let pokemon = sqlx::query_as::<_, (i64, String, String, String, String, i64)>(
        "SELECT p.id, p.name, p.pokedex_entry, p.sprite, t.hexcolor, p.dex_number FROM pokemon p JOIN types t ON p.type1_id = t.id ORDER BY p.dex_number, p.id"
    ).fetch_all(&pool).await?;

    let all_types = sqlx::query_as::<_, (i64, String, String)>(
        "SELECT id, name, hexcolor FROM types WHERE id > 0 ORDER BY id"
    ).fetch_all(&pool).await?;

    let type_map: std::collections::HashMap<i64, (String, String)> = all_types.iter()
        .map(|(id, name, color)| (*id, (name.clone(), color.clone())))
        .collect();

    let pokemon_t2 = sqlx::query_as::<_, (i64, Option<i64>)>(
        "SELECT id, type2_id FROM pokemon"
    ).fetch_all(&pool).await?;
    let p2_map: std::collections::HashMap<i64, Option<i64>> = pokemon_t2.into_iter().collect();

    let mut pokemon_count: usize = 0;
    let mut pokemon_type_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut fe_count: usize = 0;
    let mut bf_count: usize = 0;

    for (id, name, entry, sprite, color, dex_number) in &pokemon {
        let type1_info = all_types.iter().find(|(_, _, c)| c == color).map(|(_, n, c)| (n.clone(), c.clone()));
        let t2_info = p2_map.get(id).and_then(|t2_id| t2_id.and_then(|tid| type_map.get(&tid).cloned()));
        let type1_name = type1_info.as_ref().map(|(n, _)| n.as_str()).unwrap_or("");
        let t2_name = t2_info.as_ref().map(|(n, _)| n.as_str()).unwrap_or("");
        let dex_str = format!("#{}", dex_number);
        let is_fe = !evolving_from_ids.contains(id);
        let is_bf = *id < 2000 && !form_ids.contains(id);

        let sc = score(&q_lower, name, &[type1_name, t2_name, &dex_str], &[entry.as_str()]);
        if let Some(s) = sc {
            let t1_lower = type1_info.as_ref().map(|(n, _)| n.to_lowercase());
            let t2_lower = t2_info.as_ref().map(|(n, _)| n.to_lowercase());

            pokemon_count += 1;
            if let Some(ref tl) = t1_lower { *pokemon_type_counts.entry(tl.clone()).or_insert(0) += 1; }
            if let Some(ref tl) = t2_lower { *pokemon_type_counts.entry(tl.clone()).or_insert(0) += 1; }
            if is_fe { fe_count += 1; }
            if is_bf { bf_count += 1; }

            all_results.push(SearchResult {
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
                category_lower: None,
                stats: Vec::new(),
                power: None,
                accuracy: None,
                pp: None,
                score: s,
                dex_number: Some(*dex_number),
                is_fully_evolved: is_fe,
                is_base_form: is_bf,
                type1_lower: t1_lower,
                type2_lower: t2_lower,
            });
        }
    }

    let moves_data = sqlx::query_as::<_, (i64, String, String, String, String, String, Option<i64>, Option<i64>, Option<i64>)>(
        "SELECT m.id, m.name, t.name as type_name, t.hexcolor, m.description, m.category, m.power, m.accuracy, m.pp FROM moves m JOIN types t ON m.type_id = t.id ORDER BY m.name"
    ).fetch_all(&pool).await?;

    let mut move_count: usize = 0;
    let mut move_type_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut cat_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    for (_id, name, type_name, color, desc, category, power, accuracy, pp) in &moves_data {
        let sc = score(&q_lower, name, &[type_name.as_str()], &[desc.as_str()]);
        if let Some(s) = sc {
            move_count += 1;
            *move_type_counts.entry(type_name.to_lowercase()).or_insert(0) += 1;
            *cat_counts.entry(category.to_lowercase()).or_insert(0) += 1;

            all_results.push(SearchResult {
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
                category_lower: Some(category.to_lowercase()),
                stats: Vec::new(),
                power: *power,
                accuracy: *accuracy,
                pp: *pp,
                score: s,
                dex_number: None,
                is_fully_evolved: true,
                is_base_form: true,
                type1_lower: Some(type_name.to_lowercase()),
                type2_lower: None,
            });
        }
    }

    let abilities = sqlx::query_as::<_, (String, String)>(
        "SELECT name, description FROM abilities ORDER BY name"
    ).fetch_all(&pool).await?;

    let mut ability_count: usize = 0;
    for (name, desc) in &abilities {
        let sc = score(&q_lower, name, &[], &[desc.as_str()]);
        if let Some(s) = sc {
            ability_count += 1;
            all_results.push(SearchResult {
                entity_type: "ability".to_string(),
                name: name.clone(),
                url: format!("/wiki/abilities/{}", to_slug(name)),
                snippet: truncate(desc, 120),
                badge: None, badge_color: None, icon: Some("/static/icons/items/ability_patch.png?v=1".to_string()),
                type1: None, type1_color: None, type2: None, type2_color: None,
                entity_label: "Ability".to_string(),
                category_lower: None,
                stats: Vec::new(),
                power: None, accuracy: None, pp: None,
                score: s,
                dex_number: None,
                is_fully_evolved: true,
                is_base_form: true,
                type1_lower: None,
                type2_lower: None,
            });
        }
    }

    let mut type_count: usize = 0;
    for (_type_id, type_name, type_color) in &all_types {
        let sc = score(&q_lower, type_name, &[], &[]);
        if let Some(s) = sc {
            type_count += 1;
            all_results.push(SearchResult {
                entity_type: "type".to_string(),
                name: type_name.clone(),
                url: format!("/wiki/types/{}", to_slug(type_name)),
                snippet: String::new(),
                badge: None, badge_color: Some(type_color.clone()),
                icon: Some(format!("/static/icons/types/{}.png?v=1", to_slug(type_name))),
                type1: Some(type_name.clone()), type1_color: Some(type_color.clone()),
                type2: None, type2_color: None,
                entity_label: "Type".to_string(),
                category_lower: None,
                stats: Vec::new(),
                power: None, accuracy: None, pp: None,
                score: s,
                dex_number: None,
                is_fully_evolved: true,
                is_base_form: true,
                type1_lower: Some(type_name.to_lowercase()),
                type2_lower: None,
            });
        }
    }

    let natures = sqlx::query_as::<_, (String, Option<String>, Option<String>)>(
        "SELECT name, increased_stat, decreased_stat FROM natures ORDER BY name"
    ).fetch_all(&pool).await?;

    let mut nature_count: usize = 0;
    let mut stat_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for (name, inc, dec) in &natures {
        let type_fields: Vec<&str> = [inc.as_deref(), dec.as_deref()].into_iter().flatten().collect();
        let sc = score(&q_lower, name, &type_fields, &[]);
        if let Some(s) = sc {
            nature_count += 1;
            let mut stats = Vec::new();
            if let Some(s) = inc { stats.push(s.to_lowercase()); *stat_counts.entry(s.to_lowercase()).or_insert(0) += 1; }
            if let Some(s) = dec { stats.push(s.to_lowercase()); *stat_counts.entry(s.to_lowercase()).or_insert(0) += 1; }
            let mint_icon = match inc.as_deref().map(|s| s.to_lowercase()).as_deref() {
                Some("spa") => "modest_mint",
                Some("spe") => "timid_mint",
                Some("spd") => "calm_mint",
                Some("def") => "bold_mint",
                Some("atk") => "lonely_mint",
                _ => "modest_mint",
            };
            all_results.push(SearchResult {
                entity_type: "nature".to_string(),
                name: name.clone(),
                url: "/wiki/mechanics/nature".to_string(),
                snippet: format!("+10% {} / -10% {}", inc.as_deref().unwrap_or("—"), dec.as_deref().unwrap_or("—")),
                badge: None, badge_color: None, icon: Some(format!("/static/icons/items/{}.png?v=1", mint_icon)),
                type1: None, type1_color: None, type2: None, type2_color: None,
                entity_label: "Nature".to_string(),
                category_lower: None,
                stats,
                power: None, accuracy: None, pp: None,
                score: s,
                dex_number: None,
                is_fully_evolved: true,
                is_base_form: true,
                type1_lower: None,
                type2_lower: None,
            });
        }
    }

    let statuses = sqlx::query_as::<_, (String, String)>(
        "SELECT name, description FROM status_conditions ORDER BY name"
    ).fetch_all(&pool).await?;

    let mut status_count: usize = 0;
    for (name, desc) in &statuses {
        let sc = score(&q_lower, name, &[], &[desc.as_str()]);
        if let Some(s) = sc {
            status_count += 1;
            all_results.push(SearchResult {
                entity_type: "status".to_string(),
                name: name.clone(),
                url: "/wiki/mechanics/status".to_string(),
                snippet: truncate(desc, 120),
                badge: None, badge_color: None, icon: Some("/static/icons/items/burn_heal.png?v=1".to_string()),
                type1: None, type1_color: None, type2: None, type2_color: None,
                entity_label: "Status".to_string(),
                category_lower: None,
                stats: Vec::new(),
                power: None, accuracy: None, pp: None,
                score: s,
                dex_number: None,
                is_fully_evolved: true,
                is_base_form: true,
                type1_lower: None,
                type2_lower: None,
            });
        }
    }

    // Filter by entity type (default pokemon)
    let mut results: Vec<SearchResult> = all_results.into_iter()
        .filter(|r| r.entity_type == et)
        .collect();

    // Filter by type(s)
    if t1.is_some() || t2.is_some() {
        results.retain(|r| matches_type(r, t1, t2));
    }

    // Filter by category (moves only)
    if let Some(c) = cat {
        if et == "move" {
            let c_lower = c.to_lowercase();
            results.retain(|r| r.category_lower.as_deref() == Some(&c_lower));
        }
    }

    // Filter by stat (natures only)
    if let Some(s) = st {
        if et == "nature" {
            let s_lower = s.to_lowercase();
            results.retain(|r| r.stats.iter().any(|st| st == &s_lower));
        }
    }

    // Filter by fully evolved (pokemon only)
    if fe && et == "pokemon" {
        results.retain(|r| r.is_fully_evolved);
    }

    // Filter by base form only (pokemon only)
    if obf && et == "pokemon" {
        results.retain(|r| r.is_base_form);
    }

    // Sort results
    match sort.as_str() {
        "name" => results.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase())),
        "dex" => results.sort_by(|a, b| {
            let a_dex = a.dex_number.unwrap_or(i64::MAX);
            let b_dex = b.dex_number.unwrap_or(i64::MAX);
            a_dex.cmp(&b_dex).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        }),
        _ => results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))),
    }

    results.truncate(200);
    let result_count = results.len();

    // Build type options based on selected entity type
    let type_options: Vec<TypeOption> = if et == "pokemon" {
        all_types.iter().map(|(_id, tn, tc)| {
            let key = tn.to_lowercase();
            let count = pokemon_type_counts.get(&key).copied().unwrap_or(0);
            TypeOption {
                value: key.clone(),
                label: tn.clone(),
                count,
                color: Some(tc.clone()),
                is_active: t1 == Some(&key),
                is_active_2: t2 == Some(&key),
            }
        }).collect()
    } else if et == "move" {
        all_types.iter().map(|(_id, tn, tc)| {
            let key = tn.to_lowercase();
            let count = move_type_counts.get(&key).copied().unwrap_or(0);
            TypeOption {
                value: key.clone(),
                label: tn.clone(),
                count,
                color: Some(tc.clone()),
                is_active: t1 == Some(&key),
                is_active_2: t2 == Some(&key),
            }
        }).collect()
    } else {
        Vec::new()
    };

    // Build category options (moves only)
    let category_options: Vec<SimpleOption> = if et == "move" {
        [("physical", "Physical"), ("special", "Special"), ("status", "Status")].iter().map(|(key, label)| {
            let count = cat_counts.get(*key).copied().unwrap_or(0);
            SimpleOption {
                value: key.to_string(),
                label: label.to_string(),
                count,
                is_active: cat == Some(*key),
            }
        }).collect()
    } else {
        Vec::new()
    };

    // Build stat options (natures only)
    let stat_options: Vec<SimpleOption> = if et == "nature" {
        [("hp", "HP"), ("atk", "ATK"), ("def", "DEF"), ("spa", "SPA"), ("spd", "SPD"), ("spe", "SPE")].iter().map(|(key, label)| {
            let count = stat_counts.get(*key).copied().unwrap_or(0);
            SimpleOption {
                value: key.to_string(),
                label: label.to_string(),
                count,
                is_active: st == Some(*key),
            }
        }).collect()
    } else {
        Vec::new()
    };

    Ok(render(SearchResultsTemplate {
        query: q,
        results,
        selected_entity: entity,
        pokemon_count,
        move_count,
        ability_count,
        type_count,
        nature_count,
        status_count,
        type_options,
        category_options,
        stat_options,
        selected_type: t1.map(|s| s.to_string()),
        selected_type_2: t2.map(|s| s.to_string()),
        selected_category: cat.map(|s| s.to_string()),
        selected_stat: st.map(|s| s.to_string()),
        selected_fully_evolved: fe,
        selected_only_base_form: obf,
        selected_sort: sort,
        fully_evolved_count: fe_count,
        base_form_count: bf_count,
        result_count,
        user_role,
        error: error_msg,
    }))
}

pub async fn search_suggestions(State(pool): State<SqlitePool>, Query(params): Query<SuggestParams>) -> Json<Vec<SearchSuggestion>> {
    let q = params.q.clone().unwrap_or_default();
    let q_lower = q.to_lowercase();

    if q_lower.len() < 2 {
        return Json(Vec::new());
    }

    let mut suggestions: Vec<SearchSuggestion> = Vec::new();

    if let Ok(pokemon) = sqlx::query_as::<_, (String, String)>(
        "SELECT name, sprite FROM pokemon ORDER BY dex_number, id"
    ).fetch_all(&pool).await {
        for (name, sprite) in pokemon {
            let nl = name.to_lowercase();
            if nl.starts_with(&q_lower) || nl.contains(&q_lower) {
                suggestions.push(SearchSuggestion {
                    name: name.clone(),
                    entity_type: "pokemon".to_string(),
                    url: format!("/wiki/pokemon/{}", to_slug(&name)),
                    icon: Some(format!("/static/sprites/pokemon/{}?v=1", sprite)),
                });
            }
            if suggestions.len() >= 8 { break; }
        }
    }

    if suggestions.len() < 8 {
        if let Ok(moves_data) = sqlx::query_as::<_, (String,)>(
            "SELECT name FROM moves ORDER BY name"
        ).fetch_all(&pool).await {
            for (name,) in moves_data {
                if name.to_lowercase().starts_with(&q_lower) || name.to_lowercase().contains(&q_lower) {
                    suggestions.push(SearchSuggestion {
                        name: name.clone(),
                        entity_type: "move".to_string(),
                        url: format!("/wiki/moves/{}", to_slug(&name)),
                        icon: None,
                    });
                }
                if suggestions.len() >= 8 { break; }
            }
        }
    }

    if suggestions.len() < 8 {
        if let Ok(abilities) = sqlx::query_as::<_, (String,)>(
            "SELECT name FROM abilities ORDER BY name"
        ).fetch_all(&pool).await {
            for (name,) in abilities {
                if name.to_lowercase().starts_with(&q_lower) || name.to_lowercase().contains(&q_lower) {
                    suggestions.push(SearchSuggestion {
                        name: name.clone(),
                        entity_type: "ability".to_string(),
                        url: format!("/wiki/abilities/{}", to_slug(&name)),
                        icon: None,
                    });
                }
                if suggestions.len() >= 8 { break; }
            }
        }
    }

    if suggestions.len() < 8 {
        if let Ok(types) = sqlx::query_as::<_, (String,)>(
            "SELECT name FROM types WHERE id > 0 ORDER BY id"
        ).fetch_all(&pool).await {
            for (name,) in types {
                if name.to_lowercase().starts_with(&q_lower) || name.to_lowercase().contains(&q_lower) {
                    suggestions.push(SearchSuggestion {
                        name: name.clone(),
                        entity_type: "type".to_string(),
                        url: format!("/wiki/types/{}", to_slug(&name)),
                        icon: None,
                    });
                }
                if suggestions.len() >= 8 { break; }
            }
        }
    }

    suggestions.sort_by(|a, b| {
        let a_starts = a.name.to_lowercase().starts_with(&q_lower);
        let b_starts = b.name.to_lowercase().starts_with(&q_lower);
        b_starts.cmp(&a_starts).then_with(|| a.entity_type.cmp(&b.entity_type)).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    suggestions.truncate(8);

    Json(suggestions)
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        let end = s.char_indices().take_while(|(i, _)| *i < max_len).last().map(|(i, c)| i + c.len_utf8()).unwrap_or(max_len);
        format!("{}...", &s[..end.min(s.len())])
    }
}
