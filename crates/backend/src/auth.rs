use askama::Template;
use axum::{
    extract::{State, Form, FromRequestParts},
    response::{Html, IntoResponse, Redirect},
    http::{StatusCode, request::Parts},
    routing::{get, post},
    Router,
};
use serde::Deserialize;
use sqlx::SqlitePool;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use uuid::Uuid;
use tower_sessions::Session;
use crate::models::{Trainer, TrainerRole};
use crate::error::render;

// Extrator que lê a sessão e retorna o trainer atual (ou redireciona)
pub struct AuthSession(pub Trainer);

impl<S> FromRequestParts<S> for AuthSession
where
    State<SqlitePool>: FromRequestParts<S>,
    S: Send + Sync,
{
    type Rejection = Redirect;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let session = Session::from_request_parts(parts, state)
            .await
            .map_err(|_| Redirect::to("/trainer/login"))?;

        let trainer_id: Option<String> = session.get("trainer_id").await.unwrap_or(None);

        if let Some(id) = trainer_id {
            let State(pool) = State::<SqlitePool>::from_request_parts(parts, state)
                .await
                .map_err(|_| Redirect::to("/trainer/login"))?;

            let record = sqlx::query_as::<_, Trainer>(
                "SELECT * FROM trainers WHERE id = ?"
            )
            .bind(id)
            .fetch_optional(&pool)
            .await
            .unwrap_or(None);

            if let Some(trainer) = record {
                return Ok(AuthSession(trainer));
            }
        }

        Err(Redirect::to("/trainer/login"))
    }
}

#[derive(Template)]
#[template(path = "home.html")]
pub struct HomeTemplate {
    pub error: Option<String>,
    pub user_role: Option<String>,
}

#[derive(Deserialize)]
pub struct HomeParams {
    pub error: Option<String>,
}

pub async fn home_handler(
    session: Session,
    axum::extract::Query(params): axum::extract::Query<HomeParams>,
) -> impl IntoResponse {
    let msg = params.error.and_then(|e| match e.as_str() {
        "unauthorized" => Some("Você precisa estar logado para acessar o editor.".to_string()),
        "forbidden" => Some("Acesso negado: Você não tem permissão de Editor.".to_string()),
        _ => None,
    });

    let user_role = get_user_role(&session).await;

    Html(HomeTemplate { error: msg, user_role }.render().unwrap())
}

// ... (Resto do código até EditorSession) ...

// Extrator com proteção por role
pub struct EditorSession(pub Trainer);

impl<S> FromRequestParts<S> for EditorSession
where
    State<SqlitePool>: FromRequestParts<S>,
    S: Send + Sync,
{
    type Rejection = Redirect;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // Tenta obter AuthSession, se falhar redireciona para login (via AuthSession)
        // Mas o usuário pediu para redirecionar para Home com erro.
        // Vamos fazer a lógica manual aqui para maior controle.

        let session = Session::from_request_parts(parts, state)
            .await
            .map_err(|_| Redirect::to("/?error=unauthorized"))?;

        let trainer_id: Option<String> = session.get("trainer_id").await.unwrap_or(None);

        if let Some(id) = trainer_id {
            let State(pool) = State::<SqlitePool>::from_request_parts(parts, state)
                .await
                .map_err(|_| Redirect::to("/?error=unauthorized"))?;

            let record = sqlx::query_as::<_, Trainer>(
                "SELECT * FROM trainers WHERE id = ?"
            )
            .bind(id)
            .fetch_optional(&pool)
            .await
            .unwrap_or(None);

            if let Some(trainer) = record {
                if trainer.role == TrainerRole::Editor {
                    return Ok(EditorSession(trainer));
                } else {
                    return Err(Redirect::to("/?error=forbidden"));
                }
            }
        }

        Err(Redirect::to("/?error=unauthorized"))
    }
}

#[derive(Template)]
#[template(path = "auth/login.html")]
pub struct LoginTemplate {
    pub error: Option<String>,
    pub user_role: Option<String>,
}

#[derive(Template)]
#[template(path = "auth/register.html")]
pub struct RegisterTemplate {
    pub error: Option<String>,
    pub user_role: Option<String>,
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

pub async fn get_user_role(session: &Session) -> Option<String> {
    session.get::<String>("trainer_role").await.unwrap_or(None)
}



pub async fn login_page(session: Session) -> impl IntoResponse {
    if session.get::<String>("trainer_id").await.unwrap_or(None).is_some() {
        return Redirect::to("/trainer/dashboard").into_response();
    }
    let user_role = get_user_role(&session).await;
    render(LoginTemplate { error: None, user_role }).into_response()
}

pub async fn register_page(session: Session) -> impl IntoResponse {
    if session.get::<String>("trainer_id").await.unwrap_or(None).is_some() {
        return Redirect::to("/trainer/dashboard").into_response();
    }
    let user_role = get_user_role(&session).await;
    render(RegisterTemplate { error: None, user_role }).into_response()
}

pub async fn register_post(
    State(pool): State<SqlitePool>,
    session: Session,
    Form(payload): Form<RegisterForm>,
) -> impl IntoResponse {
    let user_role = get_user_role(&session).await;
    if payload.password != payload.confirm_password {
        return render(RegisterTemplate {
            error: Some("As senhas não coincidem.".to_string()),
            user_role,
        }).into_response();
    }

    if payload.password.len() < 6 {
        return render(RegisterTemplate {
            error: Some("A senha deve ter pelo menos 6 caracteres.".to_string()),
            user_role,
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
                user_role,
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
            let parsed_hash = match argon2::PasswordHash::new(&trainer.password_hash) {
                Ok(h) => h,
                Err(_) => {
                    eprintln!("[ERROR] Hash malformado para trainer {:?}", trainer.id);
                    let user_role = get_user_role(&session).await;
                    return render(LoginTemplate {
                        error: Some("Erro interno de autenticação.".to_string()),
                        user_role,
                    }).into_response();
                }
            };

            let argon2 = Argon2::default();

            if argon2.verify_password(payload.password.as_bytes(), &parsed_hash).is_ok() {
                session.insert("trainer_id", trainer.id).await.unwrap();
                session.insert("trainer_role", trainer.role.to_string()).await.unwrap();

                return Redirect::to("/trainer/dashboard").into_response()
            }
        }
        _ => {}
    }

    let user_role = get_user_role(&session).await;
    render(LoginTemplate {
        error: Some("Credenciais inválidas.".to_string()),
        user_role,
    }).into_response()
}

#[derive(Template)]
#[template(path = "auth/dashboard.html")]
pub struct DashboardTemplate {
    pub username: String,
    pub role: String,
    pub logged_in: bool,
    pub user_role: Option<String>,
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
            let user_role = get_user_role(&session).await;
            return render(DashboardTemplate {
                username: trainer.username,
                role: trainer.role,
                logged_in: get_logged_in(&session).await,
                user_role,
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

pub fn routes() -> Router<SqlitePool> {
    Router::new()
        .route("/login", get(login_page).post(login_post))
        .route("/register", get(register_page).post(register_post))
        .route("/dashboard", get(dashboard))
        .route("/logout", post(logout))
}
