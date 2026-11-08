//! Geometric matching primitives used by `assemble` to recover a diagram graph from
//! recovered outlines, connectors and labels.

use std::collections::HashMap;

use super::{
    ARROWHEAD_AREA_RATIO, ARROWHEAD_MAX_SIDE_RATIO, CAPTION_CEILING, CAPTION_REACH, CONCENTRIC_AREA_RATIO,
    CONTAINER_MIN_MEMBERS, FRAME_MIN_FLUSH_EDGES, GRID_CLUSTER_RATIO, GRID_MIN_CELLS, GRIDLINE_AXIS_EPSILON,
    GRIDLINE_MIN_COUNT, GRIDLINE_SPACING_TOLERANCE_RATIO,
};
use super::{Connector, Label, Outline, Rect};

/// Drop outlines that are the inner ring of a double border.
///
/// The test is containment plus similar size. A panel enclosing several boxes
/// also contains them, but it is far larger, so the ratio keeps the two cases
/// apart. The outer outline survives, since that is the shape a connector
/// actually reaches.
/// Median of a set of areas, or `None` when the set is empty.
pub(super) fn median_area(areas: impl Iterator<Item = f32>) -> Option<f32> {
    let mut sorted: Vec<f32> = areas.filter(|a| a.is_finite()).collect();
    sorted.sort_by(f32::total_cmp);
    sorted.get(sorted.len() / 2).copied()
}

/// Whether an outline's text is laid out as a grid, which makes it a table
/// rather than a node.
///
/// The discriminator is rows *and* columns together, because each on its own is
/// something a real node does. A node's caption wraps onto several lines, which
/// is many rows in one column. A Graphviz record divides one node into fields
/// side by side, which is one row in many columns. Only a table is both, and a
/// ruled table sitting on the same page as a diagram is otherwise a perfect
/// node: one closed rectangle with text inside it.
pub(super) fn holds_a_grid(bbox: &Rect, texts: &[&Label]) -> bool {
    if texts.len() < GRID_MIN_CELLS {
        return false;
    }
    let rows = cluster_count(texts.iter().map(|t| t.y), bbox.height() * GRID_CLUSTER_RATIO);
    let columns = cluster_count(texts.iter().map(|t| t.x), bbox.width() * GRID_CLUSTER_RATIO);
    rows > 1 && columns > 1
}

/// How many distinct values a set of coordinates falls into, given how far
/// apart two of them must be to count as distinct.
pub(super) fn cluster_count(values: impl Iterator<Item = f32>, tolerance: f32) -> usize {
    let mut sorted: Vec<f32> = values.filter(|v| v.is_finite()).collect();
    sorted.sort_by(f32::total_cmp);
    let mut clusters = 0;
    let mut current = f32::NEG_INFINITY;
    for value in sorted {
        if clusters == 0 || value - current > tolerance.max(f32::EPSILON) {
            clusters += 1;
        }
        current = value;
    }
    clusters
}

