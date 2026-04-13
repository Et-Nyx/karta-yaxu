pub mod auth;
pub mod wiki;
pub mod models;
pub mod seed;
pub mod editor;
pub mod error;

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
