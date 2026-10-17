//! Frozen design-content package v1 (ADR0015). No database, network or account data.
//!
//! Keyed outer vectors are canonical-sorted; authored inner arrays retain order and
//! multiplicity. The reader accepts JSON whitespace but otherwise requires exactly
//! the canonical JSON tokens emitted by `to_bytes` (including field order, explicit
//! nullable fields and numeric encodings). Checksums detect mismatch, not authenticity.
use crate::{
    inspection::{
        CapturedWorld, DomainIdentity, DomainKind, DomainNodeKey, InspectionPayload,
        InventoryAbsence, RootRole,
    },
    inspection_analysis::{InspectionFinding, analyze_world},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, io::Write};

mod references;
mod validation;
// V2 supplies exact native-card seeds; the v1 entry point still supplies none.
pub use references::{References, payload_key, references};
pub(crate) use validation::payload_occurrences;
pub(crate) use validation::validate_with_external_seeds;

pub const SCHEMA_VERSION: u32 = 1;
pub const ROOT_POLICY: &str = "active-world-catalog-v1";
pub const FORMAT: &str = "karta-design-package";
pub const PRODUCER: &str = "karta-yaxu";
pub const MAX_PACKAGE_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_ROOTS: usize = 2048;
pub const MAX_RECORDS: usize = 8192;
pub const MAX_OCCURRENCES: usize = 65_536;
const HASH_DOMAIN: &[u8] = b"karta-design-package-content-sha256-v1\n";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageRecord {
    pub key: DomainNodeKey,
    pub payload: InspectionPayload,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdentityBinding {
    pub identity: DomainIdentity,
    pub target: DomainNodeKey,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageContent {
    pub schema_version: u32,
    pub root_policy: String,
    pub world: DomainNodeKey,
    pub roots: Vec<DomainNodeKey>,
    pub records: Vec<PackageRecord>,
    pub bindings: Vec<IdentityBinding>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub format: String,
    pub schema_version: u32,
    pub producer: String,
    pub content_sha256: String,
    pub root_count: usize,
    pub record_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Package {
    pub manifest: Manifest,
    pub content: PackageContent,
}

impl Package {
    /// Canonicalize only the keyed outer collections, then validate full closure.
    pub fn build(mut content: PackageContent) -> Result<Self, String> {
        validation::preflight(&content)?;
        content.roots.sort();
        content.records.sort_by(|a, b| a.key.cmp(&b.key));
        content.bindings.sort_by(|a, b| a.identity.cmp(&b.identity));
        validation::validate(&content)?;
        let content_sha256 = checksum(&content)?;
        let package = Self {
            manifest: Manifest {
                format: FORMAT.into(),
                schema_version: SCHEMA_VERSION,
                producer: PRODUCER.into(),
                content_sha256,
                root_count: content.roots.len(),
                record_count: content.records.len(),
            },
            content,
        };
        // Include the manifest in the output-byte budget too.
        bounded_json(&package)?;
        Ok(package)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > MAX_PACKAGE_BYTES {
            return Err("Package exceeds the 32 MiB input limit.".into());
        }
        let package: Self = serde_json::from_slice(bytes)
            .map_err(|error| format!("Invalid package JSON: {error}"))?;
        let canonical = package.to_bytes()?;
        // Comparing tokens, not a generic JSON Value, also detects duplicate
        // keys, missing Option fields, floats/exponents and alternate escapes.
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
        if self.manifest.format != FORMAT
            || self.manifest.schema_version != SCHEMA_VERSION
            || self.manifest.producer != PRODUCER
            || self.manifest.root_count != self.content.roots.len()
            || self.manifest.record_count != self.content.records.len()
        {
            return Err("Unsupported or inconsistent package manifest.".into());
        }
        validation::validate(&self.content)?;
        if self.manifest.content_sha256 != checksum(&self.content)? {
            return Err("Package content checksum does not match.".into());
        }
        Ok(())
    }

    /// Authoring findings only. Public mutable DTOs are revalidated first; an
    /// invalid package has no trusted diagnostics. Use `to_bytes` for its error.
    pub fn diagnostics(&self) -> Vec<InspectionFinding> {
        if self.validate().is_err() {
            return vec![];
        }
        Self::analyze_content(&self.content)
    }

    pub(crate) fn analyze_content(content: &PackageContent) -> Vec<InspectionFinding> {
        let nodes: BTreeMap<_, _> = content
            .records
            .iter()
            .map(|r| (r.key.clone(), r.payload.clone()))
            .collect();
        let Some(InspectionPayload::World(world)) = nodes.get(&content.world) else {
            return vec![];
        };
        let roots: BTreeMap<_, _> = content
            .roots
            .iter()
            .map(|k| (k.clone(), RootRole::Active))
            .collect();
        let mut absences = vec![];
        for kind in [
            DomainKind::Chart,
            DomainKind::StatRoster,
            DomainKind::Ruleset,
        ] {
            if !roots.keys().any(|key| key.kind == kind) {
                absences.push(InventoryAbsence {
                    kind,
                    owner_id: None,
                });
            }
        }
        for variant in roots.keys().filter(|k| k.kind == DomainKind::Variant) {
            for kind in [DomainKind::Learnset, DomainKind::Preset] {
                if !content.roots.iter().any(|key| match nodes.get(key) {
                    Some(InspectionPayload::Learnset(r)) if kind == DomainKind::Learnset => {
                        r.variant_id.as_ref() == Some(&variant.id)
                    }
                    Some(InspectionPayload::Preset(r)) if kind == DomainKind::Preset => {
                        r.variant_id == variant.id
                    }
                    _ => false,
                }) {
                    absences.push(InventoryAbsence {
                        kind,
                        owner_id: Some(variant.id.clone()),
                    });
                }
            }
        }
        let edges = content
            .records
            .iter()
            .flat_map(|r| references::edges(&r.payload))
            .collect();
        analyze_world(
            &CapturedWorld {
                contract_version: 1,
                world: world.clone(),
                roots,
                nodes,
                edges,
                // No inferred latest/head observations: the package only
                // knows selected roots and explicit capture-time bindings.
                heads: BTreeMap::new(),
                absences,
                gaps: vec![],
                card_data: Default::default(),
            },
            false,
        )
    }
}

fn checksum(content: &PackageContent) -> Result<String, String> {
    let mut digest = Sha256::new();
    digest.update(HASH_DOMAIN);
    digest.update(bounded_json(content)?);
    Ok(hex::encode(digest.finalize()))
}

struct BoundedBuffer(Vec<u8>);
impl Write for BoundedBuffer {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > MAX_PACKAGE_BYTES.saturating_sub(self.0.len()) {
            return Err(std::io::Error::other("Package exceeds the 32 MiB limit."));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
pub(crate) fn bounded_json(value: &impl Serialize) -> Result<Vec<u8>, String> {
    let mut output = BoundedBuffer(Vec::new());
    serde_json::to_writer(&mut output, value)
        .map_err(|error| format!("Cannot encode package: {error}"))?;
    Ok(output.0)
}

pub(crate) fn without_whitespace(bytes: &[u8]) -> impl Iterator<Item = u8> + '_ {
    let (mut quoted, mut escaped) = (false, false);
    bytes.iter().copied().filter(move |byte| {
        if quoted {
            if escaped {
                escaped = false;
            } else if *byte == b'\\' {
                escaped = true;
            } else if *byte == b'"' {
                quoted = false;
            }
            true
        } else if *byte == b'"' {
            quoted = true;
            true
        } else {
            !matches!(byte, b' ' | b'\n' | b'\r' | b'\t')
        }
    })
}

#[cfg(test)]
mod tests;
