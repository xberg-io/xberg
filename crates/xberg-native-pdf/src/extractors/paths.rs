//! Path extraction from PDF content streams.
//!
//! This module provides the `PathExtractor` type for extracting vector graphics
//! (paths) from PDF content streams.
//!
//! # PDF Path Operations
//!
//! PDF paths are constructed using a sequence of operators:
//! - `m` (MoveTo): Begin a new subpath
//! - `l` (LineTo): Add a line segment
//! - `c`, `v`, `y` (CurveTo variants): Add Bezier curve segments
//! - `re` (Rectangle): Add a rectangle as a complete subpath
//! - `h` (ClosePath): Close the current subpath
//!
//! Paths are then painted using:
//! - `S` (Stroke): Stroke the path
//! - `f`, `F`, `f*` (Fill): Fill the path
//! - `B`, `B*`, `b`, `b*`: Fill and stroke
//! - `n` (EndPath): End path without painting (used with clipping)
//!
//! # Example
//!
//! ```ignore
//! use xberg_native_pdf::extractors::paths::PathExtractor;
//!
//! let mut extractor = PathExtractor::new();
//!
//! // Process path construction operators
//! extractor.move_to(100.0, 100.0);
//! extractor.line_to(200.0, 100.0);
//! extractor.line_to(200.0, 200.0);
//! extractor.close_path();
//!
//! // Finalize with a painting operator
//! extractor.stroke();
//!
//! // Get extracted paths
//! let paths = extractor.finish();
//! ```

use crate::content::graphics_state::{GraphicsState, Matrix};
use crate::elements::{LineCap, LineJoin, PathContent, PathOperation, path_operations_bbox};
use crate::geometry::Point;
use crate::layout::Color;

/// Lightweight graphics state for path extraction. The dash array is empty (no allocation)
/// for solid lines, so q/Q save/restore allocates only while a dash is active.
#[derive(Debug, Clone)]
pub(crate) struct PathGraphicsState {
    pub ctm: Matrix,
    pub stroke_color_rgb: (f32, f32, f32),
    pub fill_color_rgb: (f32, f32, f32),
    pub line_width: f32,
    pub stroke_alpha: f32,
    pub line_cap: u8,
    pub line_join: u8,
    /// `d` operand: on/off lengths in user space and phase; an empty array is solid.
    pub dash: (Vec<f32>, f32),
}

impl PathGraphicsState {
    pub fn new() -> Self {
        Self {
            ctm: Matrix::identity(),
            stroke_color_rgb: (0.0, 0.0, 0.0),
            fill_color_rgb: (0.0, 0.0, 0.0),
            line_width: 1.0,
            stroke_alpha: 1.0,
            line_cap: 0,
            line_join: 0,
            dash: (Vec::new(), 0.0),
        }
    }
}

/// Graphics state stack using [`PathGraphicsState`]; save/restore clones it, allocating only while a dash is set.
pub(crate) struct PathGraphicsStateStack {
    stack: Vec<PathGraphicsState>,
}

impl PathGraphicsStateStack {
    pub fn new() -> Self {
        Self {
            stack: vec![PathGraphicsState::new()],
        }
    }

    pub fn current(&self) -> &PathGraphicsState {
        self.stack.last().expect("Stack should never be empty")
    }

    pub fn current_mut(&mut self) -> &mut PathGraphicsState {
        self.stack.last_mut().expect("Stack should never be empty")
    }

    pub fn save(&mut self) {
        let state = self.current().clone();
        self.stack.push(state);
    }

    pub fn restore(&mut self) {
        if self.stack.len() > 1 {
            self.stack.pop();
        }
    }
}

/// The dash pattern as drawn, in page space, or `None` when it paints a solid line: an empty array,
/// lengths that are negative or non-finite, a collapsed CTM scale, or gaps of zero length (§8.4.3.6).
fn visible_dash((array, phase): &(Vec<f32>, f32), scale: f32) -> Option<(Vec<f32>, f32)> {
    if array.is_empty() {
        return None;
    }
    let scaled: Vec<f32> = array.iter().map(|length| length * scale).collect();
    if scaled.iter().any(|length| !length.is_finite() || *length < 0.0) || !phase.is_finite() {
        return None;
    }
    // An odd-length array repeats with the roles swapped, so every length is a gap at some point. ~keep
    let gaps_are_open = if scaled.len() % 2 == 1 {
        scaled.iter().any(|length| *length > 0.0)
    } else {
        scaled.iter().skip(1).step_by(2).any(|gap| *gap > 0.0)
    };
    gaps_are_open.then(|| (scaled, phase * scale))
}

