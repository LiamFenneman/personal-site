//! Generic External Links.
//!
//! This is allows for specific link types to have their own type. This can then
//! be used to handle links differently within the HTML templates.

use std::fmt;

use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer,
};

/// An external link for a project.
///
/// This is allows for specific links to have different handling within the HTML
/// template.
///
/// **NOTE**: when adding a new link type, ensure that it is added to the
/// `LinksVisitor` since the visitor is using the catch-all pattern match for
/// `Other` type links.
#[derive(Debug, Clone, serde::Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Link {
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
pub struct Links(pub Vec<Link>);

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
