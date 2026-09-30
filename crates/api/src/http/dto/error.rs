use serde::Serialize;
use utoipa::ToSchema;

/// The body every failing request returns. One shape for all errors, so a
/// client can parse failures without branching on the status code.
#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorResponse {
    /// Human-readable description of what went wrong.
    #[schema(example = "user 999999 not found")]
    pub error: String,
}

impl ErrorResponse {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            error: message.into(),
        }
    }
}
