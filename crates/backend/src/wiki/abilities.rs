use axum::{extract::{Path, State}, response::IntoResponse};
use sqlx::SqlitePool;
use askama::Template;
use engine::models::{Pokemon, Type, Ability};
use tower_sessions::Session;
use crate::auth;
use crate::filters;
use crate::filters::to_slug;
use crate::error::render;

#[derive(Template)]
#[template(path = "wiki/abilities/list.html")]
struct AbilitiesListTemplate {
    abilities: Vec<Ability>,
    user_role: Option<String>,
}

pub async fn abilities_list(session: Session, State(pool): State<SqlitePool>) -> Result<impl IntoResponse, crate::error::AppError> {
    let abilities = sqlx::query_as::<_, Ability>(
        "SELECT id, name, description FROM abilities ORDER BY name"
    ).fetch_all(&pool).await?;

    let user_role = auth::get_user_role(&session).await;
    Ok(render(AbilitiesListTemplate { abilities, user_role }))
}

#[derive(Template)]
#[template(path = "wiki/abilities/detail.html")]
struct AbilityDetailTemplate {
    a: Ability,
    pokemon: Vec<(Pokemon, Type, Option<Type>)>,
    user_role: Option<String>,
}

pub async fn ability_detail(
    session: Session,
    State(pool): State<SqlitePool>,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let all_abilities = sqlx::query_as::<_, Ability>(
        "SELECT id, name, description FROM abilities"
    ).fetch_all(&pool).await?;

    let a = match all_abilities.into_iter().find(|a| to_slug(&a.name) == slug) {
        Some(a) => a,
        None => return Err(crate::error::AppError::NotFound),
    };

    let pokemon = sqlx::query_as::<_, Pokemon>(
        "SELECT p.id, p.dex_number, p.name, p.sprite, p.sprite_shiny,
                p.type1_id, p.type2_id, p.hp, p.atk, p.def, p.spa, p.spd, p.spe, p.pokedex_entry
         FROM pokemon_abilities pa
         JOIN pokemon p ON p.id = pa.pokemon_id
         WHERE pa.ability_id = ?
         ORDER BY p.dex_number"
    ).bind(a.id).fetch_all(&pool).await?;

    let types = sqlx::query_as::<_, Type>("SELECT id, name, hexcolor, icon FROM types").fetch_all(&pool).await?;

    let mut mapped_pokemon = Vec::new();
    for p in pokemon {
        let t1 = match types.iter().find(|t| t.id == p.type1_id) {
            Some(t) => t.clone(),
            None => continue,
        };
        let t2 = p.type2_id.and_then(|id| types.iter().find(|t| t.id == id).cloned());
        mapped_pokemon.push((p, t1, t2));
    }

    let user_role = auth::get_user_role(&session).await;
    Ok(render(AbilityDetailTemplate { a, pokemon: mapped_pokemon, user_role }))
}
