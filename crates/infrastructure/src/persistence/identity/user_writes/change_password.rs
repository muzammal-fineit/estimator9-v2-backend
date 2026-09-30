use domain::audit::AuditEntry;
use domain::identity::{PasswordHash, UserId};
use domain::shared::RepositoryError;
use sqlx::PgPool;

use super::queries::{CHANGE_PASSWORD, END_SESSIONS_PASSWORD, REVOKE_USER_TOKENS};
use crate::persistence::audit::audit_writer;
use crate::persistence::sql_error::classify;

pub async fn execute(
    pool: &PgPool,
    user: UserId,
    password: &PasswordHash,
    entry: AuditEntry,
) -> Result<(), RepositoryError> {
    let mut tx = pool.begin().await.map_err(classify)?;

    let changed = sqlx::query(CHANGE_PASSWORD)
        .bind(user.value())
        .bind(password.as_str())
        .execute(&mut *tx)
        .await
        .map_err(classify)?;

    if changed.rows_affected() == 0 {
        return Err(RepositoryError::NotFound {
            entity: "user",
            id: user.to_string(),
        });
    }

    // Every session, including the caller's. A password changed because it may
    // be known to someone else is worthless if sessions opened with the old
    // one keep working — and there is no way to tell the thief's session from
    // the owner's.
    sqlx::query(END_SESSIONS_PASSWORD)
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
