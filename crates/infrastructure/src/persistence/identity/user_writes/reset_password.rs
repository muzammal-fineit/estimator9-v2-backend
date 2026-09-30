use domain::audit::AuditEntry;
use domain::identity::{PasswordHash, PasswordResetId, UserId};
use domain::shared::RepositoryError;
use sqlx::PgPool;

use super::queries::{CHANGE_PASSWORD, END_SESSIONS_PASSWORD, REVOKE_USER_TOKENS};
use crate::persistence::audit::audit_writer;
use crate::persistence::sql_error::classify;

/// Guarded by `used_at IS NULL` so two requests racing with the same link
/// cannot both succeed — the database decides which one wins.
const CONSUME_TOKEN: &str = "UPDATE password_reset_tokens SET used_at = now() \
                             WHERE id = $1 AND used_at IS NULL";

pub async fn execute(
    pool: &PgPool,
    user: UserId,
    token: PasswordResetId,
    password: &PasswordHash,
    entry: AuditEntry,
) -> Result<(), RepositoryError> {
    let mut tx = pool.begin().await.map_err(classify)?;

    let consumed = sqlx::query(CONSUME_TOKEN)
        .bind(token.value())
        .execute(&mut *tx)
        .await
        .map_err(classify)?;

    if consumed.rows_affected() == 0 {
        return Err(RepositoryError::NotFound {
            entity: "password reset token",
            id: token.value().to_string(),
        });
    }

    sqlx::query(CHANGE_PASSWORD)
        .bind(user.value())
        .bind(password.as_str())
        .execute(&mut *tx)
        .await
        .map_err(classify)?;

    // A reset is for an account someone may have lost control of, so every
    // session goes — the same reasoning as a deliberate password change, only
    // more so.
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
