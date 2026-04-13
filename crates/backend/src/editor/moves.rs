use askama::Template;
use axum::{
    extract::{State, Form, Path},
    response::{IntoResponse, Redirect, Html},
    http::StatusCode,
};
use sqlx::SqlitePool;
use engine::models::{Move, Type};
use crate::auth::EditorSession;
use crate::filters;
use super::EditForm;

// ── Templates ────────────────────────────────────────────────────────────────

pub struct MoveRow {
    pub name: String,
    pub type_id: i64,
    pub type_name: String,
    pub type_color: String,
    pub category: engine::models::MoveCategory,
    pub power: Option<i64>,
    pub accuracy: Option<i64>,
    pub pp: i64,
}

#[derive(Template)]
#[template(path = "editor/moves/list.html")]
pub struct MoveListTemplate {
    pub moves: Vec<MoveRow>,
    pub user_role: Option<String>,
}

#[derive(Template)]
#[template(path = "editor/moves/edit.html")]
pub struct MoveEditTemplate {
    pub slug: String,
    pub toml_content: String,
    pub error: Option<String>,
    pub user_role: Option<String>,
}

// ── Handlers ─────────────────────────────────────────────────────────────────

pub async fn move_list(
    _session: EditorSession,
    State(pool): State<SqlitePool>,
) -> impl IntoResponse {
    let raw_moves = sqlx::query_as::<_, Move>("SELECT * FROM moves ORDER BY name")
        .fetch_all(&pool).await.unwrap_or_default();
    let types = sqlx::query_as::<_, Type>("SELECT * FROM types")
        .fetch_all(&pool).await.unwrap_or_default();

    let moves: Vec<MoveRow> = raw_moves.into_iter().map(|m| {
        let (tname, tcolor) = types.iter()
            .find(|t| t.id == m.type_id)
            .map(|t| (t.name.clone(), t.hexcolor.clone()))
            .unwrap_or(("???".into(), "#888".into()));
        MoveRow {
            name: m.name, type_id: m.type_id, type_name: tname, type_color: tcolor,
            category: m.category, power: m.power, accuracy: m.accuracy, pp: m.pp,
        }
    }).collect();

    Html(MoveListTemplate { moves, user_role: Some("editor".into()) }.render().unwrap_or_default())
}

pub async fn move_new(
    _session: EditorSession,
    State(pool): State<SqlitePool>,
) -> impl IntoResponse {
    let max_id: i64 = sqlx::query_scalar("SELECT COALESCE(MAX(id), 0) FROM moves")
        .fetch_one(&pool).await.unwrap_or(0);
    let default_m = Move {
        id: max_id + 1, name: "Novo Golpe".to_string(), type_id: 1,
        category: engine::models::MoveCategory::Physical,
        power: Some(40), accuracy: Some(100), pp: 20,
        description: "Descrição do golpe".to_string(),
    };
    Html(MoveEditTemplate {
        slug: "new".to_string(),
        toml_content: toml::to_string(&default_m).unwrap_or_default(),
        error: None,
        user_role: Some("editor".into()),
    }.render().unwrap_or_default())
}

