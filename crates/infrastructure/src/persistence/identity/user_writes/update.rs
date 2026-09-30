use domain::audit::AuditEntry;
use domain::identity::{Email, UserId};
use domain::shared::RepositoryError;
use sqlx::PgPool;

use super::queries::UPDATE_USER;
use crate::persistence::audit::audit_writer;
use crate::persistence::sql_error::classify;

pub async fn execute(
    pool: &PgPool,
    user: UserId,
    name: Option<&str>,
    email: Option<&Email>,
    entry: AuditEntry,
) -> Result<(), RepositoryError> {
    let mut tx = pool.begin().await.map_err(classify)?;

    let changed = match sqlx::query(UPDATE_USER)
        .bind(user.value())
        .bind(name)
        .bind(email.map(Email::as_str))
        .execute(&mut *tx)
        .await
    {
        Ok(result) => result,

        // Same index as creation decides, for the same reason: reading first
        // to check would open a race two requests could both pass.
        Err(sqlx::Error::Database(err)) if err.is_unique_violation() => {
            return Err(RepositoryError::AlreadyExists {
                entity: "email",
                value: email.map(Email::to_string).unwrap_or_default(),
            });
        }

        Err(err) => return Err(classify(err)),
    };

    if changed.rows_affected() == 0 {
        return Err(RepositoryError::NotFound {
            entity: "user",
            id: user.to_string(),
        });
    }

    audit_writer::insert(&mut *tx, &entry).await?;

    tx.commit().await.map_err(classify)?;

    Ok(())
}
