use askama::Template;
use axum::{
    extract::{State, Form, Path},
    response::{IntoResponse, Redirect, Html},
    http::StatusCode,
};
use sqlx::SqlitePool;
use engine::models::Ability;
use crate::auth::EditorSession;
use crate::filters;
use super::EditForm;

// ── Templates ────────────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "editor/abilities/list.html")]
pub struct AbilityListTemplate {
    pub abilities: Vec<Ability>,
    pub user_role: Option<String>,
}

#[derive(Template)]
#[template(path = "editor/abilities/edit.html")]
pub struct AbilityEditTemplate {
    pub slug: String,
    pub toml_content: String,
    pub error: Option<String>,
    pub user_role: Option<String>,
}

// ── Handlers ─────────────────────────────────────────────────────────────────

pub async fn ability_list(
    _session: EditorSession,
    State(pool): State<SqlitePool>,
) -> impl IntoResponse {
    let abilities = sqlx::query_as::<_, Ability>("SELECT * FROM abilities ORDER BY name")
        .fetch_all(&pool).await.unwrap_or_default();
    Html(AbilityListTemplate { abilities, user_role: Some("editor".into()) }.render().unwrap_or_default())
}

pub async fn ability_new(
    _session: EditorSession,
    State(pool): State<SqlitePool>,
) -> impl IntoResponse {
    let max_id: i64 = sqlx::query_scalar("SELECT COALESCE(MAX(id), 0) FROM abilities")
        .fetch_one(&pool).await.unwrap_or(0);
    let default_a = Ability { id: max_id + 1, name: "Nova Habilidade".to_string(), description: "Descrição da habilidade".to_string() };
    Html(AbilityEditTemplate {
        slug: "new".to_string(),
        toml_content: toml::to_string(&default_a).unwrap_or_default(),
        error: None, user_role: Some("editor".into()),
    }.render().unwrap_or_default())
}

pub async fn ability_create(
    EditorSession(trainer): EditorSession,
    State(pool): State<SqlitePool>,
    Form(payload): Form<EditForm>,
) -> impl IntoResponse {
    let new_a: Ability = match toml::from_str(&payload.toml_content) {
        Ok(a) => a,
        Err(e) => return Html(AbilityEditTemplate {
            slug: "new".to_string(), toml_content: payload.toml_content,
            error: Some(format!("Erro no TOML: {}", e)), user_role: Some("editor".into()),
        }.render().unwrap_or_default()).into_response(),
    };
    let result = sqlx::query("INSERT INTO abilities (id, name, description) VALUES (?, ?, ?)")
        .bind(new_a.id).bind(&new_a.name).bind(&new_a.description).execute(&pool).await;
    match result {
        Ok(_) => {
            let _ = sqlx::query("INSERT INTO edits (trainer_id, table_name, record_id, diff) VALUES (?, ?, ?, ?)")
                .bind(&trainer.id).bind("abilities").bind(new_a.id).bind(&format!("CREATE---{}", payload.toml_content))
                .execute(&pool).await;
            Redirect::to(&format!("/wiki/abilities/{}", filters::to_slug(&new_a.name))).into_response()
        }
        Err(e) => Html(AbilityEditTemplate {
            slug: "new".to_string(), toml_content: payload.toml_content,
            error: Some(format!("Erro no banco de dados: {}", e)), user_role: Some("editor".into()),
        }.render().unwrap_or_default()).into_response(),
    }
}

pub async fn ability_edit(
    _session: EditorSession,
    State(pool): State<SqlitePool>,
    Path(slug): Path<String>,
) -> impl IntoResponse {
    let all_abilities = sqlx::query_as::<_, Ability>("SELECT * FROM abilities")
        .fetch_all(&pool).await.unwrap_or_default();
    let a = all_abilities.into_iter().find(|a| filters::to_slug(&a.name) == slug);
    if let Some(a) = a {
        let toml_content = toml::to_string(&a).unwrap_or_default();
        Html(AbilityEditTemplate { slug, toml_content, error: None, user_role: Some("editor".into()) }
            .render().unwrap_or_default()).into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

pub async fn ability_save(
    EditorSession(trainer): EditorSession,
    State(pool): State<SqlitePool>,
    Path(slug): Path<String>,
    Form(payload): Form<EditForm>,
) -> impl IntoResponse {
    let new_a: Ability = match toml::from_str(&payload.toml_content) {
        Ok(a) => a,
        Err(e) => {
            return Html(AbilityEditTemplate {
                slug, toml_content: payload.toml_content,
                error: Some(format!("Erro no TOML: {}", e)), user_role: Some("editor".into()),
            }.render().unwrap_or_default()).into_response();
        }
    };
    let all_abilities = sqlx::query_as::<_, Ability>("SELECT * FROM abilities")
        .fetch_all(&pool).await.unwrap_or_default();
    let old_a = match all_abilities.into_iter().find(|a| filters::to_slug(&a.name) == slug) {
        Some(a) => a,
        None => return StatusCode::NOT_FOUND.into_response(),
    };
    let mut new_a = new_a;
    new_a.id = old_a.id; // Evita ID Hijacking
    let diff = format!("{}---{}", toml::to_string(&old_a).unwrap_or_default(), payload.toml_content);
    let result = sqlx::query("UPDATE abilities SET name = ?, description = ? WHERE id = ?")
        .bind(&new_a.name).bind(&new_a.description).bind(new_a.id).execute(&pool).await;
    match result {
        Ok(_) => {
            let _ = sqlx::query("INSERT INTO edits (trainer_id, table_name, record_id, diff) VALUES (?, ?, ?, ?)")
                .bind(&trainer.id).bind("abilities").bind(new_a.id).bind(&diff).execute(&pool).await;
            Redirect::to(&format!("/wiki/abilities/{}", slug)).into_response()
        }
        Err(e) => Html(AbilityEditTemplate {
            slug, toml_content: payload.toml_content,
            error: Some(format!("Erro no banco de dados: {}", e)), user_role: Some("editor".into()),
        }.render().unwrap_or_default()).into_response(),
    }
}

pub async fn ability_delete(
    _session: EditorSession,
    State(pool): State<SqlitePool>,
    Path(slug): Path<String>,
) -> impl IntoResponse {
    let all_abilities = sqlx::query_as::<_, Ability>("SELECT * FROM abilities")
        .fetch_all(&pool).await.unwrap_or_default();
    let a = all_abilities.into_iter().find(|a| filters::to_slug(&a.name) == slug);
    if let Some(a) = a {
        if a.id == 0 {
            return (StatusCode::BAD_REQUEST, "Cannot delete the fallback ability").into_response();
        }
        let mut tx = match pool.begin().await {
            Ok(tx) => tx,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Erro: {}", e)).into_response(),
        };
        let res = async {
            sqlx::query("UPDATE pokemon_abilities SET ability_id = 0 WHERE ability_id = ?").bind(a.id).execute(&mut *tx).await?;
            sqlx::query("UPDATE team_members SET ability_id = 0 WHERE ability_id = ?").bind(a.id).execute(&mut *tx).await?;
            sqlx::query("DELETE FROM abilities WHERE id = ?").bind(a.id).execute(&mut *tx).await?;
            Ok::<_, sqlx::Error>(())
        }.await;
        match res {
            Ok(_) => match tx.commit().await {
                Ok(_) => Redirect::to("/editor/abilities").into_response(),
                Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Erro: {}", e)).into_response(),
            },
            Err(e) => { let _ = tx.rollback().await; (StatusCode::INTERNAL_SERVER_ERROR, format!("Erro ao deletar: {}", e)).into_response() }
        }
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}
