use std::path::PathBuf;

use askama::Template;
use axum::{
    extract::Path, http::StatusCode, response::IntoResponse, routing::get,
    Router,
};
use markdown::mdast::{Node, Root, Yaml};
use markdown::{
    to_html_with_options, to_mdast, Constructs, Options, ParseOptions,
};
use tracing::error;

use crate::links::{Link, Links};

/// A page listing all projects.
#[derive(Template)]
#[template(path = "pages/projects.html")]
struct ProjectsPage {
    list: Vec<Project>,
}

/// Individual project page.
#[derive(Template)]
#[template(path = "pages/project.html")]
struct ProjectPage {
    project: Project,
    content: String,
}

/// Project information. This includes data from the frontmatter and data from
/// other sources.
#[derive(Debug, Clone, serde::Deserialize)]
struct Project {
    name: String,
    description: String,
    metadata: Metadata,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct Metadata {
    slug: String,
    created_at: String,
    updated_at: Option<String>,
    links: Option<Links>,
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

/// Parse the project markdown file and return the project information.
///
/// This is different to `parse_frontmatter` since it also includes additional
/// information (e.g. URL slug).
fn parse_project(path: &PathBuf) -> Result<Project, StatusCode> {
    let frontmatter = parse_frontmatter(path)?;

    // Set the slug to the filename.
    let slug =
        format!("/projects/{}", path.file_stem().unwrap().to_str().unwrap());

    Ok(Project {
        name: frontmatter.name,
        description: frontmatter.description,
        metadata: Metadata {
            slug,
            created_at: frontmatter.created_at,
            updated_at: frontmatter.updated_at,
            links: frontmatter.links,
        },
    })
}

/// Read and parse the markdown AST from the file path.
///
/// This returns an instance of `ProjectFrontmatter`.
fn parse_frontmatter(path: &PathBuf) -> Result<Frontmatter, StatusCode> {
    // read the project markdown file
    let Ok(file) = &std::fs::read_to_string(path) else {
        error!("could not read file: {path:?}");
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    };

    // parse the file into a Markdown AST
    let ast = to_mdast(
        file,
        // use default options, except enable frontmatter parsing
        &ParseOptions {
            constructs: Constructs {
                frontmatter: true,
                ..Constructs::default()
            },
            ..ParseOptions::default()
        },
    );

    match ast {
        // the first node in the markdown AST *should* be a `Root`.
        // the first node within the `Root` node is the `Yaml` frontmatter.
        Ok(Node::Root(Root { children, .. })) => {
            // extract the `Yaml` node from the AST
            // if the first node is not `Yaml` then the markdown file doesn't
            // have frontmatter
            let Some(Node::Yaml(Yaml { value, .. })) = children.get(0) else {
                error!("frontmatter not found");
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            };

            // parse the `Yaml` into the frontmatter struct
            let frontmatter = serde_yaml::from_str::<Frontmatter>(value)
                .map_err(|e| {
                    error!("could not parse frontmatter: {e}");
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;

            Ok(frontmatter)
        }
        _ => {
            error!("invalid markdown file: {path:?}");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn get_project_list() -> Result<impl IntoResponse, StatusCode> {
    // search `./projects` directory for markdown files
    let projects = std::fs::read_dir("projects")
        .map_err(|e| {
            error!("could not read projects directory: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .map(|res| res.map(|e| e.path()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| {
            error!("error while reading directory: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // read each file and parse frontmatter
    // return a project page with each project
    let mut projects = projects
        .iter()
        .filter(|&path| {
            // filter out files that start with _ (underscore) this allows for
            // "private" projects that are not shown in the list
            path.file_stem().is_some_and(|stem| {
                stem.to_str()
                    .is_some_and(|s| s.chars().next().is_some_and(|c| c != '_'))
            })
        })
        .map(parse_project)
        // collect into a Vec<_> and propagate Result errors
        .collect::<Result<Vec<_>, _>>()?;

    // TODO: sort by date: either created_at or updated_at whichever is more
    // recent
    projects.sort_by(|a, b| b.metadata.created_at.cmp(&a.metadata.created_at));

    Ok(ProjectsPage { list: projects })
}

async fn get_project_by_name(
    Path(file): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    // get the path to the project markdown file
    let path: PathBuf = format!("projects/{}.md", file).into();

    // TODO: both `parse_frontmatter` and `read_to_string` below are reading the
    // file. This *should* be avoided if possible.

    // parse the project (metadata) so we have access to title, etc.
    let project = parse_project(&path)?;

    // read the project markdown file
    let Ok(file) = &std::fs::read_to_string(path.clone()) else {
        error!("could not read file: {path:?}");
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    };

    // parse the file into HTML
    let Ok(content) = to_html_with_options(
        file,
        // use default options but enable frontmatter parsing
        &Options {
            parse: ParseOptions {
                constructs: Constructs {
                    frontmatter: true,
                    ..Constructs::default()
                },
                ..ParseOptions::default()
            },
            ..Options::default()
        },
    ) else {
        error!("could not parse markdown file: {path:?}");
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    };

    // ensure that all links open in a new tab and don't use HTMX
    let content = content
        .replace("<a href=", "<a hx-boost=\"false\" target=\"_blank\" href=");

    Ok(ProjectPage { project, content })
}

pub fn router() -> Router {
    Router::new()
        .route("/", get(get_project_list))
        .route("/:file", get(get_project_by_name))
}