/// Fill rule for path filling operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillRule {
    /// Non-zero winding number rule (f, F operators)
    NonZero,
    /// Even-odd rule (f* operator)
    EvenOdd,
}

/// Cache source fields only: omitted fields must inherit the state at each application. ~keep
#[derive(Debug, Default)]
pub(crate) struct PathGraphicsPatch {
    pub(crate) dash: Option<(Vec<f32>, f32)>,
    pub(crate) stroke_alpha: Option<f32>,
}

/// Path extractor for accumulating path operations from content streams.
///
/// This struct maintains state during content stream processing and accumulates
/// path operations. When a painting operator is encountered, the current path
/// is finalized and added to the extracted paths list.
///
/// # XObject Support
///
/// The extractor can recursively process Form XObjects by maintaining a reference
/// to page resources and the document. When encountering a `Do` operator with a
/// Form XObject name, it will:
/// 1. Resolve the XObject from resources
/// 2. Check if it's a Form (not Image)
/// 3. Apply coordinate transformations
/// 4. Recursively extract paths from the XObject stream
///
/// XObjects are tracked to prevent infinite loops from circular references.
#[derive(Debug)]
pub struct PathExtractor {
    /// Accumulated complete paths
    paths: Vec<PathContent>,
    /// Current path being constructed
    current_operations: Vec<PathOperation>,
    /// Current point (set by MoveTo, LineTo, CurveTo, etc.)
    current_point: Option<Point>,
    /// Start point of current subpath (for ClosePath)
    subpath_start: Option<Point>,
    /// Current graphics state snapshot (for colors, line style)
    current_stroke_color: Option<Color>,
    current_fill_color: Option<Color>,
    current_line_width: f32,
    current_stroke_alpha: f32,
    current_line_cap: LineCap,
    current_line_join: LineJoin,
    current_dash: (Vec<f32>, f32),
    /// Current transformation matrix for coordinate transformation
    ctm: Matrix,
    /// Page resources for XObject resolution
    resources: Option<crate::object::Object>,
    /// Stack of XObjects being processed to detect cycles
    xobject_processing_stack: Vec<crate::object::ObjectRef>,
    /// Set of XObjects already fully processed. Prevents combinatorial
    /// explosion when multiple parent XObjects reference the same children
    /// (e.g., chart/plot pages where 9 top-level Form XObjects each contain
    /// 25-42 nested `Do` operators pointing to the same shared XObjects).
    processed_xobjects: std::collections::HashSet<(crate::object::ObjectRef, [i32; 6])>,
    /// Maximum XObject nesting depth (prevent stack overflow)
    max_xobject_depth: usize,
    /// Cached XObject name → ObjectRef mapping, built on first lookup.
    cached_xobject_dict: Option<std::collections::HashMap<String, crate::object::ObjectRef>>,
    cached_extgstates: std::collections::HashMap<String, PathGraphicsPatch>,
    /// The scope's indirect `/ExtGState` container, loaded once. `None` until first resolved;
    /// `Some(None)` when the entry is absent or fails to load.
    cached_extgstate_container: Option<Option<crate::object::Object>>,
    /// Stack of active Optional Content Group (PDF "layer") names. Each
    /// `BDC`/`BMC` operator (any tag) pushes one entry and each `EMC` pops
    /// one, so the depth tracks the marked-content nesting precisely
    /// (ISO 32000-1:2008 §8.11, §14.6). Entries are stored *pre-resolved*
    /// to the effective layer at that level: an `/OC` tag whose property
    /// dict resolves to a named OCG stores that name, while non-`/OC`
    /// marked content (and `/OC` regions whose name did not resolve)
    /// inherits the layer already in effect. The top entry is therefore
    /// always the active layer, making `current_layer` O(1).
    oc_layer_stack: Vec<Option<String>>,
}

