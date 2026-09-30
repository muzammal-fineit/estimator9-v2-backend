use domain::identity::{Role, RoleId, RoleName};
use domain::shared::RepositoryError;

/// A role joined to the user who holds it. Carries `user_id` so a batch load
/// can group roles by user without a second round trip per row.
#[derive(Debug, sqlx::FromRow)]
pub struct UserRoleRow {
    pub user_id: i64,
    pub role_id: i64,
    pub name: String,
    pub description: String,
}

impl UserRoleRow {
    pub fn into_entity(self) -> Result<Role, RepositoryError> {
        Ok(Role::rehydrate(
            RoleId::new(self.role_id),
            RoleName::parse(self.name)?,
            self.description,
        ))
    }
}
