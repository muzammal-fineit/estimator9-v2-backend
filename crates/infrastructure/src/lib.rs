//! The infrastructure layer: concrete adapters for the ports in `domain`.
//!
//! Everything that knows SQL exists lives below this line. Rows are mapped to
//! entities at the edge of `persistence`, so no `sqlx` type ever travels
//! upward into `application` or `api`.

pub mod database;
pub mod licensing;
pub mod notification;
pub mod persistence;
pub mod security;

pub use database::{PgPool, connect};