/// Mark the outlines that group other shapes rather than being shapes
/// themselves.
///
/// A Graphviz cluster, a BPMN pool and a PlantUML swimlane are all a box drawn
/// around their members with no label of their own attached to the box: a
/// caption on the rim, if there is one, sits closer to the members than to
/// anything else and is theirs, not the container's. Left in, a container both
/// invents a node and steals the edges: a connector running between two boxes
/// inside a cluster reaches the cluster's border first, so it lands there
/// instead of on the box it was drawn to.
///
/// A node's own incoming arrowheads are drawn just inside its border and
/// satisfy enclosure exactly as a real member would, so they are excluded from
/// the count: two arrowheads landing inside an unlabelled node are not two
/// members grouped by it.
///
/// A label on the enclosure is not on its own enough to make it a node. Every
/// renderer that draws clusters captions them — Graphviz puts the cluster's
/// name inside the border above its members, PlantUML and BPMN put the lane or
/// pool name in a header band — and that caption belongs to no member, so the
/// enclosure owns it. What separates such a caption from a UML class box's own
/// name, or a BPMN task's, is what the enclosure holds: a class box draws
/// compartments and markers, which are interior detail nothing connects to,
/// while a cluster groups nodes that the diagram's own connectors land on.
/// So a labelled enclosure is a container only when the shapes it encloses are
/// themselves joined to the graph.
///
/// "Joined" has to mean joined *within* the enclosure, or a class box would
/// fail the test for the wrong reason: its compartments run the full width, so
/// an association arriving at the class's own border is at zero distance from
/// whichever compartment reaches that height. Requiring the endpoint to sit
/// clear of the enclosure's border by the snap tolerance separates the two —
/// a cluster's members are joined well inside it, while everything touching a
/// class box happens on its rim.
///
/// An unlabelled enclosure keeps the older, unconditional rule. Not because
/// the argument above stops applying, but because it has nothing else it could
/// be: a shape that groups two others and never says what it is has no claim
/// to be a node, whereas a labelled one does.
///
/// Enclosure is otherwise the whole test. Nesting is allowed to any depth,
/// since each container is judged against what it directly contains.
pub(super) fn find_containers(
    outlines: &[Outline],
    arrowheads: &[bool],
    owned: &[Vec<&Label>],
    landings: &[Vec<(f32, f32)>],
    snap: f32,
) -> Vec<bool> {
    outlines
        .iter()
        .enumerate()
        .map(|(i, outer)| {
            let members = outlines
                .iter()
                .enumerate()
                .filter(|(j, inner)| *j != i && !arrowheads[*j] && outer.bbox.encloses(&inner.bbox));
            if owned[i].is_empty() {
                return members.count() >= CONTAINER_MIN_MEMBERS;
            }
            members
                .filter(|(j, _)| {
                    landings
                        .get(*j)
                        .is_some_and(|points| points.iter().any(|(x, y)| outer.bbox.depth_of(*x, *y) > snap))
                })
                .take(CONTAINER_MIN_MEMBERS)
                .count()
                >= CONTAINER_MIN_MEMBERS
        })
        .collect()
}

pub(super) fn collapse_concentric(outlines: &mut Vec<Outline>) {
    let mut inner = vec![false; outlines.len()];
    // Styling is split across the two rings. Graphviz fills the inner disc of a
    // `doublecircle` and leaves the outer ring unpainted, so keeping the outer
    // geometry while dropping the inner one loses the node's colour unless the
    // two are merged.
    let mut merged: Vec<(usize, Outline)> = Vec::new();
    for (i, a) in outlines.iter().enumerate() {
        for (j, b) in outlines.iter().enumerate() {
            if i == j || inner[j] {
                continue;
            }
            let area = a.bbox.area();
            if area <= 0.0 || area >= b.bbox.area() {
                continue;
            }
            if b.bbox.encloses(&a.bbox) && area >= b.bbox.area() * CONCENTRIC_AREA_RATIO {
                inner[i] = true;
                let mut outer = b.clone();
                outer.fill = outer.fill.or_else(|| a.fill.clone());
                outer.stroke = outer.stroke.or_else(|| a.stroke.clone());
                outer.stroke_width = outer.stroke_width.or(a.stroke_width);
                outer.dashed |= a.dashed;
                merged.push((j, outer));
                break;
            }
        }
    }
    for (index, outline) in merged {
        outlines[index] = outline;
    }
    let mut keep = inner.iter().map(|i| !i);
    outlines.retain(|_| keep.next().unwrap_or(true));
}

