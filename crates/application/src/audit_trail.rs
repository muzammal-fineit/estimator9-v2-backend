//! Helpers shared by every use case that writes to the audit trail.

use domain::audit::{Actor, AuditEntry, AuditLogger};
use domain::identity::User;

/// Writes an entry, logging loudly if it fails but never failing the caller.
///
/// Correct for authentication events, which have no business transaction to
/// join: turning an audit outage into a login outage helps nobody. Mutations
/// are different — those write inside the same transaction as the change.
pub async fn record(audit: &dyn AuditLogger, entry: AuditEntry) {
    let action = entry.action;

    if let Err(err) = audit.record(entry).await {
        tracing::error!(%action, error = ?err, "could not write audit entry");
    }
}

pub fn actor_for(user: &User) -> Actor {
    Actor::User {
        id: user.id(),
        email: user.email().clone(),
    }
}
