use askama::Template;
use axum::{
    response::{Html, IntoResponse},
    http::{StatusCode, header::{CONTENT_TYPE, CONTENT_DISPOSITION}},
};
use tokio::fs;
use crate::auth::EditorSession;
use crate::error::render;

#[derive(Template)]
#[template(path = "editor/index.html")]
struct EditorIndexTemplate {
    user_role: Option<String>,
}

pub async fn index(
    _editor: EditorSession,
    session: tower_sessions::Session,
) -> Result<Html<String>, StatusCode> {
    let user_role = crate::auth::get_user_role(&session).await;
    render(EditorIndexTemplate { user_role }).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub async fn export_db(
    _editor: EditorSession,
) -> impl IntoResponse {
    let db_path = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:./data/rom.db".to_string());
    let db_path = db_path.strip_prefix("sqlite:").unwrap_or(&db_path);

    match fs::read(db_path).await {
        Ok(bytes) => (
            [
                (CONTENT_TYPE, "application/vnd.sqlite3"),
                (CONTENT_DISPOSITION, "attachment; filename=\"karta_yaxu_export.db\""),
            ],
            bytes,
        ).into_response(),
        Err(e) => {
            tracing::error!("Falha ao ler db para exportar: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Erro ao ler banco de dados",
            ).into_response()
        }
    }
}