/// Mark the outlines that are arrowheads rather than nodes.
///
/// A renderer that draws arrows draws them as filled closed shapes, so they
/// arrive here indistinguishable from small boxes. Three things separate them,
/// and all three are required: an arrowhead carries no label, it sits on a
/// connector's endpoint, and it is small next to the largest shape on the
/// canvas. A real node can satisfy any one of those; satisfying all three and
/// still being a node is not something a diagram does.
pub(super) fn find_arrowheads(
    outlines: &[Outline],
    connectors: &[Connector],
    label_owners: &[Option<usize>],
    snap: f32,
    canvas_max: f32,
) -> Vec<bool> {
    let max_side = canvas_max * ARROWHEAD_MAX_SIDE_RATIO;
    let mut marked = vec![false; outlines.len()];
    let Some(largest) = outlines
        .iter()
        .map(|o| o.bbox.area())
        .max_by(|a, b| a.total_cmp(b))
        .filter(|a| *a > 0.0)
    else {
        return marked;
    };

    // Precomputed once so the label test below is a lookup, not a scan of
    // every label for every outline: `label_owners.contains(&Some(index))`
    // repeated inside this loop is `outlines x labels`. ~keep
    let mut labelled = vec![false; outlines.len()];
    for owner in label_owners.iter().flatten() {
        labelled[*owner] = true;
    }

    for (index, outline) in outlines.iter().enumerate() {
        if outline.bbox.area() > largest * ARROWHEAD_AREA_RATIO {
            continue;
        }
        if outline.bbox.width() > max_side || outline.bbox.height() > max_side {
            continue;
        }
        if labelled[index] {
            continue;
        }
        let touches = connectors.iter().any(|c| {
            outline.bbox.distance_to(c.start.0, c.start.1) <= snap || outline.bbox.distance_to(c.end.0, c.end.1) <= snap
        });
        marked[index] = touches;
    }
    marked
}

/// Mark the connectors that are chart gridlines rather than diagram
/// connectors.
///
/// A gridline is not one open stroke that happens to land on two shapes; it
/// is one member of a family. A chart draws its gridlines as a set of
/// parallel strokes, all the same direction, all the same colour and dash
/// style, and spaced apart by the same interval, because that is what a
/// scale is: equal steps. Read in isolation, one gridline is indistinguishable
/// from a connector — that is exactly the GH#1420 defect, where the two ends
/// of one such stroke land on a chart's first and last bar and read as an
/// edge between them. What tells the family apart from a diagram is the
/// other members: nothing a layout engine draws puts three-or-more
/// same-coloured, axis-aligned strokes at a regular spacing
/// that all cover the same span of the canvas.
///
/// A straightforward "does this stroke cross a shape's interior" rule was
/// tried and rejected (see GH#1420): it costs real edges in `graphviz_large`
/// and `mermaid_flow`, and breaks the two tests that deliberately let a
/// connector pass through, or start deep inside, a third shape. The
/// regularity of the *family* is the part a real connector never
/// coincidentally reproduces, so it is what this checks instead of anything
/// about where a single stroke sits relative to a shape.
///
/// A vertical (or horizontal) chain of linked shapes is the closest a real
/// diagram comes to looking like this: consecutive edges in the same
/// direction, evenly spaced because the nodes are. `is_gridline_family`
/// separates the two cases on span: a chain's edges each run only the short,
/// non-overlapping distance between one pair of nodes, while gridlines all
/// cross the same plot and so all cover the same ground. A chain drawn
/// perfectly straight also shares its run-axis position exactly across every
/// edge (they are collinear), which the spacing check rejects outright,
/// since gridlines are offset from each other on purpose.
pub(super) fn find_gridlines(connectors: &[Connector], snap: f32) -> Vec<bool> {
    let mut marked = vec![false; connectors.len()];

    // Horizontal and vertical are judged separately: a family only ever runs
    // one direction, and a diagonal or curved stroke belongs to neither.
    for horizontal in [true, false] {
        // Families are tried by colour alone and by colour and dash style. Colour alone catches a
        // grid whose lines mix dashed and solid strokes (a dashed minor grid between solid majors);
        // the finer key still isolates a regular dashed grid drawn beside irregular solid axes of
        // the same colour, which would break the spacing test if pooled with it. ~keep
        let mut by_colour: HashMap<String, Vec<usize>> = HashMap::new();
        let mut by_style: HashMap<(String, bool), Vec<usize>> = HashMap::new();
        for (index, connector) in connectors.iter().enumerate() {
            let axis_aligned = if horizontal {
                (connector.start.1 - connector.end.1).abs() <= GRIDLINE_AXIS_EPSILON
            } else {
                (connector.start.0 - connector.end.0).abs() <= GRIDLINE_AXIS_EPSILON
            };
            if !axis_aligned {
                continue;
            }
            let stroke = connector.stroke.clone().unwrap_or_default();
            by_colour.entry(stroke.clone()).or_default().push(index);
            by_style.entry((stroke, connector.dashed)).or_default().push(index);
        }

        for indices in by_colour.values().chain(by_style.values()) {
            if indices.len() >= GRIDLINE_MIN_COUNT && is_gridline_family(connectors, indices, horizontal, snap) {
                for &index in indices {
                    marked[index] = true;
                }
            }
        }
    }

    marked
}

