use std::fmt::Write as _;

use super::{
    Diagram, Element, Kind,
    text::{bare, quoted},
};
use crate::cli::C4Level;

/// Mermaid C4 (<https://mermaid.js.org/syntax/c4.html>), which GitLab and GitHub render inline.
pub fn mermaid(diagram: &Diagram) -> String {
    let header = match diagram.level {
        C4Level::System => "C4Context",
        C4Level::Container => "C4Container",
        C4Level::Component => "C4Component",
    };
    // A title ends at a line break, `;` or `#`:
    // https://github.com/mermaid-js/mermaid/blob/develop/packages/mermaid/src/diagrams/c4/parser/c4Diagram.jison
    format!("{header}\n{}", body(diagram, " ", &[';', '#']))
}

/// C4-PlantUML, included from the standard library (<https://plantuml.com/stdlib>).
pub fn plantuml(diagram: &Diagram) -> String {
    let library = match diagram.level {
        C4Level::System => "C4_Context",
        C4Level::Container => "C4_Container",
        C4Level::Component => "C4_Component",
    };
    format!(
        "@startuml\n!include <C4/{library}>\n\n{}@enduml\n",
        body(diagram, "\\n", &[])
    )
}

/// The macros both formats share: `title`, nested boundaries, elements and `Rel`. A line break
/// inside a quoted label is written `newline`; the title holds none of `title_forbidden`.
fn body(diagram: &Diagram, newline: &str, title_forbidden: &[char]) -> String {
    let q = |s: &str| quoted(s, newline);
    let mut out = format!("title {}\n", bare(&diagram.title, title_forbidden));
    let depth = diagram.boundaries.len();
    for (level, boundary) in diagram.boundaries.iter().enumerate() {
        let macro_name = if boundary.kind == Kind::System {
            "System_Boundary"
        } else {
            "Container_Boundary"
        };
        let _ = writeln!(
            out,
            "{}{macro_name}({}, {}) {{",
            "  ".repeat(level),
            boundary.id,
            q(&boundary.name)
        );
    }
    for element in &diagram.inner {
        let _ = writeln!(out, "{}{}", "  ".repeat(depth), element_line(element, &q));
    }
    for level in (0..depth).rev() {
        let _ = writeln!(out, "{}}}", "  ".repeat(level));
    }
    for element in &diagram.outer {
        let _ = writeln!(out, "{}", element_line(element, &q));
    }
    for rel in &diagram.rels {
        let _ = writeln!(out, "Rel({}, {}, {})", rel.from, rel.to, q(&rel.label));
    }
    out
}

fn element_line(element: &Element, q: &impl Fn(&str) -> String) -> String {
    let (id, name, description) = (&element.id, q(&element.name), q(&element.description));
    match element.kind {
        Kind::System => format!("System({id}, {name}, {description})"),
        Kind::SystemExt => format!("System_Ext({id}, {name}, {description})"),
        Kind::Container => format!(
            "Container({id}, {name}, {}, {description})",
            q(&element.technology)
        ),
        Kind::Component => format!(
            "Component({id}, {name}, {}, {description})",
            q(&element.technology)
        ),
    }
}
