use askama::Template;
use axum::{routing::get, Router};

#[derive(Template)]
#[template(path = "pages/wishlist.html")]
struct WishlistPage;

pub fn router() -> Router {
    Router::new().route("/", get(async || WishlistPage))
}
