//! Audit context: an append-only record of what the application did and who
//! asked for it.
//!
//! Its own context rather than part of `shared`, because it has entities of
//! its own and every other context will use it. Entries are written by the
//! application, never by a database trigger — a trigger sees `UPDATE users`,
//! the application knows it was `user.roles.assigned` and why.

pub mod action;
pub mod context;
pub mod entry;
pub mod logger;
pub mod metadata;

pub use action::{AuditAction, LoginFailure};
pub use context::{Actor, RequestContext};
pub use entry::AuditEntry;
pub use logger::AuditLogger;
pub use metadata::AuditMetadata;
