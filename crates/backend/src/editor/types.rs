use askama::Template;
use axum::{
    extract::{State, Form, Path},
    response::{IntoResponse, Redirect, Html},
    http::StatusCode,
};
use sqlx::SqlitePool;
use engine::models::{Type, TypeEffectiveness};
use crate::auth::EditorSession;
use crate::filters;
use super::{EditForm, TypeFull};

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
    pub toml_content: String,
    pub error: Option<String>,
    pub user_role: Option<String>,
}

// ── Handlers ─────────────────────────────────────────────────────────────────

pub async fn type_list(
    _session: EditorSession,
    State(pool): State<SqlitePool>,
) -> impl IntoResponse {
    let types = sqlx::query_as::<_, Type>("SELECT * FROM types WHERE id > 0 ORDER BY id")
        .fetch_all(&pool).await.unwrap_or_default();
    Html(TypeListTemplate { types, user_role: Some("editor".into()) }.render().unwrap_or_default())
}

pub async fn type_new(
    _session: EditorSession,
    State(pool): State<SqlitePool>,
) -> impl IntoResponse {
    let max_id: i64 = sqlx::query_scalar("SELECT COALESCE(MAX(id), 0) FROM types")
        .fetch_one(&pool).await.unwrap_or(0);
    let default_t = Type { id: max_id + 1, name: "Novo Tipo".to_string(), hexcolor: "#FFFFFF".to_string(), icon: "icon.png".to_string() };
    let full = TypeFull {
        types: default_t,
        effectiveness: vec![TypeEffectiveness { attacker_id: max_id + 1, defender_id: 1, multiplier: 1.0 }],
    };
    Html(TypeEditTemplate {
        slug: "new".to_string(),
        toml_content: toml::to_string(&full).unwrap_or_default(),
        error: None,
        user_role: Some("editor".into()),
    }.render().unwrap_or_default())
}

pub async fn type_create(
    EditorSession(trainer): EditorSession,
    State(pool): State<SqlitePool>,
    Form(payload): Form<EditForm>,
) -> impl IntoResponse {
    let full: TypeFull = match toml::from_str(&payload.toml_content) {
        Ok(t) => t,
        Err(e) => return Html(TypeEditTemplate {
            slug: "new".to_string(), toml_content: payload.toml_content,
            error: Some(format!("Erro no TOML: {}", e)), user_role: Some("editor".into()),
        }.render().unwrap_or_default()).into_response(),
    };
    let new_t = full.types;

    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Erro: {}", e)).into_response(),
    };

    let res = async {
        sqlx::query("INSERT INTO types (id, name, hexcolor, icon) VALUES (?, ?, ?, ?)")
            .bind(new_t.id).bind(&new_t.name).bind(&new_t.hexcolor).bind(&new_t.icon)
            .execute(&mut *tx).await?;
        for eff in &full.effectiveness {
            sqlx::query("INSERT INTO type_effectiveness (attacker_id, defender_id, multiplier) VALUES (?, ?, ?)")
                .bind(new_t.id).bind(eff.defender_id).bind(eff.multiplier)
                .execute(&mut *tx).await?;
        }
        Ok::<_, sqlx::Error>(())
    }.await;

    let result = match res {
        Ok(_) => tx.commit().await,
        Err(e) => { let _ = tx.rollback().await; Err(e) }
    };

    match result {
        Ok(_) => {
            let _ = sqlx::query("INSERT INTO edits (trainer_id, table_name, record_id, diff) VALUES (?, ?, ?, ?)")
                .bind(&trainer.id).bind("types").bind(new_t.id).bind(&format!("CREATE---{}", payload.toml_content))
                .execute(&pool).await;
            Redirect::to(&format!("/wiki/types/{}", filters::to_slug(&new_t.name))).into_response()
        }
        Err(e) => Html(TypeEditTemplate {
            slug: "new".to_string(), toml_content: payload.toml_content,
            error: Some(format!("Erro no banco de dados: {}", e)), user_role: Some("editor".into()),
        }.render().unwrap_or_default()).into_response(),
    }
}

