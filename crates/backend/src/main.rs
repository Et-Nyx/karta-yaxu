use axum::{
    routing::get,
    Router,
};
use sqlx::sqlite::SqlitePoolOptions;
use std::env;

pub mod models;
pub mod seed;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_url = env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:./data/rom.db".to_string());

    println!("Conectando ao banco de dados...");

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;

    println!("Configurando as rotas Axum...");

    let app = Router::new()
        .route("/", get(home_handler))
        .route("/health", get(health_handler))
        .route("/admin/seed", get(seed::run_seed))
        .with_state(pool);

    println!("Servidor rodando localmente em: http://0.0.0.0:3000");

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn home_handler() -> &'static str {
    "Bem-vindo ao Backend do Karta Yaxu!"
}

async fn health_handler() -> &'static str {
    "Servidor online e saudável"
}