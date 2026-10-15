//! Validated, read-only development content. No HTTP or storage dependencies.
use serde::Deserialize;
use std::collections::HashSet;

pub mod authoring;
pub mod charts;
pub mod stats;
pub mod types;

pub const BUNDLED_FIXTURES: &str = include_str!("../../../fixtures/creatures.json");

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Creature {
    id: String,
    slug: String,
    display_name: String,
    description: String,
}

impl Creature {
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn slug(&self) -> &str {
        &self.slug
    }
    pub fn display_name(&self) -> &str {
        &self.display_name
    }
    pub fn description(&self) -> &str {
        &self.description
    }
}

#[derive(Debug)]
pub struct Catalog {
    creatures: Vec<Creature>,
}

impl Catalog {
    pub fn from_json(json: &str) -> Result<Self, String> {
        let mut creatures: Vec<Creature> = serde_json::from_str(json)
            .map_err(|error| format!("Invalid creature fixture JSON: {error}"))?;
        let mut ids = HashSet::new();
        let mut slugs = HashSet::new();
        for (index, creature) in creatures.iter().enumerate() {
            let valid_id = creature.id.len() == 32
                && creature
                    .id
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
            let fields = authoring::ContentFields {
                slug: creature.slug.clone(),
                display_name: creature.display_name.clone(),
                description: creature.description.clone(),
            };
            // Fixtures must already be canonical; do not silently rewrite baselines.
            let valid_fields = fields
                .normalized()
                .is_ok_and(|normalized| normalized == fields);
            if !valid_id || !valid_fields {
                return Err(format!(
                    "Creature {} has invalid ID, slug, name, or description",
                    index + 1
                ));
            }
            if !ids.insert(&creature.id) || !slugs.insert(&creature.slug) {
                return Err(format!("Creature {} repeats an ID or slug", index + 1));
            }
        }
        creatures.sort_by(|a, b| a.display_name.cmp(&b.display_name).then(a.id.cmp(&b.id)));
        Ok(Self { creatures })
    }

    pub fn creatures(&self) -> &[Creature] {
        &self.creatures
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_content_is_valid_and_ids_survive_renaming() {
        let catalog = Catalog::from_json(BUNDLED_FIXTURES).unwrap();
        assert_eq!(catalog.creatures().len(), 3);
        let renamed =
            Catalog::from_json(&BUNDLED_FIXTURES.replace("Emberling", "Zemberling")).unwrap();
        assert!(
            renamed
                .creatures()
                .iter()
                .any(|c| c.display_name() == "Zemberling"
                    && c.id() == "6a64df7c88604b3f9381536710449a8b")
        );
    }

    #[test]
    fn rejects_invalid_and_ambiguous_content() {
        let valid = serde_json::from_str::<serde_json::Value>(BUNDLED_FIXTURES).unwrap();
        for field in ["id", "slug", "display_name", "description"] {
            let mut data = valid.clone();
            data[0][field] = " ".into();
            assert!(Catalog::from_json(&data.to_string()).is_err());
        }
        for field in ["id", "slug"] {
            let mut data = valid.clone();
            data[1][field] = data[0][field].clone();
            assert!(Catalog::from_json(&data.to_string()).is_err());
        }
        let mut data = valid;
        data[0]["unexpected"] = true.into();
        assert!(Catalog::from_json(&data.to_string()).is_err());
        assert!(Catalog::from_json("not json").is_err());
        assert!(Catalog::from_json("[]").unwrap().creatures().is_empty());
    }

    #[test]
    fn fixture_paragraphs_share_authoring_validation_but_require_canonical_text() {
        let mut data = serde_json::from_str::<serde_json::Value>(BUNDLED_FIXTURES).unwrap();
        data[0]["description"] = "First paragraph.\n\nSecond paragraph.".into();
        let catalog = Catalog::from_json(&data.to_string()).unwrap();
        assert_eq!(
            catalog.creatures()[0].description(),
            "First paragraph.\n\nSecond paragraph."
        );
        for description in [
            "First\r\n\r\nSecond",
            "First\rSecond",
            " First\nSecond",
            "First\nSecond ",
            "First\tSecond",
        ] {
            data[0]["description"] = description.into();
            assert!(
                Catalog::from_json(&data.to_string()).is_err(),
                "{description:?}"
            );
        }
    }
}
