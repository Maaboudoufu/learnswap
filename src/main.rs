mod handlers;
use axum::{Router, routing::get, routing::post};

#[tokio::main] // starts async
async fn main() {
    let app = Router::new().route("/", get(handlers::fetch::fetch)).route("/publish", post(handlers::publish::publish));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap(); 

    axum::serve(listener, app).await.unwrap();
}
