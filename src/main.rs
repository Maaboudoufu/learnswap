use std::net::SocketAddr;
use std::path::Path;

use learnswap::{AppState, Store, router};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            // Quiet by default, but show our own request logs.
            "learnswap=debug,tower_http=debug,info".into()
        }))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // PORT lets a host (or a teammate with 3000 already taken) override this.
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);
    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    warn_about_missing_assets();

    let app = router(AppState::new(Store::with_seed_data()));

    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("LearnSwap listening on http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
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
            "assets/css/app.css not found - the UI will render unstyled. Build it with              scripts/tailwind.ps1 (Windows) or ./scripts/tailwind.sh (macOS/Linux)"
        );
    }
}
