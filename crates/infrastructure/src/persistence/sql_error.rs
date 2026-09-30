use domain::shared::RepositoryError;

/// Classifies a `sqlx` failure so the HTTP layer can tell "try again shortly"
/// from "something is wrong".
///
/// Every repository maps through here rather than reaching for
/// `RepositoryError::backend` directly, so the distinction is made once.
pub fn classify(err: sqlx::Error) -> RepositoryError {
    match err {
        // Transient: the database is up but we could not get to it, or the
        // pool is saturated. A client retry is reasonable and likely to work.
        sqlx::Error::PoolTimedOut
        | sqlx::Error::PoolClosed
        | sqlx::Error::Io(_)
        | sqlx::Error::Tls(_) => RepositoryError::unavailable(err),

        // Everything else — a bad query, a constraint violation, a decode
        // failure — is a defect. Retrying will produce the same result.
        other => RepositoryError::backend(other),
    }
}
