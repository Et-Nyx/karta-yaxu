pub mod auth;
pub mod wiki;
pub mod models;
pub mod seed;
pub mod editor;

pub mod filters {
    pub fn slugify<T: std::fmt::Display>(s: T) -> askama::Result<String> {
        Ok(s.to_string().to_lowercase().replace(' ', "-").replace('\'', ""))
    }
}
