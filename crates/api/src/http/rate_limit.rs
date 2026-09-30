use std::net::IpAddr;
use std::sync::Arc;

use axum::Json;
use axum::http::{Request, StatusCode, header};
use axum::response::{IntoResponse, Response};
use tower_governor::GovernorLayer;
use tower_governor::errors::GovernorError;
use tower_governor::governor::GovernorConfigBuilder;
use tower_governor::key_extractor::KeyExtractor;
use utoipa_axum::router::OpenApiRouter;

use super::client_ip::trusted_client_ip;
use super::dto::error::ErrorResponse;
use crate::state::AppState;

/// Keys buckets on the caller's address, from the sources
/// [`trusted_client_ip`] trusts.
///
/// Not `SmartIpKeyExtractor`: that one reads `X-Forwarded-For` first, and this
/// deployment's nginx *appends* to that header rather than replacing it. A
/// caller could vary it per request and never meet the limit.
#[derive(Clone, Copy)]
pub struct TrustedIpExtractor;

impl KeyExtractor for TrustedIpExtractor {
    type Key = IpAddr;

    fn extract<T>(&self, req: &Request<T>) -> Result<Self::Key, GovernorError> {
        trusted_client_ip(req.headers(), req.extensions()).ok_or(GovernorError::UnableToExtractKey)
    }
}

/// Applies a per-address quota of `burst` requests per `window` to `routes`.
///
/// Builds and attaches the layer in one place so its concrete type never has
/// to be written down — the middleware type parameter comes from the
/// `governor` crate, which is not a direct dependency here.
///
/// The quota replenishes steadily rather than resetting on the minute; a fixed
/// window lets an attacker send a full quota either side of the boundary.
/// Returns `routes` unchanged if the quota is unusable, which config
/// validation already rejects.
pub fn throttle(
    routes: OpenApiRouter<AppState>,
    burst: u32,
    window: std::time::Duration,
) -> OpenApiRouter<AppState> {
    let window_ms = u64::try_from(window.as_millis()).unwrap_or(u64::MAX);

    let period_ms = match window_ms.checked_div(u64::from(burst)) {
        Some(ms) => ms.max(1),
        None => return routes,
    };

    let Some(config) = GovernorConfigBuilder::default()
        .key_extractor(TrustedIpExtractor)
        .per_millisecond(period_ms)
        .burst_size(burst)
        .finish()
    else {
        return routes;
    };

    routes.layer(GovernorLayer::new(Arc::new(config)).error_handler(on_limited))
}

/// Rate-limit rejections use the same body shape as every other error, so a
/// client parses failures one way.
fn on_limited(err: GovernorError) -> Response {
    match err {
        GovernorError::TooManyRequests { wait_time, .. } => {
            let mut response = (
                StatusCode::TOO_MANY_REQUESTS,
                Json(ErrorResponse::new("too many requests")),
            )
                .into_response();

            if let Ok(value) = wait_time.to_string().parse() {
                response.headers_mut().insert(header::RETRY_AFTER, value);
            }

            response
        }

        // No usable address: neither a proxy header nor peer info. That means
        // the connect-info layer is misconfigured, which is an operator
        // problem rather than a caller problem.
        GovernorError::UnableToExtractKey => {
            tracing::warn!("rate limiter could not determine the caller's address");

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("internal server error")),
            )
                .into_response()
        }

        GovernorError::Other { code, msg, .. } => {
            tracing::error!(?msg, "rate limiter failure");

            (code, Json(ErrorResponse::new("internal server error"))).into_response()
        }
    }
}
