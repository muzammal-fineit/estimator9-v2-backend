use chrono::{DateTime, Utc};
use domain::identity::{Role, User};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// A user as the API publishes them.
///
/// There is deliberately no password field of any kind. Because `User` keeps
/// its hash private and this struct is written by hand, adding one would have
/// to be a conscious act rather than a `#[derive(Serialize)]` accident.
#[derive(Debug, Serialize, ToSchema)]
pub struct UserResponse {
    #[schema(example = 1)]
    pub id: i64,

    #[schema(example = "analyst@fineit.io")]
    pub email: String,

    #[schema(example = "Test Analyst")]
    pub name: String,

    /// Deactivated accounts are retained, not deleted.
    #[schema(example = true)]
    pub is_active: bool,

    /// Role names, alphabetically. Permissions are not expanded here — they are
    /// issued in the access token at login.
    #[schema(example = json!(["analyst"]))]
    pub roles: Vec<String>,

    /// True when the account holds `superadmin` and therefore bypasses every
    /// permission check.
    #[schema(example = false)]
    pub is_superadmin: bool,

    #[schema(example = "2026-09-18T12:41:29.785Z")]
    pub created_at: DateTime<Utc>,

    #[schema(example = "2026-09-18T12:41:29.785Z")]
    pub updated_at: DateTime<Utc>,
}

/// A new account.
#[derive(Deserialize, ToSchema)]
pub struct CreateUserRequest {
    #[schema(example = "analyst@fineit.io")]
    pub email: String,

    #[schema(example = "Test Analyst")]
    pub name: String,

    /// At least 12 characters. Never echoed back and never audited.
    #[schema(example = "correct horse battery staple")]
    pub password: String,

    /// May be empty; the account then has no permissions until roles are
    /// assigned.
    #[schema(example = json!(["analyst"]))]
    pub roles: Vec<String>,
}

/// `Debug` is hand-written so the password cannot reach a log through a
/// rejection that includes the request body.
impl std::fmt::Debug for CreateUserRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CreateUserRequest")
            .field("email", &self.email)
            .field("name", &self.name)
            .field("password", &"<redacted>")
            .field("roles", &self.roles)
            .finish()
    }
}

/// Fields to change. Omit one to leave it alone.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateUserRequest {
    #[schema(example = "Test Analyst")]
    pub name: Option<String>,

    #[schema(example = "analyst@fineit.io")]
    pub email: Option<String>,
}

/// The complete set of roles the user should end up with.
#[derive(Debug, Deserialize, ToSchema)]
pub struct AssignRolesRequest {
    /// Exhaustive, not additive — an empty list removes every role.
    #[schema(example = json!(["analyst", "viewer"]))]
    pub roles: Vec<String>,
}

impl From<&User> for UserResponse {
    fn from(user: &User) -> Self {
        Self {
            id: user.id().value(),
            email: user.email().to_string(),
            name: user.name().to_owned(),
            is_active: user.is_active(),
            roles: user
                .roles()
                .iter()
                .map(|role: &Role| role.name().to_string())
                .collect(),
            is_superadmin: user.is_superadmin(),
            created_at: user.created_at(),
            updated_at: user.updated_at(),
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use domain::identity::{Email, LoginControls, PasswordHash, Role, RoleId, RoleName, UserId};

    use super::*;

    const SECRET: &str = "$argon2id$v=19$m=19456,t=2,p=1$c29tZXNhbHQ$hunter2hash";

    fn user_with(roles: Vec<Role>) -> User {
        User::rehydrate(
            UserId::new(1),
            Email::parse("analyst@fineit.io").unwrap(),
            "Test Analyst".to_owned(),
            PasswordHash::parse(SECRET).unwrap(),
            true,
            roles,
            LoginControls::clear(),
            Utc::now(),
            Utc::now(),
        )
        .unwrap()
    }

    fn role(name: &str) -> Role {
        Role::rehydrate(
            RoleId::new(1),
            RoleName::parse(name).unwrap(),
            String::new(),
        )
    }

    #[test]
    fn the_password_hash_never_reaches_the_response() {
        let json = serde_json::to_string(&UserResponse::from(&user_with(vec![]))).unwrap();

        assert!(!json.contains(SECRET));
        assert!(!json.contains("argon2"));
        assert!(!json.contains("password"));
    }

    #[test]
    fn reports_superadmin_separately_from_the_role_list() {
        let response = UserResponse::from(&user_with(vec![role("superadmin")]));

        assert!(response.is_superadmin);
        assert_eq!(response.roles, ["superadmin"]);

        let plain = UserResponse::from(&user_with(vec![role("analyst")]));

        assert!(!plain.is_superadmin);
    }
}
