use anyhow::{bail, Context};
use askama::Template;
use axum::{routing::get, Router};

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

async fn get_wishlist() -> crate::error::Result<WishlistPage> {
    // search `./posts/wishlist` directory for markdown files
    let list = std::fs::read_dir("posts/wishlist")
        .context("could not read the `posts/wishlist` directory")?
        .filter_map(|res| res.ok()) // filter out errors (TODO: log this)
        .map(|res| res.path())
        .collect::<Vec<_>>();

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

    // TODO: sort by date: either created_at or updated_at whichever is more
    // recent
    list.sort_by(|a, b| {
        b.frontmatter.created_at.cmp(&a.frontmatter.created_at)
    });

    for post in list.iter_mut() {
        post.parse_content()
            .context("failed to parse wishlist post")?;
    }

    Ok(WishlistPage { list })
}

pub fn router() -> Router {
    Router::new().route("/", get(get_wishlist))
}
