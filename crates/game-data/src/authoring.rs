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
        let normalized = Self {
            slug: self.slug.trim().to_owned(),
            display_name: self.display_name.trim().to_owned(),
            description: self
                .description
                .replace("\r\n", "\n")
                .replace('\r', "\n")
                .trim()
                .to_owned(),
        };
        let mut errors = FieldErrors::new();
        if normalized.slug.is_empty()
            || normalized.slug.len() > 80
            || !normalized.slug.split('-').all(|part| {
                !part.is_empty()
                    && part
                        .bytes()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
            })
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
