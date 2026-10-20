mod handlers;
use minijinja::{Environment};
use std::sync::Arc;
use axum::{Router, routing::get, routing::post};

#[tokio::main] // starts async
async fn main() {
    // minijinja
    let mut env = Environment::new();
    env.add_template("index.html", include_str!("../templates/index.html")).unwrap();
    // arc mutli-thread
    let env = Arc::new(env);
    // axum http handlers
    let app = Router::new().route("/", get(handlers::fetch::fetch)).route("/publish", post(handlers::publish::publish)).with_state(env);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap(); 

    axum::serve(listener, app).await.unwrap();
}
