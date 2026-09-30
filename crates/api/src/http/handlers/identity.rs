//! User endpoints, one file per operation.

pub mod assign_roles;
pub mod create_user;
pub mod deactivate_user;
pub mod get_user;
pub mod list_users;
pub mod update_user;

// Glob re-exports: `#[utoipa::path]` generates a `__path_<name>` struct beside
// each handler, and `routes!(identity::get_user)` needs both in scope.
pub use assign_roles::*;
pub use create_user::*;
pub use deactivate_user::*;
pub use get_user::*;
pub use list_users::*;
pub use update_user::*;