/// Whether a same-direction, same-style group of connectors is a gridline
/// family. See [`find_gridlines`] for why span and spacing together are the
/// test.
pub(super) fn is_gridline_family(connectors: &[Connector], indices: &[usize], horizontal: bool, snap: f32) -> bool {
    let mut axis_positions = Vec::with_capacity(indices.len());
    let mut run_starts = Vec::with_capacity(indices.len());
    let mut run_ends = Vec::with_capacity(indices.len());
    for &index in indices {
        let connector = &connectors[index];
        let (axis, run_a, run_b) = if horizontal {
            (
                (connector.start.1 + connector.end.1) / 2.0,
                connector.start.0,
                connector.end.0,
            )
        } else {
            (
                (connector.start.0 + connector.end.0) / 2.0,
                connector.start.1,
                connector.end.1,
            )
        };
        axis_positions.push(axis);
        run_starts.push(run_a.min(run_b));
        run_ends.push(run_a.max(run_b));
    }

    // A real connector stops at the shapes it joins, so a chain's successive
    // edges cover different, non-overlapping ground. Gridlines all cross the
    // same plot, so every member covers the same ground the family's own
    // snap tolerance considers "the same place".
    if spread(run_starts.iter().copied()) > snap || spread(run_ends.iter().copied()) > snap {
        return false;
    }

    // Offset from each other, and by a consistent amount. A connector chain
    // running straight down (or across) a page shares this axis position
    // exactly, since its edges are collinear, so a gap at or below the snap
    // tolerance here means "chain", not "gridlines": gridlines are spread
    // apart on purpose, by however much the scale's interval is.
    let mut sorted = axis_positions;
    sorted.sort_by(f32::total_cmp);
    let gaps: Vec<f32> = sorted.windows(2).map(|pair| pair[1] - pair[0]).collect();
    if gaps.iter().any(|gap| *gap <= snap) {
        return false;
    }
    let average = gaps.iter().sum::<f32>() / gaps.len() as f32;
    gaps.iter()
        .all(|gap| (gap - average).abs() <= average * GRIDLINE_SPACING_TOLERANCE_RATIO)
}

/// Mark the connectors that are a chart's own axis or frame line rather than a
/// diagram connector.
///
/// A gridline never travels alone; [`find_gridlines`] catches those by their
/// peers sharing style, span and spacing. A plot's axis and outer frame do
/// not need a peer to be chart furniture: a plotting library draws the
/// x-axis as one stroke running the width of the plot, flush with every
/// bar's baseline, and the y-axis as one stroke flush with the plot's left
/// edge, and each is typically the only stroke drawn in its own colour and
/// orientation, so the family test never sees three of them to group.
///
/// What gives such a stroke away instead is what it lies against: it is
/// collinear with the same boundary edge of several shapes at once, and for
/// a real stretch of each one's own span, not a single point where two
/// unrelated things happen to meet. A real connector's endpoints touch at
/// most the two shapes it joins, each at a point; nothing a layout engine
/// draws lines a connector up flush against the shared edge of three or more
/// shapes it does not connect. This is deliberately narrower than testing
/// whether a stroke crosses a shape's interior — that was tried for GH#1420
/// and rejected, since it costs real edges that legitimately pass near or
/// through a third shape on their way between the two they join. Requiring
/// the stroke to run *along a shared boundary*, not merely *near* several
/// shapes, is what a real connector never does by coincidence.
pub(super) fn find_frame_lines(outlines: &[Outline], connectors: &[Connector], snap: f32) -> Vec<bool> {
    connectors
        .iter()
        .map(|connector| is_frame_line(outlines, connector, snap))
        .collect()
}