pub async fn type_edit(
    _session: EditorSession,
    State(pool): State<SqlitePool>,
    Path(slug): Path<String>,
) -> impl IntoResponse {
    let t = sqlx::query_as::<_, Type>("SELECT * FROM types WHERE name LIKE ?")
        .bind(&slug).fetch_optional(&pool).await.unwrap_or(None);
    if let Some(t) = t {
        let effectiveness = sqlx::query_as::<_, TypeEffectiveness>(
            "SELECT * FROM type_effectiveness WHERE attacker_id = ?"
        ).bind(t.id).fetch_all(&pool).await.unwrap_or_default();
        let full = TypeFull { types: t, effectiveness };
        let toml_content = toml::to_string(&full).unwrap_or_default();
        Html(TypeEditTemplate { slug, toml_content, error: None, user_role: Some("editor".into()) }
            .render().unwrap_or_default()).into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

pub async fn type_save(
    EditorSession(trainer): EditorSession,
    State(pool): State<SqlitePool>,
    Path(slug): Path<String>,
    Form(payload): Form<EditForm>,
) -> impl IntoResponse {
    let full: TypeFull = match toml::from_str(&payload.toml_content) {
        Ok(t) => t,
        Err(e) => {
            return Html(TypeEditTemplate {
                slug, toml_content: payload.toml_content,
                error: Some(format!("Erro no TOML: {}", e)), user_role: Some("editor".into()),
            }.render().unwrap_or_default()).into_response();
        }
    };
    let old_t = match sqlx::query_as::<_, Type>("SELECT * FROM types WHERE name LIKE ?")
        .bind(&slug).fetch_optional(&pool).await
    {
        Ok(Some(t)) => t,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Erro: {}", e)).into_response(),
    };
    let mut new_t = full.types;
    new_t.id = old_t.id; // Evita ID Hijacking
    let full = TypeFull { types: new_t.clone(), ..full };
    let diff = format!("{}---{}", toml::to_string(&old_t).unwrap_or_default(), payload.toml_content);

    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Erro: {}", e)).into_response(),
    };

    let res = async {
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
        Ok::<_, sqlx::Error>(())
    }.await;

    let result = match res {
        Ok(_) => tx.commit().await,
        Err(e) => { let _ = tx.rollback().await; Err(e) }
    };

    match result {
        Ok(_) => {
            let _ = sqlx::query("INSERT INTO edits (trainer_id, table_name, record_id, diff) VALUES (?, ?, ?, ?)")
                .bind(&trainer.id).bind("types").bind(new_t.id).bind(&diff).execute(&pool).await;
            Redirect::to(&format!("/wiki/types/{}", slug)).into_response()
        }
        Err(e) => Html(TypeEditTemplate {
            slug, toml_content: payload.toml_content,
            error: Some(format!("Erro no banco de dados: {}", e)), user_role: Some("editor".into()),
        }.render().unwrap_or_default()).into_response(),
    }
}

pub async fn type_delete(
    _session: EditorSession,
    State(pool): State<SqlitePool>,
    Path(slug): Path<String>,
) -> impl IntoResponse {
    let t = sqlx::query_as::<_, Type>("SELECT * FROM types WHERE name LIKE ?")
        .bind(&slug).fetch_optional(&pool).await.unwrap_or(None);
    if let Some(t) = t {
        if t.id == 0 {
            return (StatusCode::BAD_REQUEST, "Cannot delete the fallback type").into_response();
        }
        let mut tx = match pool.begin().await {
            Ok(tx) => tx,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Erro: {}", e)).into_response(),
        };
        let res = async {
            sqlx::query("UPDATE pokemon SET type1_id = 0 WHERE type1_id = ?").bind(t.id).execute(&mut *tx).await?;
            sqlx::query("UPDATE pokemon SET type2_id = NULL WHERE type2_id = ?").bind(t.id).execute(&mut *tx).await?;
            sqlx::query("UPDATE moves SET type_id = 0 WHERE type_id = ?").bind(t.id).execute(&mut *tx).await?;
            sqlx::query("DELETE FROM type_effectiveness WHERE attacker_id = ? OR defender_id = ?").bind(t.id).bind(t.id).execute(&mut *tx).await?;
            sqlx::query("DELETE FROM types WHERE id = ?").bind(t.id).execute(&mut *tx).await?;
            Ok::<_, sqlx::Error>(())
        }.await;
        match res {
            Ok(_) => match tx.commit().await {
                Ok(_) => Redirect::to("/editor/types").into_response(),
                Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Erro: {}", e)).into_response(),
            },
            Err(e) => { let _ = tx.rollback().await; (StatusCode::INTERNAL_SERVER_ERROR, format!("Erro ao deletar: {}", e)).into_response() }
        }
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}
