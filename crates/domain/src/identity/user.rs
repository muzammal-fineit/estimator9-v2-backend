use chrono::{DateTime, Utc};

use super::{Email, LoginControls, PasswordHash, Role, RoleName, UserId};
use crate::shared::DomainError;

/// A user account, with the roles it holds.
///
/// Fields are private on purpose: outside this module a `User` can only come
/// from a constructor that checked its invariants, so no layer above can
/// assemble one that the database would reject — and `password_hash` cannot be
/// read into a response by accident, because the DTO has to ask for it.
#[derive(Debug, Clone)]
pub struct User {
    id: UserId,
    email: Email,
    name: String,
    password_hash: PasswordHash,
    is_active: bool,
    roles: Vec<Role>,
    login_controls: LoginControls,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl User {
    /// Rebuild a user that already exists in the store. Only repository
    /// adapters should call this — it trusts the id and the timestamps.
    #[allow(clippy::too_many_arguments)]
    pub fn rehydrate(
        id: UserId,
        email: Email,
        name: String,
        password_hash: PasswordHash,
        is_active: bool,
        roles: Vec<Role>,
        login_controls: LoginControls,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        if name.trim().is_empty() {
            return Err(DomainError::invalid("name", "must not be blank"));
        }

        Ok(Self {
            id,
            email,
            name,
            password_hash,
            is_active,
            roles,
            login_controls,
            created_at,
            updated_at,
        })
    }

    pub fn id(&self) -> UserId {
        self.id
    }

    pub fn email(&self) -> &Email {
        &self.email
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn password_hash(&self) -> &PasswordHash {
        &self.password_hash
    }

    pub fn is_active(&self) -> bool {
        self.is_active
    }

    pub fn roles(&self) -> &[Role] {
        &self.roles
    }

    pub fn login_controls(&self) -> &LoginControls {
        &self.login_controls
    }

    /// Whether this account bypasses permission checks entirely.
    ///
    /// Note what this does *not* consider: `is_active`. A deactivated
    /// superadmin is still a superadmin — they simply cannot authenticate.
    /// Keeping the two checks separate stops one from masking the other.
    pub fn is_superadmin(&self) -> bool {
        self.roles.iter().any(Role::is_superadmin)
    }

    pub fn has_role(&self, name: &RoleName) -> bool {
        self.roles.iter().any(|role| role.name() == name)
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}