/// Resources, XObject name cache, and ExtGState caches of the scope a Form XObject replaced.
pub(crate) type SavedResourceScope = (
    Option<crate::object::Object>,
    Option<std::collections::HashMap<String, crate::object::ObjectRef>>,
    std::collections::HashMap<String, PathGraphicsPatch>,
    Option<Option<crate::object::Object>>,
);

impl PathExtractor {
    /// Create a new path extractor.
    pub fn new() -> Self {
        Self {
            paths: Vec::new(),
            current_operations: Vec::new(),
            current_point: None,
            subpath_start: None,
            current_stroke_color: Some(Color::black()),
            current_fill_color: Some(Color::black()),
            current_line_width: 1.0,
            current_stroke_alpha: 1.0,
            current_line_cap: LineCap::Butt,
            current_line_join: LineJoin::Miter,
            current_dash: (Vec::new(), 0.0),
            ctm: Matrix::identity(),
            resources: None,
            xobject_processing_stack: Vec::new(),
            processed_xobjects: std::collections::HashSet::new(),
            max_xobject_depth: 100,
            cached_xobject_dict: None,
            cached_extgstates: std::collections::HashMap::new(),
            cached_extgstate_container: None,
            oc_layer_stack: Vec::new(),
        }
    }

    /// Push an entry onto the marked-content layer stack.
    ///
    /// Pass `Some(name)` for `BDC /OC <<...>>` (or `BDC /OC /MC0`) where
    /// the property dict resolves to a named Optional Content Group. Pass
    /// `None` for any other `BDC`/`BMC` (e.g. `BDC /Span <<...>>`,
    /// `BDC /Artifact`) so the stack stays balanced and outer OCG context
    /// continues to apply to nested non-OC content.
    pub fn push_oc_layer(&mut self, layer: Option<String>) {
        // Store the *effective* layer for this nesting level: an `/OC` tag
        // with a resolved OCG name takes effect; every other entry (non-OC
        // `BDC`/`BMC`, or an `/OC` whose name did not resolve) inherits the
        // layer already in effect. Pre-resolving here keeps `current_layer`
        // O(1) instead of an O(depth) scan on every finalized path. ~keep
        let effective = layer.or_else(|| self.oc_layer_stack.last().cloned().flatten());
        self.oc_layer_stack.push(effective);
    }

    /// Pop the most recently pushed marked-content entry (called on `EMC`).
    /// No-op if the stack is empty — keeps extraction robust against PDFs
    /// with unbalanced markers.
    pub fn pop_oc_layer(&mut self) {
        self.oc_layer_stack.pop();
    }

    /// Current marked-content nesting depth. Paired with [`truncate_oc_layers`]
    /// to isolate a Form XObject's marked-content scope from its caller.
    pub(crate) fn oc_layer_depth(&self) -> usize {
        self.oc_layer_stack.len()
    }

    /// Drop any marked-content entries pushed above `depth`. Called when a
    /// Form XObject content stream returns so an unbalanced `BDC` left open
    /// inside the XObject cannot leak its layer onto the caller's subsequent
    /// paths. The spec requires per-stream balance (§14.6), but real-world
    /// PDFs occasionally violate it.
    pub(crate) fn truncate_oc_layers(&mut self, depth: usize) {
        self.oc_layer_stack.truncate(depth);
    }

    /// Return the active Optional Content Group ("layer") name, or `None`
    /// if no `/OC` region is currently in effect. O(1): each stack entry is
    /// pre-resolved to its effective layer by [`push_oc_layer`].
    fn current_layer(&self) -> Option<String> {
        self.oc_layer_stack.last().cloned().flatten()
    }

    /// Set the page resources for XObject resolution.
    pub fn set_resources(&mut self, resources: crate::object::Object) {
        self.resources = Some(resources);
        self.cached_extgstates.clear();
        self.cached_extgstate_container = None;
    }

    /// The resource dictionary currently in scope — the page resources, or
    /// the active Form XObject's own `/Resources` after a [`swap_resources`].
    /// Used to resolve `BDC /OC /Name` property references against the
    /// current `/Properties` subdictionary (ISO 32000-1:2008 §14.6.2).
    pub(crate) fn current_resources(&self) -> Option<&crate::object::Object> {
        self.resources.as_ref()
    }

