use domain::audit::LoginFailure;
use domain::identity::TokenError;
use domain::shared::RepositoryError;

use crate::ApplicationError;

/// Separates "the credentials were refused" from "something broke".
///
/// The two must not be confused: a refusal is audited with its reason and then
/// flattened to a single indistinguishable response, while a failure is a real
/// error that should surface as a 500. Without a type saying which is which, a
/// database outage during login would quietly look like a wrong password.
pub enum Rejected {
    Refused(LoginFailure),

    /// The password was right and the account is locked. Audited like any
    /// other refusal, but reported distinctly — see `Authenticate`.
    Locked {
        until: chrono::DateTime<chrono::Utc>,
    },

    Failed(ApplicationError),
}

impl From<LoginFailure> for Rejected {
    fn from(reason: LoginFailure) -> Self {
        Self::Refused(reason)
    }
}

// Listed one by one rather than as a blanket `impl<E: Into<ApplicationError>>`,
// which would overlap with the impl above — the compiler cannot rule out a
// future `LoginFailure: Into<ApplicationError>`. These are the error types the
// `?` operator actually meets in the use case.
impl From<RepositoryError> for Rejected {
    fn from(err: RepositoryError) -> Self {
        Self::Failed(err.into())
    }
}

impl From<TokenError> for Rejected {
    fn from(err: TokenError) -> Self {
        Self::Failed(err.into())
    }
}

impl From<ApplicationError> for Rejected {
    fn from(err: ApplicationError) -> Self {
        Self::Failed(err)
    }
}
