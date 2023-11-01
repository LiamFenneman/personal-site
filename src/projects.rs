use std::fmt;
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
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer};
use tracing::error;

/// A page listing all projects.
#[derive(Template)]
#[template(path = "pages/projects.html")]
struct ProjectsPage {
    list: Vec<ProjectFrontmatter>,
}

/// Frontmatter from the `.md` files used to generate the posts.
#[derive(Debug, Clone, serde::Deserialize)]
struct ProjectFrontmatter {
    /// Slug is set after serialisation based on the file name, this is done
    /// since the slug is *not* included in the frontmatter.
    ///
    /// TODO: extract this out to some `Project` struct that includes the
    /// `ProjectFrontmatter` and the `slug` since it makes more sense if more
    /// attributes are to be added programmatically instead of parsed from the
    /// frontmatter.
    #[serde(skip)]
    slug: String,
    name: String,
    description: String,
    created_at: String,
    updated_at: Option<String>,
    links: Option<Links>,
}

/// Individual project page.
#[derive(Template)]
#[template(path = "pages/project.html")]
struct ProjectPage {
    frontmatter: ProjectFrontmatter,
    content: String,
}

/// An external link for a project.
///
/// This is allows for specific links to have different handling within the HTML
/// template.
///
/// **NOTE**: when adding a new link type, ensure that it is added to the
/// `LinksVisitor` since the visitor is using the catch-all pattern match for
/// `Other` type links.
#[derive(Debug, Clone, serde::Deserialize, PartialEq, Eq, PartialOrd, Ord)]
enum Link {
    /// A link to a GitHub repository.
    GitHub(String),
    /// A link to a website with the title (left) and the URL (right).
    Other(String, String),
}

/// A list of links for a project.
///
/// This wraps a `Vec<Link>` since we want to have custom deserialisation.
/// The alternative is to use a BTreeMap which is a bit more annoying to handle
/// the specific link types (e.g. GitHub) which have a different handling.
#[derive(Debug, Clone)]
struct Links(Vec<Link>);

/// Custom deserialisation for a list of `Link`s.
struct LinksVisitor;

impl<'de> Visitor<'de> for LinksVisitor {
    type Value = Links;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a map of String (link title) to String (link URL)")
    }

    fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
    where
        M: de::MapAccess<'de>,
    {
        let mut links = Links(Vec::new());

        // loop over every entry in the map and create a `Link`
        while let Some((key, value)) = access.next_entry()? {
            let link = match key {
                "GitHub" => Link::GitHub(value),
                _ => Link::Other(key.to_owned(), value),
            };
            links.0.push(link);
        }

        links.0.sort();

        Ok(links)
    }
}

impl<'de> Deserialize<'de> for Links {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // use the custom visitor (`LinksVisitor`) to deserialise
        deserializer.deserialize_map(LinksVisitor)
    }
}

pub fn router() -> Router {
    Router::new()
        .route("/", get(get_project_list))
        .route("/:file", get(get_project_by_name))
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
        .map(parse_frontmatter)
        // collect into a Vec<_> and propagate Result errors
        .collect::<Result<Vec<_>, _>>()?;

    // TODO: sort by date: either created_at or updated_at whichever is more
    // recent
    projects.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    Ok(ProjectsPage { list: projects })
}

async fn get_project_by_name(
    Path(file): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    // get the path to the project markdown file
    let path: PathBuf = format!("projects/{}.md", file).into();

    // TODO: both `parse_frontmatter` and `read_to_string` below are reading the
    // file. This *should* be avoided if possible.

    // parse the frontmatter so we have access to title, etc.
    let frontmatter = parse_frontmatter(&path)?;

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

    Ok(ProjectPage {
        frontmatter,
        content,
    })
}

/// Read and parse the markdown AST from the file path.
///
/// This returns an instance of `ProjectFrontmatter`.
fn parse_frontmatter(path: &PathBuf) -> Result<ProjectFrontmatter, StatusCode> {
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
            let mut frontmatter = serde_yaml::from_str::<ProjectFrontmatter>(
                value,
            )
            .map_err(|e| {
                error!("could not parse frontmatter: {e}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

            // Set the slug to the filename.
            //
            // This cannot be done by the deserialiser since the slug is not
            // included in the frontmatter.
            frontmatter.slug = format!(
                "/projects/{}",
                path.file_stem().unwrap().to_str().unwrap()
            );

            Ok(frontmatter)
        }
        _ => {
            error!("invalid markdown file: {path:?}");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
