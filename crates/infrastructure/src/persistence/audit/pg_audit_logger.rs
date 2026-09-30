use async_trait::async_trait;
use domain::audit::{AuditEntry, AuditLogger};
use domain::shared::RepositoryError;
use sqlx::PgPool;

use super::audit_writer;

/// Writes audit entries outside any transaction.
///
/// Used for authentication events, which have no business transaction to join.
/// A mutation records its entry through [`domain::identity::UserWriteRepository`]
/// instead, so the entry and the change commit together.
pub struct PgAuditLogger {
    pool: PgPool,
}

impl PgAuditLogger {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AuditLogger for PgAuditLogger {
    async fn record(&self, entry: AuditEntry) -> Result<(), RepositoryError> {
        audit_writer::insert(&self.pool, &entry).await
    }
}
