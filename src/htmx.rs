//! Small helpers for talking to HTMX.

use std::convert::Infallible;

use axum::extract::FromRequestParts;
use axum::http::request::Parts;

/// True when the request came from HTMX rather than a plain browser
/// navigation. Handlers use it to return a fragment instead of a whole page,
/// which keeps every URL working with JavaScript disabled.
pub struct HxRequest(pub bool);

impl<S> FromRequestParts<S> for HxRequest
where
    S: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(HxRequest(parts.headers.contains_key("hx-request")))
    }
}
