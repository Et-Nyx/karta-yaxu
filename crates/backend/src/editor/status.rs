use askama::Template;
use axum::{
    extract::{State, Form, Path},
    response::{IntoResponse, Redirect, Html},
    http::StatusCode,
};
use sqlx::SqlitePool;
use engine::models::StatusCondition;
use crate::auth::EditorSession;
use crate::filters;
use super::EditForm;

// ── Templates ────────────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "editor/status/list.html")]
pub struct StatusListTemplate {
    pub status_list: Vec<StatusCondition>,
    pub user_role: Option<String>,
}

#[derive(Template)]
#[template(path = "editor/status/edit.html")]
pub struct StatusEditTemplate {
    pub slug: String,
    pub toml_content: String,
    pub error: Option<String>,
    pub user_role: Option<String>,
}

// ── Handlers ─────────────────────────────────────────────────────────────────

pub async fn status_list(
    _session: EditorSession,
    State(pool): State<SqlitePool>,
) -> impl IntoResponse {
    let status_list = sqlx::query_as::<_, StatusCondition>("SELECT * FROM status_conditions ORDER BY id")
        .fetch_all(&pool).await.unwrap_or_default();
    Html(StatusListTemplate { status_list, user_role: Some("editor".into()) }.render().unwrap_or_default())
}

pub async fn status_new(
    _session: EditorSession,
    State(pool): State<SqlitePool>,
) -> impl IntoResponse {
    let max_id: i64 = sqlx::query_scalar("SELECT COALESCE(MAX(id), 0) FROM status_conditions")
        .fetch_one(&pool).await.unwrap_or(0);
    let default_s = StatusCondition {
        id: max_id + 1, name: "Novo Status".to_string(),
        description: "Descrição do status".to_string(),
        icon: "icon.png".to_string(), is_volatile: false,
    };
    Html(StatusEditTemplate {
        slug: "new".to_string(),
        toml_content: toml::to_string(&default_s).unwrap_or_default(),
        error: None, user_role: Some("editor".into()),
    }.render().unwrap_or_default())
}

pub async fn status_create(
    EditorSession(trainer): EditorSession,
    State(pool): State<SqlitePool>,
    Form(payload): Form<EditForm>,
) -> impl IntoResponse {
    let new_s: StatusCondition = match toml::from_str(&payload.toml_content) {
        Ok(s) => s,
        Err(e) => return Html(StatusEditTemplate {
            slug: "new".to_string(), toml_content: payload.toml_content,
            error: Some(format!("Erro no TOML: {}", e)), user_role: Some("editor".into()),
        }.render().unwrap_or_default()).into_response(),
    };
    let result = sqlx::query("INSERT INTO status_conditions (id, name, description, icon, is_volatile) VALUES (?, ?, ?, ?, ?)")
        .bind(new_s.id).bind(&new_s.name).bind(&new_s.description).bind(&new_s.icon).bind(new_s.is_volatile)
        .execute(&pool).await;
    match result {
        Ok(_) => {
            let _ = sqlx::query("INSERT INTO edits (trainer_id, table_name, record_id, diff) VALUES (?, ?, ?, ?)")
                .bind(&trainer.id).bind("status_conditions").bind(new_s.id).bind(&format!("CREATE---{}", payload.toml_content))
                .execute(&pool).await;
            Redirect::to("/wiki/mechanics/status").into_response()
        }
        Err(e) => Html(StatusEditTemplate {
            slug: "new".to_string(), toml_content: payload.toml_content,
            error: Some(format!("Erro no banco de dados: {}", e)), user_role: Some("editor".into()),
        }.render().unwrap_or_default()).into_response(),
    }
}

pub async fn status_edit(
    _session: EditorSession,
    State(pool): State<SqlitePool>,
    Path(slug): Path<String>,
) -> impl IntoResponse {
    let s = sqlx::query_as::<_, StatusCondition>("SELECT * FROM status_conditions WHERE name LIKE ?")
        .bind(&slug).fetch_optional(&pool).await.unwrap_or(None);
    if let Some(s) = s {
        let toml_content = toml::to_string(&s).unwrap_or_default();
        Html(StatusEditTemplate { slug, toml_content, error: None, user_role: Some("editor".into()) }
            .render().unwrap_or_default()).into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

pub async fn status_save(
    EditorSession(trainer): EditorSession,
    State(pool): State<SqlitePool>,
    Path(slug): Path<String>,
    Form(payload): Form<EditForm>,
) -> impl IntoResponse {
    let new_s: StatusCondition = match toml::from_str(&payload.toml_content) {
        Ok(s) => s,
        Err(e) => {
            return Html(StatusEditTemplate {
                slug, toml_content: payload.toml_content,
                error: Some(format!("Erro no TOML: {}", e)), user_role: Some("editor".into()),
            }.render().unwrap_or_default()).into_response();
        }
    };
    let old_s = match sqlx::query_as::<_, StatusCondition>("SELECT * FROM status_conditions WHERE name LIKE ?")
        .bind(&slug).fetch_optional(&pool).await
    {
        Ok(Some(s)) => s,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Erro: {}", e)).into_response(),
    };
    let mut new_s = new_s;
    new_s.id = old_s.id; // Evita ID Hijacking
    let diff = format!("{}---{}", toml::to_string(&old_s).unwrap_or_default(), payload.toml_content);
    let result = sqlx::query("UPDATE status_conditions SET name = ?, description = ?, icon = ?, is_volatile = ? WHERE id = ?")
        .bind(&new_s.name).bind(&new_s.description).bind(&new_s.icon).bind(new_s.is_volatile).bind(new_s.id)
        .execute(&pool).await;
    match result {
        Ok(_) => {
            let _ = sqlx::query("INSERT INTO edits (trainer_id, table_name, record_id, diff) VALUES (?, ?, ?, ?)")
                .bind(&trainer.id).bind("status_conditions").bind(new_s.id).bind(&diff).execute(&pool).await;
            Redirect::to("/wiki/mechanics/status").into_response()
        }
        Err(e) => Html(StatusEditTemplate {
            slug, toml_content: payload.toml_content,
            error: Some(format!("Erro no banco de dados: {}", e)), user_role: Some("editor".into()),
        }.render().unwrap_or_default()).into_response(),
    }
}

pub async fn status_delete(
    _session: EditorSession,
    State(pool): State<SqlitePool>,
    Path(slug): Path<String>,
) -> impl IntoResponse {
    let s = sqlx::query_as::<_, StatusCondition>("SELECT * FROM status_conditions WHERE name LIKE ?")
        .bind(&slug).fetch_optional(&pool).await.unwrap_or(None);
    if let Some(s) = s {
        let result = sqlx::query("DELETE FROM status_conditions WHERE id = ?").bind(s.id).execute(&pool).await;
        match result {
            Ok(_) => Redirect::to("/editor/status").into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Erro ao deletar: {}", e)).into_response(),
        }
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}
