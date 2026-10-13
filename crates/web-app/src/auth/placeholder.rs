//! Development-only placeholder provider.
//!
//! This performs no real authentication: any username accepted by
//! [`normalize`] maps to a subject. Access is still gated by invite-only
//! membership in the accounts service. It must never be enabled outside local
//! development and is gated behind `KARTA_DEV_AUTH=1`.

use super::{AuthError, Credential, IdentityProvider, Subject};
use async_trait::async_trait;

pub const PROVIDER_ID: &str = "placeholder";

const MAX_LEN: usize = 64;

#[derive(Debug, Default, Clone, Copy)]
pub struct PlaceholderProvider;

impl PlaceholderProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl IdentityProvider for PlaceholderProvider {
    fn id(&self) -> &'static str {
        PROVIDER_ID
    }

    async fn authenticate(&self, credential: Credential) -> Result<Subject, AuthError> {
        match credential {
            Credential::DevUsername(raw) => {
                let name = normalize(&raw)?;
                Ok(Subject {
                    provider: PROVIDER_ID.to_owned(),
                    provider_subject: name.clone(),
                    display_name: name,
                })
            }
        }
    }
}

/// Normalize a placeholder username or explain why it is unusable.
pub fn normalize(raw: &str) -> Result<String, AuthError> {
    let name = raw.trim();
    if name.is_empty() {
        return Err(AuthError::InvalidCredential("Enter a username."));
    }
    if name.chars().count() > MAX_LEN {
        return Err(AuthError::InvalidCredential(
            "That username is too long (64 characters maximum).",
        ));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    {
        return Err(AuthError::InvalidCredential(
            "Use only letters, digits, hyphen, underscore, or period.",
        ));
    }
    Ok(name.to_ascii_lowercase())
}
