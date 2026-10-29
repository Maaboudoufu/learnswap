mod routes;
use axum::{Router, routing::get};
use minijinja::Environment;
use std::sync::Arc;
use tower_http::services::ServeDir;

#[tokio::main] // starts async
async fn main() {
    
    // minijinja
    let mut env = Environment::new();

    env.add_template("index.html", include_str!("../templates/index.html"))
        .unwrap();

    // arc multi-thread
    let env = Arc::new(env);
    // axum http handlers
    let app = Router::new()
        .route("/", get(routes::pages::home_handler))
        .route("/health", get(routes::health::health))
        .nest_service("/static/images", ServeDir::new("./static/images"))
        .with_state(env);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    axum::serve(listener, app).await.unwrap();
}
