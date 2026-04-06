use axum::{extract::{Path, State}, http::StatusCode, response::{Html, IntoResponse}};
use sqlx::SqlitePool;
use askama::Template;
use engine::models::{Pokemon, Type, Ability, Nature, StatusCondition, Move};

// ── Helpers ───────────────────────────────────────────────────────────────────

mod filters {
    pub fn slugify<T: std::fmt::Display>(s: T) -> askama::Result<String> {
        Ok(s.to_string().to_lowercase().replace(' ', "-").replace('\'', ""))
    }
}

fn to_slug(name: &str) -> String {
    name.to_lowercase().replace(' ', "-").replace('\'', "")
}

fn render(t: impl Template) -> Result<Html<String>, StatusCode> {
    let html = t.render().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Html(html))
}

// ── Pokémon ───────────────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "wiki/pokemon/list.html")]
struct PokemonListTemplate {
    pokemon: Vec<Pokemon>,
}

pub async fn pokemon_list(State(pool): State<SqlitePool>) -> impl IntoResponse {
    let pokemon = sqlx::query_as::<_, Pokemon>(
        "SELECT id, dex_number, name, sprite, sprite_shiny,
                type1_id, type2_id, hp, atk, def, spa, spd, spe, pokedex_entry
         FROM pokemon ORDER BY dex_number, id"
    ).fetch_all(&pool).await.unwrap_or_default();

    render(PokemonListTemplate { pokemon }).unwrap_or_else(|s| Html(s.to_string()))
}

#[derive(Template)]
#[template(path = "wiki/pokemon/detail.html")]
struct PokemonDetailTemplate {
    p: Pokemon,
    type1: Type,
    type2: Option<Type>,
    abilities: Vec<Ability>,
    evolutions: Vec<Pokemon>,
    forms: Vec<Pokemon>,
    learnset: Vec<(Move, Type, String, Option<i64>)>, // (Golpe, Tipo do golpe, método, nível)
}

pub async fn pokemon_detail(
    State(pool): State<SqlitePool>,
    Path(slug): Path<String>,
) -> impl IntoResponse {
    let all = sqlx::query_as::<_, Pokemon>(
        "SELECT id, dex_number, name, sprite, sprite_shiny,
                type1_id, type2_id, hp, atk, def, spa, spd, spe, pokedex_entry
         FROM pokemon"
    ).fetch_all(&pool).await.unwrap_or_default();

    let p = match all.iter().find(|p| to_slug(&p.name) == slug).cloned() {
        Some(p) => p,
        None => return Html(StatusCode::NOT_FOUND.to_string()),
    };

    let types = sqlx::query_as::<_, Type>(
        "SELECT id, name, hexcolor, icon FROM types"
    ).fetch_all(&pool).await.unwrap_or_default();

    let type1 = types.iter().find(|t| t.id == p.type1_id).cloned()
        .unwrap_or(Type { id: 0, name: "???".into(), hexcolor: "#999".into(), icon: "".into() });
    let type2 = p.type2_id.and_then(|id| types.iter().find(|t| t.id == id).cloned());

    let abilities = sqlx::query_as::<_, Ability>(
        "SELECT a.id, a.name, a.description
         FROM pokemon_abilities pa
         JOIN abilities a ON a.id = pa.ability_id
         WHERE pa.pokemon_id = ? ORDER BY pa.slot"
    ).bind(p.id).fetch_all(&pool).await.unwrap_or_default();

    let all_forms = sqlx::query_as::<_, (i64, i64)>("SELECT base_id, form_id FROM pokemon_forms").fetch_all(&pool).await.unwrap_or_default();
    
    let true_base_id = all_forms.iter()
        .find(|&&(_, form)| form == p.id)
        .map(|&(base, _)| base)
        .unwrap_or(p.id);

    let all_evos = sqlx::query_as::<_, (i64, i64)>("SELECT from_id, to_id FROM evolutions").fetch_all(&pool).await.unwrap_or_default();

    // Uma forma só herda a linha evolutiva se não estiver atrelada a NENHUMA evolução própria
    let has_own_evos = all_evos.iter().any(|&(from, to)| from == p.id || to == p.id);
    let evo_base_id = if has_own_evos { p.id } else { true_base_id };

    // Volta o id até o estágio mais básico (Baby/Base) apenas seguindo evoluções
    let mut root_id = evo_base_id;
    loop {
        if let Some((from_id, _)) = all_evos.iter().find(|&&(_, to)| to == root_id) {
            root_id = *from_id;
        } else {
            break;
        }
    }

    // Desce e coleta toda a árvore
    let mut evo_ids = vec![root_id];
    let mut head = 0;
    while head < evo_ids.len() {
        let current = evo_ids[head];
        for &(from, to) in &all_evos {
            if from == current && !evo_ids.contains(&to) { 
                evo_ids.push(to); 
            }
        }
        head += 1;
    }

    let mut evolutions: Vec<Pokemon> = evo_ids.into_iter()
        .filter_map(|id| all.iter().find(|all_p| all_p.id == id).cloned())
        .collect();
    evolutions.sort_by_key(|e| e.dex_number);

    let mut forms_ids = vec![true_base_id];
    for &(base, form) in &all_forms {
        if base == true_base_id { forms_ids.push(form); }
    }
    forms_ids.retain(|&id| id != p.id); // Remove si mesmo

    let mut forms: Vec<Pokemon> = forms_ids.into_iter()
        .filter_map(|id| all.iter().find(|all_p| all_p.id == id).cloned())
        .collect();
    forms.sort_by_key(|f| f.id);

    // Buscar lista de moves para cruzar:
    let all_moves = sqlx::query_as::<_, Move>("SELECT * FROM moves").fetch_all(&pool).await.unwrap_or_default();

    let mut learn_rels = sqlx::query_as::<_, (i64, String, Option<i64>)>(
        "SELECT move_id, method, level FROM learnset WHERE pokemon_id = ? ORDER BY level, move_id"
    ).bind(p.id).fetch_all(&pool).await.unwrap_or_default();

    // Fallback: Se não tem moveset próprio e é uma forma alternativa, herda o da base
    if learn_rels.is_empty() && p.id != true_base_id {
        learn_rels = sqlx::query_as::<_, (i64, String, Option<i64>)>(
            "SELECT move_id, method, level FROM learnset WHERE pokemon_id = ? ORDER BY level, move_id"
        ).bind(true_base_id).fetch_all(&pool).await.unwrap_or_default();
    }

    let mut learnset = Vec::new();
    for (m_id, method, level) in learn_rels {
        if let Some(m) = all_moves.iter().find(|m| m.id == m_id) {
            let t = types.iter().find(|t| t.id == m.type_id).cloned().unwrap_or(type1.clone());
            learnset.push((m.clone(), t, method, level));
        }
    }

    render(PokemonDetailTemplate { p, type1, type2, abilities, evolutions, forms, learnset })
        .unwrap_or_else(|s| Html(s.to_string()))
}

