//! Closed faces from a page's line segments.
//!
//! The pass is opt-in (`pdf_options.plan_regions` plus the `plan-regions`
//! feature). It takes the line segments of part 1's vector geometry, snaps
//! near-coincident endpoints, bridges door-sized gaps between open wall ends,
//! splits crossings, and walks the resulting planar graph's half-edges into
//! faces. [`geo`] supplies the polygon type, its area and point containment.
//!
//! Every step is bounded: more segments than the configured `max_segments`
//! skips the page, the crossing pass stops after a fixed number of pair tests,
//! and the face walk caps the result. Malformed geometry (zero-length segments,
//! non-finite coordinates, collinear edges) is ignored rather than panicking.

use std::collections::{BTreeMap, HashMap, HashSet};

use geo::{Area, Contains, Coord, LineString, Point, Polygon};

use crate::core::config::PlanRegionConfig;
use crate::types::{BoundingBox, PageContent, PlanRegion, VectorPath, VectorPoint, VectorSegment};

/// A face smaller than this many square page points is drawing noise.
const MIN_REGION_AREA_SQUARE_POINTS: f64 = 1.0;

/// Cap on faces returned for one page.
const MAX_REGIONS_PER_PAGE: usize = 10_000;

/// Cap on segment-pair tests in the crossing pass.
const MAX_INTERSECTION_TESTS: usize = 500_000;

/// Attach each page's closed regions, and drop the raw paths when the caller did
/// not ask for them.
pub(crate) fn attach_plan_regions(
    doc: &xberg_native_pdf::PdfDocument,
    pages: &mut [PageContent],
    config: &PlanRegionConfig,
    keep_paths: bool,
) {
    for page in pages {
        let Some(geometry) = page.geometry.as_mut() else {
            continue;
        };
        let page_index = page.page_number.saturating_sub(1) as usize;
        let spans = match doc.extract_spans(page_index) {
            Ok(spans) => spans,
            Err(error) => {
                tracing::debug!(
                    page = page.page_number,
                    "skipping closed regions whose text could not be read: {error}"
                );
                continue;
            }
        };
        geometry.regions = close_regions(&geometry.paths, &spans, config);
        if !keep_paths {
            geometry.paths.clear();
        }
    }
}

/// Recover closed faces from a set of paths and attach the page text inside
/// each one.
///
/// `paths` come from [`PageGeometry`] and `spans` from the native text pass, both
/// in page coordinates. Returns an empty vector when the page has more segments
/// than `config.max_segments`.
pub(crate) fn close_regions(
    paths: &[VectorPath],
    spans: &[xberg_native_pdf::layout::TextSpan],
    config: &PlanRegionConfig,
) -> Vec<PlanRegion> {
    let segments = collect_segments(paths);
    if segments.len() > config.max_segments {
        tracing::debug!(
            segments = segments.len(),
            limit = config.max_segments,
            "skipping closed regions: the page has more segments than the configured limit"
        );
        return Vec::new();
    }
    if segments.is_empty() {
        return Vec::new();
    }

    let snap = f64::from(config.snap_tolerance);
    let gap = f64::from(config.gap_tolerance);
    let mut graph = Graph::new(snap);
    for (a, b) in segments {
        graph.add_segment(a, b);
    }
    graph.split_crossings();
    graph.close_gaps(gap);

    let faces = graph.faces();
    let mut regions: Vec<PlanRegion> = faces
        .into_iter()
        .filter_map(|face| build_region(&graph.coords, &face, spans))
        .collect();

    regions.sort_by(|left, right| {
        right
            .area
            .total_cmp(&left.area)
            .then(left.bounding_box.x0.total_cmp(&right.bounding_box.x0))
            .then(right.bounding_box.y1.total_cmp(&left.bounding_box.y1))
            .then(left.bounding_box.y0.total_cmp(&right.bounding_box.y0))
    });
    regions.truncate(MAX_REGIONS_PER_PAGE);
    regions
}

