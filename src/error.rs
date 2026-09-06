//! Error type shared by every handler.

use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

#[derive(Debug)]
pub enum AppError {
    /// A template failed to load or render -- always a bug in our templates.
    Template(minijinja::Error),
}

impl From<minijinja::Error> for AppError {
    fn from(err: minijinja::Error) -> Self {
        AppError::Template(err)
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Template(err) => write!(f, "template error: {err:#}"),
        }
    }
}

impl std::error::Error for AppError {}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        // Log the detail, show the user something plain. Template rendering is
        // the one thing we cannot fall back to for the error page itself.
        tracing::error!("{self}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html("<h1>500 - Something went wrong</h1>"),
        )
            .into_response()
    }
}
