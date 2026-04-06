#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite:./data/rom.db".to_string());

    let opts = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(db_url.trim_start_matches("sqlite:"))
        .create_if_missing(true);

    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .connect_with(opts)
        .await?;

    // Garante que o schema existe antes de popular
    sqlx::migrate!("./migrations").run(&pool).await?;

    backend::seed::run_seed_cli(&pool).await?;

    println!("Seed completado");
    Ok(())
}