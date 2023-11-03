use std::path::PathBuf;

use anyhow::{bail, Context};
use askama::Template;
use axum::http::StatusCode;
use axum::{
    extract::Path,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};

use crate::links::{Link, Links};
use crate::posts::Post;

/// A page listing all projects.
#[derive(Template)]
#[template(path = "pages/projects.html")]
struct ProjectsPage {
    list: Vec<Post<Frontmatter, Metadata>>,
}

/// Individual project page.
#[derive(Template)]
#[template(path = "pages/project.html")]
struct ProjectPage {
    project: Post<Frontmatter, Metadata>,
}

/// Metadata about the project.
#[derive(Debug, Clone, serde::Deserialize)]
struct Metadata {
    slug: String,
}

/// Frontmatter from the `.md` files used to generate the posts.
#[derive(Debug, Clone, serde::Deserialize)]
struct Frontmatter {
    name: String,
    description: String,
    created_at: String,
    updated_at: Option<String>,
    links: Option<Links>,
}

#[instrument]
async fn get_project_list() -> crate::error::Result<ProjectsPage> {
    // search `./posts/projects` directory for markdown files
    let projects = std::fs::read_dir("posts/projects")
        .context("could not read the `posts/projects` directory")?
        .filter_map(|res| res.ok()) // filter out errors (TODO: log this)
        .map(|res| res.path())
        .collect::<Vec<_>>();

    debug!("found {} project files", projects.len());

    // read each file and parse into `Post`
    // return a project page with each project
    let mut projects = projects
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
        .map(|(path, stem)| {
            Post::<Frontmatter, Metadata>::from_file_with_metadata(
                path,
                Metadata {
                    slug: format!("/projects/{stem}"),
                },
            )
        })
        // collect into a Vec<_> and propagate Result errors
        .collect::<Result<Vec<_>, _>>()?;

    debug!("{} projects parsed", projects.len());

    // TODO: sort by date: either created_at or updated_at whichever is more
    // recent
    projects.sort_by(|a, b| {
        b.frontmatter.created_at.cmp(&a.frontmatter.created_at)
    });

    Ok(ProjectsPage { list: projects })
}

#[instrument]
async fn get_project_by_name(
    Path(file): Path<String>,
) -> crate::error::Result<Response> {
    // get the path to the project markdown file
    let path: PathBuf = format!("posts/projects/{}.md", file).into();

    // if the file doesn't exist then return a 404
    if !path.exists() {
        info!("project file not found: {:?}", path);
        return Ok(StatusCode::NOT_FOUND.into_response());
    }

    trace!("project file: {:?}", path);

    let mut project = Post::from_file_with_metadata(
        &path,
        Metadata {
            slug: format!("/projects/{file}"),
        },
    )
    .context("failed to create project")?;

    // actually parse the content into HTML
    project.parse_content()?;

    // make sure the content is HTML. this is a bit redundant since we just
    // parsed the content into HTML, however, this check should remain so
    // that the invariant doesn't get lost
    assert!(
        project.content.is_html(),
        "project content must be HTML to render"
    );

    Ok(ProjectPage { project }.into_response())
}

pub fn router() -> Router {
    Router::new()
        .route("/", get(get_project_list))
        .route("/:file", get(get_project_by_name))
}
