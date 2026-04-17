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
        seed::run_seed_cli(&pool).await?;
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
        // Trainers - auth
        .route("/trainer/login", get(auth::login_page).post(auth::login_post))
        .route("/trainer/register", get(auth::register_page).post(auth::register_post))
        .route("/trainer/dashboard", get(auth::dashboard))
        .route("/trainer/logout", axum::routing::post(auth::logout))
        // Editor
        .route("/editor", get(editor::index))
        .route("/editor/pokemon", get(editor::pokemon_list))
        .route("/editor/pokemon/new", get(editor::pokemon_new).post(editor::pokemon_create))
        .route("/editor/pokemon/{slug}", get(editor::pokemon_edit).post(editor::pokemon_save))
        .route("/editor/pokemon/{slug}/delete", axum::routing::post(editor::pokemon_delete))
        .route("/editor/moves", get(editor::move_list))
        .route("/editor/moves/new", get(editor::move_new).post(editor::move_create))
        .route("/editor/moves/{slug}", get(editor::move_edit).post(editor::move_save))
        .route("/editor/moves/{slug}/delete", axum::routing::post(editor::move_delete))
        .route("/editor/types", get(editor::type_list))
        .route("/editor/types/new", get(editor::type_new).post(editor::type_create))
        .route("/editor/types/{slug}", get(editor::type_edit).post(editor::type_save))
        .route("/editor/types/{slug}/delete", axum::routing::post(editor::type_delete))
        .route("/editor/abilities", get(editor::ability_list))
        .route("/editor/abilities/new", get(editor::ability_new).post(editor::ability_create))
        .route("/editor/abilities/{slug}", get(editor::ability_edit).post(editor::ability_save))
        .route("/editor/abilities/{slug}/delete", axum::routing::post(editor::ability_delete))
        .route("/editor/natures", get(editor::nature_list))
        .route("/editor/natures/new", get(editor::nature_new).post(editor::nature_create))
        .route("/editor/natures/{slug}", get(editor::nature_edit).post(editor::nature_save))
        .route("/editor/natures/{slug}/delete", axum::routing::post(editor::nature_delete))
        .route("/editor/status", get(editor::status_list))
        .route("/editor/status/new", get(editor::status_new).post(editor::status_create))
        .route("/editor/status/{slug}", get(editor::status_edit).post(editor::status_save))
        .route("/editor/status/{slug}/delete", axum::routing::post(editor::status_delete))
        // Wiki — Index
        .route("/wiki", get(backend::wiki::index))
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
