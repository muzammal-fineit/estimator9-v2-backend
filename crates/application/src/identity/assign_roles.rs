use std::sync::Arc;

use domain::audit::{Actor, AuditAction, AuditEntry, RequestContext};
use domain::identity::{RoleName, TokenClaims, User, UserId, UserRepository, UserWriteRepository};

use crate::ApplicationError;

/// Replaces a user's roles.
///
/// The change and its audit entry are written in one transaction by the
/// repository, so a grant cannot exist without a record of who made it.
pub struct AssignRoles {
    users: Arc<dyn UserRepository>,
    admin: Arc<dyn UserWriteRepository>,
}

impl AssignRoles {
    pub fn new(users: Arc<dyn UserRepository>, admin: Arc<dyn UserWriteRepository>) -> Self {
        Self { users, admin }
    }

    pub async fn execute(
        &self,
        actor: &TokenClaims,
        target: UserId,
        roles: Vec<RoleName>,
        context: RequestContext,
    ) -> Result<(), ApplicationError> {
        // Nobody edits their own roles, not even a superadmin. Holding
        // `users.manage` would otherwise be enough to grant yourself anything
        // else, which makes every other permission boundary decorative.
        if actor.user_id == target {
            return Err(ApplicationError::Forbidden {
                permission: "another administrator must change your own roles".to_owned(),
            });
        }

        // `superadmin` is the vendor's account, not a role the client's
        // administrators hand out. Were it assignable here, anyone with
        // `users.manage` could promote themselves out of the permission system
        // entirely — the one escalation path this whole model exists to close.
        if roles.iter().any(RoleName::is_superadmin) {
            return Err(ApplicationError::Forbidden {
                permission: "superadmin cannot be granted through this endpoint".to_owned(),
            });
        }

        let user = self
            .users
            .find(target)
            .await?
            .ok_or_else(|| ApplicationError::not_found("user", target))?;

        // Removing it is barred for the same reason granting it is: a client
        // administrator must not be able to lock the vendor out of the
        // installation they support.
        if user.is_superadmin() {
            return Err(ApplicationError::Forbidden {
                permission: "superadmin roles cannot be changed through this endpoint".to_owned(),
            });
        }

        let entry = AuditEntry::new(
            AuditAction::USER_ROLES_ASSIGNED,
            actor_for_claims(actor),
            context,
        )
        .during(actor.session_id)
        .via_superadmin(actor.is_superadmin)
        .about("user", target)
        // Before and after, so the trail answers "what changed" without
        // replaying every earlier entry for this account.
        .with("roles.before", names(&user))
        .with("roles.after", joined(&roles));

        self.admin.set_roles(target, &roles, entry).await?;

        Ok(())
    }
}

/// The acting administrator, built from their token rather than re-read: the
/// claims are what authorised the call, so they are what should be recorded.
fn actor_for_claims(claims: &TokenClaims) -> Actor {
    Actor::User {
        id: claims.user_id,
        email: claims.email.clone(),
    }
}

fn names(user: &User) -> String {
    if user.roles().is_empty() {
        return "none".to_owned();
    }

    user.roles()
        .iter()
        .map(|role| role.name().as_str())
        .collect::<Vec<_>>()
        .join(",")
}

fn joined(roles: &[RoleName]) -> String {
    if roles.is_empty() {
        return "none".to_owned();
    }

    roles
        .iter()
        .map(RoleName::as_str)
        .collect::<Vec<_>>()
        .join(",")
}
