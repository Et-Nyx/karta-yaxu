use askama::Template;
use axum::{
    response::Html,
    http::StatusCode,
};
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
