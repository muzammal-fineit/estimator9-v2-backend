use std::collections::HashMap;

use async_trait::async_trait;
use domain::identity::{Email, LoginControls, PasswordHash, Role, User, UserId, UserRepository};
use domain::shared::RepositoryError;
use sqlx::PgPool;

use super::role_row::UserRoleRow;
use super::user_queries::{
    SELECT_ALL, SELECT_BY_EMAIL, SELECT_BY_ID, SELECT_ROLES_FOR_USERS, UPDATE_LOGIN_CONTROLS,
    UPDATE_PASSWORD,
};
use super::user_row::UserRow;

pub struct PgUserRepository {
    pool: PgPool,
}

impl PgUserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn roles_for(
        &self,
        user_ids: &[i64],
    ) -> Result<HashMap<i64, Vec<Role>>, RepositoryError> {
        if user_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let rows = sqlx::query_as::<_, UserRoleRow>(SELECT_ROLES_FOR_USERS)
            .bind(user_ids)
            .fetch_all(&self.pool)
            .await
            .map_err(crate::persistence::sql_error::classify)?;

        let mut grouped: HashMap<i64, Vec<Role>> = HashMap::new();

        for row in rows {
            let user_id = row.user_id;
            grouped.entry(user_id).or_default().push(row.into_entity()?);
        }

        Ok(grouped)
    }
}

#[async_trait]
impl UserRepository for PgUserRepository {
    async fn all(&self) -> Result<Vec<User>, RepositoryError> {
        let rows = sqlx::query_as::<_, UserRow>(SELECT_ALL)
            .fetch_all(&self.pool)
            .await
            .map_err(crate::persistence::sql_error::classify)?;

        let ids: Vec<i64> = rows.iter().map(|row| row.id).collect();
        let mut roles = self.roles_for(&ids).await?;

        rows.into_iter()
            .map(|row| {
                let user_roles = roles.remove(&row.id).unwrap_or_default();
                row.into_entity(user_roles)
            })
            .collect()
    }

    async fn find(&self, id: UserId) -> Result<Option<User>, RepositoryError> {
        let row = sqlx::query_as::<_, UserRow>(SELECT_BY_ID)
            .bind(id.value())
            .fetch_optional(&self.pool)
            .await
            .map_err(crate::persistence::sql_error::classify)?;

        let Some(row) = row else {
            return Ok(None);
        };

        let mut roles = self.roles_for(&[row.id]).await?;
        let user_roles = roles.remove(&row.id).unwrap_or_default();

        row.into_entity(user_roles).map(Some)
    }

    async fn find_by_email(&self, email: &Email) -> Result<Option<User>, RepositoryError> {
        let row = sqlx::query_as::<_, UserRow>(SELECT_BY_EMAIL)
            .bind(email.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(crate::persistence::sql_error::classify)?;

        let Some(row) = row else {
            return Ok(None);
        };

        let mut roles = self.roles_for(&[row.id]).await?;
        let user_roles = roles.remove(&row.id).unwrap_or_default();

        row.into_entity(user_roles).map(Some)
    }

    async fn set_password(&self, id: UserId, hash: &PasswordHash) -> Result<(), RepositoryError> {
        let result = sqlx::query(UPDATE_PASSWORD)
            .bind(id.value())
            .bind(hash.as_str())
            .execute(&self.pool)
            .await
            .map_err(crate::persistence::sql_error::classify)?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound {
                entity: "user",
                id: id.to_string(),
            });
        }

        Ok(())
    }

    async fn save_login_controls(
        &self,
        id: UserId,
        controls: &LoginControls,
    ) -> Result<(), RepositoryError> {
        // Postgres has no unsigned integer type; the domain caps this well
        // below i32::MAX so the cast cannot wrap in practice.
        let attempts = i32::try_from(controls.failed_attempts()).unwrap_or(i32::MAX);

        sqlx::query(UPDATE_LOGIN_CONTROLS)
            .bind(id.value())
            .bind(attempts)
            .bind(controls.locked_until())
            .execute(&self.pool)
            .await
            .map_err(crate::persistence::sql_error::classify)?;

        Ok(())
    }
}
