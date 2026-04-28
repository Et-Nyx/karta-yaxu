use axum::{
    routing::get,
    Router,
};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::sqlite::SqliteConnectOptions;
use std::str::FromStr;
use std::env;
use tower_http::services::ServeDir;
use tower_sessions::SessionManagerLayer;
use tower_sessions_sqlx_store::SqliteStore;
use backend::{auth, editor, seed};
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_url = env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:./data/rom.db".to_string());
    let port: u16 = env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);

    let opts = SqliteConnectOptions::from_str(&db_url)?
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .synchronous(sqlx::sqlite::SqliteSynchronous::Normal);

    println!("Conectando ao banco de dados...");
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(opts)
        .await?;

    println!("Aplicando migrações");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await?;

    let pokemon_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM pokemon")
        .fetch_one(&pool)
        .await
        .unwrap_or(0);

    if pokemon_count == 0 {
        println!("Banco vazio — executando seed automático...");
        seed::run_seed_cli(&pool, Vec::new()).await?;
    } else {
        println!("Banco já populado ({} pokemon), seed pulado.", pokemon_count);
    }

    println!("Configurando o gerenciador de sessões...");
    let session_store = SqliteStore::new(pool.clone());
    session_store.migrate().await?;

    let secure_cookie = env::var("SECURE_COOKIE")
        .map(|v| v == "true")
        .unwrap_or(false);

    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(secure_cookie)
        .with_http_only(true)
        .with_same_site(tower_sessions::cookie::SameSite::Lax);

    let governor_conf = std::sync::Arc::new(
        GovernorConfigBuilder::default()
            .per_second(5)
            .burst_size(20)
            .use_headers()
            .finish()
            .unwrap()
    );
    let governor_layer = GovernorLayer::new(governor_conf);

    println!("Configurando as rotas Axum...");
    let api_routes = Router::new()
        .route("/", get(backend::search::search_page))
        .route("/health", get(health_handler))
        .nest("/trainer", auth::routes())
        .nest("/editor", editor::routes())
        .nest("/wiki", backend::wiki::routes())
        .route("/search", get(backend::search::search_page))
        .route("/api/search/suggestions", get(backend::search::search_suggestions))
        .layer(governor_layer)
        .layer(session_layer);

    let app = Router::new()
        .nest_service("/static", ServeDir::new("static"))
        .merge(api_routes)
        .with_state(pool);

    let addr = format!("0.0.0.0:{}", port);
    println!("Servidor rodando em: http://{}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    ).await?;

    Ok(())
}

async fn health_handler() -> &'static str {
    "Servidor online e saudável"
}
