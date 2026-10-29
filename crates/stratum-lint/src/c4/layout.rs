//! Where Mermaid C4 puts each shape and label, so the cell of each shape, the shapes in a row
//! and the offset of each label can keep arrows out of boxes and labels apart.
//!
//! Mermaid places shapes on a grid in declaration order, shapes outside every boundary first;
//! an empty cell is an invisible shape. It draws the first relationship straight and the
//! others as quadratic curves, each label starting at the middle of its two end points. The
//! numbers below are measured on mermaid 12 renders:
//! <https://github.com/mermaid-js/mermaid/blob/develop/packages/mermaid/src/diagrams/c4/c4Renderer.js>

use std::collections::BTreeMap;

use super::{Diagram, Element};

const WIDTH: f64 = 216.0;
const GAP: f64 = 100.0;
const EMPTY_HEIGHT: f64 = 52.0;
const BOUNDARY_INSET_X: f64 = 50.0;
const BOUNDARY_INSET_Y: f64 = 207.0;
const LABEL_HEIGHT: f64 = 14.0;
const CHAR_WIDTH: f64 = 6.0;
const DESCRIPTION_CHARS_PER_LINE: usize = 34;
const NAME_CHARS_PER_LINE: usize = 22;

/// Shapes in a row tried, the first winning ties; wider rows shrink the text on a page.
const ROW_WIDTHS: [usize; 3] = [3, 2, 4];
/// Placements tried one by one at most; past it, a local search from the packed placement.
const EXHAUSTIVE: usize = 5000;
const SEARCH_ROUNDS: usize = 30;
/// Past this many shapes inside the boundaries, the shapes stay packed in the model's order.
const MAX_SEARCHED: usize = 24;
/// Placements with the best arrows whose labels are then placed: placing labels takes time
/// in the square of the relationships, so fewer of them for more relationships.
const LABEL_WORK: usize = 20_000;
const SHORTLIST: (usize, usize) = (16, 256);
/// What an empty cell costs: the diagram grows, so only for clearer arrows or labels.
const EMPTY_CELL: f64 = 10.0;
/// What a label costs over a box or a label, crossed by an arrow or a boundary side, and per
/// step away from the middle of its arrow.
const LABEL_BLOCKED: f64 = 100.0;
const LABEL_CROSSED: f64 = 25.0;
const LABEL_STEP: f64 = 0.5;
/// A label over or under both ends of its arrow, when no spot beside the arrow is free.
const LABEL_AWAY: f64 = 80.0;

pub struct Layout<'d> {
    /// The cells outside every boundary in declaration order, `None` for an empty one.
    pub top: Vec<Option<&'d Element>>,
    /// The cells inside the innermost boundary in declaration order, `None` for an empty one.
    pub nested: Vec<Option<&'d Element>>,
    pub shapes_in_row: usize,
    /// Per relationship of the diagram, how far to move its label from where Mermaid puts it.
    pub offsets: Vec<(f64, f64)>,
}

