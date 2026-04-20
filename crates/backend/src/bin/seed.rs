#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite:./data/rom.db".to_string());

    use std::str::FromStr;
    let opts = sqlx::sqlite::SqliteConnectOptions::from_str(&db_url)?
        .create_if_missing(true)
        .foreign_keys(true);

    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .connect_with(opts)
        .await?;

    // Garante que o schema existe antes de popular
    sqlx::migrate!("./migrations").run(&pool).await?;

    let args: Vec<String> = std::env::args().collect();
    let target_pokemon_id = if args.len() > 1 {
        args[1].parse::<i64>().ok()
    } else {
        None
    };

    backend::seed::run_seed_cli(&pool, target_pokemon_id).await?;

    println!("Seed completado");
    Ok(())
}