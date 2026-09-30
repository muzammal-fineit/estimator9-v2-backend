use application::ApplicationError;
use application::identity::NewUser;
use axum::extract::State;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::IntoResponse;
use domain::identity::{Email, Password, PermissionName, RoleName};

use crate::http::dto::error::ErrorResponse;
use crate::http::dto::identity::CreateUserRequest;
use crate::http::{ApiError, ApiJson, AuthenticatedUser, ClientContext};
use crate::state::AppState;

/// Create a user account.
///
/// The password is supplied by the administrator and must meet the same policy
/// as any other — length only, no composition rules. `superadmin` cannot be
/// granted here; that is the vendor's account, not one a client administrator
/// mints.
#[utoipa::path(
    post,
    path = "/users",
    tag = "identity",
    security(("bearer" = [])),
    request_body = CreateUserRequest,
    responses(
        (status = 201, description = "Created; the Location header names the new account"),
        (status = 400, description = "The body is not valid JSON", body = ErrorResponse),
        (status = 401, description = "Missing, malformed or expired access token", body = ErrorResponse),
        (status = 403, description = "Lacks users.manage, or attempted to grant superadmin", body = ErrorResponse),
        (status = 404, description = "A named role does not exist", body = ErrorResponse),
        (status = 409, description = "That email address is already in use", body = ErrorResponse),
        (status = 422, description = "Email, name, password or role name rejected", body = ErrorResponse),
        (status = 503, description = "The data store is unavailable; retry", body = ErrorResponse),
    ),
)]
pub async fn create_user(
    State(state): State<AppState>,
    AuthenticatedUser(claims): AuthenticatedUser,
    ClientContext(context): ClientContext,
    ApiJson(body): ApiJson<CreateUserRequest>,
) -> Result<axum::response::Response, ApiError> {
    state
        .identity
        .authorize
        .require(&claims, PermissionName::USERS_MANAGE, context.clone())
        .await?;

    let new_user = NewUser {
        email: Email::parse(body.email).map_err(ApplicationError::Invalid)?,
        name: body.name,
        password: Password::parse(body.password).map_err(ApplicationError::Invalid)?,
        roles: body
            .roles
            .into_iter()
            .map(RoleName::parse)
            .collect::<Result<Vec<_>, _>>()
            .map_err(ApplicationError::Invalid)?,
    };

    let id = state
        .identity
        .create_user
        .execute(&claims, new_user, context)
        .await?;

    // 201 with a Location, so a client can follow up without guessing the URL.
    let location = HeaderValue::from_str(&format!("/users/{id}")).ok();

    Ok(match location {
        Some(value) => (StatusCode::CREATED, [(header::LOCATION, value)]).into_response(),
        None => StatusCode::CREATED.into_response(),
    })
}
