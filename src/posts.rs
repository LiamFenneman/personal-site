//! Generic post type.
//!
//! This can then be used to generate different types of posts (e.g. projects,
//! blog posts, etc.).

use std::fmt::{Debug, Display};
use std::path::Path;

use anyhow::bail;
use markdown::{Constructs, Options, ParseOptions};

/// A post with some generic frontmatter and parsed markdown content.
pub struct Post<Frontmatter, Metadata> {
    /// Frontmatter parsed from the markdown file.
    pub frontmatter: Frontmatter,

    /// The content of the post.
    ///
    /// By default this will be `Markdown` since this will skip the HTML
    /// parsing step.
    pub content: Content,

    /// Additional metadata that can be used to pass to the template.
    ///
    /// This is not an `Option` since if you wanted to not use metadata you can
    /// simply use `from_path` which sets the type of `Metadata` to the
    /// unit type `()`.
    pub metadata: Metadata,
}

#[derive(Debug, Clone)]
pub enum Content {
    Markdown(String),
    Html(String),
}

impl Display for Content {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Use the underlying `String` to display the content.
        match self {
            Content::Markdown(content) => write!(f, "{}", content),
            Content::Html(content) => write!(f, "{}", content),
        }
    }
}

impl<Frontmatter> Post<Frontmatter, ()>
where
    Frontmatter: serde::de::DeserializeOwned,
{
    /// Create a new post from a file path.
    pub fn from_file(path: &Path) -> anyhow::Result<Self> {
        Self::from_file_with_metadata(path, ())
    }
}

impl<Frontmatter, Metadata> Post<Frontmatter, Metadata>
where
    Frontmatter: serde::de::DeserializeOwned,
{
    /// Create a new post from a file path and metadata.
    pub fn from_file_with_metadata(
        path: &Path,
        metadata: Metadata,
    ) -> anyhow::Result<Self> {
        let file = std::fs::read_to_string(path)?;
        let frontmatter = parse_frontmatter(&file)?;
        // let content = parse_content(&file)?;

        Ok(Post {
            frontmatter,
            content: Content::Markdown(file),
            metadata,
        })
    }

    /// Parse the Markdown content within the post.
    ///
    /// This will replace the `Markdown` variant with `Html` variant.
    pub fn parse_content(&mut self) -> anyhow::Result<()> {
        match self.content {
            Content::Markdown(ref file) => {
                let content = parse_content(file)?;
                self.content = Content::Html(content);
                Ok(())
            }
            Content::Html(_) => {
                bail!("post content already parsed");
            }
        }
    }
}

/// Parse the frontmatter from the given file.
///
/// The frontmatter is the first `Yaml` node within a `Root` node.
fn parse_frontmatter<Frontmatter>(file: &str) -> anyhow::Result<Frontmatter>
where
    Frontmatter: serde::de::DeserializeOwned,
{
    use markdown::mdast::*;

    // parse the file into a Markdown AST
    let ast = markdown::to_mdast(file, &parse_options());

    match ast {
        // the first node in the markdown AST *should* be a `Root`.
        // the first node within the `Root` node is the `Yaml` frontmatter.
        Ok(Node::Root(Root { children, .. })) => {
            // extract the `Yaml` node from the AST
            // if the first node is not `Yaml` then the markdown file doesn't
            // have frontmatter
            let Some(Node::Yaml(Yaml { value, .. })) = children.get(0) else {
                bail!("frontmatter not found");
            };

            // parse the `Yaml` into the frontmatter struct
            let frontmatter = serde_yaml::from_str::<Frontmatter>(value)?;

            Ok(frontmatter)
        }
        _ => {
            bail!("invalid markdown file");
        }
    }
}

/// Parse the given file into HTML.
///
/// This is assuming that the file is markdown.
fn parse_content(file: &str) -> anyhow::Result<String> {
    let Ok(content) = markdown::to_html_with_options(file, &full_options())
    else {
        bail!("could not parse markdown file");
    };

    // ensure that all links open in a new tab and don't use HTMX
    let content = content
        .replace("<a href=", "<a hx-boost=\"false\" target=\"_blank\" href=");

    Ok(content)
}

// use default options, except enable frontmatter parsing
fn parse_options() -> ParseOptions {
    ParseOptions {
        constructs: Constructs {
            frontmatter: true,
            ..Constructs::default()
        },
        ..ParseOptions::default()
    }
}

// use `parse_options()` for parse options and everything else is default
fn full_options() -> Options {
    Options {
        parse: ParseOptions { ..parse_options() },
        ..Options::default()
    }
}

impl<F: Debug, M: Debug> Debug for Post<F, M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Post")
            .field("frontmatter", &self.frontmatter)
            .field("content", &self.content)
            .field("metadata", &self.metadata)
            .finish()
    }
}

impl<F: Clone, M: Clone> Clone for Post<F, M> {
    fn clone(&self) -> Self {
        Post {
            frontmatter: self.frontmatter.clone(),
            content: self.content.clone(),
            metadata: self.metadata.clone(),
        }
    }
}
