use domain::identity::{HashingError, TokenError};
use domain::licensing::StoreError;
use domain::shared::{DomainError, RepositoryError};

/// What a use case can fail with. The HTTP layer maps these onto status codes;
/// nothing here knows that status codes exist.
#[derive(Debug, thiserror::Error)]
pub enum ApplicationError {
    #[error("{resource} {id} not found")]
    NotFound { resource: &'static str, id: String },

    /// Every login rejection, whatever the underlying reason. Unknown address,
    /// wrong password, locked account and disabled account all produce this —
    /// distinguishing them lets a caller enumerate accounts.
    #[error("invalid credentials")]
    InvalidCredentials,

    /// The password was correct but the account is locked. Only ever returned
    /// to someone who has already proven they know the password — a caller who
    /// has not cannot tell this apart from `InvalidCredentials`.
    #[error("account locked until {until}")]
    AccountLocked {
        until: chrono::DateTime<chrono::Utc>,
    },

    /// No usable token: absent, malformed, expired, or wrongly signed. The
    /// client's move is to refresh and retry.
    #[error("authentication required")]
    Unauthenticated,

    /// Authenticated, but lacking the permission. Retrying will not help, and
    /// an Angular interceptor needs that distinction from `Unauthenticated`.
    #[error("permission {permission} is required")]
    Forbidden { permission: String },

    #[error(transparent)]
    Invalid(#[from] DomainError),

    #[error(transparent)]
    Repository(#[from] RepositoryError),

    #[error(transparent)]
    Hashing(#[from] HashingError),

    #[error(transparent)]
    Token(#[from] TokenError),

    /// The feature state could not be read. Never falls back to defaults —
    /// that would turn tampering into a way to change behaviour.
    #[error(transparent)]
    FeatureStore(#[from] StoreError),
}

impl ApplicationError {
    pub fn not_found(resource: &'static str, id: impl std::fmt::Display) -> Self {
        Self::NotFound {
            resource,
            id: id.to_string(),
        }
    }
}
