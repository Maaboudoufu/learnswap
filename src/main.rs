mod routes;
use axum::{Router, routing::get};
use minijinja::Environment;
use std::sync::Arc;

#[tokio::main] // starts async
async fn main() {
    // minijinja
    let mut env = Environment::new();
    env.add_template("index.html", include_str!("../templates/pages/index.html"))
        .unwrap();
    // arc mutli-thread
    let env = Arc::new(env);
    // axum http handlers
    let app = Router::new()
        .route("/", get(routes::pages::pages))
        .route("/health", get(routes::health::health))
        .with_state(env);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    axum::serve(listener, app).await.unwrap();
}