/// Line segments of every subpath, with a closed subpath's closing edge.
///
/// Cubic segments are skipped: a face built from their chord would not be the
/// painted boundary. A closed subpath still contributes its closing line.
fn collect_segments(paths: &[VectorPath]) -> Vec<(VectorPoint, VectorPoint)> {
    let mut segments = Vec::new();
    for path in paths {
        for subpath in &path.subpaths {
            let mut current = subpath.start;
            let start = subpath.start;
            for segment in &subpath.segments {
                match segment {
                    VectorSegment::Line { to } => {
                        push_segment(&mut segments, current, *to);
                        current = *to;
                    }
                    VectorSegment::Cubic { to, .. } => {
                        // The curve paints a boundary the straight-line graph
                        // cannot represent; advance without inventing a chord. ~keep
                        current = *to;
                    }
                }
            }
            if subpath.closed {
                push_segment(&mut segments, current, start);
            }
        }
    }
    segments
}

fn push_segment(segments: &mut Vec<(VectorPoint, VectorPoint)>, from: VectorPoint, to: VectorPoint) {
    if !finite(from) || !finite(to) {
        return;
    }
    let dx = f64::from(to.x) - f64::from(from.x);
    let dy = f64::from(to.y) - f64::from(from.y);
    if dx * dx + dy * dy > f64::EPSILON {
        segments.push((from, to));
    }
}

fn finite(point: VectorPoint) -> bool {
    point.x.is_finite() && point.y.is_finite()
}

/// A planar graph with quantised nodes.
struct Graph {
    snap: f64,
    coords: Vec<VectorPoint>,
    node_index: BTreeMap<(i64, i64), usize>,
    edges: HashSet<(usize, usize)>,
}

impl Graph {
    fn new(snap: f64) -> Self {
        Self {
            snap,
            coords: Vec::new(),
            node_index: BTreeMap::new(),
            edges: HashSet::new(),
        }
    }

    fn key(&self, point: VectorPoint) -> (i64, i64) {
        if self.snap > 0.0 {
            (
                (f64::from(point.x) / self.snap).round() as i64,
                (f64::from(point.y) / self.snap).round() as i64,
            )
        } else {
            (i64::from(point.x.to_bits()), i64::from(point.y.to_bits()))
        }
    }

    fn node(&mut self, point: VectorPoint) -> usize {
        let key = self.key(point);
        if let Some(index) = self.node_index.get(&key) {
            return *index;
        }
        let coordinate = if self.snap > 0.0 {
            VectorPoint {
                x: (key.0 as f64 * self.snap) as f32,
                y: (key.1 as f64 * self.snap) as f32,
            }
        } else {
            point
        };
        let index = self.coords.len();
        self.coords.push(coordinate);
        self.node_index.insert(key, index);
        index
    }

    fn add_segment(&mut self, from: VectorPoint, to: VectorPoint) {
        let from = self.node(from);
        let to = self.node(to);
        self.add_edge(from, to);
    }

    fn add_edge(&mut self, from: usize, to: usize) {
        if from == to {
            return;
        }
        let edge = if from < to { (from, to) } else { (to, from) };
        self.edges.insert(edge);
    }

    fn adjacency(&self) -> Vec<Vec<usize>> {
        let mut adjacency = vec![Vec::new(); self.coords.len()];
        for &(from, to) in &self.edges {
            adjacency[from].push(to);
            adjacency[to].push(from);
        }
        for (node, neighbors) in adjacency.iter_mut().enumerate() {
            neighbors.sort_unstable();
            neighbors.dedup();
            let origin = self.coords[node];
            neighbors.sort_by(|left, right| {
                angle(origin, self.coords[*left])
                    .total_cmp(&angle(origin, self.coords[*right]))
                    .then(left.cmp(right))
            });
        }
        adjacency
    }

