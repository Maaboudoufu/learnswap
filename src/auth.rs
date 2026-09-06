//! Password hashing and "who is logged in?".

use argon2::Argon2;
use argon2::password_hash::{PasswordHasher, PasswordVerifier, phc::PasswordHash};
use axum::extract::{FromRequestParts, OptionalFromRequestParts};
use axum::http::request::Parts;
use axum::response::{IntoResponse, Redirect, Response};
use tower_sessions::Session;
use uuid::Uuid;

/// Session key holding the signed-in user's id.
const USER_ID_KEY: &str = "user_id";

/// Hashes a password into a PHC string (`$argon2id$v=19$...`).
///
/// Argon2 generates its own random salt and embeds it in the output, so the
/// hash is the only thing that needs storing.
pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    Ok(Argon2::default()
        .hash_password(password.as_bytes())?
        .to_string())
}

/// Checks a password against a stored PHC string.
///
/// Returns `false` for a malformed stored hash rather than erroring: a corrupt
/// row should fail the login, not take the request down.
pub fn verify_password(password: &str, phc: &str) -> bool {
    match PasswordHash::new(phc) {
        Ok(parsed) => Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => false,
    }
}

pub async fn log_in(
    session: &Session,
    user_id: Uuid,
) -> Result<(), tower_sessions::session::Error> {
    // Rotate the session id so a session fixated before login is not reused.
    session.cycle_id().await?;
    session.insert(USER_ID_KEY, user_id).await
}

pub async fn log_out(session: &Session) -> Result<(), tower_sessions::session::Error> {
    session.flush().await
}

/// A logged-in user's id, pulled from the session.
///
/// Use `AuthUserId` in a handler to require a login (unauthenticated requests
/// are redirected), or `Option<AuthUserId>` to merely find out whether someone
/// is signed in.
#[derive(Debug, Clone, Copy)]
pub struct AuthUserId(pub Uuid);

/// Rejection for a handler that requires a login.
pub struct RedirectToLogin;

impl IntoResponse for RedirectToLogin {
    fn into_response(self) -> Response {
        Redirect::to("/login").into_response()
    }
}

async fn user_id_from_session(parts: &mut Parts, state: &impl Sync) -> Option<Uuid> {
    let _ = state;
    let session = Session::from_request_parts(parts, &()).await.ok()?;
    session.get::<Uuid>(USER_ID_KEY).await.ok().flatten()
}

impl<S> FromRequestParts<S> for AuthUserId
where
    S: Send + Sync,
{
    type Rejection = RedirectToLogin;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        user_id_from_session(parts, state)
            .await
            .map(AuthUserId)
            .ok_or(RedirectToLogin)
    }
}

impl<S> OptionalFromRequestParts<S> for AuthUserId
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> Result<Option<Self>, Self::Rejection> {
        Ok(user_id_from_session(parts, state).await.map(AuthUserId))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_password_verifies_against_its_own_hash() {
        let phc = hash_password("correct horse battery staple").unwrap();
        assert!(verify_password("correct horse battery staple", &phc));
        assert!(!verify_password("wrong password", &phc));
    }

    #[test]
    fn the_same_password_hashes_differently_each_time() {
        // Distinct random salts; equal hashes would mean the salt is fixed.
        let a = hash_password("hunter2").unwrap();
        let b = hash_password("hunter2").unwrap();
        assert_ne!(a, b);
        assert!(verify_password("hunter2", &a));
        assert!(verify_password("hunter2", &b));
    }

    #[test]
    fn a_corrupt_stored_hash_fails_closed() {
        assert!(!verify_password("anything", "not-a-phc-string"));
        assert!(!verify_password("anything", ""));
    }
}
