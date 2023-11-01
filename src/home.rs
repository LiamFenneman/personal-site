use askama::Template;
use axum::{routing::get, Router};

#[derive(Template)]
#[template(path = "pages/index.html")]
struct HomePage;

pub fn router() -> Router {
    Router::new().route("/", get(async || HomePage))
}
