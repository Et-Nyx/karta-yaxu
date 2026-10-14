//! Accounts and sessions application service.
//!
//! Enforces invite-only membership, stable local user ids, and opaque server
//! sessions. Depends on the identity-provider port for proof and on the
//! persistence layer for storage. Membership administration methods are trusted
//! local operator use cases; they must not be exposed directly to the browser.

use crate::{
    auth::{Actor, AuthError, Role, Subject},
    config::validate_session_ttl,
    persistence::{self as store, UserRow},
};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct Accounts {
    pool: SqlitePool,
    session_ttl_seconds: i64,
}

impl Accounts {
    pub fn new(pool: SqlitePool, session_ttl_seconds: i64) -> Result<Self, String> {
        validate_session_ttl(session_ttl_seconds)?;
        Ok(Self {
            pool,
            session_ttl_seconds,
        })
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub fn session_ttl(&self) -> i64 {
        self.session_ttl_seconds
    }

    /// Resolve a provider-verified subject to a local account, claiming a
    /// matching invite on first sign-in. The user, invite consumption, and
    /// (for new users) creation commit in one transaction.
    pub async fn sign_in(&self, subject: Subject) -> Result<Actor, AuthError> {
        let mut tx = store::begin_write(&self.pool)
            .await
            .map_err(auth_storage_error)?;
        let now = unix_now()?;

        if let Some(user) =
            store::user_by_subject(&mut *tx, &subject.provider, &subject.provider_subject)
                .await
                .map_err(auth_storage_error)?
        {
            if user.is_disabled != 0 {
                return Err(AuthError::Disabled);
            }
            tx.commit().await.map_err(auth_storage_error)?;
            return actor_from(user).ok_or(AuthError::Unavailable);
        }

        let invite = store::pending_invite(&mut *tx, &subject.provider, &subject.provider_subject)
            .await
            .map_err(auth_storage_error)?
            .ok_or(AuthError::NotInvited)?;
        let role = Role::parse(&invite.role).ok_or(AuthError::Unavailable)?;
        let user_id = random_hex(16).map_err(|_| AuthError::Unavailable)?;

        store::insert_user(
            &mut *tx,
            &user_id,
            &subject.provider,
            &subject.provider_subject,
            &subject.display_name,
            role.as_str(),
            now,
        )
        .await
        .map_err(auth_storage_error)?;
        store::consume_invite(&mut *tx, &invite.id, &user_id, now)
            .await
            .map_err(auth_storage_error)?;
        tx.commit().await.map_err(auth_storage_error)?;

        Ok(Actor {
            user_id,
            display_name: subject.display_name,
            role,
        })
    }

    /// Create a new server-side session. Returns the raw token (only its hash
    /// is stored) and its expiry.
    pub async fn start_session(&self, user_id: &str) -> Result<(String, i64), AuthError> {
        let token = random_hex(32).map_err(|_| AuthError::Unavailable)?;
        let mut tx = store::begin_write(&self.pool)
            .await
            .map_err(auth_storage_error)?;
        let user = store::user_by_id(&mut *tx, user_id)
            .await
            .map_err(auth_storage_error)?
            .ok_or(AuthError::NotInvited)?;
        if user.is_disabled != 0 {
            return Err(AuthError::Disabled);
        }
        let now = unix_now()?;
        let expires_at = now
            .checked_add(self.session_ttl_seconds)
            .ok_or(AuthError::Unavailable)?;
        store::insert_session(&mut *tx, &hash_token(&token), user_id, now, expires_at)
            .await
            .map_err(auth_storage_error)?;
        tx.commit().await.map_err(auth_storage_error)?;
        Ok((token, expires_at))
    }

    /// Resolve an opaque session token to an actor, or `None` when the session
    /// is missing, expired, revoked, or belongs to a disabled account.
    pub async fn authenticate(&self, token: &str) -> Result<Option<Actor>, AuthError> {
        let row = store::session_with_user(&self.pool, &hash_token(token))
            .await
            .map_err(auth_storage_error)?;
        let Some(row) = row else {
            return Ok(None);
        };
        if row.revoked_at.is_some() || row.expires_at <= unix_now()? || row.is_disabled != 0 {
            return Ok(None);
        }
        let role = Role::parse(&row.role).ok_or(AuthError::Unavailable)?;
        Ok(Some(Actor {
            user_id: row.user_id,
            display_name: row.display_name,
            role,
        }))
    }

    /// Authoring checks sessions on its already-locked transaction connection.
    pub async fn authenticate_in_transaction(
        &self,
        connection: &mut sqlx::SqliteConnection,
        token: &str,
    ) -> Result<Option<Actor>, AuthError> {
        let row = store::session_with_user(connection, &hash_token(token))
            .await
            .map_err(auth_storage_error)?;
        let Some(row) = row else {
            return Ok(None);
        };
        if row.revoked_at.is_some() || row.expires_at <= unix_now()? || row.is_disabled != 0 {
            return Ok(None);
        }
        Ok(Some(Actor {
            user_id: row.user_id,
            display_name: row.display_name,
            role: Role::parse(&row.role).ok_or(AuthError::Unavailable)?,
        }))
    }

    pub async fn sign_out(&self, token: &str) -> Result<(), AuthError> {
        store::revoke_session(&self.pool, &hash_token(token), unix_now()?)
            .await
            .map_err(auth_storage_error)
    }

    /// Trusted operator operation: pre-authorize an identity for first sign-in.
    pub async fn invite(
        &self,
        provider: &str,
        subject: &str,
        role: Role,
        note: Option<&str>,
    ) -> Result<String, String> {
        let id = random_hex(16).map_err(|_| "randomness unavailable".to_owned())?;
        let mut tx = store::begin_write(&self.pool)
            .await
            .map_err(operator_storage_error)?;
        if store::user_by_subject(&mut *tx, provider, subject)
            .await
            .map_err(operator_storage_error)?
            .is_some()
        {
            return Err("An account already exists for that identity. Invitations do not change account roles.".to_owned());
        }
        store::insert_invite(
            &mut *tx,
            &id,
            provider,
            subject,
            role.as_str(),
            note,
            unix_now().map_err(|error| error.to_string())?,
        )
        .await
        .map_err(|error| {
            if error
                .as_database_error()
                .is_some_and(|e| e.is_unique_violation())
            {
                "A pending invite already exists for that subject.".to_owned()
            } else {
                operator_storage_error(error)
            }
        })?;
        tx.commit().await.map_err(operator_storage_error)?;
        Ok(id)
    }

    /// Trusted operator operation. Disabling and session revocation are atomic.
    pub async fn set_disabled(&self, user_id: &str, disabled: bool) -> Result<(), String> {
        let mut tx = store::begin_write(&self.pool)
            .await
            .map_err(operator_storage_error)?;
        let now = unix_now().map_err(|error| error.to_string())?;
        if !store::set_disabled(&mut *tx, user_id, disabled, now)
            .await
            .map_err(operator_storage_error)?
        {
            return Err("Account not found. Use an immutable user id from list-users.".to_owned());
        }
        if disabled {
            store::revoke_user_sessions(&mut *tx, user_id, now)
                .await
                .map_err(operator_storage_error)?;
        }
        tx.commit().await.map_err(operator_storage_error)
    }

    pub async fn users(&self) -> Result<Vec<UserRow>, String> {
        store::list_users(&self.pool)
            .await
            .map_err(operator_storage_error)
    }

    pub async fn invites(&self) -> Result<Vec<store::InviteRow>, String> {
        store::list_invites(&self.pool)
            .await
            .map_err(operator_storage_error)
    }
}

fn actor_from(user: UserRow) -> Option<Actor> {
    Some(Actor {
        user_id: user.id,
        display_name: user.display_name,
        role: Role::parse(&user.role)?,
    })
}

fn unix_now() -> Result<i64, AuthError> {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| AuthError::Unavailable)?
        .as_secs();
    i64::try_from(seconds).map_err(|_| AuthError::Unavailable)
}

fn auth_storage_error(error: sqlx::Error) -> AuthError {
    // Error messages can embed database values. Log only fixed categories/codes.
    match error {
        sqlx::Error::Database(error) => eprintln!(
            "Authentication storage failure: database code {}",
            error.code().as_deref().unwrap_or("unknown")
        ),
        sqlx::Error::PoolTimedOut => eprintln!("Authentication storage failure: pool timeout"),
        sqlx::Error::PoolClosed => eprintln!("Authentication storage failure: pool closed"),
        _ => eprintln!("Authentication storage failure: query or connection error"),
    }
    AuthError::Unavailable
}

fn operator_storage_error(error: sqlx::Error) -> String {
    auth_storage_error(error);
    "Account operation could not be completed because storage is unavailable. Check the server diagnostics and retry.".to_owned()
}

fn random_hex(bytes: usize) -> Result<String, ()> {
    let mut buffer = vec![0u8; bytes];
    getrandom::fill(&mut buffer).map_err(|_| ())?;
    Ok(hex::encode(buffer))
}

fn hash_token(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    hex::encode(digest)
}
