use std::sync::Arc;

use domain::identity::{Permission, Role, RoleRepository};

use crate::ApplicationError;

/// Reads the authorization vocabulary: which roles exist and which permissions
/// they can be built from.
///
/// Both are reference data, changed by a seed rather than through the API, so
/// this is read-only by design — there is no endpoint to invent a permission
/// the code does not check for.
pub struct ListRoles {
    roles: Arc<dyn RoleRepository>,
}

impl ListRoles {
    pub fn new(roles: Arc<dyn RoleRepository>) -> Self {
        Self { roles }
    }

    pub async fn roles(&self) -> Result<Vec<Role>, ApplicationError> {
        Ok(self.roles.all().await?)
    }

    pub async fn permissions(&self) -> Result<Vec<Permission>, ApplicationError> {
        Ok(self.roles.all_permissions().await?)
    }
}
