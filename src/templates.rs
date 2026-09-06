//! MiniJinja wiring.
//!
//! Templates live on disk under `templates/` and are watched for changes, so a
//! template edit shows up on the next page load without restarting the server.

use axum::response::Html;
use minijinja::{Environment, Value, path_loader};
use minijinja_autoreload::AutoReloader;

use crate::error::AppError;

pub const TEMPLATE_DIR: &str = "templates";

pub fn build_reloader() -> AutoReloader {
    AutoReloader::new(|notifier| {
        let mut env = Environment::new();
        env.set_loader(path_loader(TEMPLATE_DIR));
        notifier.watch_path(TEMPLATE_DIR, true);
        Ok(env)
    })
}

/// Renders `name` with `ctx` into an HTML response body.
pub fn render(reloader: &AutoReloader, name: &str, ctx: Value) -> Result<Html<String>, AppError> {
    let env = reloader.acquire_env()?;
    let template = env.get_template(name)?;
    Ok(Html(template.render(ctx)?))
}
