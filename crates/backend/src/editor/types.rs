use askama::Template;
use axum::{
    extract::{State, Form, Path},
    response::{IntoResponse, Redirect, Html},
};
use sqlx::SqlitePool;
use engine::models::{Type, TypeEffectiveness};
use crate::auth::EditorSession;
use crate::filters;
use serde::Deserialize;
use super::TypeFull;

#[derive(Deserialize)]
pub struct JsonEditForm {
    pub json_content: String,
}

// ── Templates ────────────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "editor/types/list.html")]
pub struct TypeListTemplate {
    pub types: Vec<Type>,
    pub user_role: Option<String>,
}

#[derive(Template)]
#[template(path = "editor/types/edit.html")]
pub struct TypeEditTemplate {
    pub slug: String,
    pub json_content: String,
    pub all_types: Vec<Type>,
    pub error: Option<String>,
    pub user_role: Option<String>,
}

// ── Handlers ─────────────────────────────────────────────────────────────────

pub async fn type_list(
    _session: EditorSession,
    State(pool): State<SqlitePool>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let types = sqlx::query_as::<_, Type>("SELECT * FROM types WHERE id > 0 ORDER BY id")
        .fetch_all(&pool).await?;
    Ok(Html(TypeListTemplate { types, user_role: Some("editor".into()) }.render()?).into_response())
}

pub async fn type_new(
    _session: EditorSession,
    State(pool): State<SqlitePool>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let max_id: i64 = sqlx::query_scalar("SELECT COALESCE(MAX(id), 0) FROM types")
        .fetch_one(&pool).await.unwrap_or(0);
    let all_types = sqlx::query_as::<_, Type>("SELECT * FROM types")
        .fetch_all(&pool).await?;
    
    let default_t = Type { id: max_id + 1, name: "Novo Tipo".to_string(), hexcolor: "#FFFFFF".to_string(), icon: "icon.png".to_string() };
    let full = TypeFull {
        types: default_t,
        effectiveness: vec![TypeEffectiveness { attacker_id: max_id + 1, defender_id: 1, multiplier: 1.0 }],
    };
    Ok(Html(TypeEditTemplate {
        slug: "new".to_string(),
        json_content: serde_json::to_string(&full).unwrap_or_default(),
        all_types,
        error: None,
        user_role: Some("editor".into()),
    }.render()?).into_response())
}

pub async fn type_create(
    EditorSession(trainer): EditorSession,
    State(pool): State<SqlitePool>,
    Form(payload): Form<JsonEditForm>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let all_types = sqlx::query_as::<_, Type>("SELECT * FROM types")
        .fetch_all(&pool).await?;

    let full: TypeFull = match serde_json::from_str(&payload.json_content) {
        Ok(t) => t,
        Err(e) => return Ok(Html(TypeEditTemplate {
            slug: "new".to_string(), json_content: payload.json_content, all_types,
            error: Some(format!("Erro no JSON: {}", e)), user_role: Some("editor".into()),
        }.render()?).into_response()),
    };
    let new_t = full.types;

    let mut tx = pool.begin().await?;

    sqlx::query("INSERT INTO types (id, name, hexcolor, icon) VALUES (?, ?, ?, ?)")
        .bind(new_t.id).bind(&new_t.name).bind(&new_t.hexcolor).bind(&new_t.icon)
        .execute(&mut *tx).await?;
    for eff in &full.effectiveness {
        sqlx::query("INSERT INTO type_effectiveness (attacker_id, defender_id, multiplier) VALUES (?, ?, ?)")
            .bind(new_t.id).bind(eff.defender_id).bind(eff.multiplier)
            .execute(&mut *tx).await?;
    }

    tx.commit().await?;

    sqlx::query("INSERT INTO edits (trainer_id, table_name, record_id, diff) VALUES (?, ?, ?, ?)")
        .bind(&trainer.id).bind("types").bind(new_t.id).bind(&format!("CREATE---{}", payload.json_content))
        .execute(&pool).await?;
        
    Ok(Redirect::to(&format!("/wiki/types/{}", filters::to_slug(&new_t.name))).into_response())
}

