use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::identity::{
    IssuedPasswordReset, PasswordResetId, PasswordResetRepository, PasswordResetSecret,
    StoredPasswordReset, UserId,
};
use domain::shared::RepositoryError;
use sha2::{Digest, Sha256};
use sqlx::PgPool;

use crate::persistence::sql_error::classify;

const SECRET_BYTES: usize = 32;

const SUPERSEDE: &str = "UPDATE password_reset_tokens SET used_at = $2 \
                         WHERE user_id = $1 AND used_at IS NULL";

const INSERT: &str = "INSERT INTO password_reset_tokens (user_id, token_hash, expires_at) \
                      VALUES ($1, $2, $3) RETURNING id";

/// No filter on `used_at` or `expires_at`: a replayed token must be
/// distinguishable from one that never existed.
const SELECT_BY_HASH: &str = "SELECT id, user_id, expires_at, used_at \
                              FROM password_reset_tokens WHERE token_hash = $1";

pub struct PgPasswordResetRepository {
    pool: PgPool,
}

impl PgPasswordResetRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn digest(secret: &PasswordResetSecret) -> String {
    hex::encode(Sha256::digest(secret.expose().as_bytes()))
}

fn random_hex(bytes: usize) -> Result<String, RepositoryError> {
    let mut buffer = vec![0u8; bytes];

    getrandom::fill(&mut buffer).map_err(RepositoryError::backend)?;

    Ok(hex::encode(buffer))
}

#[async_trait]
impl PasswordResetRepository for PgPasswordResetRepository {
    async fn issue(
        &self,
        user: UserId,
        expires_at: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> Result<IssuedPasswordReset, RepositoryError> {
        let secret = PasswordResetSecret::new(random_hex(SECRET_BYTES)?);

        let mut tx = self.pool.begin().await.map_err(classify)?;

        // Marked used rather than deleted, so an old link in a mailbox reads
        // as spent rather than as never issued.
        sqlx::query(SUPERSEDE)
            .bind(user.value())
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(classify)?;

        let id: i64 = sqlx::query_scalar(INSERT)
            .bind(user.value())
            .bind(digest(&secret))
            .bind(expires_at)
            .fetch_one(&mut *tx)
            .await
            .map_err(classify)?;

        tx.commit().await.map_err(classify)?;

        Ok(IssuedPasswordReset {
            secret,
            stored: StoredPasswordReset {
                id: PasswordResetId::new(id),
                user_id: user,
                expires_at,
                used_at: None,
            },
        })
    }

    async fn find(
        &self,
        secret: &PasswordResetSecret,
    ) -> Result<Option<StoredPasswordReset>, RepositoryError> {
        let row: Option<(i64, i64, DateTime<Utc>, Option<DateTime<Utc>>)> =
            sqlx::query_as(SELECT_BY_HASH)
                .bind(digest(secret))
                .fetch_optional(&self.pool)
                .await
                .map_err(classify)?;

        Ok(
            row.map(|(id, user_id, expires_at, used_at)| StoredPasswordReset {
                id: PasswordResetId::new(id),
                user_id: UserId::new(user_id),
                expires_at,
                used_at,
            }),
        )
    }
}