    /// Split edges where two other edges cross in their interiors.
    ///
    /// Endpoint contacts are already handled by snapping. The pair loop is
    /// bounded: once [`MAX_INTERSECTION_TESTS`] boxes have been compared the pass
    /// stops, leaving later crossings unsplit rather than spending unbounded time
    /// on a very dense sheet.
    fn split_crossings(&mut self) {
        let mut edges: Vec<(usize, usize)> = self.edges.iter().copied().collect();
        edges.sort_unstable();
        if edges.len() < 2 {
            return;
        }

        let mut splits: BTreeMap<usize, Vec<f64>> = BTreeMap::new();
        let mut tests = 0usize;
        let mut exhausted = false;
        'outer: for first in 0..edges.len() {
            let (a, b) = edges[first];
            let (ax, ay) = (self.coords[a], self.coords[b]);
            for second in (first + 1)..edges.len() {
                let (c, d) = edges[second];
                if !boxes_overlap(ax, ay, self.coords[c], self.coords[d]) {
                    continue;
                }
                tests += 1;
                if tests > MAX_INTERSECTION_TESTS {
                    exhausted = true;
                    break 'outer;
                }
                let (cx, cy) = (self.coords[c], self.coords[d]);
                if let Some((t, u)) = segment_intersection(ax, ay, cx, cy) {
                    splits.entry(first).or_default().push(t);
                    splits.entry(second).or_default().push(u);
                }
            }
        }
        if exhausted {
            tracing::debug!("closed-region crossing pass reached its pair-test limit");
        }
        if splits.is_empty() {
            return;
        }

        for (edge_index, mut parameters) in splits {
            let (a, b) = edges[edge_index];
            let start = self.coords[a];
            let end = self.coords[b];
            parameters.push(0.0);
            parameters.push(1.0);
            parameters.sort_by(f64::total_cmp);
            parameters.dedup_by(|left, right| (*left - *right).abs() < 1e-9);
            for window in parameters.windows(2) {
                let (t0, t1) = (window[0], window[1]);
                let from = interpolate(start, end, t0);
                let to = interpolate(start, end, t1);
                self.add_segment(from, to);
            }
            self.edges.remove(&if a < b { (a, b) } else { (b, a) });
        }
    }

    /// Join open ends (degree-one nodes) that are at most `gap` apart.
    ///
    /// A door opening leaves two wall ends facing each other; the nearest
    /// partner within the tolerance closes the room. Matching is greedy over
    /// pairs sorted by distance and then node index, so the result does not
    /// depend on hash order.
    fn close_gaps(&mut self, gap: f64) {
        if gap <= 0.0 {
            return;
        }
        let adjacency = self.adjacency();
        let mut leaves: Vec<usize> = (0..adjacency.len())
            .filter(|node| adjacency[*node].len() == 1)
            .collect();
        leaves.sort_unstable();
        if leaves.len() < 2 {
            return;
        }

        let mut candidates = Vec::new();
        for (index, &first) in leaves.iter().enumerate() {
            for &second in &leaves[index + 1..] {
                let distance = distance(self.coords[first], self.coords[second]);
                if distance <= gap {
                    candidates.push((distance, first, second));
                }
            }
        }
        candidates.sort_by(|left, right| {
            left.0
                .total_cmp(&right.0)
                .then(left.1.cmp(&right.1))
                .then(left.2.cmp(&right.2))
        });

        let mut used = HashSet::new();
        for (_, first, second) in candidates {
            if used.contains(&first) || used.contains(&second) {
                continue;
            }
            used.insert(first);
            used.insert(second);
            self.add_edge(first, second);
        }
    }

    /// Walk each directed half-edge into a face and keep the bounded ones.
    ///
    /// At `v`, the next edge is the one immediately clockwise from `v -> u`.
    /// That walks a bounded face counter-clockwise (positive signed area) and
    /// the unbounded face clockwise, so a positive-area filter keeps the faces a
    /// caller wants.
    fn faces(&self) -> Vec<Vec<usize>> {
        let adjacency = self.adjacency();
        let positions: HashMap<(usize, usize), usize> = adjacency
            .iter()
            .enumerate()
            .flat_map(|(node, neighbors)| {
                neighbors
                    .iter()
                    .enumerate()
                    .map(move |(position, neighbor)| ((node, *neighbor), position))
            })
            .collect();

        let mut visited = HashSet::new();
        let mut faces = Vec::new();
        for from in 0..adjacency.len() {
            for &to in &adjacency[from] {
                let start = (from, to);
                if visited.contains(&start) {
                    continue;
                }
                let Some(face) = walk_face(&adjacency, &positions, &mut visited, start) else {
                    continue;
                };
                if face.len() >= 3 {
                    let area = signed_area(&self.coords, &face);
                    if area > MIN_REGION_AREA_SQUARE_POINTS {
                        faces.push(face);
                    }
                }
            }
        }
        faces
    }
}

