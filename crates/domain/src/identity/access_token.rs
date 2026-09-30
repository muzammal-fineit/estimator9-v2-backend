use chrono::{DateTime, Utc};

use super::{Email, PermissionName, SessionId, UserId};

/// A signed token, opaque to everything above this port.
#[derive(Clone, PartialEq, Eq)]
pub struct AccessToken(String);

impl AccessToken {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A bearer token is a credential: anyone holding one can act as its subject
/// until it expires. It is redacted for the same reason a password hash is.
impl std::fmt::Debug for AccessToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AccessToken(<redacted>)")
    }
}

/// What a verified token asserts.
///
/// Permissions are carried in the token rather than looked up per request, so
/// a revoked role keeps working until the token expires. That is the trade the
/// short TTL exists to bound.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenClaims {
    pub user_id: UserId,

    /// The login this token belongs to. Carried so that an audit entry for
    /// anything done with this token can name its session — without it the
    /// join only reaches login, refresh and logout, which is most of the
    /// value of having sessions at all.
    pub session_id: SessionId,

    pub email: Email,
    pub permissions: Vec<PermissionName>,
    pub is_superadmin: bool,
    pub expires_at: DateTime<Utc>,
}

impl TokenClaims {
    /// Superadmin short-circuits before the permission set is consulted, which
    /// is why that role is granted no permissions in the seed.
    pub fn allows(&self, permission: &PermissionName) -> bool {
        self.is_superadmin || self.permissions.contains(permission)
    }
}

/// Issuing and verifying access tokens.
///
/// Synchronous: signing is a single HMAC over a few hundred bytes, unlike
/// password hashing there is nothing here worth moving off the runtime.
pub trait TokenIssuer: Send + Sync + 'static {
    fn issue(&self, claims: &TokenClaims) -> Result<AccessToken, TokenError>;

    fn verify(&self, raw: &str) -> Result<TokenClaims, TokenError>;
}

#[derive(Debug, thiserror::Error)]
pub enum TokenError {
    /// Well-formed and correctly signed, but past its expiry.
    #[error("the token has expired")]
    Expired,

    /// Malformed, signed with the wrong key, or using an algorithm this
    /// issuer does not accept. Deliberately one variant: telling a caller
    /// *which* is telling an attacker how close they are.
    #[error("the token is not valid")]
    Invalid,

    #[error("the token could not be issued")]
    Backend(#[source] Box<dyn std::error::Error + Send + Sync>),
}

impl TokenError {
    pub fn backend(source: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Backend(Box::new(source))
    }
}
