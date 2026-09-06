//! LearnSwap -- a peer skill exchange platform.
//!
//! Members list what they can teach and what they want to learn; the app finds
//! pairs where the trade works in both directions. Pages are rendered on the
//! server with MiniJinja and updated in place with HTMX, so there is no
//! client-side framework to build or ship.

pub mod error;
pub mod htmx;
pub mod models;
pub mod routes;
pub mod state;
pub mod store;
pub mod templates;

pub use routes::router;
pub use state::AppState;
pub use store::Store;
