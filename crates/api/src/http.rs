//! The HTTP interface: the only place that knows about routes, status codes,
//! and JSON. Entities never leave here — they are mapped to DTOs first.

pub mod authenticated_user;
pub mod client_context;
pub mod client_ip;
pub mod cors;
pub mod dto;
pub mod error;
pub mod handlers;
pub mod json;
pub mod openapi;
pub mod rate_limit;
pub mod refresh_cookie;
pub mod routes;

pub use authenticated_user::AuthenticatedUser;
pub use client_context::ClientContext;
pub use error::ApiError;
pub use json::ApiJson;
pub use routes::router;
