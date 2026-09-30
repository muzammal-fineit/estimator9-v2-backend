use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::identity::{
    IssuedRefreshToken, RefreshTokenId, RefreshTokenRepository, RefreshTokenSecret, SessionId,
    StoredRefreshToken, UserId,
};
use domain::shared::RepositoryError;
use sha2::{Digest, Sha256};
use sqlx::PgPool;

use super::refresh_token_queries::{
    INSERT_TOKEN, REVOKE_ALL_FOR_USER, REVOKE_ONE, REVOKE_SESSION, SELECT_BY_HASH,
};
use super::refresh_token_row::RefreshTokenRow;

/// 256 bits. The secret is never guessed at, only stolen or not, so entropy is
/// the only property that matters.
const SECRET_BYTES: usize = 32;

pub struct PgRefreshTokenRepository {
    pool: PgPool,
}

impl PgRefreshTokenRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// Hex-encoded SHA-256. Fast on purpose — see the migration for why a slow
/// hash would be the wrong tool here.
fn digest(secret: &RefreshTokenSecret) -> String {
    let hash = Sha256::digest(secret.expose().as_bytes());

    hex::encode(hash)
}

/// Fails rather than falling back to a weaker source: a predictable refresh
/// token is a silent authentication bypass.
fn random_hex(bytes: usize) -> Result<String, RepositoryError> {
    let mut buffer = vec![0u8; bytes];

    getrandom::fill(&mut buffer).map_err(RepositoryError::backend)?;

    Ok(hex::encode(buffer))
}

#[async_trait]
impl RefreshTokenRepository for PgRefreshTokenRepository {
    async fn issue(
        &self,
        user: UserId,
        session: SessionId,
        expires_at: DateTime<Utc>,
    ) -> Result<IssuedRefreshToken, RepositoryError> {
        let secret = RefreshTokenSecret::new(random_hex(SECRET_BYTES)?);

        let id: i64 = sqlx::query_scalar(INSERT_TOKEN)
            .bind(user.value())
            .bind(digest(&secret))
            .bind(session.value())
            .bind(expires_at)
            .fetch_one(&self.pool)
            .await
            .map_err(crate::persistence::sql_error::classify)?;

        Ok(IssuedRefreshToken {
            secret,
            stored: StoredRefreshToken {
                id: RefreshTokenId::new(id),
                user_id: user,
                session_id: session,
                expires_at,
                revoked_at: None,
            },
        })
    }

    async fn find(
        &self,
        secret: &RefreshTokenSecret,
    ) -> Result<Option<StoredRefreshToken>, RepositoryError> {
        let row = sqlx::query_as::<_, RefreshTokenRow>(SELECT_BY_HASH)
            .bind(digest(secret))
            .fetch_optional(&self.pool)
            .await
            .map_err(crate::persistence::sql_error::classify)?;

        Ok(row.map(RefreshTokenRow::into_entity))
    }

    async fn revoke(&self, id: RefreshTokenId, at: DateTime<Utc>) -> Result<(), RepositoryError> {
        sqlx::query(REVOKE_ONE)
            .bind(id.value())
            .bind(at)
            .execute(&self.pool)
            .await
            .map_err(crate::persistence::sql_error::classify)?;

        Ok(())
    }

    async fn revoke_session(
        &self,
        session: SessionId,
        at: DateTime<Utc>,
    ) -> Result<(), RepositoryError> {
        sqlx::query(REVOKE_SESSION)
            .bind(session.value())
            .bind(at)
            .execute(&self.pool)
            .await
            .map_err(crate::persistence::sql_error::classify)?;

        Ok(())
    }

    async fn revoke_all_for_user(
        &self,
        user: UserId,
        at: DateTime<Utc>,
    ) -> Result<(), RepositoryError> {
        sqlx::query(REVOKE_ALL_FOR_USER)
            .bind(user.value())
            .bind(at)
            .execute(&self.pool)
            .await
            .map_err(crate::persistence::sql_error::classify)?;

        Ok(())
    }
}