    /// Swap in a new resource scope for `Do` name lookups and return the
    /// previous scope so the caller can restore it after descending into a
    /// nested Form XObject. The ExtGState caches belong to the scope, so they travel with
    /// it and the entered scope starts empty. Clears the name→ref
    /// cache so the next `resolve_xobject_ref` call rebuilds it against the
    /// new scope.
    ///
    /// Needed because Form XObjects that carry their own /Resources define a
    /// fresh XObject name scope — without swapping, nested `/Name Do`
    /// operators resolve against the parent scope and can trigger pathological
    /// cross-recursion between sibling forms whose local resource names happen
    /// to collide with parent form names.
    pub(crate) fn swap_resources(
        &mut self,
        new_resources: Option<crate::object::Object>,
    ) -> SavedResourceScope {
        let prev_resources = std::mem::replace(&mut self.resources, new_resources);
        let prev_cache = self.cached_xobject_dict.take();
        let prev_extgstates = std::mem::take(&mut self.cached_extgstates);
        let prev_container = self.cached_extgstate_container.take();
        (prev_resources, prev_cache, prev_extgstates, prev_container)
    }

    /// Restore a (resources, cached_dict) pair previously returned by
    /// [`swap_resources`].
    pub(crate) fn restore_resources(&mut self, saved: SavedResourceScope) {
        self.resources = saved.0;
        self.cached_xobject_dict = saved.1;
        self.cached_extgstates = saved.2;
        self.cached_extgstate_container = saved.3;
    }

    /// Load the scope's indirect `/ExtGState` container on first use and keep it, so each
    /// `gs` name borrows it instead of re-cloning the whole dictionary.
    pub(crate) fn extgstate_container<F>(&mut self, load: F) -> Option<&crate::object::Object>
    where
        F: FnOnce(crate::object::ObjectRef) -> Option<crate::object::Object>,
    {
        if self.cached_extgstate_container.is_none() {
            let loaded = self
                .resources
                .as_ref()
                .and_then(|resources| resources.as_dict()?.get("ExtGState")?.as_reference())
                .and_then(load);
            self.cached_extgstate_container = Some(loaded);
        }
        self.loaded_extgstate_container()
    }

    /// The container cached by [`extgstate_container`], if it loaded.
    pub(crate) fn loaded_extgstate_container(&self) -> Option<&crate::object::Object> {
        self.cached_extgstate_container.as_ref()?.as_ref()
    }

    pub(crate) fn cache_path_extgstate(&mut self, name: &str, patch: PathGraphicsPatch) {
        self.cached_extgstates.insert(name.to_owned(), patch);
    }

    pub(crate) fn apply_cached_path_extgstate(&mut self, name: &str, state: &mut PathGraphicsState) -> bool {
        let Some(patch) = self.cached_extgstates.get(name) else {
            return false;
        };
        let alpha = patch.stroke_alpha;
        if let Some((lengths, phase)) = &patch.dash {
            state.dash.0.clone_from(lengths);
            state.dash.1 = *phase;
            self.current_dash.0.clone_from(lengths);
            self.current_dash.1 = *phase;
        }
        if let Some(alpha) = alpha {
            state.stroke_alpha = alpha;
            self.set_stroke_alpha(alpha);
        }
        true
    }

    /// Resolve an XObject name to its ObjectRef, caching the XObject dict on first call.
    pub(crate) fn resolve_xobject_ref<F>(&mut self, name: &str, mut load_object: F) -> Option<crate::object::ObjectRef>
    where
        F: FnMut(crate::object::ObjectRef) -> crate::error::Result<crate::object::Object>,
    {
        if self.cached_xobject_dict.is_none() {
            let mut map = std::collections::HashMap::new();

            let resources = self.resources.as_ref()?;
            let resolved_resources = if let Some(ref_obj) = resources.as_reference() {
                load_object(ref_obj).ok()?
            } else {
                resources.clone()
            };
            let resources_dict = resolved_resources.as_dict()?;
            let xobject_obj = resources_dict.get("XObject")?;
            let resolved_xobject_obj = if let Some(ref_obj) = xobject_obj.as_reference() {
                load_object(ref_obj).ok()?
            } else {
                xobject_obj.clone()
            };
            if let Some(xobject_dict) = resolved_xobject_obj.as_dict() {
                for (key, val) in xobject_dict.iter() {
                    if let Some(obj_ref) = val.as_reference() {
                        map.insert(key.clone(), obj_ref);
                    }
                }
            }
            self.cached_xobject_dict = Some(map);
        }

        self.cached_xobject_dict.as_ref()?.get(name).copied()
    }

