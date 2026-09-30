use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use domain::identity::{
    AccessToken, RefreshTokenRepository, RefreshTokenSecret, RoleRepository, SessionId,
    TokenClaims, TokenIssuer, User,
};

use crate::ApplicationError;

/// The credentials handed back after a successful authentication.
///
/// Named `IssuedSession` rather than `Session` because `domain::identity::Session`
/// is the session itself — the tracked login. This is what a client receives
/// to act within one.
pub struct IssuedSession {
    pub session_id: SessionId,
    pub token: AccessToken,
    pub expires_at: DateTime<Utc>,

    /// Handed to the client once, as an httpOnly cookie. The access token is
    /// short-lived; this is what buys a new one without a password.
    pub refresh: RefreshTokenSecret,

    pub user: User,
}

/// Turns an already-authenticated user into a signed session.
///
/// Separate from [`super::Authenticate`] because proving who someone is and
/// minting them a token are two jobs. `/auth/refresh` will do the second
/// without the first — it has a valid refresh token, so there is no password
/// to check, but the permissions must still be re-read so a role granted or
/// revoked since login takes effect.
pub struct SessionIssuer {
    roles: Arc<dyn RoleRepository>,
    tokens: Arc<dyn TokenIssuer>,
    refresh_tokens: Arc<dyn RefreshTokenRepository>,
    access_ttl: Duration,
    refresh_ttl: Duration,
}

impl SessionIssuer {
    pub fn new(
        roles: Arc<dyn RoleRepository>,
        tokens: Arc<dyn TokenIssuer>,
        refresh_tokens: Arc<dyn RefreshTokenRepository>,
        access_ttl: Duration,
        refresh_ttl: Duration,
    ) -> Self {
        Self {
            roles,
            tokens,
            refresh_tokens,
            access_ttl,
            refresh_ttl,
        }
    }

    /// Issues tokens for an existing session.
    ///
    /// The session is created by the caller — `Authenticate` starts one,
    /// `RefreshSession` passes the one it was given — because only they know
    /// whether this is a new login or a continuation.
    pub async fn issue(
        &self,
        user: User,
        session: SessionId,
        now: DateTime<Utc>,
    ) -> Result<IssuedSession, ApplicationError> {
        let permissions = if user.is_superadmin() {
            // The guard never consults the set for a superadmin, and an empty
            // list keeps the token from implying a grant that was not made.
            Vec::new()
        } else {
            self.roles.permissions_for(user.id()).await?
        };

        let expires_at = now + self.access_ttl;

        let claims = TokenClaims {
            user_id: user.id(),
            session_id: session,
            email: user.email().clone(),
            permissions,
            is_superadmin: user.is_superadmin(),
            expires_at,
        };

        let token = self.tokens.issue(&claims)?;

        let refresh = self
            .refresh_tokens
            .issue(user.id(), session, now + self.refresh_ttl)
            .await?;

        Ok(IssuedSession {
            session_id: session,
            token,
            expires_at,
            refresh: refresh.secret,
            user,
        })
    }
}
