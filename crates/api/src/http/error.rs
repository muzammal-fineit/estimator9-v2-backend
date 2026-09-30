use application::ApplicationError;
use axum::Json;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use domain::shared::RepositoryError;

use super::dto::error::ErrorResponse;

/// The single place where a failure becomes a status code.
///
/// Adding a variant to `ApplicationError` makes this match fail to compile,
/// which is the point. The status codes listed in each handler's
/// `#[utoipa::path]` `responses(...)` must agree with what happens here — that
/// is the one pairing OpenAPI cannot check.
pub struct ApiError(ApplicationError);

impl From<ApplicationError> for ApiError {
    fn from(err: ApplicationError) -> Self {
        Self(err)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = classify(&self.0);

        let mut response = (status, Json(ErrorResponse::new(message))).into_response();

        // RFC 7235 requires a 401 to say how to authenticate. Without it a
        // strict client has no way to know a bearer token is what is wanted.
        if status == StatusCode::UNAUTHORIZED {
            response
                .headers_mut()
                .insert(header::WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"));
        }

        // Transient failures are worth retrying, and saying how soon is more
        // use to a client than making it guess.
        if status == StatusCode::SERVICE_UNAVAILABLE {
            response
                .headers_mut()
                .insert(header::RETRY_AFTER, HeaderValue::from_static("5"));
        }

        response
    }
}

fn classify(err: &ApplicationError) -> (StatusCode, String) {
    match err {
        ApplicationError::NotFound { .. } => (StatusCode::NOT_FOUND, err.to_string()),

        // One response for every login rejection. The reason is audited,
        // never returned — see `Authenticate` for why.
        ApplicationError::InvalidCredentials => {
            (StatusCode::UNAUTHORIZED, "invalid credentials".to_owned())
        }

        // 423 Locked is precise and, unlike 401, will not send an Angular
        // interceptor into a refresh-and-retry loop over something that will
        // not clear until the lock expires.
        ApplicationError::AccountLocked { until } => (
            StatusCode::LOCKED,
            format!(
                "account locked until {}",
                until.format("%Y-%m-%d %H:%M:%S UTC")
            ),
        ),

        // 401 means "authenticate and try again"; 403 means "do not bother".
        // An Angular interceptor needs the difference: one triggers a refresh
        // and retry, the other must not, or a permission error becomes a loop.
        ApplicationError::Unauthenticated => (StatusCode::UNAUTHORIZED, err.to_string()),
        ApplicationError::Forbidden { .. } => (StatusCode::FORBIDDEN, err.to_string()),

        // Syntactically valid, semantically rejected — a password under the
        // minimum length, an address that is not one. 422, not 400: the
        // request was understood and refused on its content.
        ApplicationError::Invalid(_) => (StatusCode::UNPROCESSABLE_ENTITY, err.to_string()),

        // A corrupt stored hash is a data problem with one row, but to the
        // caller it must look like any other failed login.
        ApplicationError::Hashing(inner) => {
            tracing::error!(error = ?inner, "password hashing failure");
            (StatusCode::UNAUTHORIZED, "invalid credentials".to_owned())
        }

        ApplicationError::Token(inner) => {
            tracing::error!(error = ?inner, "token failure");
            internal()
        }

        // Refusing rather than falling back to defaults is the point: a
        // fallback would make editing the file a way to change behaviour
        // instead of a way to break the installation visibly.
        ApplicationError::FeatureStore(inner) => {
            tracing::error!(error = ?inner, "feature state unreadable");
            internal()
        }

        ApplicationError::Repository(inner) => from_repository(inner),
    }
}

fn from_repository(err: &RepositoryError) -> (StatusCode, String) {
    match err {
        // A write aimed at a row that is not there. Previously a 500, which
        // told the client its own request was fine when it was not.
        RepositoryError::NotFound { entity, id } => (
            StatusCode::NOT_FOUND,
            format!("{entity} {id} does not exist"),
        ),

        // The caller can fix this one, so it is a 4xx. 409 rather than 422:
        // the payload is well formed, it conflicts with what already exists.
        RepositoryError::AlreadyExists { entity, value } => (
            StatusCode::CONFLICT,
            format!("{entity} {value} already exists"),
        ),

        // The database is unreachable rather than broken. 503 and a
        // Retry-After say "come back", where 500 says "give up".
        RepositoryError::Unavailable(inner) => {
            tracing::error!(error = ?inner, "data store unavailable");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                "the service is temporarily unavailable".to_owned(),
            )
        }

        RepositoryError::Corrupt(inner) => {
            tracing::error!(error = ?inner, "stored record violates a domain invariant");
            internal()
        }

        RepositoryError::Backend(inner) => {
            tracing::error!(error = ?inner, "repository failure");
            internal()
        }
    }
}

/// Never leaks the cause. The detail is in the log, where an operator can see
/// it and a caller cannot.
fn internal() -> (StatusCode, String) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        "internal server error".to_owned(),
    )
}
