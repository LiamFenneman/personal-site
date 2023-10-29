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
}

#[derive(Template)]
#[template(path = "pages/project.html")]
struct ProjectPage {
    content: String,
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
    Ok(ProjectsPage {
        list: projects
            .iter()
            .filter(|&path| {
                // filter out files that start with _ (underscore)
                path.file_stem().is_some_and(|stem| {
                    stem.to_str().is_some_and(|s| {
                        s.chars().next().is_some_and(|c| c != '_')
                    })
                })
            })
            .map(|path| {
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
                        let Some(Node::Yaml(Yaml { value, .. })) =
                            children.get(0)
                        else {
                            error!("frontmatter not found");
                            return Err(StatusCode::INTERNAL_SERVER_ERROR);
                        };

                        // parse the YAML into the frontmatter struct
                        let Ok(mut frontmatter) =
                            serde_yaml::from_str::<ProjectFrontmatter>(value)
                        else {
                            error!("could not parse frontmatter");
                            return Err(StatusCode::INTERNAL_SERVER_ERROR);
                        };

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
            })
            .collect::<Result<Vec<_>, _>>()?,
    })
}

async fn get_project_by_name(
    Path(file): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    let path = format!("projects/{}.md", file);

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

    Ok(ProjectPage { content })
}
