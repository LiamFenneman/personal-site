use anyhow::Context;
use askama::Template;
use axum::{response::IntoResponse, routing::get, Router};

#[derive(Debug, Clone, Template, serde::Deserialize)]
#[template(path = "pages/resume.html")]
struct ResumePage {
    education: Vec<Education>,
    skills: Vec<Skill>,
    projects: Vec<Project>,
    experience: Vec<Experience>,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
struct Education {
    what: String,
    r#where: String,
    when: String,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
struct Skill {
    title: String,
    list: Vec<String>,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
struct Project {
    title: String,
    url: Option<String>,
    list: Vec<String>,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
struct Experience {
    r#where: String,
    role: String,
    location: String,
    when: String,
    list: Vec<String>,
}

async fn get_resume() -> crate::error::Result<impl IntoResponse> {
    let file = std::fs::read_to_string("posts/resume.ron")
        .context("could not open RON file: posts/resume.ron")?;

    let page = ron::from_str::<ResumePage>(&file)
        .context("could not parse RON file: posts/resume.ron")?;

    Ok(page)
}

pub fn router() -> Router {
    Router::new().route("/", get(get_resume))
}
