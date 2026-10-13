//! Environment-driven configuration. No secrets are read from tracked files.

pub const DEFAULT_SESSION_TTL_SECONDS: i64 = 604_800;
pub const MAX_SESSION_TTL_SECONDS: i64 = 2_592_000;

#[derive(Debug, Clone)]
pub struct Config {
    pub bind: String,
    pub database_url: String,
    pub public_origin: String,
    pub cookie_secure: bool,
    pub session_ttl_seconds: i64,
    /// Enables the development-only placeholder identity provider.
    pub dev_auth: bool,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        Ok(Self {
            bind: env_or("KARTA_BIND", "127.0.0.1:3000"),
            database_url: env_or("KARTA_DATABASE_URL", "sqlite://data/karta.db"),
            public_origin: env_or("KARTA_PUBLIC_ORIGIN", "http://127.0.0.1:3000"),
            cookie_secure: env_flag("KARTA_COOKIE_SECURE"),
            session_ttl_seconds: match std::env::var("KARTA_SESSION_TTL_SECONDS") {
                Ok(raw) => parse_session_ttl(&raw)?,
                Err(std::env::VarError::NotPresent) => DEFAULT_SESSION_TTL_SECONDS,
                Err(_) => return Err("KARTA_SESSION_TTL_SECONDS must be valid text".to_owned()),
            },
            dev_auth: env_flag("KARTA_DEV_AUTH"),
        })
    }
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_owned())
}

fn env_flag(key: &str) -> bool {
    matches!(
        std::env::var(key).as_deref(),
        Ok("1") | Ok("true") | Ok("yes")
    )
}

fn parse_session_ttl(raw: &str) -> Result<i64, String> {
    let value = raw
        .trim()
        .parse::<i64>()
        .map_err(|_| "KARTA_SESSION_TTL_SECONDS must be an integer number of seconds".to_owned())?;
    validate_session_ttl(value)?;
    Ok(value)
}

pub(crate) fn validate_session_ttl(value: i64) -> Result<(), String> {
    if !(1..=MAX_SESSION_TTL_SECONDS).contains(&value) {
        return Err(format!(
            "Session lifetime must be between 1 and {MAX_SESSION_TTL_SECONDS} seconds (30 days); check KARTA_SESSION_TTL_SECONDS"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_lifetime_is_bounded() {
        for raw in [
            "0",
            "-1",
            "abc",
            "",
            "2592001",
            "9223372036854775807",
            "99999999999999999999",
        ] {
            assert!(parse_session_ttl(raw).is_err(), "accepted {raw}");
        }
        assert_eq!(parse_session_ttl(" 1 ").unwrap(), 1);
        assert_eq!(
            parse_session_ttl("2592000").unwrap(),
            MAX_SESSION_TTL_SECONDS
        );
        assert_eq!(
            parse_session_ttl("604800").unwrap(),
            DEFAULT_SESSION_TTL_SECONDS
        );
    }
}
