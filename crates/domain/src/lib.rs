//! The domain layer: entities, value objects, and the repository ports that
//! describe what persistence must provide. No I/O, no framework, no database.
//!
//! Each bounded context is a module. `shared` holds the kernel every context
//! may use — keep it small; a type belongs to a context until proven otherwise.

pub mod audit;
pub mod identity;
pub mod licensing;
pub mod notification;
pub mod shared;
