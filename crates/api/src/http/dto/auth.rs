use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::identity::UserResponse;

#[derive(Deserialize, ToSchema)]
pub struct LoginRequest {
    #[schema(example = "akmal@tech.fineit.io")]
    pub email: String,

    #[schema(example = "correct horse battery staple")]
    pub password: String,
}

/// `Debug` is hand-written; a derived one would print the password into any
/// rejection log that includes the request body.
impl std::fmt::Debug for LoginRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoginRequest")
            .field("email", &self.email)
            .field("password", &"<redacted>")
            .finish()
    }
}

/// Returned by both `/auth/login` and `/auth/refresh` — the same thing is
/// being handed over in each case.
#[derive(Debug, Serialize, ToSchema)]
pub struct SessionResponse {
    /// Send as `Authorization: Bearer <access_token>`.
    pub access_token: String,

    #[schema(example = "Bearer")]
    pub token_type: &'static str,

    /// When the token stops being accepted. Short by design — permissions are
    /// carried inside it, so this bounds how long a revoked role keeps working.
    pub expires_at: DateTime<Utc>,

    /// Saves the client an immediate follow-up call to render a header.
    pub user: UserResponse,
}

/// A password change. Both values go through the same policy check.
#[derive(Deserialize, ToSchema)]
pub struct ChangePasswordRequest {
    pub current_password: String,

    /// At least 12 characters.
    pub new_password: String,
}

/// Hand-written so neither password can reach a log through a rejection that
/// includes the request body.
impl std::fmt::Debug for ChangePasswordRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChangePasswordRequest")
            .field("current_password", &"<redacted>")
            .field("new_password", &"<redacted>")
            .finish()
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateProfileRequest {
    #[schema(example = "Test Analyst")]
    pub name: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ForgotPasswordRequest {
    #[schema(example = "akmal@tech.fineit.io")]
    pub email: String,
}

/// The token from the emailed link, plus the password to set.
#[derive(Deserialize, ToSchema)]
pub struct ResetPasswordRequest {
    pub token: String,

    /// At least 12 characters.
    pub new_password: String,
}

/// Hand-written: the token is a credential until it is spent, and the password
/// is one afterwards. Neither belongs in a log.
impl std::fmt::Debug for ResetPasswordRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResetPasswordRequest")
            .field("token", &"<redacted>")
            .field("new_password", &"<redacted>")
            .finish()
    }
}
