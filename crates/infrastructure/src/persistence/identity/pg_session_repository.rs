use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::audit::RequestContext;
use domain::identity::{Session, SessionEndReason, SessionId, SessionRepository, UserId};
use domain::shared::RepositoryError;
use sqlx::PgPool;
use sqlx::types::ipnetwork::IpNetwork;

use super::session_queries::{END_ALL_FOR_USER, END_ONE, INSERT_SESSION, SELECT_BY_ID, TOUCH};
use super::session_row::SessionRow;
use crate::persistence::sql_error::classify;

pub struct PgSessionRepository {
    pool: PgPool,
}

impl PgSessionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SessionRepository for PgSessionRepository {
    async fn start(
        &self,
        user: UserId,
        context: &RequestContext,
        _at: DateTime<Utc>,
    ) -> Result<Session, RepositoryError> {
        let row = sqlx::query_as::<_, SessionRow>(INSERT_SESSION)
            .bind(user.value())
            .bind(context.ip.map(IpNetwork::from))
            .bind(context.user_agent.as_deref())
            .fetch_one(&self.pool)
            .await
            .map_err(classify)?;

        Ok(row.into_entity())
    }

    async fn find(&self, id: SessionId) -> Result<Option<Session>, RepositoryError> {
        let row = sqlx::query_as::<_, SessionRow>(SELECT_BY_ID)
            .bind(id.value())
            .fetch_optional(&self.pool)
            .await
            .map_err(classify)?;

        Ok(row.map(SessionRow::into_entity))
    }

    async fn touch(&self, id: SessionId, at: DateTime<Utc>) -> Result<(), RepositoryError> {
        sqlx::query(TOUCH)
            .bind(id.value())
            .bind(at)
            .execute(&self.pool)
            .await
            .map_err(classify)?;

        Ok(())
    }

    async fn end(
        &self,
        id: SessionId,
        reason: SessionEndReason,
        at: DateTime<Utc>,
    ) -> Result<(), RepositoryError> {
        sqlx::query(END_ONE)
            .bind(id.value())
            .bind(at)
            .bind(reason.as_str())
            .execute(&self.pool)
            .await
            .map_err(classify)?;

        Ok(())
    }

    async fn end_all_for_user(
        &self,
        user: UserId,
        reason: SessionEndReason,
        at: DateTime<Utc>,
    ) -> Result<(), RepositoryError> {
        sqlx::query(END_ALL_FOR_USER)
            .bind(user.value())
            .bind(at)
            .bind(reason.as_str())
            .execute(&self.pool)
            .await
            .map_err(classify)?;

        Ok(())
    }
}
