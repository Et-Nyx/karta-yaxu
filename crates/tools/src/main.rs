use game_data::package::{MAX_PACKAGE_BYTES, Package};
use game_data::{BUNDLED_FIXTURES, Catalog};
use std::{io::Read, process::ExitCode};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() > 2 {
        usage();
        return ExitCode::FAILURE;
    }
    if args.first().map(String::as_str) == Some("validate-package") {
        return validate_package(args.get(1).map(String::as_str));
    }
    if args.first().map(String::as_str) != Some("validate-fixtures") {
        usage();
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

fn usage() {
    eprintln!(
        "Usage: tools validate-fixtures [path.json]\n       tools validate-package [path.json|-] (defaults to stdin)"
    );
}

fn read_package(reader: impl Read) -> Result<Package, String> {
    let mut bytes = vec![];
    reader
        .take((MAX_PACKAGE_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("Cannot read package: {error}"))?;
    Package::from_bytes(&bytes)
}

fn validate_package(path: Option<&str>) -> ExitCode {
    let result = match path {
        Some(path) if path != "-" => std::fs::File::open(path)
            .map_err(|error| format!("Cannot open package: {error}"))
            .and_then(read_package),
        _ => read_package(std::io::stdin().lock()),
    };
    match result {
        Ok(package) => {
            use game_data::inspection_analysis::FindingCategory as C;
            let findings = package.diagnostics();
            let count = |category| findings.iter().filter(|f| f.category == category).count();
            println!(
                "Accepted design draft: {} roots, {} exact records",
                package.manifest.root_count, package.manifest.record_count
            );
            println!("Content SHA-256: {}", package.manifest.content_sha256);
            println!(
                "Authoring diagnostics: {} contradictions, {} incomplete, {} context differences, {} not evaluated",
                count(C::Contradiction),
                count(C::Incomplete),
                count(C::ContextDifference),
                count(C::NotEvaluated)
            );
            println!("Structural acceptance only; combat readiness is not evaluated.");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("Package validation failed: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn package_input_is_bounded_before_parsing() {
        struct Endless {
            read: usize,
        }
        impl Read for Endless {
            fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
                self.read += buffer.len();
                assert!(self.read <= MAX_PACKAGE_BYTES + 1);
                buffer.fill(b' ');
                Ok(buffer.len())
            }
        }
        assert!(
            read_package(Endless { read: 0 })
                .unwrap_err()
                .contains("32 MiB")
        );
    }
}
