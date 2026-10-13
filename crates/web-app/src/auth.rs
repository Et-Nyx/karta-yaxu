//! Authentication boundary. Use cases depend on this port, never on a concrete
//! provider. A future provider also needs its credential-specific HTTP bootstrap
//! and explicit mapping to preserve existing local account ids.

pub mod placeholder;

use async_trait::async_trait;

/// Capability granted to an account. Defaults are intentionally minimal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Viewer,
    Editor,
    Administrator,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Viewer => "viewer",
            Role::Editor => "editor",
            Role::Administrator => "administrator",
        }
    }

    pub fn parse(value: &str) -> Option<Role> {
        match value {
            "viewer" => Some(Role::Viewer),
            "editor" => Some(Role::Editor),
            "administrator" => Some(Role::Administrator),
            _ => None,
        }
    }
}

/// A provider-verified identity. The provider id plus provider subject is the
/// stable external key; the local user id is assigned separately.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subject {
    pub provider: String,
    pub provider_subject: String,
    pub display_name: String,
}

/// A provider-specific sign-in attempt. New variants arrive with new providers.
#[derive(Debug, Clone)]
pub enum Credential {
    /// Development placeholder: an operator-seeded local username.
    DevUsername(String),
}

#[derive(Debug, PartialEq, Eq)]
pub enum AuthError {
    /// The submitted credential is malformed or wrong (safe to explain).
    InvalidCredential(&'static str),
    NotInvited,
    Disabled,
    /// Internal failure (database/storage); not the user's fault.
    Unavailable,
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::InvalidCredential(message) => write!(f, "{message}"),
            AuthError::NotInvited => write!(f, "This account has not been invited."),
            AuthError::Disabled => write!(f, "This account is disabled."),
            AuthError::Unavailable => write!(f, "Sign-in is temporarily unavailable."),
        }
    }
}

/// An authenticated local account, resolved from a session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Actor {
    pub user_id: String,
    pub display_name: String,
    pub role: Role,
}

/// The authentication seam. Implementations only prove identity; they never
/// touch accounts, sessions, or authorization policy.
#[async_trait]
pub trait IdentityProvider: Send + Sync {
    fn id(&self) -> &'static str;
    async fn authenticate(&self, credential: Credential) -> Result<Subject, AuthError>;
}
