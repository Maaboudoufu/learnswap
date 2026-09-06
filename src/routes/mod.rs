//! Route table.

mod auth;
mod members;
mod pages;

use axum::Router;
use axum::routing::get;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tower_sessions::{MemoryStore, SessionManagerLayer};

use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    // ponytail: sessions live in memory, so a restart signs everyone out and a
    // second instance would not share them. Swap in tower-sessions-sqlx-store
    // (it can use the pool we already have) when either starts to matter.
    let sessions = SessionManagerLayer::new(MemoryStore::default())
        // The app is served over plain HTTP in development; set this to true
        // behind TLS so the cookie is never sent in the clear.
        .with_secure(false)
        .with_http_only(true)
        .with_same_site(tower_sessions::cookie::SameSite::Lax);

    Router::new()
        .route("/", get(pages::index))
        .route("/browse", get(members::browse))
        // HTMX target for the live search box on /browse.
        .route("/browse/results", get(members::browse_results))
        .route("/members/{id}", get(members::profile))
        .route("/register", get(auth::register_form).post(auth::register))
        .route("/login", get(auth::login_form).post(auth::login))
        .route("/logout", get(auth::logout).post(auth::logout))
        .route("/me", get(auth::me))
        .route("/health", get(pages::health))
        .fallback(get(pages::not_found))
        .nest_service("/assets", ServeDir::new("assets"))
        .layer(sessions)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
