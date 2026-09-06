//! Static-ish pages: the landing page, the health check and the 404.

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use minijinja::context;

use crate::error::AppError;
use crate::state::AppState;
use crate::templates::render;

pub async fn index(State(state): State<AppState>) -> Result<Response, AppError> {
    let store = state.store();
    let featured = store.featured_swaps(3);
    let page = render(
        state.templates(),
        "pages/index.html",
        context! {
            member_count => store.count(),
            skill_count => store.distinct_skill_count(),
            featured => featured
                .iter()
                .map(|(viewer, swap)| context! {
                    viewer => viewer,
                    partner => swap.member,
                    they_teach => swap.they_teach,
                    you_teach => swap.you_teach,
                })
                .collect::<Vec<_>>(),
        },
    )?;
    Ok(page.into_response())
}

/// Liveness probe for deployments and for CI smoke tests.
pub async fn health() -> &'static str {
    "ok"
}

pub async fn not_found(State(state): State<AppState>) -> Result<Response, AppError> {
    let page = render(state.templates(), "pages/not_found.html", context! {})?;
    Ok((StatusCode::NOT_FOUND, page).into_response())
}
