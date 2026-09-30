use axum::Json;

use crate::VERSION;
use crate::http::dto::health::HealthResponse;

/// Liveness probe.
///
/// Reports the running binary's version. Does not touch the database — a 200
/// here means the process is up, not that it can serve data.
#[utoipa::path(
    get,
    path = "/health",
    tag = "meta",
    responses(
        (status = 200, description = "The process is running", body = HealthResponse),
    ),
)]
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: VERSION,
    })
}
