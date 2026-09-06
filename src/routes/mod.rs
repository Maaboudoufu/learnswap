//! Route table.

mod members;
mod pages;

use axum::Router;
use axum::routing::get;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(pages::index))
        .route("/browse", get(members::browse))
        // HTMX target for the live search box on /browse.
        .route("/browse/results", get(members::browse_results))
        .route("/members/{id}", get(members::profile))
        .route("/join", get(members::join_form).post(members::create))
        .route("/health", get(pages::health))
        .fallback(get(pages::not_found))
        .nest_service("/assets", ServeDir::new("assets"))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