/// The layout with the fewest arrows through boxes, the lightest of them first, then the
/// fewest labels over a box, a label or an arrow, among the layouts tried.
pub fn arrange(diagram: &Diagram) -> Layout<'_> {
    let (top, nested): (Vec<&Element>, Vec<&Element>) = if diagram.boundaries.is_empty() {
        (
            diagram.outer.iter().chain(&diagram.inner).collect(),
            Vec::new(),
        )
    } else {
        (
            diagram.outer.iter().collect(),
            diagram.inner.iter().collect(),
        )
    };
    let depth = diagram.boundaries.len();
    let mut shortlist: Vec<(f64, usize, Placement)> = Vec::new();
    for cols in ROW_WIDTHS {
        let slack = |n: usize| n.div_ceil(cols) * cols + cols;
        let cells = if nested.is_empty() {
            [slack(top.len()), 0]
        } else {
            [top.len().div_ceil(cols) * cols, slack(nested.len())]
        };
        let cost = |p: &Placement| {
            let (t, n) = (fill(&top, &p[0]), fill(&nested, &p[1]));
            let empty = t.iter().chain(&n).filter(|c| c.is_none()).count();
            Scene::new(diagram, &t, &n, depth, cols).route_cost() + EMPTY_CELL * real(empty)
        };
        shortlist.extend(
            search([top.len(), nested.len()], cells, cost)
                .into_iter()
                .map(|(c, p)| (c, cols, p)),
        );
    }
    shortlist.sort_by(|a, b| a.0.total_cmp(&b.0));
    let rels = diagram.rels.len().max(1);
    shortlist.truncate((LABEL_WORK / (rels * rels)).clamp(SHORTLIST.0, SHORTLIST.1));
    let best = shortlist
        .into_iter()
        .map(|(route, cols, p)| {
            let (t, n) = (fill(&top, &p[0]), fill(&nested, &p[1]));
            let (labels, offsets) = Scene::new(diagram, &t, &n, depth, cols).labels();
            (route + labels, cols, t, n, offsets)
        })
        .min_by(|a, b| a.0.total_cmp(&b.0));
    match best {
        Some((_, shapes_in_row, top, nested, offsets)) => Layout {
            top,
            nested,
            shapes_in_row,
            offsets,
        },
        None => Layout {
            top: top.into_iter().map(Some).collect(),
            nested: nested.into_iter().map(Some).collect(),
            shapes_in_row: ROW_WIDTHS[0],
            offsets: vec![(0.0, 0.0); diagram.rels.len()],
        },
    }
}

/// The cell of each shape, for the shapes outside the boundaries and those inside.
type Placement = [Vec<usize>; 2];

/// The cells up to the last shape, each holding its shape or `None`.
fn fill<'d>(shapes: &[&'d Element], cells: &[usize]) -> Vec<Option<&'d Element>> {
    let mut filled = vec![None; cells.iter().map(|&c| c + 1).max().unwrap_or(0)];
    for (shape, &cell) in shapes.iter().zip(cells) {
        filled[cell] = Some(*shape);
    }
    filled
}

/// Every placement of `shapes[g]` shapes in `cells[g]` cells with its cost when there are
/// few, the packed one first; else the placements a local search visits from the packed one,
/// moving one shape to another cell, or swapping two, while that lowers the cost.
fn search(
    shapes: [usize; 2],
    cells: [usize; 2],
    cost: impl Fn(&Placement) -> f64,
) -> Vec<(f64, Placement)> {
    let packed: Placement = [(0..shapes[0]).collect(), (0..shapes[1]).collect()];
    if shapes[1] > MAX_SEARCHED {
        return vec![(cost(&packed), packed)];
    }
    let count = arrangements(cells[0], shapes[0]).saturating_mul(arrangements(cells[1], shapes[1]));
    if count <= EXHAUSTIVE {
        let inner = injections(shapes[1], cells[1]);
        return injections(shapes[0], cells[0])
            .into_iter()
            .flat_map(|t| inner.iter().map(move |n| [t.clone(), n.clone()]))
            .map(|p| (cost(&p), p))
            .collect();
    }
    let mut current = (cost(&packed), packed);
    let mut seen = vec![current.clone()];
    for _ in 0..SEARCH_ROUNDS {
        let mut best: Option<(f64, Placement)> = None;
        for group in 0..2 {
            for shape in 0..shapes[group] {
                for cell in 0..cells[group] {
                    let mut next = current.1.clone();
                    let from = next[group][shape];
                    if cell == from {
                        continue;
                    }
                    if let Some(other) = next[group].iter().position(|&c| c == cell) {
                        next[group][other] = from;
                    }
                    next[group][shape] = cell;
                    let rated = (cost(&next), next);
                    if best.as_ref().is_none_or(|b| rated.0 < b.0) {
                        best = Some(rated.clone());
                    }
                    seen.push(rated);
                }
            }
        }
        match best {
            Some(next) if next.0 < current.0 => current = next,
            _ => break,
        }
    }
    seen
}

