use std::sync::Arc;

use chrono::{DateTime, Utc};
use domain::audit::{AuditAction, AuditEntry, RequestContext};
use domain::identity::{
    Password, PasswordHasher, PasswordResetRepository, PasswordResetSecret, UserRepository,
    UserWriteRepository,
};

use crate::ApplicationError;
use crate::audit_trail::actor_for;

/// Redeems a reset link and sets a new password.
///
/// Returns nothing: after a reset the user logs in normally. Signing them in
/// automatically would mean a link forwarded to the wrong inbox grants a live
/// session rather than merely the chance to set a password.
pub struct ResetPassword {
    users: Arc<dyn UserRepository>,
    resets: Arc<dyn PasswordResetRepository>,
    writes: Arc<dyn UserWriteRepository>,
    hasher: Arc<dyn PasswordHasher>,
}

impl ResetPassword {
    pub fn new(
        users: Arc<dyn UserRepository>,
        resets: Arc<dyn PasswordResetRepository>,
        writes: Arc<dyn UserWriteRepository>,
        hasher: Arc<dyn PasswordHasher>,
    ) -> Self {
        Self {
            users,
            resets,
            writes,
            hasher,
        }
    }

    pub async fn execute(
        &self,
        secret: PasswordResetSecret,
        password: Password,
        context: RequestContext,
        now: DateTime<Utc>,
    ) -> Result<(), ApplicationError> {
        // Unknown, expired and already-used all answer the same way. Telling
        // them apart would say whether a link was ever real, and to whom.
        let Some(stored) = self.resets.find(&secret).await? else {
            return Err(ApplicationError::InvalidCredentials);
        };

        if !stored.is_usable_at(now) {
            return Err(ApplicationError::InvalidCredentials);
        }

        let Some(user) = self.users.find(stored.user_id).await? else {
            return Err(ApplicationError::InvalidCredentials);
        };

        if !user.is_active() {
            return Err(ApplicationError::InvalidCredentials);
        }

        let hash = self.hasher.hash(&password).await?;

        let entry = AuditEntry::new(
            AuditAction::PASSWORD_RESET_COMPLETED,
            actor_for(&user),
            context,
        )
        .about("user", user.id());

        // Consuming the token, setting the hash and ending every session are
        // one transaction. A token spent without the password changing would
        // lock the user out of their own recovery.
        self.writes
            .reset_password(user.id(), stored.id, &hash, entry)
            .await?;

        Ok(())
    }
}
