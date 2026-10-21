//! Offline card-aware packages (ADR0016). V1 remains a separate, frozen format.
//!
//! `typed` retains schema 1 and its active-world-catalog-v1 root policy, but is a
//! projection, not necessarily a standalone v1 package: exact native cards reached
//! through custom references may extend its closure with archived/older records.
//! The v2 reader checks that extension against the combined graph, never by
//! promoting those dependencies to active catalog roots. No database or head
//! inference is performed here; callers supply the same-snapshot catalog roots.
//!
//! Root contract: native card roots exactly mirror the typed local Variant/Type/
//! Move roots (shared Forms are dependency-only); callers additionally enumerate
//! every active local standalone card and reusable field as roots, even if unused.
//! An offline reader verifies these declared roots, not completeness against a
//! live catalog it cannot observe. Historical/archived dependencies remain legal.
//! `legacy_unmodeled` preserves explicit legacy evidence; offline validation can
//! check its shape, not establish a database migration boundary or authenticity.
//!
//! Both formats share the 32 MiB wire budget. V2 combines root, record and authored
//! occurrence budgets across typed content, card components and reusable fields.
//! Keyed vectors are sorted; all authored order, repetition and value distinctions
//! are retained. Content SHA-256 has a dedicated v2 domain, not the v1 checksum or
//! inspection fingerprint. Diagnostics describe typed authoring only.
use crate::{
    card_fields::{FieldDefinition, FieldPin},
    cards::{CardPin, CardRecord},
    inspection_analysis::InspectionFinding,
    package::{self, PackageContent, bounded_json, without_whitespace},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

mod validation;
pub use crate::package::{
    FORMAT, MAX_OCCURRENCES, MAX_PACKAGE_BYTES, MAX_RECORDS, MAX_ROOTS, PRODUCER,
};
pub use validation::{native_card_pin, native_key};
pub(crate) use validation::{occurrence_count, validate_with_external_seeds};

pub const SCHEMA_VERSION: u32 = 2;
const HASH_DOMAIN: &[u8] = b"karta-design-package-content-sha256-v2\n";

/// Bounded dispatch hint only, NOT structural acceptance. Always pass the same
/// bytes to `Package::from_bytes` or `CardPackage::from_bytes` afterwards.
pub fn schema_version_from_bytes(bytes: &[u8]) -> Result<u32, String> {
    if bytes.len() > MAX_PACKAGE_BYTES {
        return Err("Package exceeds the 32 MiB input limit.".into());
    }
    #[derive(Deserialize)]
    struct Header {
        manifest: Version,
    }
    #[derive(Deserialize)]
    struct Version {
        schema_version: u32,
    }
    let header: Header =
        serde_json::from_slice(bytes).map_err(|e| format!("Invalid package header: {e}"))?;
    match header.manifest.schema_version {
        1..=3 => Ok(header.manifest.schema_version),
        _ => Err("Unsupported package schema version (expected 1, 2 or 3).".into()),
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CardPackageContent {
    pub schema_version: u32,
    pub typed: PackageContent,
    pub card_roots: Vec<CardPin>,
    pub field_roots: Vec<FieldPin>,
    pub cards: Vec<CardRecord>,
    pub fields: Vec<FieldDefinition>,
}

/// Counts include typed records and their same-identity card components separately.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CardManifest {
    pub format: String,
    pub schema_version: u32,
    pub producer: String,
    pub content_sha256: String,
    pub root_count: usize,
    pub record_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CardPackage {
    pub manifest: CardManifest,
    pub content: CardPackageContent,
}

impl CardPackage {
    /// Sort keyed outer vectors only; authored component/field ordering is retained.
    pub fn build(mut content: CardPackageContent) -> Result<Self, String> {
        validation::preflight(&content)?;
        content.typed.roots.sort();
        content.typed.records.sort_by(|a, b| a.key.cmp(&b.key));
        content
            .typed
            .bindings
            .sort_by(|a, b| a.identity.cmp(&b.identity));
        content.card_roots.sort();
        content.field_roots.sort();
        content.cards.sort_by_key(CardRecord::pin);
        content.fields.sort_by_key(FieldDefinition::pin);
        validation::validate(&content)?;
        let package = Self {
            manifest: CardManifest {
                format: FORMAT.into(),
                schema_version: SCHEMA_VERSION,
                producer: PRODUCER.into(),
                content_sha256: checksum(&content)?,
                root_count: root_count(&content),
                record_count: record_count(&content),
            },
            content,
        };
        bounded_json(&package)?;
        Ok(package)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > MAX_PACKAGE_BYTES {
            return Err("Package exceeds the 32 MiB input limit.".into());
        }
        let package: Self =
            serde_json::from_slice(bytes).map_err(|e| format!("Invalid card package JSON: {e}"))?;
        let canonical = package.to_bytes()?;
        if !without_whitespace(bytes).eq(without_whitespace(&canonical)) {
            return Err("Package JSON must use canonical fields, ordering and value encodings (JSON whitespace is allowed).".into());
        }
        Ok(package)
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, String> {
        self.validate()?;
        bounded_json(self)
    }

    fn validate(&self) -> Result<(), String> {
        validation::preflight(&self.content)?;
        if self.manifest.format != FORMAT
            || self.manifest.schema_version != SCHEMA_VERSION
            || self.manifest.producer != PRODUCER
            || self.manifest.root_count != root_count(&self.content)
            || self.manifest.record_count != record_count(&self.content)
        {
            return Err("Unsupported or inconsistent card package manifest.".into());
        }
        validation::validate(&self.content)?;
        if self.manifest.content_sha256 != checksum(&self.content)? {
            return Err("Package content checksum does not match.".into());
        }
        Ok(())
    }

    /// Existing typed authoring diagnostics, separate from structural acceptance.
    /// Custom fields/rules text are inert, not executable rules or readiness checks.
    /// Invalid mutable DTOs yield no trusted findings (use `to_bytes` for errors).
    pub fn diagnostics(&self) -> Vec<InspectionFinding> {
        if self.validate().is_err() {
            return vec![];
        }
        package::Package::analyze_content(&self.content.typed)
    }
}

fn root_count(c: &CardPackageContent) -> usize {
    c.typed.roots.len() + c.card_roots.len() + c.field_roots.len()
}
fn record_count(c: &CardPackageContent) -> usize {
    c.typed.records.len() + c.cards.len() + c.fields.len()
}
fn checksum(content: &CardPackageContent) -> Result<String, String> {
    let mut digest = Sha256::new();
    digest.update(HASH_DOMAIN);
    digest.update(bounded_json(content)?);
    Ok(hex::encode(digest.finalize()))
}

#[cfg(test)]
mod tests;
