use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{FromRequest, Request};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::de::DeserializeOwned;

use super::dto::error::ErrorResponse;

/// `Json`, but rejections come back in the API's own error shape.
///
/// Axum's built-in rejection returns a bare `text/plain` body, so a client
/// parsing `{"error": "..."}` everywhere else gets something different the
/// moment it sends a stray comma. One shape for every failure is the contract;
/// this keeps it true at the edge where the body has not parsed yet.
pub struct ApiJson<T>(pub T);

impl<S, T> FromRequest<S> for ApiJson<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = JsonError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        Json::<T>::from_request(req, state)
            .await
            .map(|Json(value)| Self(value))
            .map_err(JsonError)
    }
}

pub struct JsonError(JsonRejection);

impl IntoResponse for JsonError {
    fn into_response(self) -> Response {
        let (status, message) = match &self.0 {
            // Not JSON at all. The request was not understood, so 400.
            JsonRejection::JsonSyntaxError(_) => (
                StatusCode::BAD_REQUEST,
                "the request body is not valid JSON".to_owned(),
            ),

            // Valid JSON, wrong shape — a missing field, a number where a
            // string belongs. Understood and refused on its content, so 422,
            // matching how a domain rejection is reported.
            JsonRejection::JsonDataError(err) => {
                (StatusCode::UNPROCESSABLE_ENTITY, err.body_text())
            }

            // No `Content-Type: application/json`.
            JsonRejection::MissingJsonContentType(_) => (
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "expected Content-Type: application/json".to_owned(),
            ),

            other => (other.status(), other.body_text()),
        };

        (status, Json(ErrorResponse::new(message))).into_response()
    }
}