    /// Compute a rounded fingerprint of the current CTM for dedup purposes.
    /// Translation components (e, f) are rounded to 0.1 while scale/rotation
    /// components (a-d) are rounded to 0.01, balancing dedup accuracy with
    /// floating-point tolerance. Uses banker's-style `f32::round` so negative
    /// values round symmetrically rather than truncating toward zero.
    fn ctm_fingerprint(ctm: &Matrix) -> [i32; 6] {
        [
            (ctm.a * 100.0).round() as i32,
            (ctm.b * 100.0).round() as i32,
            (ctm.c * 100.0).round() as i32,
            (ctm.d * 100.0).round() as i32,
            (ctm.e * 10.0).round() as i32,
            (ctm.f * 10.0).round() as i32,
        ]
    }

    pub(crate) fn can_process_xobject(&self, xobject_ref: crate::object::ObjectRef) -> bool {
        let key = (xobject_ref, Self::ctm_fingerprint(&self.ctm));
        if self.processed_xobjects.contains(&key) {
            return false;
        }
        if self.xobject_processing_stack.contains(&xobject_ref) {
            return false;
        }
        if self.xobject_processing_stack.len() >= self.max_xobject_depth {
            return false;
        }
        true
    }

    /// Push an XObject onto the processing stack (called before processing).
    pub(crate) fn push_xobject(&mut self, xobject_ref: crate::object::ObjectRef) {
        self.xobject_processing_stack.push(xobject_ref);
    }

    /// Pop an XObject from the processing stack after successful processing.
    /// Marks it as permanently processed to prevent re-processing from
    /// other parent XObjects at the same CTM.
    pub(crate) fn pop_xobject(&mut self) {
        if let Some(ref_obj) = self.xobject_processing_stack.pop() {
            let key = (ref_obj, Self::ctm_fingerprint(&self.ctm));
            self.processed_xobjects.insert(key);
        }
    }

    /// Pop an XObject from the processing stack after a failure.
    /// Does NOT mark it as permanently processed, allowing retry.
    pub(crate) fn pop_xobject_failed(&mut self) {
        self.xobject_processing_stack.pop();
    }

    /// Update the current transformation matrix.
    pub fn set_ctm(&mut self, ctm: Matrix) {
        self.ctm = ctm;
    }

    /// Update graphics state from a GraphicsState snapshot.
    pub fn update_from_state(&mut self, state: &GraphicsState) {
        self.ctm = state.ctm;
        self.current_line_width = state.line_width;
        self.current_stroke_alpha = state.stroke_alpha;

        // Convert line cap (0=butt, 1=round, 2=square) ~keep
        self.current_line_cap = match state.line_cap {
            1 => LineCap::Round,
            2 => LineCap::Square,
            _ => LineCap::Butt,
        };

        // Convert line join (0=miter, 1=round, 2=bevel) ~keep
        self.current_line_join = match state.line_join {
            1 => LineJoin::Round,
            2 => LineJoin::Bevel,
            _ => LineJoin::Miter,
        };

        let (r, g, b) = state.stroke_color_rgb;
        self.current_stroke_color = Some(Color::new(r, g, b));

        let (r, g, b) = state.fill_color_rgb;
        self.current_fill_color = Some(Color::new(r, g, b));
        self.current_dash = state.dash_pattern.clone();
    }

    /// Update extractor state from a lightweight path graphics state.
    pub(crate) fn update_from_path_state(&mut self, state: &PathGraphicsState) {
        self.ctm = state.ctm;
        self.current_line_width = state.line_width;
        self.current_stroke_alpha = state.stroke_alpha;
        self.current_line_cap = match state.line_cap {
            1 => LineCap::Round,
            2 => LineCap::Square,
            _ => LineCap::Butt,
        };
        self.current_line_join = match state.line_join {
            1 => LineJoin::Round,
            2 => LineJoin::Bevel,
            _ => LineJoin::Miter,
        };
        let (r, g, b) = state.stroke_color_rgb;
        self.current_stroke_color = Some(Color::new(r, g, b));
        let (r, g, b) = state.fill_color_rgb;
        self.current_fill_color = Some(Color::new(r, g, b));
        self.current_dash = state.dash.clone();
    }

