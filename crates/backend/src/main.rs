use axum::{
    routing::get,
    Router,
};
Use sqlx::sqlite::SqlitePoolOptions;
use std::env;
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    let db_url = env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:.data/rom.db".to_string());

    println!("Conectando ao banco de dados...");

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await

    println!("Configurando as rotas Axum...");

    let app = Router::new()
        .route("/", get(home_handler))
        .route("/health", get(health_handler))
        .with_state(pool);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));

    println!("Servidor rodando localmente em: http://{}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

async fn home_handler() -> &'static str {
    "Bem-vindo ao Backend do Karta Yaxu!";
}

async fn health_handler() -> &'static str {
    "Servidor online e saudável";
}