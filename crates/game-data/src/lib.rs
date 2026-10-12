//! Validated, read-only development content. No HTTP or storage dependencies.
use serde::Deserialize;
use std::collections::HashSet;

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
            let valid_slug = !creature.slug.is_empty()
                && creature.slug.len() <= 80
                && creature.slug.split('-').all(|part| {
                    !part.is_empty()
                        && part
                            .bytes()
                            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
                });
            let valid_text = |text: &str, max| {
                !text.trim().is_empty()
                    && text == text.trim()
                    && text.chars().count() <= max
                    && !text.chars().any(char::is_control)
            };
            if !valid_id
                || !valid_slug
                || !valid_text(&creature.display_name, 100)
                || !valid_text(&creature.description, 2000)
            {
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
}
