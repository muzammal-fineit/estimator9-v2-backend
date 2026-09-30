//! Response shapes. These are the API's published contract, kept separate from
//! the entities so renaming a domain field is not a breaking API change.
//!
//! Every type here derives `ToSchema` — this is the layer OpenAPI describes, and
//! the reason no `utoipa` annotation ever has to touch `domain`.

pub mod auth;
pub mod error;
pub mod health;
pub mod identity;
pub mod roles;
pub mod vendor;
