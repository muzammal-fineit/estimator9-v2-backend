//! Runs the SQL seed files for the engine named by `DATABASE_URL`.
//!
//! ```bash
//! cargo run -p infrastructure --bin seed              # reference data only
//! cargo run -p infrastructure --bin seed -- --dev     # reference + development
//! ```
//!
//! Seeds are re-runnable by design — every file is idempotent and there is no
//! tracking table, because on-prem installers re-run them on every upgrade.
//! That is the opposite of migrations, which are versioned and run once.

use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use sqlx::Executor;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let url = std::env::var("DATABASE_URL").context("DATABASE_URL is not set")?;
    let engine = engine_from(&url)?;
    let include_dev = std::env::args().any(|arg| arg == "--dev");

    // One connection: seeding is a single sequential transaction, so a pool
    // sized for request traffic would just hold idle handles open.
    let pool = infrastructure::connect(&url, 1)
        .await
        .context("could not connect to the database")?;

    let mut sets = vec!["reference"];
    if include_dev {
        sets.push("development");
    }

    for set in sets {
        let dir = PathBuf::from("db/seeds").join(engine).join(set);
        let applied = run_set(&pool, &dir).await?;
        println!("{set:<12} {applied} file(s) from {}", dir.display());
    }

    Ok(())
}

/// Seed sets are per engine, so the folder is chosen by the URL scheme rather
/// than by a separate setting that could disagree with it.
fn engine_from(url: &str) -> anyhow::Result<&'static str> {
    match url.split(':').next() {
        Some("postgres" | "postgresql") => Ok("postgres"),
        Some("mysql" | "mariadb") => Ok("mysql"),
        Some("oracle") => Ok("oracle"),
        _ => bail!("cannot tell the database engine from DATABASE_URL"),
    }
}

/// Every file in one set runs inside a single transaction: a half-applied
/// permission vocabulary is worse than none at all.
async fn run_set(pool: &sqlx::PgPool, dir: &Path) -> anyhow::Result<usize> {
    if !dir.is_dir() {
        bail!(
            "{} does not exist — run from the workspace root",
            dir.display()
        );
    }

    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "sql"))
        .collect();

    files.sort();

    let mut tx = pool.begin().await?;

    for file in &files {
        let sql =
            std::fs::read_to_string(file).with_context(|| format!("reading {}", file.display()))?;

        // sqlx 0.9 refuses non-`&'static str` SQL unless the injection risk is
        // asserted away. It is safe here for a reason that does not generalise:
        // the text comes from a file committed to this repository, never from a
        // request. Do not copy this line anywhere user input can reach.
        tx.execute(sqlx::raw_sql(sqlx::AssertSqlSafe(sql)))
            .await
            .with_context(|| format!("running {}", file.display()))?;
    }

    tx.commit().await?;

    Ok(files.len())
}
