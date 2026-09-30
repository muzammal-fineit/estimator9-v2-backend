use axum::extract::{Path, State};
use axum::http::StatusCode;

use crate::http::dto::error::ErrorResponse;
use crate::http::dto::vendor::SetFeatureRequest;
use crate::http::{ApiError, ApiJson, AuthenticatedUser, ClientContext};
use crate::state::AppState;

/// Enable or disable a feature for this installation.
///
/// Vendor only, and audited with `via_superadmin` so the change is visible to
/// the client's auditor even though it never touches their database.
///
/// Setting a feature to the state it already has does nothing and records
/// nothing.
#[utoipa::path(
    put,
    path = "/vendor/features/{name}",
    tag = "vendor",
    security(("bearer" = [])),
    params(
        ("name" = String, Path, description = "Feature name from the catalogue", example = "mev.fitting"),
    ),
    request_body = SetFeatureRequest,
    responses(
        (status = 204, description = "Applied"),
        (status = 400, description = "The body is not valid JSON", body = ErrorResponse),
        (status = 401, description = "Missing, malformed or expired access token", body = ErrorResponse),
        (status = 403, description = "Not the vendor account", body = ErrorResponse),
        (status = 404, description = "This release has no such feature", body = ErrorResponse),
        (status = 500, description = "The stored feature state is unreadable or has been tampered with", body = ErrorResponse),
    ),
)]
pub async fn set_feature(
    State(state): State<AppState>,
    AuthenticatedUser(claims): AuthenticatedUser,
    ClientContext(context): ClientContext,
    Path(name): Path<String>,
    ApiJson(body): ApiJson<SetFeatureRequest>,
) -> Result<StatusCode, ApiError> {
    state
        .identity
        .authorize
        .require_superadmin(&claims, context.clone())
        .await?;

    state
        .licensing
        .features
        .set(&claims, &name, body.enabled, context)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}
