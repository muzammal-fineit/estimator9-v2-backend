//! Identity context: who can use the system, and what they are allowed to do.
//!
//! Authorization is modelled as users -> roles -> permissions. Callers check
//! permissions, never role names, so policy is data rather than code. The one
//! exception is [`RoleName::SUPERADMIN`], which carries no permissions and is
//! short-circuited before any lookup.

pub mod access_token;
pub mod email;
pub mod login_controls;
pub mod password;
pub mod password_hash;
pub mod password_hasher;
pub mod password_reset;
pub mod password_reset_repository;
pub mod permission;
pub mod refresh_token;
pub mod refresh_token_repository;
pub mod role;
pub mod role_name;
pub mod role_repository;
pub mod session;
pub mod session_repository;
pub mod user;
pub mod user_id;
pub mod user_repository;
pub mod user_write_repository;

pub use access_token::{AccessToken, TokenClaims, TokenError, TokenIssuer};
pub use email::Email;
pub use login_controls::LoginControls;
pub use password::Password;
pub use password_hash::PasswordHash;
pub use password_hasher::{HashingError, PasswordHasher};
pub use password_reset::{
    IssuedPasswordReset, PasswordResetId, PasswordResetSecret, StoredPasswordReset,
};
pub use password_reset_repository::PasswordResetRepository;
pub use permission::{Permission, PermissionName};
pub use refresh_token::{
    IssuedRefreshToken, RefreshTokenId, RefreshTokenSecret, StoredRefreshToken,
};
pub use refresh_token_repository::RefreshTokenRepository;
pub use role::{Role, RoleId};
pub use role_name::RoleName;
pub use role_repository::RoleRepository;
pub use session::{Session, SessionEndReason, SessionId};
pub use session_repository::SessionRepository;
pub use user::User;
pub use user_id::UserId;
pub use user_repository::UserRepository;
pub use user_write_repository::UserWriteRepository;
