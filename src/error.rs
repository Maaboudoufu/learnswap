//! Error type shared by every handler.

use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

#[derive(Debug)]
pub enum AppError {
    /// A template failed to load or render -- always a bug in our templates.
    Template(minijinja::Error),
    /// A query failed. The user sees nothing about it; the log gets the detail.
    Database(sqlx::Error),
    /// A session could not be read or written.
    Session(tower_sessions::session::Error),
}

impl From<minijinja::Error> for AppError {
    fn from(err: minijinja::Error) -> Self {
        AppError::Template(err)
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::Database(err)
    }
}

impl From<tower_sessions::session::Error> for AppError {
    fn from(err: tower_sessions::session::Error) -> Self {
        AppError::Session(err)
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Template(err) => write!(f, "template error: {err:#}"),
            AppError::Database(err) => write!(f, "database error: {err}"),
            AppError::Session(err) => write!(f, "session error: {err}"),
        }
    }
}

impl std::error::Error for AppError {}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        // Log the detail, show the user something plain. Error text can carry
        // query fragments and connection strings, so it never reaches the page.
        tracing::error!("{self}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html("<h1>500 - Something went wrong</h1>"),
        )
            .into_response()
    }
}
