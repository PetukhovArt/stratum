mod layout;
mod mermaid;
mod model;
mod text;

use crate::cli::C4Level;

pub use model::Request;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagram {
    pub level: C4Level,
    pub title: String,
    pub boundaries: Vec<Element>,
    pub inner: Vec<Element>,
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
    pub technology: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rel {
    pub from: String,
    pub to: String,
    pub label: String,
    pub weight: usize,
}

impl Diagram {
    pub fn of(input: &stratum_engine::EngineInput, request: &Request) -> miette::Result<Self> {
        model::diagram(input, request)
    }

    #[must_use]
    pub fn render(&self) -> String {
        mermaid::mermaid(self)
    }
}
