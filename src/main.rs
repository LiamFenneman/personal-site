#![feature(async_closure)]

use anyhow::Context;
use askama::Template;
use axum::{
    http::header, middleware::map_response, response::Response, Router,
};
use tower::ServiceBuilder;
use tower_http::{
    compression::CompressionLayer,
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[macro_use]
extern crate tracing;

pub use error::AppError;

pub mod caching;
pub mod error;
mod home;
pub mod links;
pub mod posts;
mod projects;
mod resume;
mod wishlist;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| {
                    "warn,tower_http=trace,personal_site=trace".into()
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

    let router = Router::new()
        .merge(home::router())
        .nest("/resume", resume::router())
        .nest("/projects", projects::router())
        .nest("/wishlist", wishlist::router())
        // serve all files from `./public` directory
        .nest_service(
            "/public",
            ServeDir::new(std::env::current_dir()?.join("public")),
        )
        // serve the favicon separately since browsers expect it to be located
        // at a specific URL
        .route_service("/favicon.ico", ServeFile::new("public/favicon.ico"))
        .fallback(async || NotFoundPage)
        .layer(
            // add tracing and compression to all routes
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CompressionLayer::new()),
        )
        // set the vary header to (at-least) accept-encoding
        .layer(map_response(set_vary_header))
        .layer(caching::CacheLayer::default());

    info!("router initialized, now listening on port {}", port);

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
///
/// **NOTE:** this is being added to the `CompressionLayer` in a PR:
/// - https://github.com/tower-rs/tower-http/pull/399
async fn set_vary_header(mut response: Response) -> Response {
    // TODO: support appending to the end of the existing Vary header
    response
        .headers_mut()
        .insert(header::VARY, header::ACCEPT_ENCODING.into());
    response
}

#[derive(Template)]
#[template(path = "pages/404.html")]
struct NotFoundPage;
