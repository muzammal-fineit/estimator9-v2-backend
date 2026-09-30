use std::sync::Arc;

use domain::audit::{Actor, AuditAction, AuditEntry, RequestContext};
use domain::identity::{
    Email, Password, PasswordHasher, RoleName, TokenClaims, UserId, UserWriteRepository,
};

use crate::ApplicationError;

pub struct NewUser {
    pub email: Email,
    pub name: String,
    pub password: Password,
    pub roles: Vec<RoleName>,
}

pub struct CreateUser {
    admin: Arc<dyn UserWriteRepository>,
    hasher: Arc<dyn PasswordHasher>,
}

impl CreateUser {
    pub fn new(admin: Arc<dyn UserWriteRepository>, hasher: Arc<dyn PasswordHasher>) -> Self {
        Self { admin, hasher }
    }

    pub async fn execute(
        &self,
        actor: &TokenClaims,
        new_user: NewUser,
        context: RequestContext,
    ) -> Result<UserId, ApplicationError> {
        // Same rail as role assignment: `superadmin` is the vendor's account,
        // not something a client administrator can mint. Creating an account
        // that holds it would route straight around the check on assignment.
        if new_user.roles.iter().any(RoleName::is_superadmin) {
            return Err(ApplicationError::Forbidden {
                permission: "superadmin cannot be granted through this endpoint".to_owned(),
            });
        }

        let hash = self.hasher.hash(&new_user.password).await?;

        let entry = AuditEntry::new(
            AuditAction::USER_CREATED,
            Actor::User {
                id: actor.user_id,
                email: actor.email.clone(),
            },
            context,
        )
        .during(actor.session_id)
        .via_superadmin(actor.is_superadmin)
        // The address, so the trail identifies the account even after a later
        // rename. Never the password or its hash — `AuditMetadata` would
        // redact those anyway, and they have no business being offered.
        .with("email", new_user.email.as_str())
        .with("name", new_user.name.as_str())
        .with("roles", joined(&new_user.roles));

        let id = self
            .admin
            .create(
                &new_user.email,
                &new_user.name,
                &hash,
                &new_user.roles,
                entry,
            )
            .await?;

        Ok(id)
    }
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