/// Walk one face from a directed half-edge, marking each half-edge visited.
///
/// Returns `None` when the walk repeats a half-edge before closing, which means
/// the boundary is not a simple cycle and the face is unusable.
fn walk_face(
    adjacency: &[Vec<usize>],
    positions: &HashMap<(usize, usize), usize>,
    visited: &mut HashSet<(usize, usize)>,
    start: (usize, usize),
) -> Option<Vec<usize>> {
    let mut face = Vec::new();
    let (mut current, mut next) = start;
    loop {
        if !visited.insert((current, next)) {
            if (current, next) == start && !face.is_empty() {
                return Some(face);
            }
            return None;
        }
        face.push(current);
        let &position = positions.get(&(next, current))?;
        let neighbors = &adjacency[next];
        let following = neighbors[(position + neighbors.len() - 1) % neighbors.len()];
        current = next;
        next = following;
        if (current, next) == start {
            return Some(face);
        }
        if face.len() > adjacency.len() + 1 {
            return None;
        }
    }
}

fn angle(origin: VectorPoint, target: VectorPoint) -> f64 {
    (f64::from(target.y) - f64::from(origin.y)).atan2(f64::from(target.x) - f64::from(origin.x))
}

fn distance(left: VectorPoint, right: VectorPoint) -> f64 {
    let dx = f64::from(right.x) - f64::from(left.x);
    let dy = f64::from(right.y) - f64::from(left.y);
    dx.hypot(dy)
}

fn interpolate(start: VectorPoint, end: VectorPoint, t: f64) -> VectorPoint {
    VectorPoint {
        x: (f64::from(start.x) + (f64::from(end.x) - f64::from(start.x)) * t) as f32,
        y: (f64::from(start.y) + (f64::from(end.y) - f64::from(start.y)) * t) as f32,
    }
}

fn boxes_overlap(a0: VectorPoint, a1: VectorPoint, b0: VectorPoint, b1: VectorPoint) -> bool {
    let (ax0, ax1) = ordered(f64::from(a0.x), f64::from(a1.x));
    let (ay0, ay1) = ordered(f64::from(a0.y), f64::from(a1.y));
    let (bx0, bx1) = ordered(f64::from(b0.x), f64::from(b1.x));
    let (by0, by1) = ordered(f64::from(b0.y), f64::from(b1.y));
    ax0 <= bx1 && bx0 <= ax1 && ay0 <= by1 && by0 <= ay1
}

fn ordered(left: f64, right: f64) -> (f64, f64) {
    if left <= right { (left, right) } else { (right, left) }
}

/// Proper interior intersection of two segments, as `(t, u)` parameters.
///
/// Shared endpoints and collinear overlaps are not crossings; snapping resolves
/// endpoints, and collinear overlaps add no new node.
fn segment_intersection(a: VectorPoint, b: VectorPoint, c: VectorPoint, d: VectorPoint) -> Option<(f64, f64)> {
    let (ax, ay) = (f64::from(a.x), f64::from(a.y));
    let (bx, by) = (f64::from(b.x), f64::from(b.y));
    let (cx, cy) = (f64::from(c.x), f64::from(c.y));
    let (dx, dy) = (f64::from(d.x), f64::from(d.y));

    let r = (bx - ax, by - ay);
    let s = (dx - cx, dy - cy);
    let denominator = r.0 * s.1 - r.1 * s.0;
    if denominator.abs() < 1e-12 {
        return None;
    }
    let q = (cx - ax, cy - ay);
    let t = (q.0 * s.1 - q.1 * s.0) / denominator;
    let u = (q.0 * r.1 - q.1 * r.0) / denominator;
    if (1e-9..=1.0 - 1e-9).contains(&t) && (1e-9..=1.0 - 1e-9).contains(&u) {
        Some((t, u))
    } else {
        None
    }
}

fn signed_area(coords: &[VectorPoint], face: &[usize]) -> f64 {
    let mut area = 0.0;
    for window in face.windows(2) {
        let a = coords[window[0]];
        let b = coords[window[1]];
        area += f64::from(a.x) * f64::from(b.y) - f64::from(b.x) * f64::from(a.y);
    }
    if face.len() >= 3 {
        let a = coords[face[face.len() - 1]];
        let b = coords[face[0]];
        area += f64::from(a.x) * f64::from(b.y) - f64::from(b.x) * f64::from(a.y);
    }
    area / 2.0
}