// ── Tipos ─────────────────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "wiki/types/list.html")]
struct TypesListTemplate {
    types: Vec<Type>,
}

pub async fn types_list(State(pool): State<SqlitePool>) -> impl IntoResponse {
    let types = sqlx::query_as::<_, Type>(
        "SELECT id, name, hexcolor, icon FROM types ORDER BY id"
    ).fetch_all(&pool).await.unwrap_or_default();

    render(TypesListTemplate { types }).unwrap_or_else(|s| Html(s.to_string()))
}

#[derive(Template)]
#[template(path = "wiki/types/detail.html")]
struct TypeDetailTemplate {
    t: Type,
    as_attacker: Vec<(Type, f64)>, // (Tipo Defensor, multiplicador)
    as_defender: Vec<(Type, f64)>, // (Tipo Atacante, multiplicador)
}

pub async fn type_detail(
    State(pool): State<SqlitePool>,
    Path(slug): Path<String>,
) -> impl IntoResponse {
    let all_types = sqlx::query_as::<_, Type>("SELECT id, name, hexcolor, icon FROM types").fetch_all(&pool).await.unwrap_or_default();

    let t = match all_types.iter().find(|t| to_slug(&t.name) == slug).cloned() {
        Some(t) => t,
        None => return Html(StatusCode::NOT_FOUND.to_string()),
    };

    let eff_relations = sqlx::query_as::<_, (i64, i64, f64)>(
        "SELECT attacker_id, defender_id, multiplier FROM type_effectiveness"
    ).fetch_all(&pool).await.unwrap_or_default();

    let mut as_attacker = Vec::new();
    let mut as_defender = Vec::new();

    for (atk_id, def_id, mult) in eff_relations {
        if atk_id == t.id {
            if let Some(def_t) = all_types.iter().find(|t| t.id == def_id).cloned() {
                as_attacker.push((def_t, mult));
            }
        }
        if def_id == t.id {
            if let Some(atk_t) = all_types.iter().find(|t| t.id == atk_id).cloned() {
                as_defender.push((atk_t, mult));
            }
        }
    }
    
    // Sort
    as_attacker.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap().then_with(|| a.0.name.cmp(&b.0.name)));
    as_defender.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap().then_with(|| a.0.name.cmp(&b.0.name)));

    render(TypeDetailTemplate { t, as_attacker, as_defender })
        .unwrap_or_else(|s| Html(s.to_string()))
}

// ── Moves ─────────────────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "wiki/moves/list.html")]
struct MovesListTemplate {
    moves: Vec<(Move, Type)>,
}

pub async fn moves_list(State(pool): State<SqlitePool>) -> impl IntoResponse {
    let moves_db = sqlx::query_as::<_, Move>(
        "SELECT id, name, type_id, category, power, accuracy, pp, description FROM moves ORDER BY name"
    ).fetch_all(&pool).await.unwrap_or_default();

    let types_db = sqlx::query_as::<_, Type>("SELECT id, name, hexcolor, icon FROM types").fetch_all(&pool).await.unwrap_or_default();

    let moves = moves_db.into_iter().map(|m| {
        let t = types_db.iter().find(|t| t.id == m.type_id).cloned().unwrap();
        (m, t)
    }).collect();

    render(MovesListTemplate { moves }).unwrap_or_else(|s| Html(s.to_string()))
}

