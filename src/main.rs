use anyhow::Context;
use askama::Template;
use axum::{response::IntoResponse, routing::get, Router};
use tower_http::services::ServeDir;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "warn,personal_site=debug".into()),
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

    let api_router = Router::new().route("/hello", get(hello_sv));
    let router = Router::new()
        .route("/", get(home))
        .route("/resume", get(resume))
        .route("/projects", get(projects))
        .route("/wishlist", get(wishlist))
        .nest("/api", api_router)
        .nest_service("/public", ServeDir::new(public_path));

    axum::Server::bind(&addr)
        .serve(router.into_make_service())
        .await
        .context("error while starting server")?;

    Ok(())
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
handler!(resume, ResumePage, "pages/resume.html");
handler!(projects, ProjectsPage, "pages/projects.html");
handler!(wishlist, WishlistPage, "pages/wishlist.html");

async fn hello_sv() -> impl IntoResponse {
    "Hello, world!"
}
