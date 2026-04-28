use askama::Template;
use axum::{
    extract::{State, Form, Path},
    response::{IntoResponse, Redirect, Html},
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
) -> Result<impl IntoResponse, crate::error::AppError> {
    let natures = sqlx::query_as::<_, Nature>("SELECT * FROM natures WHERE id > 0 ORDER BY name")
        .fetch_all(&pool).await?;
    Ok(Html(NatureListTemplate { natures, user_role: Some("editor".into()) }.render()?).into_response())
}

pub async fn nature_new(
    _session: EditorSession,
    State(pool): State<SqlitePool>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let max_id: i64 = sqlx::query_scalar("SELECT COALESCE(MAX(id), 0) FROM natures")
        .fetch_one(&pool).await.unwrap_or(0);
    let default_n = Nature { id: max_id + 1, name: "Nova Natureza".to_string(), increased_stat: None, decreased_stat: None };
    Ok(Html(NatureEditTemplate {
        slug: "new".to_string(),
        toml_content: toml::to_string(&default_n).unwrap_or_default(),
        error: None, user_role: Some("editor".into()),
    }.render()?).into_response())
}

pub async fn nature_create(
    EditorSession(trainer): EditorSession,
    State(pool): State<SqlitePool>,
    Form(payload): Form<EditForm>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let new_n: Nature = match toml::from_str(&payload.toml_content) {
        Ok(n) => n,
        Err(e) => return Ok(Html(NatureEditTemplate {
            slug: "new".to_string(), toml_content: payload.toml_content,
            error: Some(format!("Erro no TOML: {}", e)), user_role: Some("editor".into()),
        }.render()?).into_response()),
    };
    
    sqlx::query("INSERT INTO natures (id, name, increased_stat, decreased_stat) VALUES (?, ?, ?, ?)")
        .bind(new_n.id).bind(&new_n.name).bind(&new_n.increased_stat).bind(&new_n.decreased_stat)
        .execute(&pool).await?;
        
    sqlx::query("INSERT INTO edits (trainer_id, table_name, record_id, diff) VALUES (?, ?, ?, ?)")
        .bind(&trainer.id).bind("natures").bind(new_n.id).bind(&format!("CREATE---{}", payload.toml_content))
        .execute(&pool).await?;
        
    Ok(Redirect::to("/wiki/mechanics/nature").into_response())
}

pub async fn nature_edit(
    _session: EditorSession,
    State(pool): State<SqlitePool>,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let all_natures = sqlx::query_as::<_, Nature>("SELECT * FROM natures")
        .fetch_all(&pool).await?;
    let n = all_natures.into_iter().find(|n| filters::to_slug(&n.name) == slug);
    if let Some(n) = n {
        let toml_content = toml::to_string(&n).unwrap_or_default();
        Ok(Html(NatureEditTemplate { slug, toml_content, error: None, user_role: Some("editor".into()) }
            .render()?).into_response())
    } else {
        Err(crate::error::AppError::NotFound)
    }
}

pub async fn nature_save(
    EditorSession(trainer): EditorSession,
    State(pool): State<SqlitePool>,
    Path(slug): Path<String>,
    Form(payload): Form<EditForm>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let new_n: Nature = match toml::from_str(&payload.toml_content) {
        Ok(n) => n,
        Err(e) => {
            return Ok(Html(NatureEditTemplate {
                slug, toml_content: payload.toml_content,
                error: Some(format!("Erro no TOML: {}", e)), user_role: Some("editor".into()),
            }.render()?).into_response());
        }
    };
    let all_natures = sqlx::query_as::<_, Nature>("SELECT * FROM natures")
        .fetch_all(&pool).await?;
    let old_n = match all_natures.into_iter().find(|n| filters::to_slug(&n.name) == slug) {
        Some(n) => n,
        None => return Err(crate::error::AppError::NotFound),
    };
    let mut new_n = new_n;
    new_n.id = old_n.id; // Evita ID Hijacking
    let diff = format!("{}---{}", toml::to_string(&old_n).unwrap_or_default(), payload.toml_content);
    
    sqlx::query("UPDATE natures SET name = ?, increased_stat = ?, decreased_stat = ? WHERE id = ?")
        .bind(&new_n.name).bind(&new_n.increased_stat).bind(&new_n.decreased_stat).bind(new_n.id)
        .execute(&pool).await?;
        
    sqlx::query("INSERT INTO edits (trainer_id, table_name, record_id, diff) VALUES (?, ?, ?, ?)")
        .bind(&trainer.id).bind("natures").bind(new_n.id).bind(&diff).execute(&pool).await?;
        
    Ok(Redirect::to("/wiki/mechanics/nature").into_response())
}

pub async fn nature_delete(
    _session: EditorSession,
    State(pool): State<SqlitePool>,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let all_natures = sqlx::query_as::<_, Nature>("SELECT * FROM natures")
        .fetch_all(&pool).await?;
    let n = all_natures.into_iter().find(|n| filters::to_slug(&n.name) == slug);
    if let Some(n) = n {
        if n.id == 0 {
            return Err(crate::error::AppError::Internal("Cannot delete the fallback nature".to_string()));
        }
        let mut tx = pool.begin().await?;
        
        sqlx::query("UPDATE team_members SET nature_id = 0 WHERE nature_id = ?").bind(n.id).execute(&mut *tx).await?;
        sqlx::query("DELETE FROM natures WHERE id = ?").bind(n.id).execute(&mut *tx).await?;
        
        tx.commit().await?;
        Ok(Redirect::to("/editor/natures").into_response())
    } else {
        Err(crate::error::AppError::NotFound)
    }
}
