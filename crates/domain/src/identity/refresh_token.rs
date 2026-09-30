use chrono::{DateTime, Utc};

use super::{SessionId, UserId};

/// The opaque value handed to the client, and never stored.
///
/// Redacted in `Debug` for the same reason a password hash is: anyone holding
/// it can mint access tokens until it expires.
#[derive(Clone, PartialEq, Eq)]
pub struct RefreshTokenSecret(String);

impl RefreshTokenSecret {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for RefreshTokenSecret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RefreshTokenSecret(<redacted>)")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RefreshTokenId(i64);

impl RefreshTokenId {
    pub fn new(value: i64) -> Self {
        Self(value)
    }

    pub fn value(self) -> i64 {
        self.0
    }
}

/// What the store knows about a token. Never includes the secret.
#[derive(Debug, Clone)]
pub struct StoredRefreshToken {
    pub id: RefreshTokenId,
    pub user_id: UserId,
    /// The login this token belongs to. Rotation keeps it; a new login
    /// starts a new session and therefore a new chain.
    pub session_id: SessionId,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

impl StoredRefreshToken {
    /// Revoked *and presented again* is the signal that matters: a token is
    /// revoked the moment it is exchanged, so a second use means either a
    /// thief replaying it or the rightful holder arriving after one. Neither
    /// can be told apart, and both warrant killing the family.
    pub fn was_already_used(&self) -> bool {
        self.revoked_at.is_some()
    }

    pub fn is_expired_at(&self, now: DateTime<Utc>) -> bool {
        self.expires_at <= now
    }

    pub fn is_usable_at(&self, now: DateTime<Utc>) -> bool {
        !self.was_already_used() && !self.is_expired_at(now)
    }
}

/// A freshly issued token: the record, plus the one and only chance to see the
/// secret.
#[derive(Debug)]
pub struct IssuedRefreshToken {
    pub secret: RefreshTokenSecret,
    pub stored: StoredRefreshToken,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(seconds: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(seconds, 0).unwrap_or_default()
    }

    fn token(revoked_at: Option<DateTime<Utc>>) -> StoredRefreshToken {
        StoredRefreshToken {
            id: RefreshTokenId::new(1),
            user_id: UserId::new(1),
            session_id: SessionId::new(1),
            expires_at: at(1_000),
            revoked_at,
        }
    }

    #[test]
    fn a_fresh_token_is_usable_until_its_expiry() {
        assert!(token(None).is_usable_at(at(999)));
        assert!(!token(None).is_usable_at(at(1_000)));
        assert!(!token(None).is_usable_at(at(1_001)));
    }

    #[test]
    fn an_exchanged_token_is_never_usable_again() {
        let used = token(Some(at(500)));

        assert!(used.was_already_used());
        assert!(!used.is_usable_at(at(600)));
    }

    #[test]
    fn the_secret_never_renders_in_debug_output() {
        let secret = RefreshTokenSecret::new("2f6a9c...");

        assert_eq!(format!("{secret:?}"), "RefreshTokenSecret(<redacted>)");
    }
}
