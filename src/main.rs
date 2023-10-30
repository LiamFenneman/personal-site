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

    let host: std::net::IpAddr = std::option_env!("HOST")
        .map(|h| h.parse().expect("could not parse HOST"))
        .unwrap_or([127, 0, 0, 1].into());
    let port = std::option_env!("PORT")
        .map(|p| p.parse().expect("could not parse PORT"))
        .unwrap_or(3000);
    let addr = std::net::SocketAddr::from((host, port));

    let public_path = {
        let mut buf = std::env::current_dir()
            .expect("could not find current working dir");
        buf.push("public");
        buf
    };

    info!("router initialized, now listening on port {}", port);

    let router = Router::new()
        .route("/", get(home))
        .route("/wishlist", get(wishlist))
        .nest("/resume", resume::router())
        .nest("/projects", projects::router())
        .fallback_service(ServeDir::new(public_path))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CompressionLayer::new()),
        )
        .layer(map_response(set_vary_header));

    axum::Server::bind(&addr)
        .serve(router.into_make_service())
        .await
        .context("error while starting server")?;

    Ok(())
}

async fn set_vary_header<T>(mut response: Response<T>) -> Response<T> {
    match response.headers().get(header::VARY) {
        Some(_vary) => {
            todo!("append ACCEPT_ENCODING to VARY header")
        }
        None => {
            // If the response doesn't already have a Vary header, then add one
            // with the value `Accept-Encoding`
            // This needs to be done since we are using compression.
            //
            // Source:
            // https://developer.mozilla.org/en-US/docs/Web/HTTP/Compression
            response
                .headers_mut()
                .insert(header::VARY, header::ACCEPT_ENCODING.into());
        }
    }
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