pub async fn type_edit(
    _session: EditorSession,
    State(pool): State<SqlitePool>,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let all_types = sqlx::query_as::<_, Type>("SELECT * FROM types")
        .fetch_all(&pool).await?;
    let t = all_types.iter().find(|t| filters::to_slug(&t.name) == slug).cloned();
    if let Some(t) = t {
        let effectiveness = sqlx::query_as::<_, TypeEffectiveness>(
            "SELECT * FROM type_effectiveness WHERE attacker_id = ?"
        ).bind(t.id).fetch_all(&pool).await?;
        let full = TypeFull { types: t, effectiveness };
        let json_content = serde_json::to_string(&full).unwrap_or_default();
        Ok(Html(TypeEditTemplate { slug, json_content, all_types, error: None, user_role: Some("editor".into()) }
            .render()?).into_response())
    } else {
        Err(crate::error::AppError::NotFound)
    }
}

pub async fn type_save(
    EditorSession(trainer): EditorSession,
    State(pool): State<SqlitePool>,
    Path(slug): Path<String>,
    Form(payload): Form<JsonEditForm>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let all_types = sqlx::query_as::<_, Type>("SELECT * FROM types")
        .fetch_all(&pool).await?;

    let full: TypeFull = match serde_json::from_str(&payload.json_content) {
        Ok(t) => t,
        Err(e) => {
            return Ok(Html(TypeEditTemplate {
                slug, json_content: payload.json_content, all_types,
                error: Some(format!("Erro no JSON: {}", e)), user_role: Some("editor".into()),
            }.render()?).into_response());
        }
    };
    let old_t = match all_types.iter().find(|t| filters::to_slug(&t.name) == slug).cloned() {
        Some(t) => t,
        None => return Err(crate::error::AppError::NotFound),
    };
    let mut new_t = full.types;
    new_t.id = old_t.id; // Evita ID Hijacking
    let full = TypeFull { types: new_t.clone(), ..full };
    let diff = format!("{}---{}", serde_json::to_string(&old_t).unwrap_or_default(), payload.json_content);

    let mut tx = pool.begin().await?;

    sqlx::query("UPDATE types SET name = ?, hexcolor = ?, icon = ? WHERE id = ?")
        .bind(&new_t.name).bind(&new_t.hexcolor).bind(&new_t.icon).bind(new_t.id)
        .execute(&mut *tx).await?;
    sqlx::query("DELETE FROM type_effectiveness WHERE attacker_id = ?").bind(new_t.id)
        .execute(&mut *tx).await?;
    for eff in &full.effectiveness {
        sqlx::query("INSERT INTO type_effectiveness (attacker_id, defender_id, multiplier) VALUES (?, ?, ?)")
            .bind(new_t.id).bind(eff.defender_id).bind(eff.multiplier)
            .execute(&mut *tx).await?;
    }

    tx.commit().await?;

    sqlx::query("INSERT INTO edits (trainer_id, table_name, record_id, diff) VALUES (?, ?, ?, ?)")
        .bind(&trainer.id).bind("types").bind(new_t.id).bind(&diff).execute(&pool).await?;
        
    Ok(Redirect::to(&format!("/wiki/types/{}", slug)).into_response())
}

pub async fn type_delete(
    _session: EditorSession,
    State(pool): State<SqlitePool>,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let all_types = sqlx::query_as::<_, Type>("SELECT * FROM types")
        .fetch_all(&pool).await?;
    let t = all_types.into_iter().find(|t| filters::to_slug(&t.name) == slug);
    if let Some(t) = t {
        if t.id == 19 {
            return Err(crate::error::AppError::Internal("Cannot delete the fallback type (Stellar)".to_string()));
        }
        let mut tx = pool.begin().await?;
        
        sqlx::query("UPDATE pokemon SET type1_id = 19 WHERE type1_id = ?").bind(t.id).execute(&mut *tx).await?;
        sqlx::query("UPDATE pokemon SET type2_id = NULL WHERE type2_id = ?").bind(t.id).execute(&mut *tx).await?;
        sqlx::query("UPDATE moves SET type_id = 19 WHERE type_id = ?").bind(t.id).execute(&mut *tx).await?;
        sqlx::query("DELETE FROM type_effectiveness WHERE attacker_id = ? OR defender_id = ?").bind(t.id).bind(t.id).execute(&mut *tx).await?;
        sqlx::query("DELETE FROM types WHERE id = ?").bind(t.id).execute(&mut *tx).await?;
        
        tx.commit().await?;
        Ok(Redirect::to("/editor/types").into_response())
    } else {
        Err(crate::error::AppError::NotFound)
    }
}
