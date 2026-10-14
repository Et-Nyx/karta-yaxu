//! Shared plain-text authoring values. Validation never modifies the submitted value.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub type FieldErrors = BTreeMap<String, String>;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ContentFields {
    pub slug: String,
    pub display_name: String,
    pub description: String,
}

impl ContentFields {
    /// Return a normalized copy; callers retain their original on failure.
    pub fn normalized(&self) -> Result<Self, FieldErrors> {
        self.normalize(true)
    }

    /// User mutations validate text independently of the ignored submitted slug.
    /// Slug allocation happens only after authorization and concurrency checks.
    pub fn normalized_for_auto_slug(&self) -> Result<Self, FieldErrors> {
        self.normalize(false)
    }

    fn normalize(&self, validate_slug: bool) -> Result<Self, FieldErrors> {
        let normalized = Self {
            slug: if validate_slug {
                self.slug.trim().to_owned()
            } else {
                String::new()
            },
            display_name: self.display_name.trim().to_owned(),
            description: self
                .description
                .replace("\r\n", "\n")
                .replace('\r', "\n")
                .trim()
                .to_owned(),
        };
        let mut errors = FieldErrors::new();
        if validate_slug
            && (normalized.slug.is_empty()
                || normalized.slug.len() > 80
                || !normalized.slug.split('-').all(|part| {
                    !part.is_empty()
                        && part
                            .bytes()
                            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
                }))
        {
            errors.insert(
                "slug".into(),
                "Use at most 80 lowercase letters, digits and single separating hyphens.".into(),
            );
        }
        for (key, text, limit, paragraphs) in [
            ("display_name", &normalized.display_name, 100, false),
            ("description", &normalized.description, 2000, true),
        ] {
            if text.is_empty()
                || text.chars().count() > limit
                || text
                    .chars()
                    .any(|c| c.is_control() && !(paragraphs && c == '\n'))
            {
                errors.insert(
                    key.into(),
                    format!("Enter 1–{limit} characters of plain text."),
                );
            }
        }
        if errors.is_empty() {
            Ok(normalized)
        } else {
            Err(errors)
        }
    }
}

/// Derive an address base from a previously normalized, validated display name.
/// Unknown characters are separators, rather than invented transliterations.
pub fn slug_from_name(display_name: &str) -> String {
    let ascii = deunicode::deunicode_with_tofu(display_name, " ");
    let mut slug = String::with_capacity(80);
    let mut separator = false;
    for byte in ascii.bytes() {
        if byte.is_ascii_alphanumeric() {
            if separator && !slug.is_empty() {
                if slug.len() == 80 {
                    break;
                }
                slug.push('-');
            }
            if slug.len() == 80 {
                break;
            }
            slug.push(char::from(byte.to_ascii_lowercase()));
            separator = false;
        } else {
            separator = true;
        }
    }
    let slug = slug.trim_end_matches('-');
    if slug.is_empty() {
        "object".into()
    } else {
        slug.into()
    }
}

/// Numbered candidates for a `slug_from_name` base and a positive search ordinal.
/// Preserve the ASCII 80-byte cap, including the suffix.
pub fn slug_candidate(base: &str, number: u32) -> String {
    if number == 1 {
        return base.into();
    }
    let suffix = format!("-{number}");
    let prefix = &base[..base.len().min(80 - suffix.len())];
    format!("{}{suffix}", prefix.trim_end_matches('-'))
}

pub fn validate_reason(reason: Option<&str>) -> Result<Option<String>, FieldErrors> {
    if let Some(reason) = reason {
        if reason.chars().count() > 500 || reason.chars().any(char::is_control) {
            return Err(BTreeMap::from([(
                "reason".into(),
                "Use at most 500 characters without control characters.".into(),
            )]));
        }
        if !reason.trim().is_empty() {
            return Ok(Some(reason.trim().to_owned()));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn automatic_slugs_transliterate_and_bound_ascii_addresses() {
        for (name, slug) in [
            ("Été — Æneid!", "ete-aeneid"),
            ("北亰", "bei-jing"),
            ("げんまい茶", "genmaicha"),
            ("🦄☣", "unicorn-biohazard"),
            ("... — !!!", "object"),
            ("\u{e000}", "object"),
            (" Hello___WORLD / 123 ", "hello-world-123"),
        ] {
            assert_eq!(slug_from_name(name), slug);
        }
        let base = slug_from_name(&"é".repeat(100));
        assert_eq!(base, "e".repeat(80));
        assert_eq!(slug_candidate(&base, 2), format!("{}-2", "e".repeat(78)));
        assert_eq!(
            slug_candidate(&base, 10_000),
            format!("{}-10000", "e".repeat(74))
        );
        assert_eq!(
            slug_from_name(&format!("{} b", "a".repeat(79))),
            "a".repeat(79)
        );
        let hyphen_at_cut = format!("{}-bc", "a".repeat(77));
        assert_eq!(
            slug_candidate(&hyphen_at_cut, 2),
            format!("{}-2", "a".repeat(77))
        );
    }

    #[test]
    fn automatic_normalization_ignores_slug_but_persisted_validation_does_not() {
        let fields = ContentFields {
            slug: "forged / INVALID".into(),
            display_name: " Éclair ".into(),
            description: " First\r\nSecond ".into(),
        };
        assert!(fields.normalized().unwrap_err().contains_key("slug"));
        let normalized = fields.normalized_for_auto_slug().unwrap();
        assert_eq!(normalized.display_name, "Éclair");
        assert_eq!(normalized.description, "First\nSecond");
        assert!(normalized.slug.is_empty());
        let mut invalid = fields;
        invalid.display_name = "bad\nname".into();
        assert!(
            invalid
                .normalized_for_auto_slug()
                .unwrap_err()
                .contains_key("display_name")
        );
    }

    #[test]
    fn character_limits_plain_text_and_lf_normalization() {
        let at_limit = ContentFields {
            slug: "a".repeat(80),
            display_name: "é".repeat(100),
            description: "界".repeat(2000),
        };
        assert_eq!(at_limit.normalized().unwrap(), at_limit);
        assert!(validate_reason(Some(&"é".repeat(500))).is_ok());
        assert!(
            validate_reason(Some(&"é".repeat(501)))
                .unwrap_err()
                .contains_key("reason")
        );
        for (field, value) in [
            ("slug", "a".repeat(81)),
            ("display_name", "é".repeat(101)),
            ("description", "界".repeat(2001)),
        ] {
            let mut fields = at_limit.clone();
            match field {
                "slug" => fields.slug = value,
                "display_name" => fields.display_name = value,
                _ => fields.description = value,
            }
            assert!(fields.normalized().unwrap_err().contains_key(field));
        }
        let submitted = ContentFields {
            slug: " plain-text ".into(),
            display_name: " Plain text ".into(),
            description: " First\r\n\r\nSecond\rThird ".into(),
        };
        assert_eq!(
            submitted.normalized().unwrap().description,
            "First\n\nSecond\nThird"
        );
        assert!(submitted.description.contains('\r'));
        for value in ["tab\ttext", "nul\0text", "escape\u{1b}text"] {
            let mut fields = submitted.clone();
            fields.description = value.into();
            assert!(fields.normalized().unwrap_err().contains_key("description"));
        }
    }
}
