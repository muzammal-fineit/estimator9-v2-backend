use std::sync::Arc;

use domain::audit::{Actor, AuditAction, AuditEntry, RequestContext};
use domain::identity::{TokenClaims, UserRepository, UserWriteRepository};

use crate::ApplicationError;

/// Lets a user change their own display name.
///
/// Name only. The email address is the login identifier, and letting someone
/// change it unverified turns one typo into a locked-out account with no way
/// back — that belongs behind an administrator, or behind a confirmation flow
/// that does not exist yet.
pub struct UpdateProfile {
    users: Arc<dyn UserRepository>,
    writes: Arc<dyn UserWriteRepository>,
}

impl UpdateProfile {
    pub fn new(users: Arc<dyn UserRepository>, writes: Arc<dyn UserWriteRepository>) -> Self {
        Self { users, writes }
    }

    pub async fn execute(
        &self,
        actor: &TokenClaims,
        name: String,
        context: RequestContext,
    ) -> Result<(), ApplicationError> {
        let user = self
            .users
            .find(actor.user_id)
            .await?
            .ok_or(ApplicationError::Unauthenticated)?;

        let entry = AuditEntry::new(
            AuditAction::USER_UPDATED,
            Actor::User {
                id: user.id(),
                email: user.email().clone(),
            },
            context,
        )
        .during(actor.session_id)
        .via_superadmin(actor.is_superadmin)
        .about("user", user.id())
        // Actor and subject being the same id is what marks this as
        // self-service rather than an administrator's edit.
        .with("name.before", user.name())
        .with("name.after", name.as_str());

        self.writes
            .update(user.id(), Some(&name), None, entry)
            .await?;

        Ok(())
    }
}
