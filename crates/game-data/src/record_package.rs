//! Pure offline schema-3 packages (ADR0018).
//!
//! `cards` is a schema-2 projection, not necessarily independently closed: rows
//! and layouts can reach otherwise unused cards, fields and typed Stats. Only the
//! combined graph establishes closure. Declared roots describe the active local
//! catalog; an offline reader cannot prove completeness against a live database.
//! Exact archived/older dependencies are legal, and prose has no mechanics.
use crate::{
    card_package::CardPackageContent,
    inspection_analysis::InspectionFinding,
    package::{self, bounded_json, without_whitespace},
    relations::{CardRelationRecord, CollectionDefinition, CollectionPin, LayoutPin, LayoutRecord},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

mod validation;
pub use crate::package::{
    FORMAT, MAX_OCCURRENCES, MAX_PACKAGE_BYTES, MAX_RECORDS, MAX_ROOTS, PRODUCER,
};
pub const SCHEMA_VERSION: u32 = 3;
const HASH_DOMAIN: &[u8] = b"karta-design-package-content-sha256-v3\n";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordPackageContent {
    pub schema_version: u32,
    pub cards: CardPackageContent,
    pub layout_roots: Vec<LayoutPin>,
    pub collection_roots: Vec<CollectionPin>,
    pub layouts: Vec<LayoutRecord>,
    pub collections: Vec<CollectionDefinition>,
    pub relations: Vec<CardRelationRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordManifest {
    pub format: String,
    pub schema_version: u32,
    pub producer: String,
    pub content_sha256: String,
    pub root_count: usize,
    pub record_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordPackage {
    pub manifest: RecordManifest,
    pub content: RecordPackageContent,
}

impl RecordPackage {
    /// Sort only keyed outer vectors; retain authored columns, rows and cells.
    pub fn build(mut content: RecordPackageContent) -> Result<Self, String> {
        validation::preflight(&content)?;
        let c = &mut content.cards;
        c.typed.roots.sort();
        c.typed.records.sort_by(|a, b| a.key.cmp(&b.key));
        c.typed.bindings.sort_by(|a, b| a.identity.cmp(&b.identity));
        c.card_roots.sort();
        c.field_roots.sort();
        c.cards.sort_by_key(crate::cards::CardRecord::pin);
        c.fields
            .sort_by_key(crate::card_fields::FieldDefinition::pin);
        content.layout_roots.sort();
        content.collection_roots.sort();
        content.layouts.sort_by_key(LayoutRecord::pin);
        content.collections.sort_by_key(CollectionDefinition::pin);
        content.relations.sort_by_key(CardRelationRecord::pin);
        validation::validate(&content)?;
        let package = Self {
            manifest: RecordManifest {
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
        let package: Self = serde_json::from_slice(bytes)
            .map_err(|e| format!("Invalid record package JSON: {e}"))?;
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
            return Err("Unsupported or inconsistent record package manifest.".into());
        }
        validation::validate(&self.content)?;
        if self.manifest.content_sha256 != checksum(&self.content)? {
            return Err("Package content checksum does not match.".into());
        }
        Ok(())
    }

    /// Typed authoring findings only, after complete v3 structural validation.
    /// Invalid mutable DTOs yield no trusted findings; use `to_bytes` for errors.
    pub fn diagnostics(&self) -> Vec<InspectionFinding> {
        if self.validate().is_err() {
            return vec![];
        }
        let used = crate::record_inspection::referenced_typed(self.content.relations.iter());
        package::Package::analyze_content(&self.content.cards.typed)
            .into_iter()
            .filter(|f| {
                f.code != "usage.unused_in_capture" || !used.contains(&f.subject.identity())
            })
            .collect()
    }
}

fn root_count(c: &RecordPackageContent) -> usize {
    c.cards.typed.roots.len()
        + c.cards.card_roots.len()
        + c.cards.field_roots.len()
        + c.layout_roots.len()
        + c.collection_roots.len()
}
fn record_count(c: &RecordPackageContent) -> usize {
    c.cards.typed.records.len()
        + c.cards.cards.len()
        + c.cards.fields.len()
        + c.layouts.len()
        + c.collections.len()
        + c.relations.len()
}
fn checksum(c: &RecordPackageContent) -> Result<String, String> {
    let mut digest = Sha256::new();
    digest.update(HASH_DOMAIN);
    digest.update(bounded_json(c)?);
    Ok(hex::encode(digest.finalize()))
}

#[cfg(test)]
mod tests;
