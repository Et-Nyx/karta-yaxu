use std::str::FromStr;
use sqlx::sqlite::SqliteConnectOptions;

fn main() {
    let url1 = "sqlite:./data/rom.db";
    let opts1 = SqliteConnectOptions::from_str(url1).unwrap();
    println!("from_str(sqlite:./data/rom.db): {:?}", opts1.get_filename());

    let opts2 = SqliteConnectOptions::new().filename(url1.trim_start_matches("sqlite:"));
    println!("filename(trim): {:?}", opts2.get_filename());
}