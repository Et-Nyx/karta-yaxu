use axum::{extract::{Path, State}, response::IntoResponse};
use sqlx::SqlitePool;
use askama::Template;
use engine::models::{Pokemon, Type, Move};
use tower_sessions::Session;
use crate::auth;
use crate::filters;
use crate::filters::to_slug;
use crate::error::render;

#[derive(Template)]
#[template(path = "wiki/moves/list.html")]
struct MovesListTemplate {
    moves: Vec<(Move, Type)>,
    user_role: Option<String>,
}

pub async fn moves_list(session: Session, State(pool): State<SqlitePool>) -> Result<impl IntoResponse, crate::error::AppError> {
    let moves_db = sqlx::query_as::<_, Move>(
        "SELECT id, name, type_id, category, power, accuracy, pp, description FROM moves ORDER BY name"
    ).fetch_all(&pool).await?;

    let types_db = sqlx::query_as::<_, Type>("SELECT id, name, hexcolor, icon FROM types").fetch_all(&pool).await?;

    let moves = moves_db.into_iter().filter_map(|m| {
        let t = types_db.iter().find(|t| t.id == m.type_id)?.clone();
        Some((m, t))
    }).collect();

    let user_role = auth::get_user_role(&session).await;
    Ok(render(MovesListTemplate { moves, user_role }))
}

#[derive(Template)]
#[template(path = "wiki/moves/detail.html")]
struct MoveDetailTemplate {
    m: Move,
    t: Type,
    learnset: Vec<((Pokemon, Type, Option<Type>), String, Option<i64>)>,
    user_role: Option<String>,
}

pub async fn move_detail(
    session: Session,
    State(pool): State<SqlitePool>,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let all_moves = sqlx::query_as::<_, Move>(
        "SELECT id, name, type_id, category, power, accuracy, pp, description FROM moves"
    ).fetch_all(&pool).await?;

    let m = match all_moves.into_iter().find(|m| to_slug(&m.name) == slug) {
        Some(m) => m,
        None => return Err(crate::error::AppError::NotFound),
    };

    let all_types = sqlx::query_as::<_, Type>("SELECT id, name, hexcolor, icon FROM types").fetch_all(&pool).await?;

    let t = match all_types.iter().find(|t| t.id == m.type_id).cloned() {
        Some(t) => t,
        None => return Err(crate::error::AppError::NotFound),
    };

    let all_pokemon = sqlx::query_as::<_, Pokemon>("SELECT id, dex_number, name, sprite, sprite_shiny, type1_id, type2_id, hp, atk, def, spa, spd, spe, pokedex_entry FROM pokemon")
        .fetch_all(&pool).await?;

    let learn_rels = sqlx::query_as::<_, (i64, String, Option<i64>)>(
        "SELECT pokemon_id, method, level FROM learnset WHERE move_id = ? ORDER BY level, pokemon_id"
    ).bind(m.id).fetch_all(&pool).await?;

    let mut learnset = Vec::new();
    for (p_id, method, level) in learn_rels {
        if let Some(p) = all_pokemon.iter().find(|p| p.id == p_id).cloned() {
            let t1 = match all_types.iter().find(|t| t.id == p.type1_id) {
                Some(t) => t.clone(),
                None => continue,
            };
            let t2 = p.type2_id.and_then(|id| all_types.iter().find(|t| t.id == id).cloned());
            learnset.push(((p, t1, t2), method, level));
        }
    }

    let user_role = auth::get_user_role(&session).await;
    Ok(render(MoveDetailTemplate { m, t, learnset, user_role }))
}