/// Ordered choices of `k` out of `n`.
fn arrangements(n: usize, k: usize) -> usize {
    if k > n {
        return 0;
    }
    (n - k + 1..=n).fold(1usize, usize::saturating_mul)
}

/// Every way to give `k` items distinct cells out of `n`, in lexicographic order.
fn injections(k: usize, n: usize) -> Vec<Vec<usize>> {
    fn extend(k: usize, n: usize, prefix: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
        if prefix.len() == k {
            out.push(prefix.clone());
            return;
        }
        for cell in 0..n {
            if !prefix.contains(&cell) {
                prefix.push(cell);
                extend(k, n, prefix, out);
                prefix.pop();
            }
        }
    }
    let mut out = Vec::new();
    extend(k, n, &mut Vec::with_capacity(k), &mut out);
    out
}

#[derive(Debug, Clone, Copy)]
struct Rect {
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
}

impl Rect {
    fn around(cx: f64, cy: f64, w: f64, h: f64) -> Self {
        Self {
            x0: cx - w / 2.0,
            y0: cy - h / 2.0,
            x1: cx + w / 2.0,
            y1: cy + h / 2.0,
        }
    }

    fn center(&self) -> (f64, f64) {
        (
            f64::midpoint(self.x0, self.x1),
            f64::midpoint(self.y0, self.y1),
        )
    }

    fn overlaps(&self, other: &Self, pad: f64) -> bool {
        self.x0 < other.x1 + pad
            && other.x0 < self.x1 + pad
            && self.y0 < other.y1 + pad
            && other.y0 < self.y1 + pad
    }

    fn holds(&self, (x, y): (f64, f64), inset: f64) -> bool {
        x > self.x0 + inset && x < self.x1 - inset && y > self.y0 + inset && y < self.y1 - inset
    }

    /// Where the ray from the center towards `(x, y)` leaves the rectangle.
    fn exit_towards(&self, (x, y): (f64, f64)) -> (f64, f64) {
        let (cx, cy) = self.center();
        let (dx, dy) = (x - cx, y - cy);
        let (hw, hh) = ((self.x1 - self.x0) / 2.0, (self.y1 - self.y0) / 2.0);
        let scale = [(dx.abs(), hw), (dy.abs(), hh)]
            .into_iter()
            .filter(|&(d, _)| d > f64::EPSILON)
            .map(|(d, half)| half / d)
            .fold(f64::INFINITY, f64::min);
        if scale.is_finite() {
            (cx + dx * scale, cy + dy * scale)
        } else {
            (cx, cy)
        }
    }

    fn crossed_by(&self, a: (f64, f64), b: (f64, f64)) -> bool {
        (0..=8).any(|k| {
            let t = f64::from(k) / 8.0;
            self.holds((a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t), 0.0)
        })
    }
}

fn real(n: usize) -> f64 {
    f64::from(u32::try_from(n).unwrap_or(u32::MAX))
}

fn height(element: &Element) -> f64 {
    let lines = |text: &str, per_line: usize| text.chars().count().div_ceil(per_line).max(1);
    let name = lines(&element.name, NAME_CHARS_PER_LINE);
    let description = lines(&element.description, DESCRIPTION_CHARS_PER_LINE);
    70.0 + 17.0 * real(name) + 12.0 * real(description - 1)
}

