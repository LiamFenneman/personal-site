#![feature(async_closure)]

use anyhow::Context;
use askama::Template;
use axum::{
    http::{header, Response},
    middleware::map_response,
    response::IntoResponse,
    routing::get,
    Router,
};
use tower::ServiceBuilder;
use tower_http::{
    compression::CompressionLayer, services::ServeDir, trace::TraceLayer,
};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod projects;
mod resume;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| {
                    "warn,tower_http=trace,personal_site=debug".into()
                }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("initializing router...");

    // get host and port from environment variables
    // default: 127.0.0.1:3000
    let host: std::net::IpAddr = std::option_env!("HOST")
        .map(|h| h.parse().expect("could not parse HOST"))
        .unwrap_or([127, 0, 0, 1].into());
    let port = std::option_env!("PORT")
        .map(|p| p.parse().expect("could not parse PORT"))
        .unwrap_or(3000);
    let addr = std::net::SocketAddr::from((host, port));

    // get public path from `./public`
    let public_path = std::env::current_dir()?.join("public");

    info!("router initialized, now listening on port {}", port);

    let router = Router::new()
        .route("/", get(home))
        .route("/wishlist", get(wishlist))
        .nest("/resume", resume::router())
        .nest("/projects", projects::router())
        .fallback_service(ServeDir::new(public_path))
        .layer(
            // add tracing and compression to all routes
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CompressionLayer::new()),
        )
        // set the vary header to (at-least) accept-encoding
        .layer(map_response(set_vary_header));

    axum::Server::bind(&addr)
        .serve(router.into_make_service())
        .await
        .context("error while starting server")?;

    Ok(())
}

/// Set the `Vary` header to include (at-least) `Accept-Encoding` since all the
/// resources are being compressed.
///
/// Recommendation by MDN:
/// "the server must send a Vary header containing at least Accept-Encoding
/// alongside this header in the response."
///
/// Source:
/// - https://developer.mozilla.org/en-US/docs/Web/HTTP/Compression#end-to-end_compression
async fn set_vary_header<T>(mut response: Response<T>) -> Response<T> {
    response
        .headers_mut()
        .append(header::VARY, header::ACCEPT_ENCODING.into());
    response
}

macro_rules! handler {
    ($fn:ident, $templ:ident, $path:literal) => {
        async fn $fn() -> impl IntoResponse {
            $templ
        }

        #[derive(Template)]
        #[template(path = $path)]
        struct $templ;
    };
}

handler!(home, HomePage, "pages/index.html");
handler!(wishlist, WishlistPage, "pages/wishlist.html");
