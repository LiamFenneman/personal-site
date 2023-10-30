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

#[derive(Template)]
#[template(path = "pages/projects.html")]
struct ProjectsPage {
    list: Vec<ProjectFrontmatter>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct ProjectFrontmatter {
    // slug is set after serialisation based on the file name
    #[serde(skip)]
    slug: String,
    name: String,
    description: String,
    created_at: String,
    updated_at: Option<String>,
    links: Option<Links>,
}

#[derive(Template)]
#[template(path = "pages/project.html")]
struct ProjectPage {
    frontmatter: ProjectFrontmatter,
    content: String,
}

#[derive(Debug, Clone, serde::Deserialize, PartialEq, Eq, PartialOrd, Ord)]
enum Link {
    /// A link to a GitHub repository providing the URL.
    GitHub(String),
    /// A link to a website providing the title (left) and the URL (right).
    Other(String, String),
}

#[derive(Debug, Clone)]
struct Links(Vec<Link>);

struct LinksVisitor;

impl<'de> Visitor<'de> for LinksVisitor {
    type Value = Links;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("an integer between -2^31 and 2^31")
    }

    fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
    where
        M: de::MapAccess<'de>,
    {
        let mut links = Links(Vec::new());

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
        deserializer.deserialize_map(LinksVisitor)
    }
}

pub fn router() -> Router {
    Router::new()
        .route("/", get(get_project_list))
        .route("/:file", get(get_project_by_name))
}

async fn get_project_list() -> Result<impl IntoResponse, StatusCode> {
    // search /projects directory for md files
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
            // filter out files that start with _ (underscore)
            path.file_stem().is_some_and(|stem| {
                stem.to_str()
                    .is_some_and(|s| s.chars().next().is_some_and(|c| c != '_'))
            })
        })
        .map(parse_frontmatter)
        .collect::<Result<Vec<_>, _>>()?;

    projects.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    Ok(ProjectsPage { list: projects })
}

async fn get_project_by_name(
    Path(file): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    let path: PathBuf = format!("projects/{}.md", file).into();

    let frontmatter = parse_frontmatter(&path)?;

    // read the project markdown file
    let Ok(file) = &std::fs::read_to_string(path.clone()) else {
        error!("could not read file: {path:?}");
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    };

    // parse the file into HTML
    let Ok(content) = to_html_with_options(
        file,
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

    Ok(ProjectPage { frontmatter, content })
}

fn parse_frontmatter(path: &PathBuf) -> Result<ProjectFrontmatter, StatusCode> {
    // read the project markdown file
    let Ok(file) = &std::fs::read_to_string(path) else {
        error!("could not read file: {path:?}");
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    };

    // parse the file into a Markdown AST
    let ast = to_mdast(
        file,
        &ParseOptions {
            constructs: Constructs {
                frontmatter: true,
                ..Constructs::default()
            },
            ..ParseOptions::default()
        },
    );

    match ast {
        Ok(Node::Root(Root { children, .. })) => {
            // extract the frontmatter from the AST
            // this is the first instance of a YAML node
            let Some(Node::Yaml(Yaml { value, .. })) = children.get(0) else {
                error!("frontmatter not found");
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            };

            // parse the YAML into the frontmatter struct
            let mut frontmatter = serde_yaml::from_str::<ProjectFrontmatter>(
                value,
            )
            .map_err(|e| {
                error!("could not parse frontmatter: {e}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

            // set the slug to the filename
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
