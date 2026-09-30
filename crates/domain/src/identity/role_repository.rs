use async_trait::async_trait;

use super::{Permission, PermissionName, Role, UserId};
use crate::shared::RepositoryError;

#[async_trait]
pub trait RoleRepository: Send + Sync + 'static {
    /// Every role, for the picker an administrator assigns from.
    async fn all(&self) -> Result<Vec<Role>, RepositoryError>;

    /// Every permission the system knows about. Reference data, changed by a
    /// seed rather than through the API.
    async fn all_permissions(&self) -> Result<Vec<Permission>, RepositoryError>;

    /// Every permission the user holds, through every role, de-duplicated.
    ///
    /// Called once at login to fill the token, not once per request. A
    /// superadmin returns an empty set — the guard never asks, because it
    /// short-circuits first.
    async fn permissions_for(&self, user: UserId) -> Result<Vec<PermissionName>, RepositoryError>;
}
