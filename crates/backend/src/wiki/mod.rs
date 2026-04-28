use axum::{routing::get, Router};
use sqlx::SqlitePool;
use axum::response::IntoResponse;
use tower_sessions::Session;
use askama::Template;

use crate::auth;
use crate::error::render;

pub mod pokemon;
pub mod types;
pub mod moves;
pub mod abilities;
pub mod mechanics;

#[derive(Template)]
#[template(path = "wiki/index.html")]
struct WikiIndexTemplate {
    user_role: Option<String>,
}

pub async fn index(session: Session) -> Result<impl IntoResponse, crate::error::AppError> {
    let user_role = auth::get_user_role(&session).await;
    Ok(render(WikiIndexTemplate { user_role }))
}

pub fn routes() -> Router<SqlitePool> {
    Router::new()
        .route("/", get(index))
        .route("/pokemon", get(pokemon::pokemon_list))
        .route("/pokemon/{slug}", get(pokemon::pokemon_detail))
        .route("/types", get(types::types_list))
        .route("/types/{slug}", get(types::type_detail))
        .route("/moves", get(moves::moves_list))
        .route("/moves/{slug}", get(moves::move_detail))
        .route("/abilities", get(abilities::abilities_list))
        .route("/abilities/{slug}", get(abilities::ability_detail))
        .route("/mechanics/nature", get(mechanics::natures_list))
        .route("/mechanics/status", get(mechanics::status_list))
}
