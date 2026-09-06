use std::net::SocketAddr;
use std::path::Path;

use learnswap::{AppState, Store, router, store::SEED_PASSWORD};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

/// Used when `DATABASE_URL` is unset. `mode=rwc` creates the file if it is
/// missing, so a fresh clone just works.
const DEFAULT_DATABASE_URL: &str = "sqlite://learnswap.db?mode=rwc";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            // Quiet by default, but show our own request logs.
            "learnswap=debug,tower_http=debug,info".into()
        }))
        .with(tracing_subscriber::fmt::layer())
        .init();

    warn_about_missing_assets();

    // Either a sqlite: or a postgres: URL; the app works the same on both.
    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_string());
    let store = Store::connect(&database_url).await?;
    tracing::info!("connected to {}", redact(&database_url));

    if store.seed_if_empty().await? {
        tracing::info!("seeded sample members -- log in with any @example.edu address");
        tracing::info!("seed account password: {SEED_PASSWORD}");
    }

    // PORT lets a host (or a teammate with 3000 already taken) override this.
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);
    // Loopback by default so a laptop does not expose the dev server to the
    // network. A container has to listen on 0.0.0.0 or published ports never
    // reach it, so the Dockerfile sets HOST.
    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let addr: SocketAddr = format!("{host}:{port}")
        .parse()
        .map_err(|e| format!("invalid HOST/PORT ({host}:{port}): {e}"))?;

    let app = router(AppState::new(store));

    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("LearnSwap listening on http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}

/// Strips the password out of a connection string before it reaches the log.
fn redact(url: &str) -> String {
    let Some((scheme, rest)) = url.split_once("://") else {
        return url.to_string();
    };
    let Some((credentials, host)) = rest.split_once('@') else {
        return url.to_string();
    };
    let user = credentials.split_once(':').map_or(credentials, |(u, _)| u);
    format!("{scheme}://{user}:***@{host}")
}

/// Templates and the stylesheet are read from disk relative to the working
/// directory, so running the binary from somewhere other than the repository
/// root silently produces a broken page. Say so up front instead.
fn warn_about_missing_assets() {
    if !Path::new(learnswap::templates::TEMPLATE_DIR).exists() {
        tracing::error!(
            "templates/ not found - run the server from the repository root (cargo run)"
        );
    }
    if !Path::new("assets/css/app.css").exists() {
        tracing::warn!(
            "assets/css/app.css not found - the UI will render unstyled. Build it with \
             scripts/tailwind.ps1 (Windows) or ./scripts/tailwind.sh (macOS/Linux)"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::redact;

    #[test]
    fn passwords_never_reach_the_log() {
        assert_eq!(
            redact("postgres://ana:s3cret@localhost/learnswap"),
            "postgres://ana:***@localhost/learnswap"
        );
        // Nothing to redact in a SQLite path.
        assert_eq!(
            redact("sqlite://learnswap.db?mode=rwc"),
            "sqlite://learnswap.db?mode=rwc"
        );
    }
}
