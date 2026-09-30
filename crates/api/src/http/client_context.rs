use std::convert::Infallible;

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use domain::audit::RequestContext;

use super::client_ip::trusted_client_ip;

/// Builds the [`RequestContext`] that goes into an audit entry.
///
/// Uses the same address source as the rate limiter — see
/// [`trusted_client_ip`] for why `X-Forwarded-For` is ignored. An audit row
/// naming an address the caller chose would be worse than one naming none.
///
/// Never fails: an unknown address is a legitimate state, not a reason to
/// refuse a request.
pub struct ClientContext(pub RequestContext);

impl<S: Send + Sync> FromRequestParts<S> for ClientContext {
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(Self(RequestContext {
            ip: trusted_client_ip(&parts.headers, &parts.extensions),
            user_agent: parts
                .headers
                .get(axum::http::header::USER_AGENT)
                .and_then(|value| value.to_str().ok())
                // Bounded: this is caller-controlled text going into storage.
                .map(|value| value.chars().take(256).collect()),
        }))
    }
}
