use axum::{
    routing::get,
    Router,
};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::sqlite::SqliteConnectOptions;
use std::str::FromStr;
use std::env;
use tower_http::services::ServeDir;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_url = env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:./data/rom.db".to_string());

    let opts = SqliteConnectOptions::from_str(&db_url)?
        .create_if_missing(true);

    println!("Conectando ao banco de dados...");
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(opts)
        .await?;

    println!("Aplicando migrações");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await?;

    println!("Configurando as rotas Axum...");
    let app = Router::new()
        .route("/", get(home_handler))
        .route("/health", get(health_handler))
        // Wiki — Pokémon
        .route("/wiki/pokemon", get(backend::wiki::pokemon_list))
        .route("/wiki/pokemon/{slug}", get(backend::wiki::pokemon_detail))
        // Wiki — Tipos
        .route("/wiki/types", get(backend::wiki::types_list))
        .route("/wiki/types/{slug}", get(backend::wiki::type_detail))
        // Wiki — Moves
        .route("/wiki/moves", get(backend::wiki::moves_list))
        .route("/wiki/moves/{slug}", get(backend::wiki::move_detail))
        // Wiki — Habilidades
        .route("/wiki/abilities", get(backend::wiki::abilities_list))
        .route("/wiki/abilities/{slug}", get(backend::wiki::ability_detail))
        // Wiki — Mecânicas
        .route("/wiki/mechanics/nature", get(backend::wiki::natures_list))
        .route("/wiki/mechanics/status", get(backend::wiki::status_list))
        // Estáticos
        .nest_service("/static", ServeDir::new("static"))
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