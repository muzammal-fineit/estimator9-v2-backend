use axum::http::{HeaderValue, Method, header};
use std::time::Duration;
use tower_http::cors::CorsLayer;

use crate::config::CorsConfig;

/// The browser policy for the Angular app.
///
/// Credentials are allowed because the refresh token is an httpOnly cookie —
/// without this the browser sends the login request happily and then silently
/// drops the cookie on `/auth/refresh`, which looks like a broken refresh
/// rather than a CORS problem.
pub fn layer(config: &CorsConfig) -> CorsLayer {
    let origins: Vec<HeaderValue> = config
        .allowed_origins
        .iter()
        .filter_map(|origin| HeaderValue::from_str(origin).ok())
        .collect();

    CorsLayer::new()
        .allow_origin(origins)
        .allow_credentials(true)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PATCH,
            Method::PUT,
            Method::DELETE,
        ])
        // Only what the client actually sends. `Authorization` for the bearer
        // token; the cookie needs no mention, browsers attach it themselves.
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
        // Without exposing these, script cannot read them even though they
        // arrive: `Retry-After` drives an interceptor's backoff and `Location`
        // names a newly created account.
        .expose_headers([header::RETRY_AFTER, header::LOCATION])
        // Preflight is cached, so a burst of calls costs one OPTIONS.
        .max_age(Duration::from_secs(600))
}