/// Whether a single axis-aligned connector lies flush with the same boundary
/// edge of at least [`FRAME_MIN_FLUSH_EDGES`] shapes. See [`find_frame_lines`].
pub(super) fn is_frame_line(outlines: &[Outline], connector: &Connector, snap: f32) -> bool {
    let horizontal = (connector.start.1 - connector.end.1).abs() <= GRIDLINE_AXIS_EPSILON;
    let vertical = (connector.start.0 - connector.end.0).abs() <= GRIDLINE_AXIS_EPSILON;
    if !horizontal && !vertical {
        return false;
    }

    // A connector that is (near enough) both a point and axis-aligned in both
    // senses is judged as horizontal; which axis a near-zero-length stroke is
    // read against does not change the answer.
    let (axis, run_min, run_max) = if horizontal {
        (
            (connector.start.1 + connector.end.1) / 2.0,
            connector.start.0.min(connector.end.0),
            connector.start.0.max(connector.end.0),
        )
    } else {
        (
            (connector.start.0 + connector.end.0) / 2.0,
            connector.start.1.min(connector.end.1),
            connector.start.1.max(connector.end.1),
        )
    };

    let flush_edges = outlines
        .iter()
        .filter(|outline| {
            let (near, far, span_min, span_max) = if horizontal {
                (outline.bbox.y0, outline.bbox.y1, outline.bbox.x0, outline.bbox.x1)
            } else {
                (outline.bbox.x0, outline.bbox.x1, outline.bbox.y0, outline.bbox.y1)
            };
            let on_boundary = (near - axis).abs() <= snap || (far - axis).abs() <= snap;
            let overlap = (span_max.min(run_max) - span_min.max(run_min)).max(0.0);
            on_boundary && overlap > snap
        })
        .count();

    flush_edges >= FRAME_MIN_FLUSH_EDGES
}

/// Distance between the smallest and largest value in a set of coordinates.
/// Zero for an empty set, so an empty group never looks like a match by
/// default.
pub(super) fn spread(values: impl Iterator<Item = f32>) -> f32 {
    let (mut min, mut max) = (f32::INFINITY, f32::NEG_INFINITY);
    for value in values {
        min = min.min(value);
        max = max.max(value);
    }
    if min.is_finite() && max.is_finite() {
        max - min
    } else {
        0.0
    }
}

/// Size of the arrowhead sitting on `point`, if one does, as the reach a
/// connector needs to see past it.
pub(super) fn touching_arrowhead(arrowheads: &[Rect], point: (f32, f32), snap: f32) -> Option<f32> {
    arrowheads
        .iter()
        .map(|bbox| (bbox.distance_to(point.0, point.1), bbox))
        .filter(|(distance, _)| *distance <= snap)
        // Nearest, not largest. Two nodes joined by a pair of opposing
        // connectors put four arrowheads within a few units of each other, and
        // taking the biggest would let one connector reach across its
        // neighbour's head and land on the wrong shape.
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, bbox)| bbox.width().hypot(bbox.height()))
}

