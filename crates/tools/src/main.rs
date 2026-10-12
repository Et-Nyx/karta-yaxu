use game_data::{BUNDLED_FIXTURES, Catalog};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) != Some("validate-fixtures") || args.len() > 2 {
        eprintln!("Usage: tools validate-fixtures [path.json]");
        return ExitCode::FAILURE;
    }
    let result = match args.get(1) {
        Some(path) => std::fs::read_to_string(path)
            .map_err(|error| format!("Cannot read fixture file: {error}")),
        None => Ok(BUNDLED_FIXTURES.to_owned()),
    }
    .and_then(|json| Catalog::from_json(&json));
    match result {
        Ok(catalog) => {
            println!("Valid: {} placeholder creatures", catalog.creatures().len());
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("Validation failed: {error}");
            ExitCode::FAILURE
        }
    }
}
