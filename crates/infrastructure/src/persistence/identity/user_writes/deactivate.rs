use domain::audit::AuditEntry;
use domain::identity::UserId;
use domain::shared::RepositoryError;
use sqlx::PgPool;

use super::queries::{DEACTIVATE_USER, END_USER_SESSIONS, REVOKE_USER_TOKENS};
use crate::persistence::audit::audit_writer;
use crate::persistence::sql_error::classify;

pub async fn execute(
    pool: &PgPool,
    user: UserId,
    entry: AuditEntry,
) -> Result<(), RepositoryError> {
    let mut tx = pool.begin().await.map_err(classify)?;

    let changed = sqlx::query(DEACTIVATE_USER)
        .bind(user.value())
        .execute(&mut *tx)
        .await
        .map_err(classify)?;

    // Either no such user or already inactive. Both mean the caller's intent
    // is already satisfied and nothing should be written — including a second
    // audit entry claiming a change that did not happen.
    if changed.rows_affected() == 0 {
        return Err(RepositoryError::NotFound {
            entity: "active user",
            id: user.to_string(),
        });
    }

    // Everything the account can still do, ended in the same transaction. A
    // deactivation that left a live session behind would be worse than none —
    // it would read as done.
    sqlx::query(END_USER_SESSIONS)
        .bind(user.value())
        .execute(&mut *tx)
        .await
        .map_err(classify)?;

    sqlx::query(REVOKE_USER_TOKENS)
        .bind(user.value())
        .execute(&mut *tx)
        .await
        .map_err(classify)?;

    audit_writer::insert(&mut *tx, &entry).await?;

    tx.commit().await.map_err(classify)?;

    Ok(())
}
