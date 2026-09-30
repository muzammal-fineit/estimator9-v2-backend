use domain::audit::AuditEntry;
use domain::identity::{RoleName, UserId};
use domain::shared::RepositoryError;
use sqlx::PgPool;

use super::roles;
use crate::persistence::audit::audit_writer;
use crate::persistence::sql_error::classify;

pub async fn execute(
    pool: &PgPool,
    user: UserId,
    role_names: &[RoleName],
    entry: AuditEntry,
) -> Result<(), RepositoryError> {
    let mut tx = pool.begin().await.map_err(classify)?;

    roles::replace(&mut tx, user.value(), role_names).await?;

    // The audit entry rides the same transaction. If the commit fails, neither
    // the grant nor its record exists; there is no ordering in which one can
    // survive without the other.
    audit_writer::insert(&mut *tx, &entry).await?;

    tx.commit().await.map_err(classify)?;

    Ok(())
}