pub async fn move_create(
    EditorSession(trainer): EditorSession,
    State(pool): State<SqlitePool>,
    Form(payload): Form<EditForm>,
) -> impl IntoResponse {
    let new_m: Move = match toml::from_str(&payload.toml_content) {
        Ok(m) => m,
        Err(e) => return Html(MoveEditTemplate {
            slug: "new".to_string(), toml_content: payload.toml_content,
            error: Some(format!("Erro no TOML: {}", e)), user_role: Some("editor".into()),
        }.render().unwrap_or_default()).into_response(),
    };
    let result = sqlx::query("INSERT INTO moves (id, name, type_id, category, power, accuracy, pp, description) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(new_m.id).bind(&new_m.name).bind(new_m.type_id).bind(&new_m.category)
        .bind(new_m.power).bind(new_m.accuracy).bind(new_m.pp).bind(&new_m.description)
        .execute(&pool).await;
    match result {
        Ok(_) => {
            let _ = sqlx::query("INSERT INTO edits (trainer_id, table_name, record_id, diff) VALUES (?, ?, ?, ?)")
                .bind(&trainer.id).bind("moves").bind(new_m.id).bind(&format!("CREATE---{}", payload.toml_content))
                .execute(&pool).await;
            Redirect::to(&format!("/wiki/moves/{}", filters::to_slug(&new_m.name))).into_response()
        }
        Err(e) => Html(MoveEditTemplate {
            slug: "new".to_string(), toml_content: payload.toml_content,
            error: Some(format!("Erro no banco de dados: {}", e)), user_role: Some("editor".into()),
        }.render().unwrap_or_default()).into_response(),
    }
}

pub async fn move_edit(
    _session: EditorSession,
    State(pool): State<SqlitePool>,
    Path(slug): Path<String>,
) -> impl IntoResponse {
    let all_moves = sqlx::query_as::<_, Move>("SELECT * FROM moves")
        .fetch_all(&pool).await.unwrap_or_default();

    let m = all_moves.into_iter().find(|m| filters::to_slug(&m.name) == slug);

    if let Some(m) = m {
        let toml_content = toml::to_string(&m).unwrap_or_default();
        Html(MoveEditTemplate { slug, toml_content, error: None, user_role: Some("editor".into()) }
            .render().unwrap_or_default()).into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

pub async fn move_save(
    EditorSession(trainer): EditorSession,
    State(pool): State<SqlitePool>,
    Path(slug): Path<String>,
    Form(payload): Form<EditForm>,
) -> impl IntoResponse {
    let new_m: Move = match toml::from_str(&payload.toml_content) {
        Ok(m) => m,
        Err(e) => {
            return Html(MoveEditTemplate {
                slug, toml_content: payload.toml_content,
                error: Some(format!("Erro no TOML: {}", e)), user_role: Some("editor".into()),
            }.render().unwrap_or_default()).into_response();
        }
    };

    let all_moves = sqlx::query_as::<_, Move>("SELECT * FROM moves")
        .fetch_all(&pool).await.unwrap_or_default();
    let old_m = match all_moves.into_iter().find(|m| filters::to_slug(&m.name) == slug) {
        Some(m) => m,
        None => return StatusCode::NOT_FOUND.into_response(),
    };
    let mut new_m = new_m;
    new_m.id = old_m.id; // Evita ID Hijacking
    let diff = format!("{}---{}", toml::to_string(&old_m).unwrap_or_default(), payload.toml_content);

    let result = sqlx::query("UPDATE moves SET name = ?, type_id = ?, category = ?, power = ?, accuracy = ?, pp = ?, description = ? WHERE id = ?")
        .bind(&new_m.name).bind(new_m.type_id).bind(&new_m.category)
        .bind(new_m.power).bind(new_m.accuracy).bind(new_m.pp).bind(&new_m.description)
        .bind(new_m.id).execute(&pool).await;

    match result {
        Ok(_) => {
            let _ = sqlx::query("INSERT INTO edits (trainer_id, table_name, record_id, diff) VALUES (?, ?, ?, ?)")
                .bind(&trainer.id).bind("moves").bind(new_m.id).bind(&diff).execute(&pool).await;
            Redirect::to(&format!("/wiki/moves/{}", slug)).into_response()
        }
        Err(e) => Html(MoveEditTemplate {
            slug, toml_content: payload.toml_content,
            error: Some(format!("Erro no banco de dados: {}", e)), user_role: Some("editor".into()),
        }.render().unwrap_or_default()).into_response(),
    }
}

pub async fn move_delete(
    _session: EditorSession,
    State(pool): State<SqlitePool>,
    Path(slug): Path<String>,
) -> impl IntoResponse {
    let all_moves = sqlx::query_as::<_, Move>("SELECT * FROM moves")
        .fetch_all(&pool).await.unwrap_or_default();
    let m = all_moves.into_iter().find(|m| filters::to_slug(&m.name) == slug);
    if let Some(m) = m {
        let mut tx = match pool.begin().await {
            Ok(tx) => tx,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Erro: {}", e)).into_response(),
        };
        let res = async {
            sqlx::query("DELETE FROM learnset WHERE move_id = ?").bind(m.id).execute(&mut *tx).await?;
            sqlx::query("UPDATE team_members SET move1_id = NULL WHERE move1_id = ?").bind(m.id).execute(&mut *tx).await?;
            sqlx::query("UPDATE team_members SET move2_id = NULL WHERE move2_id = ?").bind(m.id).execute(&mut *tx).await?;
            sqlx::query("UPDATE team_members SET move3_id = NULL WHERE move3_id = ?").bind(m.id).execute(&mut *tx).await?;
            sqlx::query("UPDATE team_members SET move4_id = NULL WHERE move4_id = ?").bind(m.id).execute(&mut *tx).await?;
            sqlx::query("DELETE FROM moves WHERE id = ?").bind(m.id).execute(&mut *tx).await?;
            Ok::<_, sqlx::Error>(())
        }.await;
        match res {
            Ok(_) => match tx.commit().await {
                Ok(_) => Redirect::to("/editor/moves").into_response(),
                Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Erro: {}", e)).into_response(),
            },
            Err(e) => { let _ = tx.rollback().await; (StatusCode::INTERNAL_SERVER_ERROR, format!("Erro: {}", e)).into_response() }
        }
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}
