//! Closed faces from a page's line segments.
//!
//! The pass is opt-in (`pdf_options.plan_regions` plus the `plan-regions`
//! feature). It takes the line segments of part 1's vector geometry, snaps
//! near-coincident endpoints, bridges door-sized gaps between open wall ends,
//! splits crossings, and walks the resulting planar graph's half-edges into
//! faces. [`geo`] supplies the polygon type, its area and point containment.
//!
//! Every step is bounded: more segments than the configured `max_segments`
//! skips the page, the crossing pass stops after a fixed number of segment-pair
//! tests, the gap pass pairs only leaves in neighbouring buckets and stops at a
//! fixed candidate count, and the face walk caps the result. Malformed geometry
//! (zero-length segments, non-finite coordinates, collinear edges) is ignored
//! rather than panicking.

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

/// Cap on leaf pairs considered when closing gaps.
const MAX_GAP_CANDIDATES: usize = 200_000;

/// A grid cell and its eight neighbours, iterated in a fixed order.
const NEIGHBOUR_CELLS: [(i64, i64); 9] = [
    (-1, -1),
    (-1, 0),
    (-1, 1),
    (0, -1),
    (0, 0),
    (0, 1),
    (1, -1),
    (1, 0),
    (1, 1),
];

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
        geometry.regions = close_page_regions(&mut geometry.paths, config, keep_paths, || {
            doc.extract_spans(page_index)
        });
    }
}

/// Recover one page's closed regions, reading the page text only when there is
/// geometry to attach it to.
///
/// The raw paths are dropped on every exit when `keep_paths` is false, including
/// the error path, so the documented contract holds whatever the native text
/// pass does.
fn close_page_regions(
    paths: &mut Vec<VectorPath>,
    config: &PlanRegionConfig,
    keep_paths: bool,
    read_spans: impl FnOnce() -> xberg_native_pdf::Result<Vec<xberg_native_pdf::layout::TextSpan>>,
) -> Vec<PlanRegion> {
    let Some(segments) = prepare_segments(paths, config) else {
        clear_unrequested_paths(paths, keep_paths);
        return Vec::new();
    };
    let spans = match read_spans() {
        Ok(spans) => spans,
        Err(error) => {
            tracing::debug!("skipping closed regions whose text could not be read: {error}");
            clear_unrequested_paths(paths, keep_paths);
            return Vec::new();
        }
    };
    let regions = close_segments(&segments, &spans, config);
    clear_unrequested_paths(paths, keep_paths);
    regions
}

fn clear_unrequested_paths(paths: &mut Vec<VectorPath>, keep_paths: bool) {
    if !keep_paths {
        paths.clear();
    }
}

/// Recover closed faces from a set of paths and attach the page text inside
/// each one.
///
/// `paths` come from [`PageGeometry`] and `spans` from the native text pass, both
/// in page coordinates. Returns an empty vector when the page has no segments or
/// more segments than `config.max_segments`.
pub(crate) fn close_regions(
    paths: &[VectorPath],
    spans: &[xberg_native_pdf::layout::TextSpan],
    config: &PlanRegionConfig,
) -> Vec<PlanRegion> {
    let Some(segments) = prepare_segments(paths, config) else {
        return Vec::new();
    };
    close_segments(&segments, spans, config)
}

/// Line segments of every path, or `None` when the page is skipped.
fn prepare_segments(paths: &[VectorPath], config: &PlanRegionConfig) -> Option<Vec<(VectorPoint, VectorPoint)>> {
    let segments = collect_segments(paths);
    if segments.len() > config.max_segments {
        tracing::debug!(
            segments = segments.len(),
            limit = config.max_segments,
            "skipping closed regions: the page has more segments than the configured limit"
        );
        return None;
    }
    if segments.is_empty() {
        return None;
    }
    Some(segments)
}

