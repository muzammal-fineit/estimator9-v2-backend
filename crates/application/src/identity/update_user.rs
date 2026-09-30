use std::sync::Arc;

use domain::audit::{Actor, AuditAction, AuditEntry, RequestContext};
use domain::identity::{Email, TokenClaims, UserId, UserRepository, UserWriteRepository};

use crate::ApplicationError;

/// The fields an administrator may change. `None` leaves one alone.
pub struct UserChanges {
    pub name: Option<String>,
    pub email: Option<Email>,
}

impl UserChanges {
    fn is_empty(&self) -> bool {
        self.name.is_none() && self.email.is_none()
    }
}

pub struct UpdateUser {
    users: Arc<dyn UserRepository>,
    admin: Arc<dyn UserWriteRepository>,
}

impl UpdateUser {
    pub fn new(users: Arc<dyn UserRepository>, admin: Arc<dyn UserWriteRepository>) -> Self {
        Self { users, admin }
    }

    pub async fn execute(
        &self,
        actor: &TokenClaims,
        target: UserId,
        changes: UserChanges,
        context: RequestContext,
    ) -> Result<(), ApplicationError> {
        // An empty patch is a client bug, and succeeding silently would write
        // an audit entry recording that nothing happened.
        if changes.is_empty() {
            return Err(ApplicationError::Invalid(
                domain::shared::DomainError::invalid("body", "no fields to change"),
            ));
        }

        let user = self
            .users
            .find(target)
            .await?
            .ok_or_else(|| ApplicationError::not_found("user", target))?;

        // The vendor's account is not the client's to rename, for the same
        // reason its roles are not theirs to change.
        if user.is_superadmin() {
            return Err(ApplicationError::Forbidden {
                permission: "superadmin accounts cannot be edited here".to_owned(),
            });
        }

        let mut entry = AuditEntry::new(
            AuditAction::USER_UPDATED,
            Actor::User {
                id: actor.user_id,
                email: actor.email.clone(),
            },
            context,
        )
        .during(actor.session_id)
        .via_superadmin(actor.is_superadmin)
        .about("user", target);

        // Only the fields that actually change are recorded, so the entry says
        // what happened rather than restating the whole row.
        if let Some(name) = &changes.name {
            entry = entry
                .with("name.before", user.name())
                .with("name.after", name.as_str());
        }

        if let Some(email) = &changes.email {
            entry = entry
                .with("email.before", user.email().as_str())
                .with("email.after", email.as_str());
        }

        self.admin
            .update(
                target,
                changes.name.as_deref(),
                changes.email.as_ref(),
                entry,
            )
            .await?;

        Ok(())
    }
}
