//! Browsing members, searching and viewing a profile.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use minijinja::context;
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::AuthUserId;
use crate::error::AppError;
use crate::state::AppState;
use crate::templates::render;

#[derive(Debug, Deserialize, Default)]
pub struct SearchParams {
    #[serde(default)]
    pub q: String,
}

pub async fn browse(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
    current_user: Option<AuthUserId>,
) -> Result<Response, AppError> {
    let results = state.store().search(&params.q).await?;
    let page = render(
        state.templates(),
        "pages/browse.html",
        context! {
            query => params.q,
            members => results,
            current_user_id => current_user.map(|u| u.0.to_string()),
        },
    )?;
    Ok(page.into_response())
}

/// The fragment the search box swaps in on every keystroke. Requesting it
/// directly in a browser still returns valid (if unstyled) HTML, which makes it
/// easy to debug.
pub async fn browse_results(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Result<Response, AppError> {
    let results = state.store().search(&params.q).await?;
    let fragment = render(
        state.templates(),
        "partials/member_list.html",
        context! { query => params.q, members => results },
    )?;
    Ok(fragment.into_response())
}

pub async fn profile(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    current_user: Option<AuthUserId>,
) -> Result<Response, AppError> {
    let Some(member) = state.store().get(id).await? else {
        let page = render(state.templates(), "pages/not_found.html", context! {})?;
        return Ok((StatusCode::NOT_FOUND, page).into_response());
    };
    let swaps = state.store().swaps_for(id).await?;
    let page = render(
        state.templates(),
        "pages/member.html",
        context! {
            member => member,
            swaps => swaps,
            is_own_profile => current_user.is_some_and(|u| u.0 == id),
            current_user_id => current_user.map(|u| u.0.to_string()),
        },
    )?;
    Ok(page.into_response())
}
