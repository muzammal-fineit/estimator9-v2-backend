//! Composition root. This file is the only place that names a concrete
//! adapter — everything below it talks to traits.

use anyhow::Context;
use api::config::Config;
use api::state::AppState;
use api::{VERSION, http};
use tokio::signal::unix::{SignalKind, signal};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    // Read and validate the whole environment before touching anything else,
    // so a misconfigured process fails immediately with a message naming the
    // variable rather than part-way through startup.
    let config = Config::from_env()?;
    let cookies = std::sync::Arc::new(config.cookie);

    let pool = infrastructure::connect(&config.database.url, config.database.max_connections)
        .await
        .context(
            "could not connect to the database — check DATABASE_URL and that Postgres is reachable",
        )?;

    // The pool is an Arc internally, so this clone is a handle to the same
    // pool — kept so shutdown can drain it after the server stops.
    let app = http::router(
        AppState::build(
            pool.clone(),
            &config.jwt,
            &config.mail,
            &config.licensing,
            cookies,
        ),
        config.server.enable_docs,
        &config.rate_limit,
        &config.cors,
    );

    let address = format!("{}:{}", config.server.bind_address, config.server.port);
    let listener = tokio::net::TcpListener::bind(&address)
        .await
        .with_context(|| format!("could not bind {address} — is another process using it?"))?;

    tracing::info!(
        version = VERSION,
        max_connections = config.database.max_connections,
        "listening on {}",
        listener.local_addr()?
    );

    // The proxy headers the rate limiter and audit trail key on are only
    // trustworthy while nginx is the sole way in. Bound to every interface,
    // anyone who can reach the port can claim any address they like — so this
    // is worth saying out loud rather than leaving in a deployment doc.
    if config.server.bind_address == "0.0.0.0" && config.rate_limit.enabled {
        tracing::warn!(
            "listening on all interfaces; X-Real-IP is only trustworthy behind a proxy. \
             Set BIND_ADDRESS=127.0.0.1 in any deployment reachable from a network."
        );
    }

    if config.server.enable_docs {
        tracing::info!("API docs at {}", http::routes::SWAGGER_UI_PATH);
    } else {
        tracing::info!("API docs disabled (ENABLE_DOCS=false)");
    }

    let served = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal()?)
    .await;

    // Runs whether the server stopped cleanly or failed, so in-flight
    // transactions are never abandoned mid-statement.
    tracing::info!("draining the connection pool");
    pool.close().await;

    served.context("the HTTP server stopped unexpectedly")?;

    Ok(())
}

/// Resolves on Ctrl+C or SIGTERM, which is what systemd sends on
/// `systemctl stop` and on a restart during deploy.
///
/// The handlers are installed *before* the returned future is awaited: a
/// failure to register them is a startup error worth reporting, not something
/// to discover at shutdown when it is too late to act on.
fn shutdown_signal() -> anyhow::Result<impl std::future::Future<Output = ()>> {
    let mut terminate =
        signal(SignalKind::terminate()).context("could not install the SIGTERM handler")?;

    Ok(async move {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => tracing::info!("received Ctrl+C"),
            _ = terminate.recv()        => tracing::info!("received SIGTERM"),
        }
    })
}
