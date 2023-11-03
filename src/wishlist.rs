use anyhow::{bail, Context};
use askama::Template;
use axum::{
    response::{IntoResponse, Response},
    routing::get,
    Router,
};

use crate::posts::Post;

#[derive(Template)]
#[template(path = "pages/wishlist.html")]
struct WishlistPage {
    list: Vec<Post<Frontmatter, ()>>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct Frontmatter {
    name: String,
    created_at: String,
    updated_at: Option<String>,
}

#[instrument]
async fn get_wishlist() -> crate::error::Result<Response> {
    // search `./posts/wishlist` directory for markdown files
    let list = std::fs::read_dir("posts/wishlist")
        .context("could not read the `posts/wishlist` directory")?
        .filter_map(|res| {
            // filter out and log errors
            if let Err(e) = res {
                warn!("could not read file: {}", e);
                return None;
            }

            res.ok()
        })
        .map(|res| res.path())
        .collect::<Vec<_>>();

    debug!("found {} wishlist project files", list.len());

    // read each file and parse into a `Post`
    let mut list = list
        .iter()
        .map(|path| {
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                bail!("could not get file stem");
            };
            Ok((path, stem))
        })
        .filter_map(|res| res.ok())
        // filter out files that start with `_` since this is how we can mark
        // hidden files
        .filter(|(_, stem)| {
            // if the `DISABLE_FILTER` environment variable is set to 1 then we
            // don't filter out any files
            if option_env!("DISABLE_FILTER").is_some_and(|v| v == "1") {
                return true;
            }

            !stem.starts_with('_')
        })
        .map(|(path, _)| Post::<Frontmatter, ()>::from_file(path))
        // collect into a Vec<_> and propagate Result errors
        .collect::<Result<Vec<_>, _>>()?;

    debug!("{} wishlist projects parsed", list.len());

    // TODO: sort by date: either created_at or updated_at whichever is more
    // recent
    list.sort_by(|a, b| {
        b.frontmatter.created_at.cmp(&a.frontmatter.created_at)
    });

    // parse the content into HTML for each post
    for post in list.iter_mut() {
        post.parse_content()
            .context("failed to parse wishlist post")?;
    }

    // make sure the content is HTML. this is a bit redundant since we just
    // parsed the content into HTML, however, this check should remain so
    // that the invariant doesn't get lost
    assert!(
        list.iter().all(|p| p.content.is_html()),
        "all wishlist files must be parsed into HTML before render"
    );

    Ok(WishlistPage { list }.into_response())
}

pub fn router() -> Router {
    Router::new().route("/", get(get_wishlist))
}
