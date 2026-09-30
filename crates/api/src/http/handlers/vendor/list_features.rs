use axum::Json;
use axum::extract::State;

use crate::http::dto::error::ErrorResponse;
use crate::http::dto::vendor::FeatureResponse;
use crate::http::{ApiError, AuthenticatedUser, ClientContext};
use crate::state::AppState;

/// List the software features and whether they are enabled.
///
/// Vendor only. A client administrator gets 403 however privileged they are —
/// there is no permission that grants this, deliberately.
#[utoipa::path(
    get,
    path = "/vendor/features",
    tag = "vendor",
    security(("bearer" = [])),
    responses(
        (status = 200, description = "The catalogue with current state", body = [FeatureResponse]),
        (status = 401, description = "Missing, malformed or expired access token", body = ErrorResponse),
        (status = 403, description = "Not the vendor account", body = ErrorResponse),
        (status = 500, description = "The stored feature state is unreadable or has been tampered with", body = ErrorResponse),
    ),
)]
pub async fn list_features(
    State(state): State<AppState>,
    AuthenticatedUser(claims): AuthenticatedUser,
    ClientContext(context): ClientContext,
) -> Result<Json<Vec<FeatureResponse>>, ApiError> {
    state
        .identity
        .authorize
        .require_superadmin(&claims, context)
        .await?;

    let features = state.licensing.features.list().await?;

    Ok(Json(
        features.into_iter().map(FeatureResponse::from).collect(),
    ))
}
