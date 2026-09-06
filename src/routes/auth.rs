//! Register, log in, log out.

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use minijinja::context;
use serde::{Deserialize, Serialize};
use tower_sessions::Session;

use crate::auth::{AuthUserId, log_in, log_out};
use crate::error::AppError;
use crate::htmx::HxRequest;
use crate::models::Skill;
use crate::state::AppState;
use crate::store::{NewAccount, RegisterError};
use crate::templates::render;

// ---------------------------------------------------------------- registering

#[derive(Debug, Deserialize, Default, Serialize)]
pub struct RegisterForm {
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub password: String,
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

/// Shortest password we accept. Long enough to matter, short enough that
/// nobody in the class is locked out of their own demo.
const MIN_PASSWORD_LEN: usize = 8;

impl RegisterForm {
    fn errors(&self) -> Vec<&'static str> {
        let mut errors = Vec::new();
        if self.name.trim().is_empty() {
            errors.push("Tell us your name so people know who they are swapping with.");
        }
        if !looks_like_email(&self.email) {
            errors.push("Enter an email address so you can sign back in.");
        }
        if self.password.chars().count() < MIN_PASSWORD_LEN {
            errors.push("Use a password of at least 8 characters.");
        }
        if Skill::parse_list(&self.teaching).is_empty() {
            errors.push("List at least one skill you can teach.");
        }
        if Skill::parse_list(&self.learning).is_empty() {
            errors.push("List at least one skill you want to learn.");
        }
        errors
    }
}

/// Deliberately permissive: the only authority on whether an address works is
/// sending mail to it, and rejecting odd-but-valid addresses is worse than
/// accepting a typo.
fn looks_like_email(raw: &str) -> bool {
    let raw = raw.trim();
    match raw.split_once('@') {
        Some((local, domain)) => {
            !local.is_empty()
                && domain.contains('.')
                && !domain.starts_with('.')
                && !domain.ends_with('.')
                && !raw.contains(char::is_whitespace)
        }
        None => false,
    }
}

pub async fn register_form(State(state): State<AppState>) -> Result<Response, AppError> {
    let page = render(
        state.templates(),
        "pages/register.html",
        context! { form => RegisterForm::default(), errors => Vec::<String>::new() },
    )?;
    Ok(page.into_response())
}

pub async fn register(
    State(state): State<AppState>,
    session: Session,
    HxRequest(is_htmx): HxRequest,
    axum::Form(mut form): axum::Form<RegisterForm>,
) -> Result<Response, AppError> {
    let mut errors = form.errors();

    if errors.is_empty() && state.store().email_exists(&form.email).await? {
        errors.push("That email is already registered. Try logging in instead.");
    }

    if errors.is_empty() {
        match state
            .store()
            .register(NewAccount {
                email: &form.email,
                password: &form.password,
                name: &form.name,
                headline: &form.headline,
                bio: &form.bio,
                teaching: &form.teaching,
                learning: &form.learning,
            })
            .await
        {
            Ok(id) => {
                log_in(&session, id).await?;
                return Ok(redirect_after_auth(is_htmx, &format!("/members/{id}")));
            }
            // Lost the race against a concurrent signup with the same email.
            Err(RegisterError::EmailTaken) => {
                errors.push("That email is already registered. Try logging in instead.")
            }
            Err(RegisterError::Database(e)) => return Err(AppError::Database(e)),
            Err(RegisterError::Hash(e)) => {
                tracing::error!("password hashing failed: {e}");
                return Ok(StatusCode::INTERNAL_SERVER_ERROR.into_response());
            }
        }
    }

    // Never echo the password back into the page.
    form.password = String::new();
    let template = if is_htmx {
        "partials/register_form.html"
    } else {
        "pages/register.html"
    };
    let body = render(
        state.templates(),
        template,
        context! { form => form, errors => errors },
    )?;
    Ok((StatusCode::UNPROCESSABLE_ENTITY, body).into_response())
}

// ------------------------------------------------------------------- logging in

#[derive(Debug, Deserialize, Default, Serialize)]
pub struct LoginForm {
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub password: String,
}

pub async fn login_form(State(state): State<AppState>) -> Result<Response, AppError> {
    let page = render(
        state.templates(),
        "pages/login.html",
        context! { form => LoginForm::default(), errors => Vec::<String>::new() },
    )?;
    Ok(page.into_response())
}

pub async fn login(
    State(state): State<AppState>,
    session: Session,
    HxRequest(is_htmx): HxRequest,
    axum::Form(mut form): axum::Form<LoginForm>,
) -> Result<Response, AppError> {
    if let Some(id) = state
        .store()
        .authenticate(&form.email, &form.password)
        .await?
    {
        log_in(&session, id).await?;
        return Ok(redirect_after_auth(is_htmx, &format!("/members/{id}")));
    }

    // One message for both a wrong email and a wrong password, so this page
    // cannot be used to find out which addresses have accounts.
    form.password = String::new();
    let template = if is_htmx {
        "partials/login_form.html"
    } else {
        "pages/login.html"
    };
    let body = render(
        state.templates(),
        template,
        context! {
            form => form,
            errors => vec!["That email and password do not match an account."],
        },
    )?;
    Ok((StatusCode::UNAUTHORIZED, body).into_response())
}

pub async fn logout(session: Session) -> Result<Response, AppError> {
    log_out(&session).await?;
    Ok(Redirect::to("/").into_response())
}

/// Where a signed-in user lands: their own profile.
pub async fn me(user: AuthUserId) -> Redirect {
    Redirect::to(&format!("/members/{}", user.0))
}

/// HTMX will not follow a 303 into a full page load, so it gets `HX-Redirect`
/// instead. A plain form post gets the ordinary redirect.
fn redirect_after_auth(is_htmx: bool, to: &str) -> Response {
    if is_htmx {
        return ([("HX-Redirect", to)], StatusCode::OK).into_response();
    }
    Redirect::to(to).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_shapes() {
        assert!(looks_like_email("ana@example.edu"));
        assert!(looks_like_email("a.b+tag@sub.example.co.uk"));
        assert!(!looks_like_email("no-at-sign"));
        assert!(!looks_like_email("@example.edu"));
        assert!(!looks_like_email("ana@localhost"));
        assert!(!looks_like_email("ana@example."));
        assert!(!looks_like_email("has space@example.edu"));
        assert!(!looks_like_email(""));
    }

    #[test]
    fn short_passwords_are_rejected() {
        let form = RegisterForm {
            email: "a@b.com".into(),
            password: "short".into(),
            name: "A".into(),
            teaching: "Rust".into(),
            learning: "Guitar".into(),
            ..Default::default()
        };
        assert!(
            form.errors()
                .iter()
                .any(|e| e.contains("at least 8 characters"))
        );
    }
}
