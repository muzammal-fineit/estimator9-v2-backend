use std::sync::Arc;

use chrono::{DateTime, Utc};
use domain::audit::{Actor, AuditAction, AuditEntry, RequestContext};
use domain::identity::{
    Password, PasswordHasher, SessionRepository, TokenClaims, UserRepository, UserWriteRepository,
};

use super::{IssuedSession, SessionIssuer};
use crate::ApplicationError;

pub struct PasswordChange {
    pub current: Password,
    pub replacement: Password,
}

/// Changes the caller's own password and re-establishes their session.
///
/// Every session ends, including this one; a new one is issued immediately so
/// the person who asked stays signed in while every other device does not.
pub struct ChangePassword {
    users: Arc<dyn UserRepository>,
    writes: Arc<dyn UserWriteRepository>,
    hasher: Arc<dyn PasswordHasher>,
    sessions: Arc<dyn SessionRepository>,
    issuer: Arc<SessionIssuer>,
}

impl ChangePassword {
    pub fn new(
        users: Arc<dyn UserRepository>,
        writes: Arc<dyn UserWriteRepository>,
        hasher: Arc<dyn PasswordHasher>,
        sessions: Arc<dyn SessionRepository>,
        issuer: Arc<SessionIssuer>,
    ) -> Self {
        Self {
            users,
            writes,
            hasher,
            sessions,
            issuer,
        }
    }

    pub async fn execute(
        &self,
        actor: &TokenClaims,
        change: PasswordChange,
        context: RequestContext,
        now: DateTime<Utc>,
    ) -> Result<IssuedSession, ApplicationError> {
        let user = self
            .users
            .find(actor.user_id)
            .await?
            .ok_or(ApplicationError::Unauthenticated)?;

        // The current password is required even though the caller holds a
        // valid token: a token is something a borrowed laptop already has, and
        // this is the operation that would lock its owner out.
        let correct = self
            .hasher
            .verify(&change.current, user.password_hash())
            .await?;

        if !correct {
            return Err(ApplicationError::InvalidCredentials);
        }

        // No check that the new password differs from the old. Refusing would
        // confirm a guess to anyone who reached this point, and re-hashing the
        // same value costs nothing worth saving.
        let hash = self.hasher.hash(&change.replacement).await?;

        let entry = AuditEntry::new(
            AuditAction::PASSWORD_CHANGED,
            Actor::User {
                id: user.id(),
                email: user.email().clone(),
            },
            context.clone(),
        )
        // The session that made the request, which is about to end with it.
        .during(actor.session_id)
        .via_superadmin(actor.is_superadmin)
        .about("user", user.id());

        self.writes.change_password(user.id(), &hash, entry).await?;

        // Only now, after every old session is gone, so the replacement cannot
        // be caught by the revocation it follows.
        let session = self.sessions.start(user.id(), &context, now).await?;

        self.issuer.issue(user, session.id, now).await
    }
}
