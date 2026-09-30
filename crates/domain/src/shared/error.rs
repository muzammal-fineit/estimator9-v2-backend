/// A broken invariant: the caller tried to build a domain object that cannot
/// legally exist. Always the caller's fault, never the infrastructure's.
#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("invalid {field}: {reason}")]
    Invalid { field: &'static str, reason: String },
}

impl DomainError {
    pub fn invalid(field: &'static str, reason: impl Into<String>) -> Self {
        Self::Invalid {
            field,
            reason: reason.into(),
        }
    }
}

/// What a repository port is allowed to fail with. Deliberately says nothing
/// about SQL: `Backend` boxes whatever the adapter hit, so the domain never
/// grows a dependency on the driver.
#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    /// A stored record no longer satisfies the domain's invariants.
    #[error("stored record violates a domain invariant")]
    Corrupt(#[from] DomainError),

    /// A write targeted a row that is not there. Reads return `Option`
    /// instead — absence is only an error when something was meant to change.
    #[error("{entity} {id} does not exist")]
    NotFound { entity: &'static str, id: String },

    /// A uniqueness rule was violated — an email already in use, most often.
    /// Distinct from `Backend` because the caller can fix it, and from
    /// `NotFound` because the row exists rather than not.
    #[error("{entity} {value} already exists")]
    AlreadyExists { entity: &'static str, value: String },

    /// The store could not be reached — pool exhausted, connection refused,
    /// network gone. Distinct from `Backend` because it is transient: the
    /// caller should retry, and the HTTP layer says so with a 503 rather than
    /// a 500 that reads as "this is broken".
    #[error("the data store is unavailable")]
    Unavailable(#[source] Box<dyn std::error::Error + Send + Sync>),

    #[error("repository backend failure")]
    Backend(#[source] Box<dyn std::error::Error + Send + Sync>),
}

impl RepositoryError {
    pub fn backend(source: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Backend(Box::new(source))
    }

    pub fn unavailable(source: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Unavailable(Box::new(source))
    }
}
