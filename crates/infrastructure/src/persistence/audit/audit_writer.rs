use std::collections::BTreeMap;

use domain::audit::AuditEntry;
use domain::shared::RepositoryError;
use sqlx::types::ipnetwork::IpNetwork;
use sqlx::{Executor, Postgres};

use crate::persistence::sql_error::classify;

const INSERT_ENTRY: &str = "INSERT INTO audit_log \
                            (actor_id, actor_email, via_superadmin, action, \
                             resource_type, resource_id, metadata, ip_address, user_agent, \
                             session_id) \
                            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)";

/// Writes one entry using whatever executor it is handed.
///
/// Generic over the executor on purpose: authentication events pass the pool
/// and are best effort, while a mutation passes its own transaction so the
/// entry and the change commit together. Both use this one statement, so the
/// two paths cannot drift into writing different columns.
pub async fn insert<'c, E>(executor: E, entry: &AuditEntry) -> Result<(), RepositoryError>
where
    E: Executor<'c, Database = Postgres>,
{
    let (resource_type, resource_id) = match entry.resource {
        Some((kind, ref id)) => (Some(kind), Some(id.as_str())),
        None => (None, None),
    };

    // The domain keeps metadata as a flat string map so it needs no
    // serialisation crate; turning it into jsonb is this layer's job.
    let metadata: BTreeMap<&str, &str> = entry
        .metadata
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();

    sqlx::query(INSERT_ENTRY)
        .bind(entry.actor.id().map(|id| id.value()))
        .bind(entry.actor.email().map(|email| email.as_str()))
        .bind(entry.via_superadmin)
        .bind(entry.action.as_str())
        .bind(resource_type)
        .bind(resource_id)
        .bind(sqlx::types::Json(metadata))
        .bind(entry.context.ip.map(IpNetwork::from))
        .bind(entry.context.user_agent.as_deref())
        .bind(entry.session.map(|id| id.value()))
        .execute(executor)
        .await
        .map_err(classify)?;

    Ok(())
}
