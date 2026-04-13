use askama::Template;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};

/// Unified error type for all backend handlers.
/// Implements `IntoResponse` so handlers can use `?` freely.
pub enum AppError {
    /// Record not found (404)
    NotFound,
    /// Internal server error with optional context
    Internal(String),
    /// Template rendering failure
    TemplateError,
    /// Database error
    Database(sqlx::Error),
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        AppError::Database(e)
    }
}

impl From<askama::Error> for AppError {
    fn from(_: askama::Error) -> Self {
        AppError::TemplateError
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::NotFound => StatusCode::NOT_FOUND.into_response(),
            AppError::Internal(msg) => {
                eprintln!("[ERROR] {}", msg);
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
            AppError::TemplateError => {
                eprintln!("[ERROR] Falha ao renderizar template");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
            AppError::Database(e) => {
                eprintln!("[ERROR] Banco de dados: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

/// Render an Askama template into `Html<String>`, returning `AppError` on failure.
pub fn render(t: impl Template) -> Result<Html<String>, AppError> {
    let html = t.render().map_err(|_| AppError::TemplateError)?;
    Ok(Html(html))
}
