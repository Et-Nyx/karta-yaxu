use axum::{extract::State, response::IntoResponse};
use sqlx::SqlitePool;
use askama::Template;
use engine::models::{Nature, StatusCondition};
use tower_sessions::Session;
use crate::auth;
use crate::error::render;

#[derive(Template)]
#[template(path = "wiki/mechanics/nature.html")]
struct NaturesListTemplate {
    natures: Vec<Nature>,
    user_role: Option<String>,
}

pub async fn natures_list(session: Session, State(pool): State<SqlitePool>) -> Result<impl IntoResponse, crate::error::AppError> {
    let natures = sqlx::query_as::<_, Nature>(
        "SELECT id, name, increased_stat, decreased_stat FROM natures ORDER BY name"
    ).fetch_all(&pool).await?;

    let user_role = auth::get_user_role(&session).await;
    Ok(render(NaturesListTemplate { natures, user_role }))
}

#[derive(Template)]
#[template(path = "wiki/mechanics/status.html")]
struct StatusListTemplate {
    conditions: Vec<StatusCondition>,
    user_role: Option<String>,
}

pub async fn status_list(session: Session, State(pool): State<SqlitePool>) -> Result<impl IntoResponse, crate::error::AppError> {
    let conditions = sqlx::query_as::<_, StatusCondition>(
        "SELECT id, name, description, icon, is_volatile FROM status_conditions ORDER BY is_volatile, name"
    ).fetch_all(&pool).await?;

    let user_role = auth::get_user_role(&session).await;
    Ok(render(StatusListTemplate { conditions, user_role }))
}
