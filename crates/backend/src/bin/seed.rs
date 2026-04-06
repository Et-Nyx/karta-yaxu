#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite:./data/rom.db".to_string());

    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .connect(&db_url)
        .await?;

    backend::seed::run_seed_cli(&pool).await?;

    println!("Seed completado");
    Ok(())
}