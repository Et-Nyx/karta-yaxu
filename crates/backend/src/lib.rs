pub mod auth;
pub mod wiki;
pub mod models;
pub mod seed;
pub mod editor;
pub mod error;
pub mod search;

pub mod filters {
    /// Askama-compatible slugify filter for use in templates.
    pub fn slugify<T: std::fmt::Display>(s: T) -> askama::Result<String> {
        Ok(to_slug(&s.to_string()))
    }

    /// Canonical slug conversion used by handlers and templates alike.
    /// Handles: lowercase, strip non-alphanumeric, collapse hyphens, trim edges.
    pub fn to_slug(name: &str) -> String {
        let raw = name.to_lowercase();
        let slug: String = raw
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect();
        // Collapse consecutive hyphens and trim leading/trailing ones
        slug.split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-")
    }
}

#[cfg(test)]
mod tests {
    use super::filters::to_slug;

    #[test]
    fn test_to_slug_basic() {
        assert_eq!(to_slug("Bulbasaur"), "bulbasaur");
        assert_eq!(to_slug("Charizard"), "charizard");
    }

    #[test]
    fn test_to_slug_with_spaces() {
        assert_eq!(to_slug("Mr. Mime"), "mr-mime");
        assert_eq!(to_slug("Type: Null"), "type-null");
        assert_eq!(to_slug("Tapu Koko"), "tapu-koko");
    }

    #[test]
    fn test_to_slug_special_characters() {
        assert_eq!(to_slug("Farfetch'd"), "farfetch-d");
        assert_eq!(to_slug("Porygon-Z"), "porygon-z");
        assert_eq!(to_slug("Ho-Oh"), "ho-oh");
        assert_eq!(to_slug("Flabébé"), "flab-b");
    }

    #[test]
    fn test_to_slug_multiple_hyphens() {
        // Ensure consecutive hyphens are collapsed
        assert_eq!(to_slug("Nidoran♀"), "nidoran"); // non-ascii replaced by hyphen
        assert_eq!(to_slug("Nidoran♂"), "nidoran");
    }
}
