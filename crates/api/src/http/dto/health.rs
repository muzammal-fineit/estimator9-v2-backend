use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    #[schema(example = "ok")]
    pub status: &'static str,

    /// The running binary's `CARGO_PKG_VERSION`.
    #[schema(example = "0.1.0")]
    pub version: &'static str,
}
