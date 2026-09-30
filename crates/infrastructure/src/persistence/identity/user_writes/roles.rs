use domain::identity::RoleName;
use domain::shared::RepositoryError;
use sqlx::{Postgres, Transaction};

use super::queries::{DELETE_USER_ROLES, INSERT_USER_ROLE, SELECT_ROLE_IDS};
use crate::persistence::sql_error::classify;

/// Replaces a user's role rows inside an existing transaction.
///
/// Names are resolved and checked as a set first: one that does not exist
/// aborts before anything is written, rather than leaving the user with
/// whichever roles happened to resolve.
pub async fn replace(
    tx: &mut Transaction<'_, Postgres>,
    user_id: i64,
    roles: &[RoleName],
) -> Result<(), RepositoryError> {
    let wanted: Vec<String> = roles.iter().map(|r| r.as_str().to_owned()).collect();

    let found: Vec<(i64, String)> = sqlx::query_as(SELECT_ROLE_IDS)
        .bind(&wanted)
        .fetch_all(&mut **tx)
        .await
        .map_err(classify)?;

    if found.len() != wanted.len() {
        let missing = wanted
            .iter()
            .find(|name| !found.iter().any(|(_, got)| got == *name))
            .cloned()
            .unwrap_or_default();

        return Err(RepositoryError::NotFound {
            entity: "role",
            id: missing,
        });
    }

    sqlx::query(DELETE_USER_ROLES)
        .bind(user_id)
        .execute(&mut **tx)
        .await
        .map_err(classify)?;

    for (role_id, _) in &found {
        sqlx::query(INSERT_USER_ROLE)
            .bind(user_id)
            .bind(role_id)
            .execute(&mut **tx)
            .await
            .map_err(classify)?;
    }

    Ok(())
}
