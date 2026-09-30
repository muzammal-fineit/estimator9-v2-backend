//! The application layer: one type per use case, each holding the ports it
//! needs and exposing a single `execute`. Orchestration lives here; business
//! rules live in `domain`; HTTP and SQL live outside both.

pub mod audit_trail;
pub mod error;
pub mod identity;
pub mod licensing;

pub use error::ApplicationError;
