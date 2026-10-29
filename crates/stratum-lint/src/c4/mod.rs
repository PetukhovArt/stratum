mod macros;
mod model;
mod structurizr;
mod text;

use crate::cli::{C4Format, C4Level};

pub use model::Request;

/// One C4 diagram, built once and written by any of the three renderers.
///
/// Every list is sorted: elements by name, relationships by `from`, then `to`. Ids are made of
/// `[A-Za-z0-9_]` only and are unique across the whole diagram.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagram {
    pub level: C4Level,
    pub title: String,
    /// The scopes around `inner`, outermost first: none at the system level, the `self` system
    /// at the container level, then the container at the component level.
    pub boundaries: Vec<Element>,
    pub inner: Vec<Element>,
    /// The other systems, outside every boundary.
    pub outer: Vec<Element>,
    pub rels: Vec<Rel>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    System,
    SystemExt,
    Container,
    Component,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Element {
    pub id: String,
    pub kind: Kind,
    pub name: String,
    /// Empty for systems.
    pub technology: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rel {
    pub from: String,
    pub to: String,
    pub label: String,
}

impl Diagram {
    /// # Errors
    /// The system level without `systems` in the config, an unknown `--feature` id, a component
    /// level without `--container` or with an unknown one; each message says what is known.
    pub fn of(input: &stratum_engine::EngineInput, request: &Request) -> miette::Result<Self> {
        model::diagram(input, request)
    }

    #[must_use]
    pub fn render(&self, format: C4Format) -> String {
        match format {
            C4Format::Mermaid => macros::mermaid(self),
            C4Format::Structurizr => structurizr::render(self),
            C4Format::Plantuml => macros::plantuml(self),
        }
    }
}
