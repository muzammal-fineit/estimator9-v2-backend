use sqlx::postgres::PgPoolOptions;

pub use sqlx::PgPool;

/// `max_connections` is the ceiling on concurrent database work, and under load
/// it — not request handling — is what sets throughput. The caller supplies it
/// so it can be tuned per environment without a rebuild.
pub async fn connect(url: &str, max_connections: u32) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(max_connections)
        .connect(url)
        .await
}