fn build_region(
    coords: &[VectorPoint],
    face: &[usize],
    spans: &[xberg_native_pdf::layout::TextSpan],
) -> Option<PlanRegion> {
    let polygon: Vec<VectorPoint> = face.iter().map(|index| coords[*index]).collect();
    if polygon.len() < 3 {
        return None;
    }
    let line: LineString<f64> = polygon
        .iter()
        .map(|point| Coord {
            x: f64::from(point.x),
            y: f64::from(point.y),
        })
        .collect();
    let geo_polygon = Polygon::new(line, Vec::new());
    let area = geo_polygon.unsigned_area();
    if !area.is_finite() || area <= MIN_REGION_AREA_SQUARE_POINTS {
        return None;
    }
    let bounding_box = polygon_bounds(&polygon);
    let text = text_inside(&geo_polygon, spans);

    Some(PlanRegion {
        polygon,
        area,
        bounding_box,
        text,
    })
}

fn polygon_bounds(polygon: &[VectorPoint]) -> BoundingBox {
    let mut bounds = BoundingBox {
        x0: f64::INFINITY,
        y0: f64::INFINITY,
        x1: f64::NEG_INFINITY,
        y1: f64::NEG_INFINITY,
    };
    for point in polygon {
        bounds.x0 = bounds.x0.min(f64::from(point.x));
        bounds.y0 = bounds.y0.min(f64::from(point.y));
        bounds.x1 = bounds.x1.max(f64::from(point.x));
        bounds.y1 = bounds.y1.max(f64::from(point.y));
    }
    bounds
}

