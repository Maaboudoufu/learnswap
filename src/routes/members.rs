//! Browsing members, viewing a profile and joining.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use minijinja::context;
use serde::Deserialize;
use uuid::Uuid;

use crate::error::AppError;
use crate::htmx::HxRequest;
use crate::models::Member;
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
) -> Result<Response, AppError> {
    let results = state.store().search(&params.q);
    let page = render(
        state.templates(),
        "pages/browse.html",
        context! { query => params.q, members => results },
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
    let results = state.store().search(&params.q);
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
) -> Result<Response, AppError> {
    let Some(member) = state.store().get(id) else {
        let page = render(state.templates(), "pages/not_found.html", context! {})?;
        return Ok((StatusCode::NOT_FOUND, page).into_response());
    };
    let swaps = state.store().swaps_for(id);
    let page = render(
        state.templates(),
        "pages/member.html",
        context! { member => member, swaps => swaps },
    )?;
    Ok(page.into_response())
}

pub async fn join_form(State(state): State<AppState>) -> Result<Response, AppError> {
    let page = render(
        state.templates(),
        "pages/join.html",
        context! { form => JoinForm::default(), errors => Vec::<String>::new() },
    )?;
    Ok(page.into_response())
}

#[derive(Debug, Deserialize, Default, serde::Serialize)]
pub struct JoinForm {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub headline: String,
    #[serde(default)]
    pub bio: String,
    #[serde(default)]
    pub teaching: String,
    #[serde(default)]
    pub learning: String,
}

impl JoinForm {
    /// Returns the reasons this submission cannot become a profile.
    fn errors(&self) -> Vec<&'static str> {
        let mut errors = Vec::new();
        if self.name.trim().is_empty() {
            errors.push("Tell us your name so people know who they are swapping with.");
        }
        if crate::models::Skill::parse_list(&self.teaching).is_empty() {
            errors.push("List at least one skill you can teach.");
        }
        if crate::models::Skill::parse_list(&self.learning).is_empty() {
            errors.push("List at least one skill you want to learn.");
        }
        errors
    }
}

pub async fn create(
    State(state): State<AppState>,
    HxRequest(is_htmx): HxRequest,
    axum::Form(form): axum::Form<JoinForm>,
) -> Result<Response, AppError> {
    let errors = form.errors();
    if !errors.is_empty() {
        // Re-render the form with what the user typed still in place. HTMX
        // swaps just the form; a plain POST gets the whole page back.
        let template = if is_htmx {
            "partials/join_form.html"
        } else {
            "pages/join.html"
        };
        let body = render(
            state.templates(),
            template,
            context! { form => form, errors => errors },
        )?;
        return Ok((StatusCode::UNPROCESSABLE_ENTITY, body).into_response());
    }

    let member = Member::new(
        &form.name,
        &form.headline,
        &form.bio,
        &form.teaching,
        &form.learning,
    );
    let id = state.store().insert(member);
    let swaps = state.store().swaps_for(id);

    if is_htmx {
        let member = state.store().get(id).expect("member was just inserted");
        let fragment = render(
            state.templates(),
            "partials/join_success.html",
            context! { member => member, swaps => swaps },
        )?;
        return Ok(fragment.into_response());
    }
    Ok(Redirect::to(&format!("/members/{id}")).into_response())
}