/// `cols` cells to a row from `(x, y)`, each row's tops aligned; returns the rectangle of
/// each shape, `None` for an empty cell, and the bottom of the last row.
fn grid(cells: &[Option<&Element>], cols: usize, x: f64, y: f64) -> (Vec<Option<Rect>>, f64) {
    let mut rects = Vec::with_capacity(cells.len());
    let mut top = y;
    let mut bottom = y;
    for row in cells.chunks(cols) {
        let row_height = row
            .iter()
            .map(|e| e.map_or(EMPTY_HEIGHT, height))
            .fold(0.0, f64::max);
        for (col, cell) in row.iter().enumerate() {
            let x0 = x + real(col) * (WIDTH + GAP);
            rects.push(cell.map(|e| Rect {
                x0,
                y0: top,
                x1: x0 + WIDTH,
                y1: top + height(e),
            }));
        }
        bottom = top + row_height;
        top = bottom + GAP;
    }
    (rects, bottom)
}

fn bounding<'r>(rects: impl IntoIterator<Item = &'r Rect>) -> Rect {
    rects.into_iter().fold(
        Rect {
            x0: f64::INFINITY,
            y0: f64::INFINITY,
            x1: f64::NEG_INFINITY,
            y1: f64::NEG_INFINITY,
        },
        |b, r| Rect {
            x0: b.x0.min(r.x0),
            y0: b.y0.min(r.y0),
            x1: b.x1.max(r.x1),
            y1: b.y1.max(r.y1),
        },
    )
}

struct Arrow {
    points: Vec<(f64, f64)>,
    span: Rect,
    start: (f64, f64),
    end: (f64, f64),
    from: usize,
    to: usize,
    rel: usize,
    weight: usize,
    chars: usize,
}

impl Arrow {
    fn at(&self, sample: usize) -> (f64, f64) {
        self.points[sample.min(self.points.len() - 1)]
    }

    fn crosses(&self, rect: &Rect) -> bool {
        self.span.overlaps(rect, 1.0) && self.points.windows(2).any(|s| rect.crossed_by(s[0], s[1]))
    }

    fn meets(&self, other: &Self) -> bool {
        let coarse =
            |a: &Self| -> Vec<(f64, f64)> { a.points.iter().step_by(3).copied().collect() };
        let (mine, theirs) = (coarse(self), coarse(other));
        mine.windows(2).any(|s| {
            theirs
                .windows(2)
                .any(|t| segments_cross(s[0], s[1], t[0], t[1]))
        })
    }
}

fn segments_cross(a: (f64, f64), b: (f64, f64), c: (f64, f64), d: (f64, f64)) -> bool {
    let side = |p: (f64, f64), q: (f64, f64), r: (f64, f64)| {
        (q.0 - p.0) * (r.1 - p.1) - (q.1 - p.1) * (r.0 - p.0)
    };
    side(a, b, c) * side(a, b, d) < 0.0 && side(c, d, a) * side(c, d, b) < 0.0
}

const SAMPLES: u32 = 24;
const ARROWS_CROSSING: f64 = 30.0;
/// The samples of an arrow a label may sit by, the middle first, a quarter of the arrow
/// at most away from it.
const ALONG: [usize; 13] = [12, 11, 13, 10, 14, 9, 15, 8, 16, 7, 17, 6, 18];

struct Scene {
    rects: Vec<Rect>,
    /// The innermost boundary's name and kind, over the first nested row.
    title: Option<Rect>,
    /// The four sides of the innermost boundary's dashed frame.
    edges: Vec<Rect>,
    arrows: Vec<Arrow>,
    rels: usize,
}