    /// Set the line dash pattern (`d` operator); an empty array restores a solid line.
    pub fn set_dash(&mut self, array: Vec<f32>, phase: f32) {
        self.current_dash = (array, phase);
    }

    pub(crate) fn set_stroke_alpha(&mut self, alpha: f32) {
        self.current_stroke_alpha = alpha;
    }

    /// Set stroke color.
    pub fn set_stroke_color(&mut self, color: Color) {
        self.current_stroke_color = Some(color);
    }

    /// Set fill color.
    pub fn set_fill_color(&mut self, color: Color) {
        self.current_fill_color = Some(color);
    }

    /// Set line width.
    pub fn set_line_width(&mut self, width: f32) {
        self.current_line_width = width;
    }

    /// Set line cap style.
    pub fn set_line_cap(&mut self, cap: LineCap) {
        self.current_line_cap = cap;
    }

    /// Set line join style.
    pub fn set_line_join(&mut self, join: LineJoin) {
        self.current_line_join = join;
    }

    /// Move to a point (m operator).
    ///
    /// Begins a new subpath at the specified point.
    pub fn move_to(&mut self, x: f32, y: f32) {
        let point = self.transform_point(x, y);
        self.current_operations.push(PathOperation::MoveTo(point.x, point.y));
        self.current_point = Some(point);
        self.subpath_start = Some(point);
    }

    /// Line to a point (l operator).
    ///
    /// Adds a line segment from the current point to the specified point.
    pub fn line_to(&mut self, x: f32, y: f32) {
        let point = self.transform_point(x, y);
        self.current_operations.push(PathOperation::LineTo(point.x, point.y));
        self.current_point = Some(point);
    }

