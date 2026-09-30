//! Writes that change a user.
//!
//! Each operation is one transaction covering both the change and its audit
//! entry, so a change cannot exist without a record of who made it. One file
//! per write; `roles::replace` is shared because creating a user and changing
//! their roles do the same thing to `user_roles`.
//!
//! Both administrator actions and self-service land here — the rules about
//! which caller may do what live in the use cases, not in the SQL.

pub mod change_password;
pub mod create;
pub mod deactivate;
pub mod queries;
pub mod reset_password;
pub mod roles;
pub mod set_roles;
pub mod update;

use async_trait::async_trait;
use domain::audit::AuditEntry;
use domain::identity::{
    Email, PasswordHash, PasswordResetId, RoleName, UserId, UserWriteRepository,
};
use domain::shared::RepositoryError;
use sqlx::PgPool;

pub struct PgUserWriteRepository {
    pool: PgPool,
}

impl PgUserWriteRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserWriteRepository for PgUserWriteRepository {
    async fn create(
        &self,
        email: &Email,
        name: &str,
        password: &PasswordHash,
        roles: &[RoleName],
        entry: AuditEntry,
    ) -> Result<UserId, RepositoryError> {
        create::execute(&self.pool, email, name, password, roles, entry).await
    }

    async fn update(
        &self,
        user: UserId,
        name: Option<&str>,
        email: Option<&Email>,
        entry: AuditEntry,
    ) -> Result<(), RepositoryError> {
        update::execute(&self.pool, user, name, email, entry).await
    }

    async fn change_password(
        &self,
        user: UserId,
        password: &PasswordHash,
        entry: AuditEntry,
    ) -> Result<(), RepositoryError> {
        change_password::execute(&self.pool, user, password, entry).await
    }

    async fn reset_password(
        &self,
        user: UserId,
        token: PasswordResetId,
        password: &PasswordHash,
        entry: AuditEntry,
    ) -> Result<(), RepositoryError> {
        reset_password::execute(&self.pool, user, token, password, entry).await
    }

    async fn deactivate(&self, user: UserId, entry: AuditEntry) -> Result<(), RepositoryError> {
        deactivate::execute(&self.pool, user, entry).await
    }

    async fn set_roles(
        &self,
        user: UserId,
        roles: &[RoleName],
        entry: AuditEntry,
    ) -> Result<(), RepositoryError> {
        set_roles::execute(&self.pool, user, roles, entry).await
    }
}
