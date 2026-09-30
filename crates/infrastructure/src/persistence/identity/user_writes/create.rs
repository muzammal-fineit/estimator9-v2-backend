use domain::audit::AuditEntry;
use domain::identity::{Email, PasswordHash, RoleName, UserId};
use domain::shared::RepositoryError;
use sqlx::PgPool;

use super::queries::INSERT_USER;
use super::roles;
use crate::persistence::audit::audit_writer;
use crate::persistence::sql_error::classify;

pub async fn execute(
    pool: &PgPool,
    email: &Email,
    name: &str,
    password: &PasswordHash,
    role_names: &[RoleName],
    mut entry: AuditEntry,
) -> Result<UserId, RepositoryError> {
    let mut tx = pool.begin().await.map_err(classify)?;

    let id: i64 = match sqlx::query_scalar(INSERT_USER)
        .bind(email.as_str())
        .bind(name)
        .bind(password.as_str())
        .fetch_one(&mut *tx)
        .await
    {
        Ok(id) => id,

        // `users_email_lower_key` is what decides, so two requests racing for
        // the same address cannot both win — and the check cannot be done by
        // reading first without opening that race.
        Err(sqlx::Error::Database(err)) if err.is_unique_violation() => {
            return Err(RepositoryError::AlreadyExists {
                entity: "email",
                value: email.to_string(),
            });
        }

        Err(err) => return Err(classify(err)),
    };

    roles::replace(&mut tx, id, role_names).await?;

    // The id only exists now, so the entry names its subject here rather than
    // in the use case.
    entry = entry.about("user", id);

    audit_writer::insert(&mut *tx, &entry).await?;

    tx.commit().await.map_err(classify)?;

    Ok(UserId::new(id))
}