impl Scene {
    fn new(
        diagram: &Diagram,
        top: &[Option<&Element>],
        nested: &[Option<&Element>],
        depth: usize,
        cols: usize,
    ) -> Self {
        let (outside, bottom) = grid(top, cols, 0.0, 0.0);
        let nested_y = if top.is_empty() {
            0.0
        } else {
            bottom + BOUNDARY_INSET_Y
        };
        let (inside, _) = grid(nested, cols, real(depth) * BOUNDARY_INSET_X, nested_y);
        let (title, edges) = match diagram.boundaries.last() {
            Some(boundary) if !nested.is_empty() => {
                let held = bounding(inside.iter().flatten());
                let w = (real(boundary.name.chars().count()) * 10.0).max(80.0);
                let (x0, y0) = (held.x0 - BOUNDARY_INSET_X, nested_y - GAP);
                let (x1, y1) = (held.x1 + BOUNDARY_INSET_X, held.y1 + BOUNDARY_INSET_X);
                let edges = vec![
                    Rect::around(f64::midpoint(x0, x1), y0, x1 - x0, 2.0),
                    Rect::around(f64::midpoint(x0, x1), y1, x1 - x0, 2.0),
                    Rect::around(x0, f64::midpoint(y0, y1), 2.0, y1 - y0),
                    Rect::around(x1, f64::midpoint(y0, y1), 2.0, y1 - y0),
                ];
                let title = Rect::around(f64::midpoint(held.x0, held.x1), nested_y - 75.0, w, 40.0);
                (Some(title), edges)
            }
            _ => (None, Vec::new()),
        };
        let shapes: Vec<(&Element, Rect)> = top
            .iter()
            .zip(&outside)
            .chain(nested.iter().zip(&inside))
            .filter_map(|(e, r)| Some(((*e)?, (*r)?)))
            .collect();
        let rects: Vec<Rect> = shapes.iter().map(|&(_, r)| r).collect();
        let index: BTreeMap<&str, usize> = shapes
            .iter()
            .enumerate()
            .map(|(i, (e, _))| (e.id.as_str(), i))
            .collect();
        let arrows = diagram
            .rels
            .iter()
            .enumerate()
            .filter_map(|(k, rel)| {
                let (from, to) = (*index.get(rel.from.as_str())?, *index.get(rel.to.as_str())?);
                let start = rects[from].exit_towards(rects[to].center());
                let end = rects[to].exit_towards(rects[from].center());
                let control = if k == 0 {
                    (f64::midpoint(start.0, end.0), f64::midpoint(start.1, end.1))
                } else {
                    (
                        start.0 + (end.0 - start.0) / 4.0,
                        start.1 + (end.1 - start.1) / 2.0,
                    )
                };
                let points: Vec<(f64, f64)> = (0..=SAMPLES)
                    .map(|s| {
                        let t = f64::from(s) / f64::from(SAMPLES);
                        let u = 1.0 - t;
                        (
                            u * u * start.0 + 2.0 * u * t * control.0 + t * t * end.0,
                            u * u * start.1 + 2.0 * u * t * control.1 + t * t * end.1,
                        )
                    })
                    .collect();
                Some(Arrow {
                    span: Rect {
                        x0: points
                            .iter()
                            .map(|p: &(f64, f64)| p.0)
                            .fold(f64::INFINITY, f64::min),
                        y0: points.iter().map(|p| p.1).fold(f64::INFINITY, f64::min),
                        x1: points.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max),
                        y1: points.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max),
                    },
                    points,
                    start,
                    end,
                    from,
                    to,
                    rel: k,
                    weight: rel.weight,
                    chars: rel.label.chars().count(),
                })
            })
            .collect();
        Self {
            rects,
            title,
            edges,
            arrows,
            rels: diagram.rels.len(),
        }
    }

    /// Each arrow through a box weighs the most, more the more it carries; then each arrow
    /// through the boundary's title, each two arrows crossing, then each arrow pointing up and
    /// its length, both more for an arrow that carries more.
    fn route_cost(&self) -> f64 {
        let mut cost = 0.0;
        for arrow in &self.arrows {
            let through = self
                .rects
                .iter()
                .enumerate()
                .filter(|&(i, r)| {
                    i != arrow.from
                        && i != arrow.to
                        && arrow.points.iter().any(|&p| r.holds(p, -6.0))
                })
                .count();
            cost += real(through) * (1000.0 + real(arrow.weight.min(500)));
            if self.title.is_some_and(|t| arrow.crosses(&t)) {
                cost += 200.0;
            }
            let heft = 1.0 + real(arrow.weight).ln_1p();
            if self.rects[arrow.to].center().1 < self.rects[arrow.from].center().1 - 1.0 {
                cost += 20.0 * heft;
            }
            let (dx, dy) = (arrow.end.0 - arrow.start.0, arrow.end.1 - arrow.start.1);
            cost += dx.hypot(dy) / 100.0 * heft;
        }
        for (i, a) in self.arrows.iter().enumerate() {
            for b in &self.arrows[i + 1..] {
                let shared_end = [a.from, a.to].iter().any(|e| *e == b.from || *e == b.to);
                if !shared_end && a.meets(b) {
                    cost += ARROWS_CROSSING;
                }
            }
        }
        cost
    }

    /// Places each label, in relationship order, beside, over or under a point of its arrow,
    /// the middle first: never over a box, the boundary's title or a label placed before it
    /// when another spot is free, and crossed by as few arrows and boundary sides as can be.
    /// Returns the cost of what could not be placed cleanly and every label's offset.
    fn labels(&self) -> (f64, Vec<(f64, f64)>) {
        let mut placed: Vec<Rect> = Vec::new();
        let mut offsets = vec![(0.0, 0.0); self.rels];
        let mut cost = 0.0;
        let bounds = bounding(&self.rects);
        for arrow in &self.arrows {
            let w = real(arrow.chars) * CHAR_WIDTH;
            let mid = (
                f64::midpoint(arrow.start.0, arrow.end.0),
                f64::midpoint(arrow.start.1, arrow.end.1),
            );
            let (from, to) = (self.rects[arrow.from], self.rects[arrow.to]);
            let away = [
                (
                    LABEL_AWAY,
                    Rect::around(mid.0, from.y0.min(to.y0) - 12.0, w, LABEL_HEIGHT),
                ),
                (
                    LABEL_AWAY,
                    Rect::around(mid.0, from.y1.max(to.y1) + 12.0, w, LABEL_HEIGHT),
                ),
            ];
            let beside = ALONG.into_iter().enumerate().flat_map(|(rank, sample)| {
                let (x, y) = arrow.at(sample);
                let step = real(rank) * LABEL_STEP;
                [
                    (step, Rect::around(x + 6.0 + w / 2.0, y, w, LABEL_HEIGHT)),
                    (step, Rect::around(x - 6.0 - w / 2.0, y, w, LABEL_HEIGHT)),
                    (step, Rect::around(x, y - 11.0, w, LABEL_HEIGHT)),
                    (step, Rect::around(x, y + 11.0, w, LABEL_HEIGHT)),
                ]
            });
            let mut best: Option<(f64, Rect)> = None;
            for (penalty, spot) in beside.chain(away) {
                let blocked = self.rects.iter().any(|r| r.overlaps(&spot, 4.0))
                    || self.title.is_some_and(|r| r.overlaps(&spot, 2.0))
                    || placed.iter().any(|p| p.overlaps(&spot, 3.0))
                    || spot.x0 < bounds.x0 - 40.0
                    || spot.x1 > bounds.x1 + 40.0;
                let crossed = self.arrows.iter().filter(|a| a.crosses(&spot)).count()
                    + self.edges.iter().filter(|e| e.overlaps(&spot, 4.0)).count();
                let spot_cost = if blocked { LABEL_BLOCKED } else { 0.0 }
                    + LABEL_CROSSED * real(crossed)
                    + penalty;
                if best.is_none_or(|(b, _)| spot_cost < b) {
                    best = Some((spot_cost, spot));
                }
            }
            let Some((spot_cost, spot)) = best else {
                continue;
            };
            cost += spot_cost;
            placed.push(spot);
            let (cx, cy) = spot.center();
            offsets[arrow.rel] = (
                (cx - mid.0 - w / 2.0).round() + 0.0,
                (cy - mid.1).round() + 0.0,
            );
        }
        (cost, offsets)
    }
}
