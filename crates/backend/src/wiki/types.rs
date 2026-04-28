use axum::{extract::{Path, State}, response::IntoResponse};
use sqlx::SqlitePool;
use askama::Template;
use engine::models::Type;
use tower_sessions::Session;
use crate::auth;
use crate::filters;
use crate::filters::to_slug;
use crate::error::render;

#[derive(Template)]
#[template(path = "wiki/types/list.html")]
struct TypesListTemplate {
    types: Vec<Type>,
    user_role: Option<String>,
}

pub async fn types_list(session: Session, State(pool): State<SqlitePool>) -> Result<impl IntoResponse, crate::error::AppError> {
    let types = sqlx::query_as::<_, Type>(
        "SELECT id, name, hexcolor, icon FROM types WHERE id > 0 ORDER BY id"
    ).fetch_all(&pool).await?;

    let user_role = auth::get_user_role(&session).await;
    Ok(render(TypesListTemplate { types, user_role }))
}

#[derive(Template)]
#[template(path = "wiki/types/detail.html")]
struct TypeDetailTemplate {
    t: Type,
    as_attacker: Vec<(Type, f64)>, // (Tipo Defensor, multiplicador)
    as_defender: Vec<(Type, f64)>, // (Tipo Atacante, multiplicador)
    user_role: Option<String>,
}

pub async fn type_detail(
    session: Session,
    State(pool): State<SqlitePool>,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let all_types = sqlx::query_as::<_, Type>("SELECT id, name, hexcolor, icon FROM types").fetch_all(&pool).await?;

    let t = match all_types.iter().find(|t| to_slug(&t.name) == slug).cloned() {
        Some(t) => t,
        None => return Err(crate::error::AppError::NotFound),
    };

    let eff_relations = sqlx::query_as::<_, (i64, i64, f64)>(
        "SELECT attacker_id, defender_id, multiplier FROM type_effectiveness"
    ).fetch_all(&pool).await?;

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

    as_attacker.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal).then_with(|| a.0.name.cmp(&b.0.name)));
    as_defender.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal).then_with(|| a.0.name.cmp(&b.0.name)));

    let user_role = auth::get_user_role(&session).await;
    Ok(render(TypeDetailTemplate { t, as_attacker, as_defender, user_role }))
}
