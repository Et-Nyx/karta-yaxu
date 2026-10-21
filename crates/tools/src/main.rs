use game_data::card_package::CardPackage;
use game_data::package::{MAX_PACKAGE_BYTES, Package};
use game_data::record_package::RecordPackage;
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

#[derive(Debug)]
enum OfflinePackage {
    V1(Package),
    V2(CardPackage),
    V3(RecordPackage),
}

fn read_package(reader: impl Read) -> Result<OfflinePackage, String> {
    let mut bytes = vec![];
    reader
        .take((MAX_PACKAGE_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("Cannot read package: {error}"))?;
    if bytes.len() > MAX_PACKAGE_BYTES {
        return Err("Package exceeds the 32 MiB input limit.".into());
    }
    // Dispatch only. The selected strict reader rechecks the complete envelope,
    // duplicate fields, schema agreement and canonical wire representation.
    match game_data::card_package::schema_version_from_bytes(&bytes)? {
        1 => Package::from_bytes(&bytes).map(OfflinePackage::V1),
        2 => CardPackage::from_bytes(&bytes).map(OfflinePackage::V2),
        3 => RecordPackage::from_bytes(&bytes).map(OfflinePackage::V3),
        _ => Err("Unsupported package schema version (expected 1, 2 or 3).".into()),
    }
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
            let (roots, records, checksum, findings) = match package {
                OfflinePackage::V1(p) => (
                    p.manifest.root_count,
                    p.manifest.record_count,
                    p.manifest.content_sha256.clone(),
                    p.diagnostics(),
                ),
                OfflinePackage::V2(p) => (
                    p.manifest.root_count,
                    p.manifest.record_count,
                    p.manifest.content_sha256.clone(),
                    p.diagnostics(),
                ),
                OfflinePackage::V3(p) => (
                    p.manifest.root_count,
                    p.manifest.record_count,
                    p.manifest.content_sha256.clone(),
                    p.diagnostics(),
                ),
            };
            let count = |category| findings.iter().filter(|f| f.category == category).count();
            println!(
                "Accepted design draft: {} roots, {} exact records",
                roots, records
            );
            println!("Content SHA-256: {checksum}");
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
    fn content() -> game_data::package::PackageContent {
        use game_data::{
            authoring::ContentFields,
            inspection::{InspectionPayload, PublicWorldContent},
            package::{PackageContent, PackageRecord, payload_key},
        };
        let payload = InspectionPayload::World(PublicWorldContent {
            id: format!("{:032x}", 1),
            fields: ContentFields {
                slug: "world".into(),
                display_name: "World".into(),
                description: "Design draft".into(),
            },
            revision: 1,
            archived_at: None,
        });
        let key = payload_key(&payload);
        PackageContent {
            schema_version: 1,
            root_policy: game_data::package::ROOT_POLICY.into(),
            world: key.clone(),
            roots: vec![key.clone()],
            records: vec![PackageRecord { key, payload }],
            bindings: vec![],
        }
    }
    #[test]
    fn offline_dispatch_accepts_both_versions_and_checks_complete_envelope() {
        let v1 = Package::build(content()).unwrap().to_bytes().unwrap();
        let v2 = CardPackage::build(game_data::card_package::CardPackageContent {
            schema_version: 2,
            typed: content(),
            card_roots: vec![],
            field_roots: vec![],
            cards: vec![],
            fields: vec![],
        })
        .unwrap()
        .to_bytes()
        .unwrap();
        assert!(matches!(
            read_package(v1.as_slice()).unwrap(),
            OfflinePackage::V1(_)
        ));
        assert!(matches!(
            read_package(v2.as_slice()).unwrap(),
            OfflinePackage::V2(_)
        ));
        let text = String::from_utf8(v2).unwrap();
        assert!(
            read_package(
                text.replacen("\"schema_version\":2", "\"schema_version\":1", 1)
                    .as_bytes()
            )
            .is_err()
        );
        assert!(
            read_package(
                text.replacen("\"schema_version\":2", "\"schema_version\":3", 1)
                    .as_bytes()
            )
            .is_err()
        );
        assert!(
            read_package(
                text.replacen(
                    "\"schema_version\":2",
                    "\"schema_version\":2,\"schema_version\":2",
                    1
                )
                .as_bytes()
            )
            .is_err()
        );
    }
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
