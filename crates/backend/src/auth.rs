use askama::Template;
use axum::{
    extract::{State, Form},
    response::{Html, IntoResponse, Redirect},
    http::StatusCode,
};
use serde::Deserialize;
use sqlx::SqlitePool;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use uuid::Uuid;
use tower_sessions::Session;

#[derive(Template)]
#[template(path = "auth/login.html")]
pub struct LoginTemplate {
    pub error: Option<String>,
}

#[derive(Template)]
#[template(path = "auth/register.html")]
pub struct RegisterTemplate {
    pub error: Option<String>,
}

#[derive(Deserialize)]
pub struct LoginForm {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct RegisterForm {
    pub username: String,
    pub password: String,
    pub confirm_password: String,
}

pub async fn get_logged_in(session: &Session) -> bool {
    session.get::<String>("trainer_id").await.unwrap_or(None).is_some()
}

fn render(t: impl Template) -> Result<Html<String>, StatusCode> {
    let html = t.render().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Html(html))
}

pub async fn login_page(session: Session) -> impl IntoResponse {
    if session.get::<String>("trainer_id").await.unwrap_or(None).is_some() {
        return Redirect::to("/trainer/dashboard").into_response();
    }
    render(LoginTemplate { error: None }).into_response()
}

pub async fn register_page(session: Session) -> impl IntoResponse {
    if session.get::<String>("trainer_id").await.unwrap_or(None).is_some() {
        return Redirect::to("/trainer/dashboard").into_response();
    }
    render(RegisterTemplate { error: None }).into_response()
}

pub async fn register_post(
    State(pool): State<SqlitePool>,
    Form(payload): Form<RegisterForm>,
) -> impl IntoResponse {
    if payload.password != payload.confirm_password {
        return render(RegisterTemplate {
            error: Some("As senhas não coincidem.".to_string()),
        }).into_response();
    }

    if payload.password.len() < 6 {
        return render(RegisterTemplate {
            error: Some("A senha deve ter pelo menos 6 caracteres.".to_string()),
        }).into_response();
    }

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    let password_hash = match argon2.hash_password(payload.password.as_bytes(), &salt) {
        Ok(hash) => hash.to_string(),
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    let new_id = Uuid::new_v4().to_string();

    let result = sqlx::query!(
        "INSERT INTO trainers (id, username, password_hash) VALUES (?, ?, ?)",
        new_id,
        payload.username,
        password_hash,
    )
    .execute(&pool)
    .await;

    match result {
        Ok(_) => {
            Redirect::to("/trainer/login").into_response()
        }
        Err(sqlx::Error::Database(err)) if err.is_unique_violation() => {
            render(RegisterTemplate {
                error: Some("Nome de usuário já existe.".to_string()),
            }).into_response()
        }
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

pub async fn login_post(
    State(pool): State<SqlitePool>,
    session: Session,
    Form(payload): Form<LoginForm>,
) -> impl IntoResponse {
    let record = sqlx::query!(
        r#"
        SELECT id, password_hash, role
        FROM trainers
        WHERE username = ?
        "#,
        payload.username
    )
    .fetch_optional(&pool)
    .await;

    match record {
        Ok(Some(trainer)) => {
            let parsed_hash = argon2::PasswordHash::new(&trainer.password_hash)
                .expect("Hash está malformado");

            let argon2 = Argon2::default();

            if argon2.verify_password(payload.password.as_bytes(), &parsed_hash).is_ok() {
                session.insert("trainer_id", trainer.id).await.unwrap();
                session.insert("trainer_role", trainer.role).await.unwrap();

                return Redirect::to("/trainer/dashboard").into_response()
            }
        }
        _ => {}
    }

    render(LoginTemplate {
        error: Some("Credenciais inválidas.".to_string()),
    }).into_response()
}

#[derive(Template)]
#[template(path = "auth/dashboard.html")]
pub struct DashboardTemplate {
    pub username: String,
    pub role: String,
    pub logged_in: bool,
}

// ...
pub async fn dashboard(
    State(pool): State<SqlitePool>,
    session: Session,
) -> impl IntoResponse {
    let trainer_id: Option<String> = session.get("trainer_id").await.unwrap_or(None);

    if let Some(id) = trainer_id {
        let record = sqlx::query!(
            "SELECT username, role FROM trainers WHERE id = ?",
            id
        )
        .fetch_optional(&pool)
        .await
        .unwrap_or(None);

        if let Some(trainer) = record {
            return render(DashboardTemplate {
                username: trainer.username,
                role: trainer.role,
                logged_in: get_logged_in(&session).await,
            }).into_response();
        }
    }

    Redirect::to("/trainer/login").into_response()
}

pub async fn logout(
    session: Session,
) -> impl IntoResponse {
    session.delete().await.unwrap();
    Redirect::to("/trainer/login").into_response()
}
