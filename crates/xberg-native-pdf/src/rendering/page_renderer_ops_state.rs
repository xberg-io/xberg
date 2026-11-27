//! Mutable state shared by the operator loop in
//! [`PageRenderer::execute_operators_with_backdrop`] and the per-operator helpers.

use super::*;

/// Loop-invariant inputs every operator helper needs.
#[derive(Clone, Copy)]
pub(super) struct OpEnv<'a> {
    pub(super) base_transform: Transform,
    pub(super) doc: &'a PdfDocument,
    pub(super) page_num: usize,
    pub(super) resources: &'a Object,
}

/// Locals of the operator loop that persist from one operator to the next.
pub(super) struct OpLoopState<'a> {
    pub(super) gs_stack: GraphicsStateStack,
    pub(super) in_text_object: bool,
    pub(super) current_path: PathBuilder,
    pub(super) pending_clip: Option<(tiny_skia::Path, tiny_skia::FillRule)>,
    /// One clip per `q` level. Levels share a page-sized mask until one of them narrows it, which
    /// copies it first (`Arc::make_mut` in `apply_pending_clip`), so `q` copies no pixels.
    pub(super) clip_stack: Vec<Option<std::sync::Arc<tiny_skia::Mask>>>,
    /// Text-clip accumulator (ISO 32000-1 §9.3.6 / Table 106, `Tr` modes 4–7).
    /// Text render modes ≥4 add the union of their glyph outlines to a "text
    /// clip path" that is intersected into the current clip at `ET`. We
    /// accumulate that union as opaque glyph coverage in a page-sized scratch
    /// pixmap's alpha channel (unioned across shows via SourceOver), then
    /// convert it to a `Mask` at `ET`. This stays allocation-free for the
    /// normal-text hot path: the scratch is created lazily only when a
    /// mode-≥4 show actually fires, and modes 0–3 never touch it. Complexity
    /// is inherently capped — coverage folds into a fixed-size buffer, so no
    /// unbounded path growth is possible. ~keep
    pub(super) text_clip_accum: Option<Pixmap>,
    pub(super) excluded_layers: &'a HashSet<String>,
    /// OCG layer exclusion tracking.
    /// `excluded_layer_depth` counts how many nested BDC/OC scopes we are
    /// inside that match an excluded layer. >0 means content is suppressed.
    /// `marked_content_is_excluded` is the per-scope stack of exclusion flags:
    /// every BDC/BMC pushes one entry and EMC pops exactly one, so a pop only
    /// decrements `excluded_layer_depth` when the entry it removes was excluded. ~keep
    pub(super) excluded_layer_depth: u32,
    pub(super) marked_content_is_excluded: Vec<bool>,
    pub(super) ext_g_states: Option<&'a HashMap<String, Object>>,
    /// Cache parsed state per `dict_name` so the inner-dict resolve happens
    /// at most once per unique name in scope. ~keep
    pub(super) ext_g_state_cache: HashMap<String, ParsedExtGState>,
}

impl<'a> OpLoopState<'a> {
    pub(super) fn new(
        gs_stack: GraphicsStateStack,
        excluded_layers: &'a HashSet<String>,
        ext_g_states: Option<&'a HashMap<String, Object>>,
    ) -> Self {
        Self {
            gs_stack,
            in_text_object: false,
            current_path: PathBuilder::new(),
            pending_clip: None,
            clip_stack: vec![None],
            text_clip_accum: None,
            excluded_layers,
            excluded_layer_depth: 0,
            marked_content_is_excluded: Vec::new(),
            ext_g_states,
            ext_g_state_cache: HashMap::new(),
        }
    }
}

impl PageRenderer {
    /// Graphics-state stack for a fresh operator run: DeviceGray black, with the
    /// Type 3 `d1` fill lock applied when one is active.
    pub(super) fn seed_operator_graphics_state(&self) -> GraphicsStateStack {
        let mut gs_stack = GraphicsStateStack::new();

        {
            let gs = gs_stack.current_mut();
            gs.fill_color_space = "DeviceGray".to_string();
            gs.stroke_color_space = "DeviceGray".to_string();
            gs.fill_color_rgb = (0.0, 0.0, 0.0);
            gs.stroke_color_rgb = (0.0, 0.0, 0.0);
        }

        // Type 3 `d1` glyph description: seed the fill colour with the locked
        // current colour so the stencil paints in it. Set the same fields the
        // `rg` operator would (RGB, colour space, and components) so the
        // colour-resolution pipeline reproduces it exactly. Colour operators
        // inside the glyph are ignored below (ISO 32000-1:2008 §9.6.5.2). ~keep
        if let Some((r, g, b)) = self.type3_fill_lock {
            let gs = gs_stack.current_mut();
            gs.fill_color_rgb = (r, g, b);
            gs.fill_color_space = "DeviceRGB".to_string();
            gs.fill_color_components.clear();
            gs.fill_color_components.extend_from_slice(&[r, g, b]);
        }

        gs_stack
    }
}
