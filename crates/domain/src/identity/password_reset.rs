use chrono::{DateTime, Utc};

use super::UserId;

/// The value that goes into the email, and is never stored.
#[derive(Clone, PartialEq, Eq)]
pub struct PasswordResetSecret(String);

impl PasswordResetSecret {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn expose(&self) -> &str {
        &self.0
    }
}

/// Anyone holding it can take over the account until it expires, so it is
/// redacted for the same reason a password hash is.
impl std::fmt::Debug for PasswordResetSecret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PasswordResetSecret(<redacted>)")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PasswordResetId(i64);

impl PasswordResetId {
    pub fn new(value: i64) -> Self {
        Self(value)
    }

    pub fn value(self) -> i64 {
        self.0
    }
}

#[derive(Debug, Clone)]
pub struct StoredPasswordReset {
    pub id: PasswordResetId,
    pub user_id: UserId,
    pub expires_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
}

impl StoredPasswordReset {
    pub fn is_usable_at(&self, now: DateTime<Utc>) -> bool {
        self.used_at.is_none() && self.expires_at > now
    }
}

#[derive(Debug)]
pub struct IssuedPasswordReset {
    pub secret: PasswordResetSecret,
    pub stored: StoredPasswordReset,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(seconds: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(seconds, 0).unwrap_or_default()
    }

    fn reset(used_at: Option<DateTime<Utc>>) -> StoredPasswordReset {
        StoredPasswordReset {
            id: PasswordResetId::new(1),
            user_id: UserId::new(1),
            expires_at: at(1_000),
            used_at,
        }
    }

    #[test]
    fn usable_until_it_expires() {
        assert!(reset(None).is_usable_at(at(999)));
        assert!(!reset(None).is_usable_at(at(1_000)));
    }

    #[test]
    fn never_usable_twice() {
        assert!(!reset(Some(at(500))).is_usable_at(at(600)));
    }

    #[test]
    fn the_secret_never_renders_in_debug_output() {
        assert_eq!(
            format!("{:?}", PasswordResetSecret::new("abc")),
            "PasswordResetSecret(<redacted>)"
        );
    }
}