fn close_segments(
    segments: &[(VectorPoint, VectorPoint)],
    spans: &[xberg_native_pdf::layout::TextSpan],
    config: &PlanRegionConfig,
) -> Vec<PlanRegion> {
    let snap = f64::from(config.snap_tolerance);
    let gap = f64::from(config.gap_tolerance);
    let mut graph = Graph::new(snap);
    for &(a, b) in segments {
        graph.add_segment(a, b);
    }
    let _ = graph.split_crossings();
    let _ = graph.close_gaps(gap);
    // A bridge can cross a wall; splitting again restores planarity before the
    // face walk. ~keep
    let _ = graph.split_crossings();

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
        if let Some(index) = self.nearest_node(key, point) {
            return index;
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

    /// The closest existing node within `snap` of `point`, looking at the
    /// point's cell and its eight neighbours.
    ///
    /// The cell key alone would miss an end that straddles a cell boundary
    /// (two ends a fraction of a point apart can round into neighbouring cells)
    /// while merging ends up to a cell diagonal apart. Nodes sit on grid
    /// coordinates, so a node within `snap` is never more than one cell away
    /// from the point's cell. `snap == 0.0` merges only identical points. ~keep
    fn nearest_node(&self, key: (i64, i64), point: VectorPoint) -> Option<usize> {
        let mut best: Option<(f64, usize)> = None;
        for (offset_x, offset_y) in NEIGHBOUR_CELLS {
            let Some(&index) = self.node_index.get(&(key.0 + offset_x, key.1 + offset_y)) else {
                continue;
            };
            let candidate = distance(point, self.coords[index]);
            if candidate <= self.snap && best.is_none_or(|(best_distance, _)| candidate < best_distance) {
                best = Some((candidate, index));
            }
        }
        best.map(|(_, index)| index)
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
    /// Endpoint contacts are already handled by snapping. A sweep over the
    /// segments' x extents pairs only segments whose x ranges overlap, and every
    /// pair the sweep examines counts against [`MAX_INTERSECTION_TESTS`], whether
    /// or not its y ranges also overlap. Once the budget is spent the pass stops,
    /// leaving later crossings unsplit rather than spending unbounded time on a
    /// very dense sheet.
    fn split_crossings(&mut self) -> bool {
        self.split_crossings_with_limit(MAX_INTERSECTION_TESTS)
    }

    /// [`Self::split_crossings`] with an explicit pair-test budget; returns
    /// whether the budget ran out.
    fn split_crossings_with_limit(&mut self, limit: usize) -> bool {
        let mut edges: Vec<(usize, usize)> = self.edges.iter().copied().collect();
        edges.sort_unstable();
        if edges.len() < 2 {
            return false;
        }
        let mut order: Vec<usize> = (0..edges.len()).collect();
        order.sort_by(|&left, &right| {
            edge_x_extent(&self.coords, edges[left])
                .0
                .total_cmp(&edge_x_extent(&self.coords, edges[right]).0)
                .then(left.cmp(&right))
        });

        let mut splits: BTreeMap<usize, Vec<f64>> = BTreeMap::new();
        let mut tests = 0usize;
        let mut exhausted = false;
        'outer: for position in 0..order.len() {
            let first = order[position];
            let (a, b) = edges[first];
            let (ax, ay) = (self.coords[a], self.coords[b]);
            let first_x1 = edge_x_extent(&self.coords, edges[first]).1;
            for &second in &order[position + 1..] {
                let (c, d) = edges[second];
                if edge_x_extent(&self.coords, edges[second]).0 > first_x1 {
                    break;
                }
                tests += 1;
                if tests > limit {
                    exhausted = true;
                    break 'outer;
                }
                if !boxes_overlap(ax, ay, self.coords[c], self.coords[d]) {
                    continue;
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
            return exhausted;
        }

        for (edge_index, mut parameters) in splits {
            let (a, b) = edges[edge_index];
            let original = if a < b { (a, b) } else { (b, a) };
            let start = self.coords[a];
            let end = self.coords[b];
            parameters.push(0.0);
            parameters.push(1.0);
            parameters.sort_by(f64::total_cmp);
            parameters.dedup_by(|left, right| (*left - *right).abs() < 1e-9);
            let mut replacements: Vec<(usize, usize)> = Vec::with_capacity(parameters.len().saturating_sub(1));
            for window in parameters.windows(2) {
                let from = self.node(interpolate(start, end, window[0]));
                let to = self.node(interpolate(start, end, window[1]));
                if from != to {
                    replacements.push((from, to));
                }
            }
            // Remove before adding: a crossing that snaps onto an endpoint makes
            // one piece identical to the original edge, and removing after the
            // re-add would delete the whole wall. ~keep
            self.edges.remove(&original);
            for (from, to) in replacements {
                self.add_edge(from, to);
            }
        }
        exhausted
    }

    /// Join open ends (degree-one nodes) that are at most `gap` apart.
    ///
    /// A door opening leaves two wall ends facing each other; the nearest
    /// partner within the tolerance closes the room. Matching is greedy over
    /// pairs sorted by distance and then node index, so the result does not
    /// depend on hash order. Leaves are bucketed on a `gap`-sized grid so a
    /// hatched sheet pairs only neighbours, and the candidate list stops at
    /// [`MAX_GAP_CANDIDATES`].
    fn close_gaps(&mut self, gap: f64) -> bool {
        self.close_gaps_with_limit(gap, MAX_GAP_CANDIDATES)
    }

    /// [`Self::close_gaps`] with an explicit candidate budget; returns whether
    /// the budget ran out.
    fn close_gaps_with_limit(&mut self, gap: f64, limit: usize) -> bool {
        if gap <= 0.0 {
            return false;
        }
        let adjacency = self.adjacency();
        let mut leaves: Vec<usize> = (0..adjacency.len())
            .filter(|node| adjacency[*node].len() == 1)
            .collect();
        leaves.sort_unstable();
        if leaves.len() < 2 {
            return false;
        }

        let mut buckets: BTreeMap<(i64, i64), Vec<usize>> = BTreeMap::new();
        for &node in &leaves {
            buckets.entry(gap_cell(self.coords[node], gap)).or_default().push(node);
        }
        let position: HashMap<usize, usize> = leaves.iter().enumerate().map(|(index, node)| (*node, index)).collect();

        let mut candidates = Vec::new();
        let mut capped = false;
        'collect: for &first in &leaves {
            let (cell_x, cell_y) = gap_cell(self.coords[first], gap);
            for (offset_x, offset_y) in NEIGHBOUR_CELLS {
                let Some(bucket) = buckets.get(&(cell_x + offset_x, cell_y + offset_y)) else {
                    continue;
                };
                for &second in bucket {
                    if position[&second] <= position[&first] {
                        continue;
                    }
                    let distance = distance(self.coords[first], self.coords[second]);
                    if distance > gap {
                        continue;
                    }
                    if candidates.len() >= limit {
                        capped = true;
                        break 'collect;
                    }
                    candidates.push((distance, first, second));
                }
            }
        }
        if capped {
            tracing::debug!("closed-region gap pass reached its candidate limit");
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
        capped
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

/// The `gap`-sized grid cell containing `point`.
fn gap_cell(point: VectorPoint, gap: f64) -> (i64, i64) {
    (
        (f64::from(point.x) / gap).floor() as i64,
        (f64::from(point.y) / gap).floor() as i64,
    )
}

fn interpolate(start: VectorPoint, end: VectorPoint, t: f64) -> VectorPoint {
    VectorPoint {
        x: (f64::from(start.x) + (f64::from(end.x) - f64::from(start.x)) * t) as f32,
        y: (f64::from(start.y) + (f64::from(end.y) - f64::from(start.y)) * t) as f32,
    }
}

/// The x extent of an edge's two endpoints, as `(min, max)`.
fn edge_x_extent(coords: &[VectorPoint], edge: (usize, usize)) -> (f64, f64) {
    let (left, right) = (coords[edge.0], coords[edge.1]);
    ordered(f64::from(left.x), f64::from(right.x))
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
#[path = "regions_tests.rs"]
mod regions_tests;