/// Text whose span centre lies inside the face, in reading order.
fn text_inside(polygon: &Polygon<f64>, spans: &[xberg_native_pdf::layout::TextSpan]) -> String {
    let mut pieces: Vec<(f64, f64, String)> = Vec::new();
    for span in spans {
        let text = span.text.split_whitespace().collect::<Vec<_>>().join(" ");
        if text.is_empty() {
            continue;
        }
        let centre = Point::new(
            f64::from(span.bbox.x) + f64::from(span.bbox.width) / 2.0,
            f64::from(span.bbox.y) + f64::from(span.bbox.height) / 2.0,
        );
        if !centre.x().is_finite() || !centre.y().is_finite() {
            continue;
        }
        if polygon.contains(&centre) {
            pieces.push((centre.y(), centre.x(), text));
        }
    }
    pieces.sort_by(|left, right| right.0.total_cmp(&left.0).then(left.1.total_cmp(&right.1)));
    pieces
        .into_iter()
        .map(|(_, _, text)| text)
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{VectorPath, VectorStroke, VectorSubpath};

    fn point(x: f32, y: f32) -> VectorPoint {
        VectorPoint { x, y }
    }

    fn path_from_subpath(subpath: VectorSubpath) -> VectorPath {
        let mut bounds = BoundingBox {
            x0: f64::INFINITY,
            y0: f64::INFINITY,
            x1: f64::NEG_INFINITY,
            y1: f64::NEG_INFINITY,
        };
        let mut update = |point: VectorPoint| {
            bounds.x0 = bounds.x0.min(f64::from(point.x));
            bounds.y0 = bounds.y0.min(f64::from(point.y));
            bounds.x1 = bounds.x1.max(f64::from(point.x));
            bounds.y1 = bounds.y1.max(f64::from(point.y));
        };
        update(subpath.start);
        for segment in &subpath.segments {
            if let VectorSegment::Line { to } = segment {
                update(*to);
            }
        }
        VectorPath {
            subpaths: vec![subpath],
            bounding_box: bounds,
            fill_color: None,
            stroke: Some(VectorStroke::default()),
            layer: None,
        }
    }

    fn rectangle(x0: f32, y0: f32, x1: f32, y1: f32) -> VectorPath {
        path_from_subpath(VectorSubpath {
            start: point(x0, y0),
            segments: vec![
                VectorSegment::Line { to: point(x1, y0) },
                VectorSegment::Line { to: point(x1, y1) },
                VectorSegment::Line { to: point(x0, y1) },
            ],
            closed: true,
        })
    }

    fn config(snap: f32, gap: f32) -> PlanRegionConfig {
        PlanRegionConfig {
            enabled: true,
            snap_tolerance: snap,
            gap_tolerance: gap,
            max_segments: 20_000,
        }
    }

    fn span(text: &str, x: f32, y: f32, width: f32, height: f32) -> xberg_native_pdf::layout::TextSpan {
        xberg_native_pdf::layout::TextSpan {
            provenance: None,
            artifact_type: None,
            text: text.to_string(),
            bbox: xberg_native_pdf::geometry::Rect::new(x, y, width, height),
            font_name: String::new(),
            font_size: 12.0,
            font_weight: xberg_native_pdf::layout::FontWeight::Normal,
            is_italic: false,
            is_monospace: false,
            color: xberg_native_pdf::layout::Color { r: 0.0, g: 0.0, b: 0.0 },
            mcid: None,
            mcid_scope: None,
            sequence: 0,
            split_boundary_before: false,
            offset_semantic: false,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 100.0,
            primary_detected: false,
            char_widths: vec![],
            char_x_offsets: Vec::new(),
            heading_level: None,
            rotation_degrees: 0.0,
            wmode: 0,
            text_rise: 0.0,
            rtl_draw_logical: false,
            ligature_glyph: false,
            mirrored: false,
            page_rotation_applied: 0,
        }
    }

    #[test]
    fn a_closed_rectangle_becomes_one_face_with_its_text() {
        let paths = vec![rectangle(0.0, 0.0, 100.0, 50.0)];
        let spans = vec![span("ROOM 101", 40.0, 20.0, 20.0, 10.0)];
        let regions = close_regions(&paths, &spans, &config(0.5, 0.0));
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].area, 5000.0);
        assert_eq!(regions[0].text, "ROOM 101");
        assert_eq!(
            regions[0].bounding_box,
            BoundingBox {
                x0: 0.0,
                y0: 0.0,
                x1: 100.0,
                y1: 50.0,
            }
        );
    }

    #[test]
    fn text_outside_a_face_is_not_attached() {
        let paths = vec![rectangle(0.0, 0.0, 100.0, 50.0)];
        let spans = vec![span("OUTSIDE", 140.0, 20.0, 20.0, 10.0)];
        let regions = close_regions(&paths, &spans, &config(0.5, 0.0));
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].text, "");
    }

    #[test]
    fn a_door_sized_gap_is_closed_within_the_tolerance() {
        // The bottom wall is split by a 10-point door opening: (0,0)->(45,0)
        // and (55,0)->(100,0), with the other three walls closed.
        let mut paths = vec![
            path_from_subpath(VectorSubpath {
                start: point(0.0, 0.0),
                segments: vec![VectorSegment::Line { to: point(45.0, 0.0) }],
                closed: false,
            }),
            path_from_subpath(VectorSubpath {
                start: point(55.0, 0.0),
                segments: vec![VectorSegment::Line { to: point(100.0, 0.0) }],
                closed: false,
            }),
            path_from_subpath(VectorSubpath {
                start: point(0.0, 0.0),
                segments: vec![
                    VectorSegment::Line { to: point(0.0, 50.0) },
                    VectorSegment::Line { to: point(100.0, 50.0) },
                    VectorSegment::Line { to: point(100.0, 0.0) },
                ],
                closed: false,
            }),
        ];
        paths.push(path_from_subpath(VectorSubpath {
            start: point(100.0, 0.0),
            segments: vec![VectorSegment::Line { to: point(55.0, 0.0) }],
            closed: false,
        }));

        let closed = close_regions(&paths, &[], &config(0.5, 20.0));
        assert_eq!(closed.len(), 1, "the 10-point gap must close at tolerance 20");
        assert!((closed[0].area - 5000.0).abs() < 1.0);

        let open = close_regions(&paths, &[], &config(0.5, 5.0));
        assert!(open.is_empty(), "a gap beyond the tolerance must stay open");
    }

    #[test]
    fn crossing_rules_form_a_single_cell() {
        // Each rule extends past the crossing, so the face only exists when the
        // crossing pass splits both rules at their mid-span intersection.
        let horizontal_top = path_from_subpath(VectorSubpath {
            start: point(-10.0, 100.0),
            segments: vec![VectorSegment::Line {
                to: point(110.0, 100.0),
            }],
            closed: false,
        });
        let horizontal_bottom = path_from_subpath(VectorSubpath {
            start: point(-10.0, 0.0),
            segments: vec![VectorSegment::Line { to: point(110.0, 0.0) }],
            closed: false,
        });
        let vertical_left = path_from_subpath(VectorSubpath {
            start: point(0.0, -10.0),
            segments: vec![VectorSegment::Line { to: point(0.0, 110.0) }],
            closed: false,
        });
        let vertical_right = path_from_subpath(VectorSubpath {
            start: point(100.0, -10.0),
            segments: vec![VectorSegment::Line {
                to: point(100.0, 110.0),
            }],
            closed: false,
        });

        let regions = close_regions(
            &[horizontal_top, horizontal_bottom, vertical_left, vertical_right],
            &[],
            &config(0.5, 0.0),
        );
        assert_eq!(regions.len(), 1);
        assert!((regions[0].area - 10_000.0).abs() < 1.0);
    }

    #[test]
    fn an_open_cross_has_no_face() {
        // A plus sign: one horizontal and one vertical rule crossing mid-span.
        let horizontal = path_from_subpath(VectorSubpath {
            start: point(0.0, 50.0),
            segments: vec![VectorSegment::Line { to: point(100.0, 50.0) }],
            closed: false,
        });
        let vertical = path_from_subpath(VectorSubpath {
            start: point(50.0, 0.0),
            segments: vec![VectorSegment::Line { to: point(50.0, 100.0) }],
            closed: false,
        });
        let regions = close_regions(&[horizontal, vertical], &[], &config(0.5, 0.0));
        assert!(regions.is_empty(), "an open cross has no closed face");
    }

    #[test]
    fn nested_rectangles_are_ordered_by_area() {
        let paths = vec![rectangle(0.0, 0.0, 100.0, 100.0), rectangle(40.0, 40.0, 60.0, 60.0)];
        let regions = close_regions(&paths, &[], &config(0.5, 0.0));
        assert_eq!(regions.len(), 2);
        assert!((regions[0].area - 10_000.0).abs() < 1.0);
        assert!((regions[1].area - 400.0).abs() < 1.0);
    }

    #[test]
    fn the_result_is_deterministic_for_reversed_input() {
        let forward = vec![rectangle(0.0, 0.0, 100.0, 100.0), rectangle(200.0, 0.0, 260.0, 60.0)];
        let mut reversed = forward.clone();
        reversed.reverse();
        let first = close_regions(&forward, &[], &config(0.5, 0.0));
        let second = close_regions(&reversed, &[], &config(0.5, 0.0));
        assert_eq!(first, second);
    }

    #[test]
    fn degeneracies_do_not_panic() {
        let zero_length = path_from_subpath(VectorSubpath {
            start: point(5.0, 5.0),
            segments: vec![VectorSegment::Line { to: point(5.0, 5.0) }],
            closed: false,
        });
        let collinear = vec![
            path_from_subpath(VectorSubpath {
                start: point(0.0, 0.0),
                segments: vec![VectorSegment::Line { to: point(10.0, 0.0) }],
                closed: false,
            }),
            path_from_subpath(VectorSubpath {
                start: point(10.0, 0.0),
                segments: vec![VectorSegment::Line { to: point(20.0, 0.0) }],
                closed: false,
            }),
        ];
        assert!(close_regions(&[zero_length], &[], &config(0.5, 0.0)).is_empty());
        assert!(close_regions(&collinear, &[], &config(0.5, 0.0)).is_empty());
    }

    #[test]
    fn a_page_over_the_segment_limit_is_skipped() {
        let paths: Vec<VectorPath> = (0..10)
            .map(|index| rectangle(index as f32 * 10.0, 0.0, index as f32 * 10.0 + 5.0, 5.0))
            .collect();
        let mut settings = config(0.5, 0.0);
        settings.max_segments = 5;
        assert!(close_regions(&paths, &[], &settings).is_empty());
    }
}
