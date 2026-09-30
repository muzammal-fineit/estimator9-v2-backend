use domain::identity::{Permission, Role};
use serde::Serialize;
use utoipa::ToSchema;

/// A role the administrator can assign.
#[derive(Debug, Serialize, ToSchema)]
pub struct RoleResponse {
    #[schema(example = "analyst")]
    pub name: String,

    /// What holding it means. The only place that explains a role to someone
    /// who did not define it, so it is published rather than kept internal.
    #[schema(example = "Fits MEV models and reviews results.")]
    pub description: String,

    /// True for the vendor account, which sits outside the permission model.
    /// Published so a UI can show it as unassignable rather than offering it
    /// and taking a 403.
    pub is_superadmin: bool,
}

impl From<&Role> for RoleResponse {
    fn from(role: &Role) -> Self {
        Self {
            name: role.name().to_string(),
            description: role.description().to_owned(),
            is_superadmin: role.is_superadmin(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PermissionResponse {
    #[schema(example = "mev.fit")]
    pub name: String,

    #[schema(example = "Run MEV regression fits")]
    pub description: String,
}

impl From<&Permission> for PermissionResponse {
    fn from(permission: &Permission) -> Self {
        Self {
            name: permission.name.to_string(),
            description: permission.description.clone(),
        }
    }
}
