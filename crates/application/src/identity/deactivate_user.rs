use std::sync::Arc;

use domain::audit::{Actor, AuditAction, AuditEntry, RequestContext};
use domain::identity::{TokenClaims, UserId, UserRepository, UserWriteRepository};

use crate::ApplicationError;

/// Deactivates an account, ending its sessions in the same transaction.
pub struct DeactivateUser {
    users: Arc<dyn UserRepository>,
    admin: Arc<dyn UserWriteRepository>,
}

impl DeactivateUser {
    pub fn new(users: Arc<dyn UserRepository>, admin: Arc<dyn UserWriteRepository>) -> Self {
        Self { users, admin }
    }

    pub async fn execute(
        &self,
        actor: &TokenClaims,
        target: UserId,
        context: RequestContext,
    ) -> Result<(), ApplicationError> {
        // Locking yourself out is not a permission question, it is an
        // accident. If it is genuinely wanted, another administrator can do it.
        if actor.user_id == target {
            return Err(ApplicationError::Forbidden {
                permission: "an account cannot deactivate itself".to_owned(),
            });
        }

        let user = self
            .users
            .find(target)
            .await?
            .ok_or_else(|| ApplicationError::not_found("user", target))?;

        // The vendor's account is not the client's to disable — it is how the
        // installation is supported.
        if user.is_superadmin() {
            return Err(ApplicationError::Forbidden {
                permission: "superadmin accounts cannot be deactivated here".to_owned(),
            });
        }

        let entry = AuditEntry::new(
            AuditAction::USER_DEACTIVATED,
            Actor::User {
                id: actor.user_id,
                email: actor.email.clone(),
            },
            context,
        )
        .during(actor.session_id)
        .via_superadmin(actor.is_superadmin)
        .about("user", target)
        .with("email", user.email().as_str());

        self.admin.deactivate(target, entry).await?;

        Ok(())
    }
}
