use super::RoleName;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RoleId(i64);

impl RoleId {
    pub fn new(value: i64) -> Self {
        Self(value)
    }

    pub fn value(self) -> i64 {
        self.0
    }
}

/// A named bundle of permissions.
///
/// Deliberately does *not* carry its permissions. Listing users would otherwise
/// have to expand every role's permission set for every row, when the only
/// place that expansion is needed is at login.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Role {
    id: RoleId,
    name: RoleName,
    description: String,
}

impl Role {
    /// Infallible: `RoleName` already carries the only invariant a role has.
    pub fn rehydrate(id: RoleId, name: RoleName, description: String) -> Self {
        Self {
            id,
            name,
            description,
        }
    }

    pub fn id(&self) -> RoleId {
        self.id
    }

    pub fn name(&self) -> &RoleName {
        &self.name
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn is_superadmin(&self) -> bool {
        self.name.is_superadmin()
    }
}