    /// Cubic Bezier curve (c operator).
    ///
    /// Adds a cubic Bezier curve from the current point to (x3, y3),
    /// using (x1, y1) and (x2, y2) as control points.
    pub fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x3: f32, y3: f32) {
        let p1 = self.transform_point(x1, y1);
        let p2 = self.transform_point(x2, y2);
        let p3 = self.transform_point(x3, y3);
        self.current_operations
            .push(PathOperation::CurveTo(p1.x, p1.y, p2.x, p2.y, p3.x, p3.y));
        self.current_point = Some(p3);
    }

    /// Cubic Bezier curve with first control point = current point (v operator).
    ///
    /// Adds a cubic Bezier curve from the current point to (x3, y3),
    /// using the current point as the first control point and (x2, y2) as the second.
    pub fn curve_to_v(&mut self, x2: f32, y2: f32, x3: f32, y3: f32) {
        let p1 = self.current_point.unwrap_or(Point::new(0.0, 0.0));
        let p2 = self.transform_point(x2, y2);
        let p3 = self.transform_point(x3, y3);
        self.current_operations
            .push(PathOperation::CurveTo(p1.x, p1.y, p2.x, p2.y, p3.x, p3.y));
        self.current_point = Some(p3);
    }

    /// Cubic Bezier curve with second control point = end point (y operator).
    ///
    /// Adds a cubic Bezier curve from the current point to (x3, y3),
    /// using (x1, y1) as the first control point and (x3, y3) as the second.
    pub fn curve_to_y(&mut self, x1: f32, y1: f32, x3: f32, y3: f32) {
        let p1 = self.transform_point(x1, y1);
        let p3 = self.transform_point(x3, y3);
        self.current_operations
            .push(PathOperation::CurveTo(p1.x, p1.y, p3.x, p3.y, p3.x, p3.y));
        self.current_point = Some(p3);
    }

    /// Rectangle (re operator).
    ///
    /// Adds a complete rectangle subpath.
    pub fn rectangle(&mut self, x: f32, y: f32, width: f32, height: f32) {
        let p1 = self.transform_point(x, y);
        let p2 = self.transform_point(x + width, y + height);

        let transformed_width = p2.x - p1.x;
        let transformed_height = p2.y - p1.y;

        self.current_operations.push(PathOperation::Rectangle(
            p1.x,
            p1.y,
            transformed_width,
            transformed_height,
        ));

        // Rectangle implicitly closes the subpath ~keep
        self.current_point = Some(p1);
        self.subpath_start = Some(p1);
    }

    /// Close the current subpath (h operator).
    ///
    /// Adds a line segment from the current point to the start of the subpath.
    pub fn close_path(&mut self) {
        self.current_operations.push(PathOperation::ClosePath);
        if let Some(start) = self.subpath_start {
            self.current_point = Some(start);
        }
    }

    /// Stroke the path (S operator).
    pub fn stroke(&mut self) {
        self.finalize_path(true, false, FillRule::NonZero);
    }

    /// Close and stroke the path (s operator).
    pub fn close_and_stroke(&mut self) {
        self.close_path();
        self.stroke();
    }

    /// Fill the path (f or F operator).
    pub fn fill(&mut self, rule: FillRule) {
        self.finalize_path(false, true, rule);
    }

    /// Fill and stroke the path (B operator).
    pub fn fill_and_stroke(&mut self, rule: FillRule) {
        self.finalize_path(true, true, rule);
    }

    /// Close, fill and stroke the path (b operator).
    pub fn close_fill_and_stroke(&mut self, rule: FillRule) {
        self.close_path();
        self.fill_and_stroke(rule);
    }

    /// End path without painting (n operator).
    ///
    /// Used primarily with clipping paths.
    pub fn end_path(&mut self) {
        self.current_operations.clear();
        self.current_point = None;
        self.subpath_start = None;
    }

    /// Set clipping path using non-zero winding rule (W operator).
    pub fn clip_non_zero(&mut self) {
        // Clipping doesn't paint the path, just sets the clipping region
        // The path is still available for subsequent painting
        // We don't extract clipping paths as separate content ~keep
    }

    /// Set clipping path using even-odd rule (W* operator).
    pub fn clip_even_odd(&mut self) {
        // Same as clip_non_zero - clipping doesn't create visible content ~keep
    }

    /// Transform a point using the current CTM.
    fn transform_point(&self, x: f32, y: f32) -> Point {
        self.ctm.transform_point(x, y)
    }

    /// Finalize the current path and add it to the extracted paths.
    fn finalize_path(&mut self, stroke: bool, fill: bool, _rule: FillRule) {
        if self.current_operations.is_empty() {
            return;
        }

        let bbox = path_operations_bbox(&self.current_operations);

        let mut path = PathContent::new(bbox);
        path.operations = std::mem::take(&mut self.current_operations);

        if stroke && self.current_stroke_alpha > 0.0 {
            path.stroke_color = self.current_stroke_color;
            // The `w` operand is in user space while the path coordinates
            // above were CTM-transformed (§8.4.3.2: the line width is
            // transformed like all other geometry). Store the effective
            // rendered width — `sqrt(|det|)` is the standard uniform-scale
            // approximation renderers use — so `stroke_width` and `bbox`
            // share one coordinate space. ~keep
            path.stroke_width = self.current_line_width * self.ctm.stroke_scale();
            path.line_cap = self.current_line_cap;
            path.line_join = self.current_line_join;
            // Dash lengths are user-space like `w`, so they scale with the CTM the same way. ~keep
            path.dash_pattern = visible_dash(&self.current_dash, self.ctm.stroke_scale());
        } else {
            path.stroke_color = None;
        }

        if fill {
            path.fill_color = self.current_fill_color;
        } else {
            path.fill_color = None;
        }

        // Attach the active Optional Content Group (PDF "layer") name, if
        // the path was emitted inside a `BDC /OC … EMC` region. ~keep
        path.layer = self.current_layer();

        self.paths.push(path);

        self.current_point = None;
        self.subpath_start = None;
    }

    /// Finish extraction and return all extracted paths.
    pub fn finish(self) -> Vec<PathContent> {
        self.paths
    }

    /// Get the number of paths extracted so far.
    pub fn path_count(&self) -> usize {
        self.paths.len()
    }

    /// Check if there's a path currently being constructed.
    pub fn has_current_path(&self) -> bool {
        !self.current_operations.is_empty()
    }
}

impl Default for PathExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
