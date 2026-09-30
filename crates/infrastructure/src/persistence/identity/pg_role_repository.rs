use async_trait::async_trait;
use domain::identity::{
    Permission, PermissionName, Role, RoleId, RoleName, RoleRepository, UserId,
};
use domain::shared::RepositoryError;
use sqlx::PgPool;

/// DISTINCT because two of a user's roles may grant the same permission; the
/// token should carry it once. Ordered so the claim set is stable, which keeps
/// tokens comparable in tests and logs.
const SELECT_ROLES: &str = "SELECT id, name, description FROM roles ORDER BY name";

const SELECT_PERMISSIONS: &str = "SELECT name, description FROM permissions ORDER BY name";

const SELECT_PERMISSIONS_FOR_USER: &str = "SELECT DISTINCT p.name \
                                           FROM user_roles ur \
                                           JOIN role_permissions rp ON rp.role_id = ur.role_id \
                                           JOIN permissions p ON p.id = rp.permission_id \
                                           WHERE ur.user_id = $1 \
                                           ORDER BY p.name";

pub struct PgRoleRepository {
    pool: PgPool,
}

impl PgRoleRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl RoleRepository for PgRoleRepository {
    async fn all(&self) -> Result<Vec<Role>, RepositoryError> {
        let rows: Vec<(i64, String, String)> = sqlx::query_as(SELECT_ROLES)
            .fetch_all(&self.pool)
            .await
            .map_err(crate::persistence::sql_error::classify)?;

        rows.into_iter()
            .map(|(id, name, description)| {
                Ok(Role::rehydrate(
                    RoleId::new(id),
                    RoleName::parse(name)?,
                    description,
                ))
            })
            .collect()
    }

    async fn all_permissions(&self) -> Result<Vec<Permission>, RepositoryError> {
        let rows: Vec<(String, String)> = sqlx::query_as(SELECT_PERMISSIONS)
            .fetch_all(&self.pool)
            .await
            .map_err(crate::persistence::sql_error::classify)?;

        rows.into_iter()
            .map(|(name, description)| {
                Ok(Permission {
                    name: PermissionName::parse(name)?,
                    description,
                })
            })
            .collect()
    }

    async fn permissions_for(&self, user: UserId) -> Result<Vec<PermissionName>, RepositoryError> {
        let names: Vec<String> = sqlx::query_scalar(SELECT_PERMISSIONS_FOR_USER)
            .bind(user.value())
            .fetch_all(&self.pool)
            .await
            .map_err(crate::persistence::sql_error::classify)?;

        names
            .into_iter()
            .map(|name| PermissionName::parse(name).map_err(RepositoryError::from))
            .collect()
    }
}