#[derive(Template)]
#[template(path = "wiki/moves/detail.html")]
struct MoveDetailTemplate {
    m: Move,
    t: Type,
    learnset: Vec<(Pokemon, String, Option<i64>)>,
}

pub async fn move_detail(
    State(pool): State<SqlitePool>,
    Path(slug): Path<String>,
) -> impl IntoResponse {
    let all_moves = sqlx::query_as::<_, Move>(
        "SELECT id, name, type_id, category, power, accuracy, pp, description FROM moves"
    ).fetch_all(&pool).await.unwrap_or_default();

    let m = match all_moves.into_iter().find(|m| to_slug(&m.name) == slug) {
        Some(m) => m,
        None => return Html(StatusCode::NOT_FOUND.to_string()),
    };

    let all_types = sqlx::query_as::<_, Type>("SELECT id, name, hexcolor, icon FROM types").fetch_all(&pool).await.unwrap_or_default();
    
    let t = all_types.into_iter().find(|t| t.id == m.type_id).unwrap();

    let all_pokemon = sqlx::query_as::<_, Pokemon>("SELECT id, dex_number, name, sprite, sprite_shiny, type1_id, type2_id, hp, atk, def, spa, spd, spe, pokedex_entry FROM pokemon")
        .fetch_all(&pool).await.unwrap_or_default();

    let learn_rels = sqlx::query_as::<_, (i64, String, Option<i64>)>(
        "SELECT pokemon_id, method, level FROM learnset WHERE move_id = ? ORDER BY level, pokemon_id"
    ).bind(m.id).fetch_all(&pool).await.unwrap_or_default();

    let mut learnset = Vec::new();
    for (p_id, method, level) in learn_rels {
        if let Some(p) = all_pokemon.iter().find(|p| p.id == p_id).cloned() {
            learnset.push((p, method, level));
        }
    }

    render(MoveDetailTemplate { m, t, learnset })
        .unwrap_or_else(|s| Html(s.to_string()))
}

// ── Habilidades ───────────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "wiki/abilities/list.html")]
struct AbilitiesListTemplate {
    abilities: Vec<Ability>,
}

pub async fn abilities_list(State(pool): State<SqlitePool>) -> impl IntoResponse {
    let abilities = sqlx::query_as::<_, Ability>(
        "SELECT id, name, description FROM abilities ORDER BY name"
    ).fetch_all(&pool).await.unwrap_or_default();

    render(AbilitiesListTemplate { abilities }).unwrap_or_else(|s| Html(s.to_string()))
}

#[derive(Template)]
#[template(path = "wiki/abilities/detail.html")]
struct AbilityDetailTemplate {
    a: Ability,
    pokemon: Vec<Pokemon>,
}

pub async fn ability_detail(
    State(pool): State<SqlitePool>,
    Path(slug): Path<String>,
) -> impl IntoResponse {
    let all_abilities = sqlx::query_as::<_, Ability>(
        "SELECT id, name, description FROM abilities"
    ).fetch_all(&pool).await.unwrap_or_default();

    let a = match all_abilities.into_iter().find(|a| to_slug(&a.name) == slug) {
        Some(a) => a,
        None => return Html(StatusCode::NOT_FOUND.to_string()),
    };

    let pokemon = sqlx::query_as::<_, Pokemon>(
        "SELECT p.id, p.dex_number, p.name, p.sprite, p.sprite_shiny,
                p.type1_id, p.type2_id, p.hp, p.atk, p.def, p.spa, p.spd, p.spe, p.pokedex_entry
         FROM pokemon_abilities pa
         JOIN pokemon p ON p.id = pa.pokemon_id
         WHERE pa.ability_id = ?
         ORDER BY p.dex_number"
    ).bind(a.id).fetch_all(&pool).await.unwrap_or_default();

    render(AbilityDetailTemplate { a, pokemon })
        .unwrap_or_else(|s| Html(s.to_string()))
}

// ── Naturezas ─────────────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "wiki/mechanics/nature.html")]
struct NaturesListTemplate {
    natures: Vec<Nature>,
}

pub async fn natures_list(State(pool): State<SqlitePool>) -> impl IntoResponse {
    let natures = sqlx::query_as::<_, Nature>(
        "SELECT id, name, increased_stat, decreased_stat FROM natures ORDER BY name"
    ).fetch_all(&pool).await.unwrap_or_default();

    render(NaturesListTemplate { natures }).unwrap_or_else(|s| Html(s.to_string()))
}

// ── Status Conditions ─────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "wiki/mechanics/status.html")]
struct StatusListTemplate {
    conditions: Vec<StatusCondition>,
}

pub async fn status_list(State(pool): State<SqlitePool>) -> impl IntoResponse {
    let conditions = sqlx::query_as::<_, StatusCondition>(
        "SELECT id, name, description, icon, is_volatile FROM status_conditions ORDER BY is_volatile, name"
    ).fetch_all(&pool).await.unwrap_or_default();

    render(StatusListTemplate { conditions }).unwrap_or_else(|s| Html(s.to_string()))
}