/// Index of the shape a connector endpoint lands on: the one containing it, or
/// failing that the nearest one within `tolerance`.
pub(super) fn snap_to_outline(outlines: &[&Outline], point: (f32, f32), tolerance: f32) -> Option<usize> {
    let (x, y) = point;
    if !x.is_finite() || !y.is_finite() {
        return None;
    }
    outlines
        .iter()
        .enumerate()
        .map(|(i, o)| (i, o.bbox.distance_to(x, y), o.bbox.area()))
        .filter(|(_, distance, _)| *distance <= tolerance)
        // Nearest wins; on a tie the smaller shape does, so an endpoint inside
        // both a box and its enclosing panel attaches to the box.
        .min_by(|a, b| a.1.total_cmp(&b.1).then(a.2.total_cmp(&b.2)))
        .map(|(i, _, _)| i)
}

/// Give an unlabelled shape the caption drawn beneath it.
///
/// The AWS and Azure architecture house style draws a node as an icon glyph
/// with its name underneath and no outline at all, so the name lies outside
/// every closed region in the drawing and containment finds it no owner.
///
/// Three things stop this from eating text that names something else. It only
/// considers a label containment left unowned, so nothing is taken from the
/// shape it sits inside. It only offers that label to a shape holding no text
/// of its own, so an annotation under a captioned box — an org chart's
/// headcount under a department — stays free, because the department already
/// says what it is. And the anchor must fall within the shape's own width and
/// just below its lower edge, which is where a caption goes and where an
/// unrelated line of prose does not.
pub(super) fn adopt_captions(
    outlines: &[Outline],
    labels: &[Label],
    arrowheads: &[bool],
    connectors: &[Connector],
    owners: &mut [Option<usize>],
    snap: f32,
) {
    // Frozen before any adoption, so that two lines of one wrapped caption both
    // reach the same shape and the result does not depend on label order.
    let mut labelled = vec![false; outlines.len()];
    for owner in owners.iter().flatten() {
        labelled[*owner] = true;
    }

    let reach = (snap * CAPTION_REACH).min(CAPTION_CEILING);
    for (label, owner) in labels.iter().zip(owners.iter_mut()) {
        if owner.is_some() {
            continue;
        }
        let Some((index, gap)) = outlines
            .iter()
            .enumerate()
            .filter(|(i, outline)| {
                !labelled[*i]
                    && !arrowheads[*i]
                    && label.x >= outline.bbox.x0
                    && label.x <= outline.bbox.x1
                    && label.y > outline.bbox.y1
                    && label.y - outline.bbox.y1 <= reach
            })
            // Nearest above the caption, and on a tie the shape it is centred
            // under. A banner or rule spanning a row of icons sits below all of
            // them and would otherwise take every caption in the row.
            .min_by(|(_, a), (_, b)| {
                (label.y - a.bbox.y1).total_cmp(&(label.y - b.bbox.y1)).then(
                    (label.x - a.bbox.centre_x())
                        .abs()
                        .total_cmp(&(label.x - b.bbox.centre_x()).abs()),
                )
            })
            .map(|(i, outline)| (i, label.y - outline.bbox.y1))
        else {
            continue;
        };

        // Text under one node and over the next is either this node's caption
        // or that connector's label, and the same geometry describes both. The
        // nearer claim wins; without this the caption test runs first and takes
        // the label even when the connector is an order of magnitude closer.
        if connectors.iter().any(|connector| {
            let dx = label.x - connector.midpoint.0;
            let dy = label.y - connector.midpoint.1;
            dx.hypot(dy) < gap
        }) {
            continue;
        }

        *owner = Some(index);
    }
}

/// Text sitting on a connector, used as the edge label.
pub(super) fn nearest_free_label(labels: &[&Label], midpoint: (f32, f32), tolerance: f32) -> Option<String> {
    labels
        .iter()
        .map(|l| {
            let dx = l.x - midpoint.0;
            let dy = l.y - midpoint.1;
            (l, (dx * dx + dy * dy).sqrt())
        })
        .filter(|(_, distance)| *distance <= tolerance)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(l, _)| l.text.clone())
}
