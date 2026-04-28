use axum::{extract::{Path, State}, response::IntoResponse};
use sqlx::SqlitePool;
use askama::Template;
use engine::models::{Pokemon, Type, Ability, Move};
use tower_sessions::Session;
use crate::auth;
use crate::filters;
use crate::filters::to_slug;
use crate::error::render;

#[derive(Template)]
#[template(path = "wiki/pokemon/list.html")]
struct PokemonListTemplate {
    pokemon: std::vec::Vec<(Pokemon, Type, std::option::Option<Type>)>,
    user_role: Option<String>,
}

pub async fn pokemon_list(session: Session, State(pool): State<SqlitePool>) -> Result<impl IntoResponse, crate::error::AppError> {
    let pokemon = sqlx::query_as::<_, Pokemon>(
        "SELECT id, dex_number, name, sprite, sprite_shiny,
                type1_id, type2_id, hp, atk, def, spa, spd, spe, pokedex_entry
         FROM pokemon ORDER BY dex_number, id"
    ).fetch_all(&pool).await?;

    let all_types = sqlx::query_as::<_, Type>("SELECT * FROM types").fetch_all(&pool).await?;
    
    let mut mapped_pokemon = Vec::new();
    for p in pokemon {
        let t1 = match all_types.iter().find(|t| t.id == p.type1_id) {
            Some(t) => t.clone(),
            None => continue,
        };
        let t2 = p.type2_id.and_then(|id| all_types.iter().find(|t| t.id == id).cloned());
        mapped_pokemon.push((p, t1, t2));
    }

    let user_role = auth::get_user_role(&session).await;
    Ok(render(PokemonListTemplate { pokemon: mapped_pokemon, user_role }))
}

#[derive(Template)]
#[template(path = "wiki/pokemon/detail.html")]
struct PokemonDetailTemplate {
    p: Pokemon,
    type1: Type,
    type2: Option<Type>,
    abilities: Vec<Ability>,
    evolutions: Vec<(Pokemon, Type, Option<Type>)>,
    forms: Vec<(Pokemon, Type, Option<Type>)>,
    learnset: Vec<(Move, Type, String, Option<i64>)>,
    weaknesses: Vec<(Type, f64)>,
    resistances: Vec<(Type, f64)>,
    immunities: Vec<Type>,
    user_role: Option<String>,
}

