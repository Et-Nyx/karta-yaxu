use askama::Template;
use axum::{
    extract::{State, Form, Path},
    response::{IntoResponse, Redirect, Html},
    http::StatusCode,
};
use sqlx::SqlitePool;
use engine::models::Nature;
use crate::auth::EditorSession;
use crate::filters;
use super::EditForm;

// ── Templates ────────────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "editor/natures/list.html")]
pub struct NatureListTemplate {
    pub natures: Vec<Nature>,
    pub user_role: Option<String>,
}

#[derive(Template)]
#[template(path = "editor/natures/edit.html")]
pub struct NatureEditTemplate {
    pub slug: String,
    pub toml_content: String,
    pub error: Option<String>,
    pub user_role: Option<String>,
}

// ── Handlers ─────────────────────────────────────────────────────────────────

pub async fn nature_list(
    _session: EditorSession,
    State(pool): State<SqlitePool>,
) -> impl IntoResponse {
    let natures = sqlx::query_as::<_, Nature>("SELECT * FROM natures WHERE id > 0 ORDER BY name")
        .fetch_all(&pool).await.unwrap_or_default();
    Html(NatureListTemplate { natures, user_role: Some("editor".into()) }.render().unwrap_or_default())
}

pub async fn nature_new(
    _session: EditorSession,
    State(pool): State<SqlitePool>,
) -> impl IntoResponse {
    let max_id: i64 = sqlx::query_scalar("SELECT COALESCE(MAX(id), 0) FROM natures")
        .fetch_one(&pool).await.unwrap_or(0);
    let default_n = Nature { id: max_id + 1, name: "Nova Natureza".to_string(), increased_stat: None, decreased_stat: None };
    Html(NatureEditTemplate {
        slug: "new".to_string(),
        toml_content: toml::to_string(&default_n).unwrap_or_default(),
        error: None, user_role: Some("editor".into()),
    }.render().unwrap_or_default())
}

pub async fn nature_create(
    EditorSession(trainer): EditorSession,
    State(pool): State<SqlitePool>,
    Form(payload): Form<EditForm>,
) -> impl IntoResponse {
    let new_n: Nature = match toml::from_str(&payload.toml_content) {
        Ok(n) => n,
        Err(e) => return Html(NatureEditTemplate {
            slug: "new".to_string(), toml_content: payload.toml_content,
            error: Some(format!("Erro no TOML: {}", e)), user_role: Some("editor".into()),
        }.render().unwrap_or_default()).into_response(),
    };
    let result = sqlx::query("INSERT INTO natures (id, name, increased_stat, decreased_stat) VALUES (?, ?, ?, ?)")
        .bind(new_n.id).bind(&new_n.name).bind(&new_n.increased_stat).bind(&new_n.decreased_stat)
        .execute(&pool).await;
    match result {
        Ok(_) => {
            let _ = sqlx::query("INSERT INTO edits (trainer_id, table_name, record_id, diff) VALUES (?, ?, ?, ?)")
                .bind(&trainer.id).bind("natures").bind(new_n.id).bind(&format!("CREATE---{}", payload.toml_content))
                .execute(&pool).await;
            Redirect::to("/wiki/mechanics/nature").into_response()
        }
        Err(e) => Html(NatureEditTemplate {
            slug: "new".to_string(), toml_content: payload.toml_content,
            error: Some(format!("Erro no banco de dados: {}", e)), user_role: Some("editor".into()),
        }.render().unwrap_or_default()).into_response(),
    }
}

pub async fn nature_edit(
    _session: EditorSession,
    State(pool): State<SqlitePool>,
    Path(slug): Path<String>,
) -> impl IntoResponse {
    let n = sqlx::query_as::<_, Nature>("SELECT * FROM natures WHERE name LIKE ?")
        .bind(&slug).fetch_optional(&pool).await.unwrap_or(None);
    if let Some(n) = n {
        let toml_content = toml::to_string(&n).unwrap_or_default();
        Html(NatureEditTemplate { slug, toml_content, error: None, user_role: Some("editor".into()) }
            .render().unwrap_or_default()).into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

pub async fn nature_save(
    EditorSession(trainer): EditorSession,
    State(pool): State<SqlitePool>,
    Path(slug): Path<String>,
    Form(payload): Form<EditForm>,
) -> impl IntoResponse {
    let new_n: Nature = match toml::from_str(&payload.toml_content) {
        Ok(n) => n,
        Err(e) => {
            return Html(NatureEditTemplate {
                slug, toml_content: payload.toml_content,
                error: Some(format!("Erro no TOML: {}", e)), user_role: Some("editor".into()),
            }.render().unwrap_or_default()).into_response();
        }
    };
    let old_n = match sqlx::query_as::<_, Nature>("SELECT * FROM natures WHERE name LIKE ?")
        .bind(&slug).fetch_optional(&pool).await
    {
        Ok(Some(n)) => n,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Erro: {}", e)).into_response(),
    };
    let mut new_n = new_n;
    new_n.id = old_n.id; // Evita ID Hijacking
    let diff = format!("{}---{}", toml::to_string(&old_n).unwrap_or_default(), payload.toml_content);
    let result = sqlx::query("UPDATE natures SET name = ?, increased_stat = ?, decreased_stat = ? WHERE id = ?")
        .bind(&new_n.name).bind(&new_n.increased_stat).bind(&new_n.decreased_stat).bind(new_n.id)
        .execute(&pool).await;
    match result {
        Ok(_) => {
            let _ = sqlx::query("INSERT INTO edits (trainer_id, table_name, record_id, diff) VALUES (?, ?, ?, ?)")
                .bind(&trainer.id).bind("natures").bind(new_n.id).bind(&diff).execute(&pool).await;
            Redirect::to("/wiki/mechanics/nature").into_response()
        }
        Err(e) => Html(NatureEditTemplate {
            slug, toml_content: payload.toml_content,
            error: Some(format!("Erro no banco de dados: {}", e)), user_role: Some("editor".into()),
        }.render().unwrap_or_default()).into_response(),
    }
}

pub async fn nature_delete(
    _session: EditorSession,
    State(pool): State<SqlitePool>,
    Path(slug): Path<String>,
) -> impl IntoResponse {
    let n = sqlx::query_as::<_, Nature>("SELECT * FROM natures WHERE name LIKE ?")
        .bind(&slug).fetch_optional(&pool).await.unwrap_or(None);
    if let Some(n) = n {
        if n.id == 0 {
            return (StatusCode::BAD_REQUEST, "Cannot delete the fallback nature").into_response();
        }
        let mut tx = match pool.begin().await {
            Ok(tx) => tx,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Erro: {}", e)).into_response(),
        };
        let res = async {
            sqlx::query("UPDATE team_members SET nature_id = 0 WHERE nature_id = ?").bind(n.id).execute(&mut *tx).await?;
            sqlx::query("DELETE FROM natures WHERE id = ?").bind(n.id).execute(&mut *tx).await?;
            Ok::<_, sqlx::Error>(())
        }.await;
        match res {
            Ok(_) => match tx.commit().await {
                Ok(_) => Redirect::to("/editor/natures").into_response(),
                Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Erro: {}", e)).into_response(),
            },
            Err(e) => { let _ = tx.rollback().await; (StatusCode::INTERNAL_SERVER_ERROR, format!("Erro ao deletar: {}", e)).into_response() }
        }
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}
