//! LearnSwap -- a peer skill exchange platform.
//!
//! Members list what they can teach and what they want to learn; the app finds
//! pairs where the trade works in both directions. Pages are rendered on the
//! server with MiniJinja and updated in place with HTMX, so there is no
//! client-side framework to build or ship.
//!
//! Accounts are stored in SQLite or Postgres (chosen at runtime from
//! `DATABASE_URL`), passwords are hashed with Argon2 and logins are kept in a
//! `tower-sessions` session.

pub mod auth;
pub mod db;
pub mod error;
pub mod htmx;
pub mod models;
pub mod routes;
pub mod state;
pub mod store;
pub mod templates;

// Re-exported so the integration tests can use the exact same versions the
// crate was built against, without declaring them again as dev-dependencies.
pub use sqlx;
pub use uuid;

pub use routes::router;
pub use state::AppState;
pub use store::Store;