pub async fn pokemon_detail(
    session: Session,
    State(pool): State<SqlitePool>,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let all = sqlx::query_as::<_, Pokemon>(
        "SELECT id, dex_number, name, sprite, sprite_shiny,
                type1_id, type2_id, hp, atk, def, spa, spd, spe, pokedex_entry
         FROM pokemon"
    ).fetch_all(&pool).await?;

    let p = match all.iter().find(|p| to_slug(&p.name) == slug).cloned() {
        Some(p) => p,
        None => return Err(crate::error::AppError::NotFound),
    };

    let types = sqlx::query_as::<_, Type>(
        "SELECT id, name, hexcolor, icon FROM types"
    ).fetch_all(&pool).await?;

    let type1 = types.iter().find(|t| t.id == p.type1_id).cloned()
        .unwrap_or(Type { id: 0, name: "???".into(), hexcolor: "#999".into(), icon: "".into() });
    let type2 = p.type2_id.and_then(|id| types.iter().find(|t| t.id == id).cloned());

    let abilities = sqlx::query_as::<_, Ability>(
        "SELECT a.id, a.name, a.description
         FROM pokemon_abilities pa
         JOIN abilities a ON a.id = pa.ability_id
         WHERE pa.pokemon_id = ? ORDER BY pa.slot"
    ).bind(p.id).fetch_all(&pool).await?;

    let eff_relations = sqlx::query_as::<_, (i64, i64, f64)>(
        "SELECT attacker_id, defender_id, multiplier FROM type_effectiveness"
    ).fetch_all(&pool).await?;

    let mut weaknesses = Vec::new();
    let mut resistances = Vec::new();
    let mut immunities = Vec::new();

    for atk_t in &types {
        let mut mult = 1.0;
        if let Some((_, _, m)) = eff_relations.iter().find(|(a, d, _)| *a == atk_t.id && *d == type1.id) {
            mult *= m;
        }
        if let Some(t2) = &type2 {
            if let Some((_, _, m)) = eff_relations.iter().find(|(a, d, _)| *a == atk_t.id && *d == t2.id) {
                mult *= m;
            }
        }
        if mult > 1.0 { weaknesses.push((atk_t.clone(), mult)); }
        else if mult == 0.0 { immunities.push(atk_t.clone()); }
        else if mult < 1.0 { resistances.push((atk_t.clone(), mult)); }
    }

    let all_forms = sqlx::query_as::<_, (i64, i64)>("SELECT base_id, form_id FROM pokemon_forms").fetch_all(&pool).await?;

    let true_base_id = all_forms.iter()
        .find(|&&(_, form)| form == p.id)
        .map(|&(base, _)| base)
        .unwrap_or(p.id);

    let all_evos = sqlx::query_as::<_, (i64, i64)>("SELECT from_id, to_id FROM evolutions").fetch_all(&pool).await?;

    let has_own_evos = all_evos.iter().any(|&(from, to)| from == p.id || to == p.id);
    let evo_base_id = if has_own_evos { p.id } else { true_base_id };

    let mut root_id = evo_base_id;
    loop {
        if let Some((from_id, _)) = all_evos.iter().find(|&&(_, to)| to == root_id) {
            root_id = *from_id;
        } else {
            break;
        }
    }

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

    let mut evolutions: Vec<(Pokemon, Type, Option<Type>)> = evo_ids.into_iter()
        .filter_map(|id| all.iter().find(|all_p| all_p.id == id).cloned())
        .filter_map(|e_p| {
            let t1 = types.iter().find(|t| t.id == e_p.type1_id)?.clone();
            let t2 = e_p.type2_id.and_then(|id| types.iter().find(|t| t.id == id).cloned());
            Some((e_p, t1, t2))
        })
        .collect();
    evolutions.sort_by_key(|(e, _, _)| e.dex_number);

    let mut forms_ids = vec![true_base_id];
    for &(base, form) in &all_forms {
        if base == true_base_id { forms_ids.push(form); }
    }
    forms_ids.retain(|&id| id != p.id); 

    let mut forms: Vec<(Pokemon, Type, Option<Type>)> = forms_ids.into_iter()
        .filter_map(|id| all.iter().find(|all_p| all_p.id == id).cloned())
        .filter_map(|f_p| {
            let t1 = types.iter().find(|t| t.id == f_p.type1_id)?.clone();
            let t2 = f_p.type2_id.and_then(|id| types.iter().find(|t| t.id == id).cloned());
            Some((f_p, t1, t2))
        })
        .collect();
    forms.sort_by_key(|(f, _, _)| f.id);

    let all_moves = sqlx::query_as::<_, Move>("SELECT * FROM moves").fetch_all(&pool).await?;

    let mut learn_rels = sqlx::query_as::<_, (i64, String, Option<i64>)>(
        "SELECT move_id, method, level FROM learnset WHERE pokemon_id = ? ORDER BY level, move_id"
    ).bind(p.id).fetch_all(&pool).await?;

    if learn_rels.is_empty() && p.id != true_base_id {
        learn_rels = sqlx::query_as::<_, (i64, String, Option<i64>)>(
            "SELECT move_id, method, level FROM learnset WHERE pokemon_id = ? ORDER BY level, move_id"
        ).bind(true_base_id).fetch_all(&pool).await?;
    }

    let mut learnset = Vec::new();
    for (m_id, method, level) in learn_rels {
        if let Some(m) = all_moves.iter().find(|m| m.id == m_id) {
            let t = types.iter().find(|t| t.id == m.type_id).cloned().unwrap_or(type1.clone());
            learnset.push((m.clone(), t, method, level));
        }
    }

    let user_role = auth::get_user_role(&session).await;
    Ok(render(PokemonDetailTemplate { 
        p, type1, type2, abilities, evolutions, forms, learnset, weaknesses, resistances, immunities, user_role 
    }))
}
