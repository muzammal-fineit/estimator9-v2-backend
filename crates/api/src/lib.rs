//! The HTTP interface layer.
//!
//! This is a library so that more than one binary can use it: `api` serves the
//! application, `openapi` dumps the spec without needing a database.

pub mod config;
pub mod http;
pub mod state;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
