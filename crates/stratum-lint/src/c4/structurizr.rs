use std::fmt::Write as _;

use super::{Diagram, Element, Kind, text::quoted};
use crate::cli::C4Level;

const EXTERNAL: &str = "External";
const STYLES: &str = r#"        styles {
            element "External" {
                background #999999
                color #ffffff
            }
        }
"#;

/// A Structurizr DSL workspace (<https://docs.structurizr.com/dsl/language>) with one view of
/// `diagram`, as Structurizr Lite opens it.
pub fn render(diagram: &Diagram) -> String {
    let q = |s: &str| quoted(s, " ");
    let mut out = format!("workspace {} {{\n    model {{\n", q(&diagram.title));
    let indent = |depth: usize| "    ".repeat(depth + 2);
    let depth = diagram.boundaries.len();
    for (level, scope) in diagram.boundaries.iter().enumerate() {
        let _ = writeln!(out, "{}{} {{", indent(level), declaration(scope, &q));
    }
    for element in &diagram.inner {
        let _ = writeln!(out, "{}{}", indent(depth), declaration(element, &q));
    }
    for level in (0..depth).rev() {
        let _ = writeln!(out, "{}}}", indent(level));
    }
    for element in &diagram.outer {
        let _ = writeln!(
            out,
            "{0}{1} {{\n{0}    tags {2}\n{0}}}",
            indent(0),
            declaration(element, &q),
            q(EXTERNAL)
        );
    }
    for rel in &diagram.rels {
        let _ = writeln!(
            out,
            "{}{} -> {} {}",
            indent(0),
            rel.from,
            rel.to,
            q(&rel.label)
        );
    }
    out.push_str("    }\n    views {\n");
    let (view, key) = match diagram.level {
        C4Level::System => ("systemContext", "context"),
        C4Level::Container => ("container", "containers"),
        C4Level::Component => ("component", "components"),
    };
    let subject = diagram
        .boundaries
        .last()
        .or_else(|| diagram.inner.first())
        .map_or("", |e| e.id.as_str());
    let _ = writeln!(out, "        {view} {subject} {} {{", q(key));
    out.push_str("            include *\n");
    if !diagram.outer.is_empty() {
        let outer: Vec<&str> = diagram.outer.iter().map(|e| e.id.as_str()).collect();
        let _ = writeln!(out, "            include {}", outer.join(" "));
    }
    out.push_str("            autoLayout\n        }\n");
    out.push_str(STYLES);
    out.push_str("    }\n}\n");
    out
}

fn declaration(element: &Element, q: &impl Fn(&str) -> String) -> String {
    let (id, name, description) = (&element.id, q(&element.name), q(&element.description));
    match element.kind {
        Kind::System | Kind::SystemExt => format!("{id} = softwareSystem {name} {description}"),
        Kind::Container => format!(
            "{id} = container {name} {description} {}",
            q(&element.technology)
        ),
        Kind::Component => format!(
            "{id} = component {name} {description} {}",
            q(&element.technology)
        ),
    }
}
