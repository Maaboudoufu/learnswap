use axum::{extract::State, response::Html};
use minijinja::Environment;
use std::sync::Arc;

pub async fn home_handler(State(env): State<Arc<Environment<'static>>>) -> Html<String> {
    let html = env.get_template("index.html").unwrap().render(()).unwrap();
    Html(html)
}
