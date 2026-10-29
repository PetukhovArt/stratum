use std::fmt::Write as _;

use super::{
    Diagram, Element, Kind,
    layout::{self, OTHERS_ID},
    text::{bare, quoted},
};
use crate::cli::C4Level;

/// Mermaid C4 (<https://mermaid.js.org/syntax/c4.html>), which GitLab and GitHub render inline,
/// with the shapes placed and the labels moved by [`layout::arrange`]; an empty cell of the
/// grid is a shape drawn transparent. Inside a boundary, the other systems go into an
/// untitled one below it.
pub fn mermaid(diagram: &Diagram) -> String {
    let header = match diagram.level {
        C4Level::System => "C4Context",
        C4Level::Container => "C4Container",
        C4Level::Component => "C4Component",
    };
    let layout = layout::arrange(diagram);
    let cells = Cells {
        top: &layout.top,
        nested: &layout.nested,
        others: &layout.others,
    };
    let mut out = format!("{header}\n{}", body(diagram, &cells));
    let empty = layout
        .top
        .iter()
        .chain(&layout.nested)
        .chain(&layout.others)
        .filter(|c| c.is_none())
        .count();
    for n in 1..=empty {
        let _ = writeln!(
            out,
            "UpdateElementStyle({EMPTY}_{n}, $fontColor=\"transparent\", $bgColor=\"transparent\", $borderColor=\"transparent\")"
        );
    }
    for (rel, (x, y)) in diagram.rels.iter().zip(&layout.offsets) {
        let _ = writeln!(
            out,
            "UpdateRelStyle({}, {}, $offsetX=\"{x}\", $offsetY=\"{y}\")",
            rel.from, rel.to
        );
    }
    let _ = writeln!(
        out,
        "UpdateLayoutConfig($c4ShapeInRow=\"{}\", $c4BoundaryInRow=\"1\")",
        layout.shapes_in_row
    );
    out
}

const EMPTY: &str = "empty";

/// The elements in declaration order, a `None` for an empty shape.
struct Cells<'c, 'd> {
    top: &'c [Option<&'d Element>],
    nested: &'c [Option<&'d Element>],
    /// In a boundary of their own after the nested ones.
    others: &'c [Option<&'d Element>],
}

/// `title`, the `top` elements, nested boundaries around the `nested` ones, the boundary of the
/// `others`, then `Rel`. A `None` is an empty shape, `empty_1`, `empty_2`… in order.
fn body(diagram: &Diagram, cells: &Cells) -> String {
    let mut out = format!("title {}\n", bare(&diagram.title));
    let mut empty = 0;
    let mut line = |cell: Option<&Element>| {
        cell.map_or_else(
            || {
                empty += 1;
                format!("Container({EMPTY}_{empty}, \"\", \"\", \"\")")
            },
            element_line,
        )
    };
    for &cell in cells.top {
        let _ = writeln!(out, "{}", line(cell));
    }
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
            quoted(&boundary.name)
        );
    }
    for &cell in cells.nested {
        let _ = writeln!(out, "{}{}", "  ".repeat(depth), line(cell));
    }
    for level in (0..depth).rev() {
        let _ = writeln!(out, "{}}}", "  ".repeat(level));
    }
    if !cells.others.is_empty() {
        let _ = writeln!(out, "Boundary({OTHERS_ID}, \"\", \"\") {{");
        for &cell in cells.others {
            let _ = writeln!(out, "  {}", line(cell));
        }
        out.push_str("}\n");
    }
    for rel in &diagram.rels {
        let _ = writeln!(out, "Rel({}, {}, {})", rel.from, rel.to, quoted(&rel.label));
    }
    out
}

fn element_line(element: &Element) -> String {
    let (id, name, description) = (
        &element.id,
        quoted(&element.name),
        quoted(&element.description),
    );
    match element.kind {
        Kind::System => format!("System({id}, {name}, {description})"),
        Kind::SystemExt => format!("System_Ext({id}, {name}, {description})"),
        Kind::Container => format!(
            "Container({id}, {name}, {}, {description})",
            quoted(&element.technology)
        ),
        Kind::Component => format!(
            "Component({id}, {name}, {}, {description})",
            quoted(&element.technology)
        ),
    }
}
