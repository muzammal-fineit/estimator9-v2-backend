use async_trait::async_trait;

use super::AuditEntry;
use crate::shared::RepositoryError;

/// Writes audit entries.
///
/// Authentication events are recorded through this port on a best-effort
/// basis: there is no business transaction for a failed login to join, and
/// refusing to serve because the trail could not be written would turn an
/// audit outage into an outage.
///
/// Mutations are different. When user administration arrives, its entries are
/// written inside the same transaction as the change, so a role grant cannot
/// exist without a record of who made it.
#[async_trait]
pub trait AuditLogger: Send + Sync + 'static {
    async fn record(&self, entry: AuditEntry) -> Result<(), RepositoryError>;
}
