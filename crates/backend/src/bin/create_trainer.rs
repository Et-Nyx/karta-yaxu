use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use dialoguer::{Confirm, Select};
use rpassword::read_password;
use sqlx::sqlite::SqlitePoolOptions;
use std::io::{self, Write};
use uuid::Uuid;

const MAX_PASSWORD_ATTEMPTS: u8 = 3;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Karta Yaxu — Trainer Creator ===\n");

    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite:./data/rom.db".to_string());

    let opts = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(db_url.trim_start_matches("sqlite:"))
        .create_if_missing(true);

    let pool = std::sync::Arc::new(
        SqlitePoolOptions::new()
            .connect_with(opts)
            .await?,
    );

    // ── 1. Username prompt ─────────────────────────────────────────────────
    print!("Username: ");
    io::stdout().flush()?;
    let mut username = String::new();
    io::stdin().read_line(&mut username)?;
    let username = username.trim().to_string();

    if username.is_empty() {
        eprintln!("Error: Username cannot be empty.");
        return Ok(());
    }

    if username.starts_with(' ') || username.ends_with(' ') {
        eprintln!("Error: Username cannot have leading or trailing spaces.");
        return Ok(());
    }

    // Check if username is already taken
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM trainers WHERE username = ?)",
    )
    .bind(&username)
    .fetch_one(&*pool)
    .await?;

    if exists {
        eprintln!(
            "Error: Username '{}' is already taken. Try another.",
            username
        );
        return Ok(());
    }

    // ── 2. Password prompt with confirmation ──────────────────────────────
    let password: String;
    let mut attempts = 0;

    loop {
        print!("Password: ");
        io::stdout().flush()?;
        let p1 = read_password()?;
        if p1.is_empty() {
            eprintln!("Error: Password cannot be empty.");
            attempts += 1;
            if attempts >= MAX_PASSWORD_ATTEMPTS {
                eprintln!("Too many attempts. Aborted.");
                return Ok(());
            }
            continue;
        }

        print!("Repeat password: ");
        io::stdout().flush()?;
        let p2 = read_password()?;
        if p1 != p2 {
            eprintln!("Passwords don't match. Try again.");
            attempts += 1;
            if attempts >= MAX_PASSWORD_ATTEMPTS {
                eprintln!("Too many attempts. Aborted.");
                return Ok(());
            }
            continue;
        }

        password = p1;
        break;
    }

    // ── 3. Role selection ─────────────────────────────────────────────────
    let roles = ["viewer", "editor"];
    let role_default = 1; // editor is index 1

    let role = match Select::new()
        .with_prompt("Role")
        .items(&roles)
        .default(role_default)
        .interact()
    {
        Ok(idx) => roles[idx],
        Err(_) => {
            eprintln!("Error reading role selection. Defaulting to editor.");
            "editor"
        }
    };

    println!();

    // ── 4. Confirmation ───────────────────────────────────────────────────
    let confirmed = Confirm::new()
        .with_prompt(format!(
            "Create trainer '{}' with role={}?",
            username, role
        ))
        .default(false)
        .interact()
        .unwrap_or(false);

    if !confirmed {
        println!("Aborted.");
        return Ok(());
    }

    // ── 5. Hash password ──────────────────────────────────────────────────
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| format!("Failed to hash password: {}", e))?
        .to_string();

    // ── 6. Insert into DB ─────────────────────────────────────────────────
    let id = Uuid::new_v4().to_string();

    sqlx::query(
        "INSERT INTO trainers (id, username, password_hash, role, is_npc, created_at) VALUES (?, ?, ?, ?, 0, datetime('now'))",
    )
    .bind(&id)
    .bind(&username)
    .bind(&password_hash)
    .bind(role)
    .execute(&*pool)
    .await
    .map_err(|e| format!("Failed to insert trainer: {}", e))?;

    println!();
    println!("\x1b[32m✓\x1b[0m Trainer created!");
    println!("  username: {}", username);
    println!("  role:     {}", role);
    println!("  id:       {}", id);

    Ok(())
}