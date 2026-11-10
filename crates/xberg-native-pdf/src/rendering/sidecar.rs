//! Per-page compositing sidecar for transparency + spot-ink rendering.
//!
//! ISO 32000-1:2008 §11.4 (and §11.4 in ISO 32000-2:2020) defines
//! transparency compositing as a *source-space* operation: each paint
//! is blended against the backdrop in the page-group blend space, and
//! only after every transparency / soft-mask / knockout operation has
//! been resolved does the output get handed off to the device. For a
//! press-target output the blend space is `DeviceCMYK` (or calibrated
//! CMYK via an `ICCBased` profile) and the final hand-off goes to
//! per-plate separations — that is the "composite-then-separate"
//! workflow §11.7.3 / §11.7.4 describe.
//!
//! The page renderer keeps a 4-channel `DeviceCMYK` plane alongside
//! the visible RGBA pixmap so the compose-first and overprint helpers
//! can read the backdrop CMYK quadruple directly instead of inverting
//! the post-ICC RGB (which is lossy under non-linear OutputIntent
//! profiles). This sidecar IS the §11.4 compositing buffer for the
//! process channels.
//!
//! # Spot inks
//!
//! ISO 32000-1 §11.3.4 enumerates the legal blend colour spaces
//! (`DeviceGray`, `DeviceRGB`, `DeviceCMYK`, CIE-based equivalents,
//! and bidirectional `ICCBased` of those) and explicitly excludes
//! `Separation` and `DeviceN`:
//!
//! > "The blending colour space shall be consulted only for process
//! > colours. … such colours shall not be converted to a blending
//! > colour space … the specified colour components shall be blended
//! > individually with the corresponding components of the backdrop."
//!
//! §11.6.6 (Table 147 `/CS` entry) carries the same restriction
//! forward for transparency-group colour spaces. §11.7.3 prescribes
//! the sidecar model:
//!
//! > "When an object is painted transparently with a spot colour
//! > component that is available in the output device, that colour
//! > shall be composited with the corresponding spot colour
//! > component of the backdrop, independently of the compositing that
//! > is performed for process colours. A spot colour retains its own
//! > identity; it shall not be subject to conversion to or from the
//! > colour space of the enclosing transparency group or page."
//!
//! Concretely: the spot lanes ride *alongside* the process blend
//! space, not inside it. They are per-component buffers that the
//! compositing math touches separately from the process lanes.
//!
//! # §11.7.4.2 blend-mode split
//!
//! §11.7.4.2 is the dispositive rule for non-separable and
//! non-white-preserving blend modes on spot channels:
//!
//! > "The PDF graphics state specifies only one current blend mode
//! > parameter, which shall always apply to process colorants and
//! > sometimes to spot colorants as well. Specifically, only
//! > separable, white-preserving blend modes shall be used for spot
//! > colours. If the specified blend mode is not separable and
//! > white-preserving, it shall apply only to process colour
//! > components, and the **Normal** blend mode shall be substituted
//! > for spot colours."
//!
//! The four non-separable modes (`/Hue`, `/Saturation`, `/Color`,
//! `/Luminosity`, §11.3.5.3) AND the two separable-but-non-white-
//! preserving modes (`/Difference`, `/Exclusion`, §11.3.5.2 Note 2)
//! all trigger `/Normal` substitution on spot lanes. This is encoded
//! by [`BlendModeClass`](crate::rendering::sidecar::BlendModeClass)
//! below.
//!
//! Process lanes always honour the requested blend mode; for non-sep
//! modes the §11.3.5.3 CMYK projection (complement `CMY → RGB`,
//! blend, complement back; `K = K_b` for Hue / Saturation / Color and
//! `K = K_s` for Luminosity) applies. That math lives in the renderer
//! (round 2 will wire it for the spot-aware paths); this module
//! supplies only the classification helper.
//!
//! # Storage layout
//!
//! The `CmykSidecar` storage type (crate-private; see the type
//! definition below) owns two separate buffers:
//!
//! - `cmyk`: a packed `4·w·h` byte plane with the four `DeviceCMYK`
//!   channels in `(C, M, Y, K)` order, row-major, top-left origin.
//!   This matches the round-4 layout exactly so every existing
//!   process-plane helper (mirror, compose-first, overprint) consumes
//!   it unchanged.
//! - `spots`: a plane-per-ink stack. For `N` discovered spot inks the
//!   buffer is `N·w·h` bytes long; spot `i`'s plane is the slice
//!   `spots[i·w·h .. (i+1)·w·h]`. Each byte is a tint value (0 = no
//!   ink, 255 = full tint) per the §8.6.6 model and §11.7.3
//!   "additive value of 1.0 (or subtractive tint value of 0.0)"
//!   resting-state rule.
//!
//! Spot names live in `spot_names`, ordered as `get_page_inks_deep`
//! returns them (sorted ASCII, deduped, with `/All` and `/None`
//! filtered out per §8.6.6.4).

use std::collections::HashMap;
use std::sync::Arc;

use crate::document::PdfDocument;
use crate::object::Object;
use crate::rendering::ext_gstate::{SoftMaskValue, parse_ext_g_state_inner};

/// Classification of a PDF blend-mode name into the three categories
/// §11.7.4.2 cares about.
///
/// Used by the compositor to decide whether the spot lanes should
/// honour the requested blend mode or substitute `/Normal`. Process
/// lanes always honour the requested mode regardless of class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlendModeClass {
    /// Separable AND white-preserving. ISO 32000-1 §11.3.5.2: the
    /// ten standard modes whose formula reduces to the source colour
    /// when the backdrop is white. Spot lanes apply the requested
    /// mode component-wise.
    ///
    /// Members: `/Normal`, `/Multiply`, `/Screen`, `/Overlay`,
    /// `/Darken`, `/Lighten`, `/ColorDodge`, `/ColorBurn`,
    /// `/HardLight`, `/SoftLight`.
    SeparableWhitePreserving,
    /// Separable but NOT white-preserving. ISO 32000-1 §11.3.5.2
    /// Note 2 names exactly two: `/Difference` and `/Exclusion`.
    /// Spot lanes substitute `/Normal` per §11.7.4.2.
    SeparableNonWhitePreserving,
    /// Non-separable. ISO 32000-1 §11.3.5.3 lists exactly four:
    /// `/Hue`, `/Saturation`, `/Color`, `/Luminosity`. Their formulas
    /// project to 3-component RGB; on a CMYK blend space the CMY
    /// channels run through the projection and the K channel follows
    /// the §11.3.5.3 rule (backdrop K for Hue/Saturation/Color,
    /// source K for Luminosity). Spot lanes substitute `/Normal` per
    /// §11.7.4.2.
    NonSeparable,
}

/// Process-lane dispatch under §11.7.4.2. The rule is one-line: the
/// process lanes always honour the requested blend mode. The enum
/// exists so the call site reads as "process_dispatch == UseRequested"
/// (single variant today) and round 2's wiring can match on it without
/// magic booleans.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessBlendDispatch {
    /// Run the requested PDF blend mode on the process lanes. For
    /// separable modes this is component-wise per §11.3.5.2; for
    /// non-separable modes this is the §11.3.5.3 RGB-projection with
    /// the K-channel rule for CMYK blend spaces.
    UseRequested,
}

/// Spot-lane dispatch under §11.7.4.2. Either "apply the requested
/// blend mode component-wise" (only when the BM is separable AND
/// white-preserving) or "substitute `/Normal`" (every other class).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpotBlendDispatch {
    /// Apply the requested blend mode to spot lanes component-wise.
    /// Reachable only when the BM is separable AND white-preserving.
    UseRequested,
    /// Substitute `/Normal` (source-over) on spot lanes regardless of
    /// the requested blend mode. The §11.7.4.2 rule: non-separable
    /// AND non-white-preserving modes have no defensible spot-lane
    /// behaviour, so the conforming reader paints spots as if the
    /// graphics state declared `/BM /Normal`.
    SubstituteNormal,
}

impl BlendModeClass {
    /// Classify a PDF blend-mode name into one of the three §11.7.4.2
    /// categories.
    ///
    /// Per ISO 32000-1 §11.6.3, an unknown blend mode name shall fall
    /// back to `/Normal`. We honour that by classifying unknown names
    /// as [`BlendModeClass::SeparableWhitePreserving`] — the same
    /// class `/Normal` itself belongs to. This matches the existing
    /// `pdf_blend_mode_to_skia` fallback in `src/rendering/mod.rs`.
    pub fn from_name(name: &str) -> Self {
        match name {
            // ISO 32000-1 §11.3.5.2: ten separable modes; all
            // white-preserving except Difference and Exclusion (Note 2). ~keep
            "Normal" | "Multiply" | "Screen" | "Overlay" | "Darken" | "Lighten" | "ColorDodge" | "ColorBurn"
            | "HardLight" | "SoftLight" => Self::SeparableWhitePreserving,
            "Difference" | "Exclusion" => Self::SeparableNonWhitePreserving,
            // ISO 32000-1 §11.3.5.3: four non-separable modes. ~keep
            "Hue" | "Saturation" | "Color" | "Luminosity" => Self::NonSeparable,
            // §11.6.3 fallback: unknown names render as /Normal. ~keep
            _ => Self::SeparableWhitePreserving,
        }
    }

    /// Process-lane dispatch decision. Always
    /// [`ProcessBlendDispatch::UseRequested`] per §11.7.4.2: "the
    /// current blend mode parameter … shall always apply to process
    /// colorants".
    pub fn process_dispatch(&self) -> ProcessBlendDispatch {
        ProcessBlendDispatch::UseRequested
    }

    /// Spot-lane dispatch decision per §11.7.4.2.
    pub fn spot_dispatch(&self) -> SpotBlendDispatch {
        match self {
            Self::SeparableWhitePreserving => SpotBlendDispatch::UseRequested,
            Self::SeparableNonWhitePreserving | Self::NonSeparable => SpotBlendDispatch::SubstituteNormal,
        }
    }
}

/// Per-page CMYK + spot-ink compositing sidecar.
///
/// Allocated once at the top of [`super::PageRenderer::render_page_with_options`]
/// when the page declares a CMYK `OutputIntent` and any
/// transparency / overprint trigger. The sidecar lives until the page
/// finishes rendering, then is dropped.
///
/// The CMYK plane is the §11.4 compositing buffer for the four
/// process channels (`DeviceCMYK` blend space). The spot planes are
/// the §11.7.3 sidecar — one byte per pixel per ink, blended
/// independently of the process channels.
///
/// Round 1 introduces the spot-plane storage and the page-level
/// discovery pre-pass; round 2 will wire per-paint-op writes from
/// `Separation` / `DeviceN` paint operators into the spot lanes.
#[derive(Debug)]
pub(crate) struct CmykSidecar {
    /// Pixmap dimensions `(width, height)`. Captured at allocation
    /// time and used for spot-plane indexing.
    dims: (u32, u32),
    /// Packed 4-byte-per-pixel `DeviceCMYK` plane in `(C, M, Y, K)`
    /// order, row-major, top-left origin. Length is `4 · w · h`.
    /// This is the round-4 layout preserved byte-for-byte so every
    /// existing process-lane helper continues to work unchanged.
    cmyk: Vec<u8>,
    /// Ordered names of every discovered spot ink. Order matches the
    /// `spots` plane stack: `spot_names[i]` is the colorant name of
    /// the plane at `spots[i·w·h .. (i+1)·w·h]`. Populated by the
    /// pre-pass via [`PdfDocument::get_page_inks_deep`] which sorts
    /// ASCII and dedups; `/All` and `/None` are filtered out by that
    /// helper per §8.6.6.4.
    spot_names: Vec<String>,
    /// Stack of per-ink tint planes. Length is `spot_names.len() · w
    /// · h`. Plane `i` lives at `spots[i·w·h .. (i+1)·w·h]`, one byte
    /// per pixel (0 = no ink, 255 = full tint). Initialised to zero
    /// per §11.7.3 ("an additive value of 1.0 or a subtractive tint
    /// value of 0.0 shall be assumed" for an unset component).
    spots: Vec<u8>,
}

impl CmykSidecar {
    /// Allocate the sidecar for a page of `(width, height)` pixels
    /// and the given set of spot ink names.
    ///
    /// The CMYK plane and every spot plane initialise to zero — the
    /// §11.7.3 subtractive resting state. The caller is responsible
    /// for driving the per-paint mirrors that update both the CMYK
    /// and spot lanes as the content stream renders.
    pub(crate) fn new(width: u32, height: u32, spot_names: Vec<String>) -> Self {
        let pixels = (width as usize) * (height as usize);
        let cmyk = vec![0u8; 4 * pixels];
        let spots = vec![0u8; spot_names.len() * pixels];
        Self {
            dims: (width, height),
            cmyk,
            spot_names,
            spots,
        }
    }

    pub(crate) fn image_buffer_bytes(&self) -> u64 {
        self.cmyk.capacity() as u64 + self.spots.capacity() as u64
    }

    /// Pixmap dimensions in `(width, height)` order.
    pub(crate) fn dims(&self) -> (u32, u32) {
        self.dims
    }

    /// Read-only slice over the packed `(C, M, Y, K)` plane.
    pub(crate) fn cmyk(&self) -> &[u8] {
        &self.cmyk
    }

    /// Mutable slice over the packed `(C, M, Y, K)` plane.
    pub(crate) fn cmyk_mut(&mut self) -> &mut [u8] {
        &mut self.cmyk
    }

    /// Ordered list of spot ink names. Empty when the page declares
    /// no `Separation` / non-process `DeviceN` colorants.
    pub(crate) fn spot_names(&self) -> &[String] {
        &self.spot_names
    }

    /// Read-only slice over the tint plane for spot ink `index`.
    /// Returns `None` when `index >= spot_count()`.
    pub(crate) fn spot_plane(&self, index: usize) -> Option<&[u8]> {
        let (w, h) = self.dims;
        let plane_size = (w as usize) * (h as usize);
        let start = index.checked_mul(plane_size)?;
        let end = start.checked_add(plane_size)?;
        if end > self.spots.len() {
            return None;
        }
        Some(&self.spots[start..end])
    }

    /// Mutable slice over the tint plane for spot ink `index`.
    /// Returns `None` when `index >= spot_count()`. The per-paint spot
    /// mirror writes through this accessor to compose new tints
    /// against the backdrop.
    pub(crate) fn spot_plane_mut(&mut self, index: usize) -> Option<&mut [u8]> {
        let (w, h) = self.dims;
        let plane_size = (w as usize) * (h as usize);
        let start = index.checked_mul(plane_size)?;
        let end = start.checked_add(plane_size)?;
        if end > self.spots.len() {
            return None;
        }
        Some(&mut self.spots[start..end])
    }

    /// Find the spot plane index for an ink name, or `None` when the
    /// name was not discovered on the page (the device has no plate
    /// for it per §8.6.6.3 — the composite path's alternate colour
    /// space then provides the approximation on the visible pixmap).
    pub(crate) fn spot_index(&self, ink: &str) -> Option<usize> {
        self.spot_names.iter().position(|n| n == ink)
    }

    /// Read-only view of every spot plane stacked end-to-end. Layout
    /// matches the internal `spots` buffer: plane `i` lives at
    /// `[i·w·h, (i+1)·w·h)`. Used by the SMask path to snapshot every
    /// spot lane before the paint mirror writes so the post-paint
    /// attenuation can blend `m·post + (1-m)·pre` per pixel per lane.
    pub(crate) fn spots_all(&self) -> &[u8] {
        &self.spots
    }

    /// Mutable counterpart of [`Self::spots_all`]. The SMask attenuation
    /// path writes the per-lane blend back through this slice.
    pub(crate) fn spots_all_mut(&mut self) -> &mut [u8] {
        &mut self.spots
    }

    /// Decompose one of the four `DeviceCMYK` process plates from the
    /// packed interleaved sidecar plane.
    ///
    /// ISO 32000-1 §10.5 (separated plate output) prescribes one
    /// grayscale plate per ink whose pixel value equals the subtractive
    /// tint of that ink at that pixel (0 = no ink, 255 = full tint).
    /// The composite-then-separate workflow §11.7.3 + §11.7.4.2 mandate
    /// arrives at the §10.5 plate by running the §11.4 compositing in
    /// the process blend space first, then extracting per-ink lanes
    /// from the composited buffer.
    ///
    /// `ink` is matched case-sensitively against the four process
    /// colorant names "Cyan" / "Magenta" / "Yellow" / "Black". Any
    /// other name returns `None`; spot inks go through
    /// [`Self::spot_plate`].
    ///
    /// Returns a fresh `Vec<u8>` (length `w · h`) because the storage
    /// layout interleaves the four process channels — the requested
    /// channel's pixels are not contiguous in memory and a slice cannot
    /// describe them. Callers wrap the buffer in their own per-plate
    /// surface type and the allocation cost is one pass over `4 · w · h`
    /// bytes regardless.
    pub(crate) fn process_plate(&self, ink: &str) -> Option<Vec<u8>> {
        let channel: usize = match ink {
            "Cyan" => 0,
            "Magenta" => 1,
            "Yellow" => 2,
            "Black" => 3,
            _ => return None,
        };
        let (w, h) = self.dims;
        let pixels = (w as usize) * (h as usize);
        let mut out = Vec::with_capacity(pixels);
        for px in 0..pixels {
            out.push(self.cmyk[px * 4 + channel]);
        }
        Some(out)
    }

    /// Borrow the spot tint plane for a named spot ink, or `None` when
    /// the ink was not in the active spot set surfaced by
    /// [`discover_page_spot_inks`].
    ///
    /// ISO 32000-1 §8.6.6.3: a `Separation` / `DeviceN` colorant for
    /// which the device has no plate falls back to the alternate
    /// colour-space approximation on the visible composite; the
    /// per-plate output (§10.5) drops the colorant. Returning `None`
    /// here lets the separation entry point allocate an all-zero plate
    /// per the spec's "no plate" semantic.
    ///
    /// Returns a borrowed slice (no allocation) because each spot
    /// plane is stored as a contiguous `w · h` byte block — see the
    /// layout note on [`Self`].
    pub(crate) fn spot_plate(&self, ink: &str) -> Option<&[u8]> {
        let idx = self.spot_index(ink)?;
        self.spot_plane(idx)
    }

    /// Overwrite the packed `(C, M, Y, K)` plane with `data`. Used by
    /// the knockout-group cumulative replay path to restore the
    /// group's initial backdrop state before composing each element so
    /// later paints compose against the backdrop rather than the
    /// accumulated paint from earlier elements
    /// (ISO 32000-1 §11.4.6.2).
    ///
    /// Panics if `data.len() != self.cmyk.len()`. The caller is the
    /// knockout-group replay which snapshots the exact buffer before
    /// the loop.
    pub(crate) fn restore_cmyk(&mut self, data: &[u8]) {
        debug_assert_eq!(data.len(), self.cmyk.len());
        self.cmyk.copy_from_slice(data);
    }

    /// Overwrite the spot plane stack with `data`. Companion to
    /// [`Self::restore_cmyk`] for the spot lanes inside a knockout
    /// group's cumulative replay. ISO 32000-1 §11.3.3 + §11.4.6.2:
    /// "a single shape value and opacity value shall be maintained at
    /// each point in the computed group results; they shall apply to
    /// both process and spot colour components" — so the knockout's
    /// "compose against backdrop" rule covers the spot lanes too,
    /// which means each replay iteration must start from the group's
    /// backdrop spot state, not the previously-composed state.
    ///
    /// Panics if `data.len() != self.spots.len()`.
    pub(crate) fn restore_spots(&mut self, data: &[u8]) {
        debug_assert_eq!(data.len(), self.spots.len());
        self.spots.copy_from_slice(data);
    }
}

/// Discover the set of `/Separation` and `/DeviceN` spot colorants
/// declared on `page_index` and within any nested Form XObject
/// `/Resources/ColorSpace` reached through `Do` operators in the
/// page's content stream.
///
/// Round 1 wraps [`PdfDocument::get_page_inks_deep`] so the sidecar's
/// spot set matches the spot set the separation renderer's per-plate
/// path already allocates. The walker filters `/All` and `/None` per
/// §8.6.6.4, sorts ASCII, and dedups. The result is stable across
/// renders of the same page.
///
/// Returns an empty vector when the page declares no spot colorants
/// (including the common case of a CMYK-only press job whose only
/// inks are the four process colorants Cyan / Magenta / Yellow /
/// Black). The four process inks are NOT surfaced here — they live
/// on the CMYK plane, not in the spot list.
///
/// # Error handling
///
/// On a parse error, malformed colorant array, or recursion-bound
/// trip from [`PdfDocument::get_page_inks_deep`], this function emits
/// a `tracing::warn!` naming the page and a stable error code, then
/// returns an empty vector. The render continues with degraded spot
/// fidelity (the sidecar allocates a zero-length spot stack and any
/// downstream paint-op writes that target spot lanes will find no
/// lane to write to — i.e. the spot ink quietly drops out of the
/// composite). This matches how the separation renderer handles the
/// same error (its per-plate path also degrades on a malformed
/// resource tree). The warning is the diagnostic signal that lets the
/// caller see the silent fidelity loss in a log scrape.
pub(crate) fn discover_page_spot_inks(doc: &PdfDocument, page_index: usize) -> Vec<String> {
    // get_page_inks_deep already enforces the §8.6.6.4 rules: filters
    // /All and /None, dedups, sorts. On error, surface via tracing::warn!
    // so the silent-degradation is visible to the host application's
    // trace pipeline — a silent unwrap_or_default would let the spot
    // lanes drop out of the composite without any signal. ~keep
    match doc.get_page_inks_deep(page_index) {
        Ok(inks) => inks,
        Err(error) => {
            tracing::warn!(
                page_index,
                error_code = error.telemetry_code(),
                error_offset = ?error.telemetry_offset(),
                "sidecar failed to discover spot inks; transparency composite will proceed with no spot lanes"
            );
            Vec::new()
        }
    }
}

/// Narrower variant of [`page_declares_transparency_or_overprint`]
/// that fires ONLY on transparency triggers (`/CA`, `/ca`, `/SMask`,
/// non-Normal `/BM`, `/Group`, XObject `/SMask`). Overprint flags
/// (`/OP`, `/op`) are intentionally NOT counted.
///
/// Used by the separation entry point to decide whether to route
/// through the composite-then-decompose path. The §11.4 transparency
/// model requires composite-first for correctness; the §11.7.4
/// overprint model is per-plate by definition (the per-plate walker
/// already implements OPM=0 / OPM=1 correctly), so routing pure-OP
/// pages through the composite path would either produce wrong plate
/// values (the page renderer's overprint handler is RGB-composite-
/// oriented, not per-plate) or require duplicating overprint logic
/// in the sidecar mirror. Drawing the line at "transparency only"
/// keeps the seam clean: detection-OFF and OP-only pages stay on the
/// per-plate walker; pages that mix transparency with overprint go
/// through composite-then-decompose where the §11.4 model evaluates
/// against the composite buffer.
pub(crate) fn page_declares_transparency(doc: &PdfDocument, resources: &Object) -> bool {
    let mut visited: std::collections::HashSet<crate::object::ObjectRef> = std::collections::HashSet::new();
    resources_declare_transparency_or_overprint(doc, resources, &mut visited, 0, false)
}

/// Reads each state through the shared ExtGState parser, so the sidecar and the
/// renderers agree on what a key means. Blend mode is compared with `"Normal"`
/// exactly as the page renderer's `non_trivial` and `transparent` tests do.
fn ext_g_states_signal(doc: &PdfDocument, ext_g_states: &HashMap<String, Object>, include_overprint: bool) -> bool {
    ext_g_states.values().any(|state| {
        let Ok(parsed) = parse_ext_g_state_inner(state, doc) else {
            return false;
        };
        parsed.fill_alpha.is_some_and(|a| a < 1.0)
            || parsed.stroke_alpha.is_some_and(|a| a < 1.0)
            || matches!(parsed.smask, Some(SoftMaskValue::Form(_)))
            || parsed.blend_mode.as_deref().is_some_and(|m| m != "Normal")
            || (include_overprint && (parsed.stroke_overprint == Some(true) || parsed.fill_overprint == Some(true)))
    })
}

fn ext_g_states_signal_transparency_only(doc: &PdfDocument, ext_g_states: &HashMap<String, Object>) -> bool {
    ext_g_states_signal(doc, ext_g_states, false)
}

/// Conservative detection: does this page declare any resource that
/// could drive transparency or overprint? Returns `true` when the
/// sidecar should be allocated for the page.
///
/// Detection criteria (matches the round-4 pre-pass):
///
///   * Any `ExtGState` in `/Resources/ExtGState` declares one of:
///     - `/OP true` or `/op true` (overprint)
///     - `/CA < 1.0` or `/ca < 1.0` (transparent paint)
///     - `/SMask` non-null (soft mask)
///     - `/BM` non-Normal (non-trivial blend mode)
///   * Any Form XObject in `/Resources/XObject` declares a `/Group`
///     dict (transparency group) or carries an `/SMask` entry.
///
/// The detection-OFF path is byte-identical to a sidecar-less render
/// because the sidecar-consuming helpers fall back to additive-clamp
/// inversion when the sidecar is `None`.
pub(crate) fn page_declares_transparency_or_overprint(doc: &PdfDocument, resources: &Object) -> bool {
    let mut visited: std::collections::HashSet<crate::object::ObjectRef> = std::collections::HashSet::new();
    resources_declare_transparency_or_overprint(doc, resources, &mut visited, 0, true)
}

/// Maximum form-XObject resource recursion depth used by the detection
/// helpers. Mirrors `MAX_FORM_XOBJECT_DEPTH` over in the renderer's
/// content-walker; bounds at well above any realistic legitimate
/// nesting so the depth cap is purely a backstop against adversarial
/// /Resources cycles that escape the `visited` set.
const MAX_DETECTION_RECURSION: u32 = 32;

fn resources_declare_transparency_or_overprint(
    doc: &PdfDocument,
    resources: &Object,
    visited: &mut std::collections::HashSet<crate::object::ObjectRef>,
    depth: u32,
    include_overprint: bool,
) -> bool {
    if depth >= MAX_DETECTION_RECURSION {
        return false;
    }
    let res_dict = match resources {
        Object::Dictionary(d) => d,
        _ => return false,
    };

    if let Some(ext_gs_obj) = res_dict.get("ExtGState")
        && let Ok(ext_gs_resolved) = doc.resolve_object(ext_gs_obj)
        && let Some(ext_g_states) = ext_gs_resolved.as_dict()
    {
        let hit = if include_overprint {
            ext_g_states_signal_transparency(doc, ext_g_states)
        } else {
            ext_g_states_signal_transparency_only(doc, ext_g_states)
        };
        if hit {
            return true;
        }
    }

    if let Some(xobj_obj) = res_dict.get("XObject")
        && let Ok(xobj_resolved) = doc.resolve_object(xobj_obj)
        && let Some(xobj_dict) = xobj_resolved.as_dict()
    {
        for raw in xobj_dict.values() {
            // Skip XObjects we've already inspected at this
            // scope: indirect refs are deduplicated by
            // ObjectRef. Inline streams cannot self-reference,
            // so the visited set only meaningfully tracks
            // refs. ~keep
            if let Some(r) = raw.as_reference()
                && !visited.insert(r)
            {
                continue;
            }
            let resolved = match doc.resolve_object(raw) {
                Ok(o) => o,
                Err(_) => continue,
            };
            let dict = match &resolved {
                Object::Stream { dict, .. } => Some(dict),
                _ => None,
            };
            let Some(dict) = dict else { continue };

            // §11.4.5 Form XObject: declaring its own /Group
            // dict — or carrying an /SMask entry — is a
            // direct transparency trigger. ~keep
            if dict.contains_key("Group") || dict.contains_key("SMask") {
                return true;
            }
            // §11.4.5 + §11.6.5.2: a Form XObject may also
            // declare its own /Resources/ExtGState whose
            // entries drive transparency from inside the
            // form. The renderer evaluates the form's content
            // under those state entries (§8.10.1), so they
            // must count toward sidecar allocation the same
            // way the page-level ExtGState does. Recurse on
            // the form's resources (or fall through to the
            // parent's when /Resources is absent). ~keep
            let form_res = match dict.get("Resources").map(|r| doc.resolve_object(r)) {
                Some(Ok(o)) => o,
                _ => continue,
            };
            if resources_declare_transparency_or_overprint(doc, &form_res, visited, depth + 1, include_overprint) {
                return true;
            }
        }
    }

    false
}

fn ext_g_states_signal_transparency(doc: &PdfDocument, ext_g_states: &HashMap<String, Object>) -> bool {
    ext_g_states_signal(doc, ext_g_states, true)
}

/// True when `name` is one of the standard blend-mode names ISO 32000-1
/// §11.3.5 enumerates (separable §11.3.5.2 or non-separable §11.3.5.3).
/// `/Normal` counts as recognised. Unknown names are NOT recognised and
/// trigger the §11.6.3 fallback at the call site.
pub(crate) fn is_recognised_mode(name: &str) -> bool {
    matches!(
        name,
        "Normal"
            | "Multiply"
            | "Screen"
            | "Overlay"
            | "Darken"
            | "Lighten"
            | "ColorDodge"
            | "ColorBurn"
            | "HardLight"
            | "SoftLight"
            | "Difference"
            | "Exclusion"
            | "Hue"
            | "Saturation"
            | "Color"
            | "Luminosity"
    )
}

/// Evaluate the §11.3.5.2 separable blend function `B(c_b, c_s)` for
/// one component. The PDF spec defines colour components as additive
/// values in `[0, 1]`. For SUBTRACTIVE-tint sidecar lanes (CMYK, spot),
/// the call site converts subtractive tint `t` to additive `1 - t`
/// before evaluating, then converts back. This helper does not do that
/// conversion — it operates on whatever component representation the
/// caller passes in, per ISO 32000-1 §11.3.5.2 Table 136.
///
/// Returns `c_s` unchanged when `mode` is not recognised (the §11.6.3
/// "unknown name → Normal" fallback), and returns `c_s` for `/Normal`.
///
/// Non-separable modes (`/Hue`, `/Saturation`, `/Color`, `/Luminosity`)
/// return `c_s` here because they cannot be evaluated component-wise —
/// the caller must dispatch on the BlendModeClass and route non-sep
/// modes through the §11.3.5.3 RGB projection helper. Spot lanes never
/// reach the non-sep formulas under §11.7.4.2 (the BM is substituted
/// to /Normal before this function is called) so the spot mirror's
/// non-sep return is unreachable in practice.
pub(crate) fn separable_blend(mode: &str, c_b: f32, c_s: f32) -> f32 {
    // ISO 32000-1 §11.3.5.2 Table 136. ~keep
    let c_b = c_b.clamp(0.0, 1.0);
    let c_s = c_s.clamp(0.0, 1.0);
    match mode {
        "Normal" => c_s,
        "Multiply" => c_b * c_s,
        "Screen" => c_b + c_s - c_b * c_s,
        "Overlay" => {
            // HardLight(c_s, c_b) — symmetric swap per Table 136. ~keep
            hard_light_component(c_s, c_b)
        }
        "Darken" => c_b.min(c_s),
        "Lighten" => c_b.max(c_s),
        "ColorDodge" => {
            if c_s >= 1.0 {
                1.0
            } else {
                (c_b / (1.0 - c_s)).min(1.0)
            }
        }
        "ColorBurn" => {
            if c_s <= 0.0 {
                0.0
            } else {
                1.0 - ((1.0 - c_b) / c_s).min(1.0)
            }
        }
        "HardLight" => hard_light_component(c_b, c_s),
        "SoftLight" => soft_light_component(c_b, c_s),
        "Difference" => (c_b - c_s).abs(),
        "Exclusion" => c_b + c_s - 2.0 * c_b * c_s,
        // §11.6.3 fallback: unknown / non-separable names render as
        // /Normal at the call site after dispatch routing. Returning
        // c_s here matches that policy if a caller reaches us with an
        // unexpected name. ~keep
        _ => c_s,
    }
}

fn hard_light_component(c_b: f32, c_s: f32) -> f32 {
    if c_s <= 0.5 {
        // Multiply(c_b, 2*c_s) ~keep
        c_b * 2.0 * c_s
    } else {
        // Screen(c_b, 2*c_s - 1) ~keep
        let twin = 2.0 * c_s - 1.0;
        c_b + twin - c_b * twin
    }
}

fn soft_light_component(c_b: f32, c_s: f32) -> f32 {
    // §11.3.5.2 Table 136 SoftLight: piecewise on c_s. ~keep
    if c_s <= 0.5 {
        c_b - (1.0 - 2.0 * c_s) * c_b * (1.0 - c_b)
    } else {
        let d = if c_b <= 0.25 {
            ((16.0 * c_b - 12.0) * c_b + 4.0) * c_b
        } else {
            c_b.sqrt()
        };
        c_b + (2.0 * c_s - 1.0) * (d - c_b)
    }
}

/// Extract the active spot ink names + tint values from a resolved
/// `Separation` / `DeviceN` colour-space array paired with the
/// operator's component values.
///
/// Per ISO 32000-1 §8.6.6.4 / §8.6.6.5:
/// - `Separation` arrays carry one colorant name and one tint. The
///   reserved names `/All` and `/None` are surfaced verbatim so the
///   §8.6.6.3 dispatch at the call site can branch on them.
/// - `DeviceN` arrays carry an N-name colorants array. If a `/Process`
///   attributes dict declares any of those names as process channels,
///   those names are filtered out here per §8.6.6.5 — they ride the
///   CMYK plane, not a spot lane.
///
/// Returns an empty vec when:
/// - the array is malformed (no type tag, no name array),
/// - the type tag is not `Separation` or `DeviceN`,
/// - the components count does not match the colorant count.
///
/// The returned ordering matches the source declaration order so the
/// caller can pair component-index N with colorant-index N.
pub(crate) fn extract_paint_spot_inks(space: &Object, components: &[f32], doc: &PdfDocument) -> Vec<(String, f32)> {
    let arr = match space.as_array() {
        Some(a) => a,
        None => return Vec::new(),
    };
    let type_name = match arr.first().and_then(Object::as_name) {
        Some(n) => n,
        None => return Vec::new(),
    };
    let deref = |obj: &Object| -> Object { doc.resolve_object(obj).unwrap_or_else(|_| obj.clone()) };

    match type_name {
        "Separation" => {
            if components.is_empty() {
                return Vec::new();
            }
            let name_obj = match arr.get(1) {
                Some(o) => deref(o),
                None => return Vec::new(),
            };
            let Some(ink) = name_obj.as_name() else {
                return Vec::new();
            };
            // /All and /None are surfaced verbatim; the call site
            // branches on them per §8.6.6.3 (paint every plate at the
            // tint, or skip every plate). ~keep
            vec![(ink.to_string(), components[0])]
        }
        "Pattern" => {
            // ISO 32000-1 §8.7.3.1: a Pattern colour space may declare
            // an underlying colour space at array index 1 (uncoloured
            // tiling pattern usage). The `scn` operator carries colour
            // components for the underlying space (before the pattern
            // name); a /Separation or /DeviceN underlying space brings
            // spot-colorant identity into the paint. The spot mirror
            // needs to walk into the underlying space so a paint
            // through a Pattern with a Separation alternate writes the
            // correct spot lane.
            //
            // The `components` slice carries the underlying space's
            // tints. For uncoloured Tiling, `name` (in SetFillColorN /
            // SetStrokeColorN) provides the pattern object, but the
            // tint is the underlying space's. For Shading patterns
            // (which use the /Shading object's own /ColorSpace), the
            // `scn` typically has no components — the underlying space
            // doesn't apply to shading patterns. We rely on the
            // recursive call's behaviour: a Shading-pattern usage with
            // no underlying space (array length 1) takes the
            // `arr.get(1)` branch as None and returns empty. ~keep
            let underlying = match arr.get(1) {
                Some(o) => deref(o),
                None => return Vec::new(),
            };
            // Recurse into the underlying space. The components passed
            // through unchanged — for an uncoloured Tiling pattern,
            // they are the underlying space's source tints. For
            // patterns whose underlying is itself an array form
            // (e.g. /Pattern [/Separation /PMS185 /DeviceCMYK
            // <tint>]), the recursive call handles the /Separation
            // arm and surfaces (PMS185, components[0]). ~keep
            extract_paint_spot_inks(&underlying, components, doc)
        }
        "DeviceN" => {
            let names_obj = match arr.get(1) {
                Some(o) => deref(o),
                None => return Vec::new(),
            };
            let Some(names) = names_obj.as_array() else {
                return Vec::new();
            };
            // ISO 32000-1 §8.6.6.5 / Table 73: the optional 5th element
            // is the attributes dictionary. When its `/Process`
            // sub-dictionary declares a `/Components` array, those
            // names are PROCESS colorants. Filter them out so the spot
            // lane mirror does not write spot lanes for /Cyan,
            // /Magenta, /Yellow, /Black on a /DeviceN /Process source.
            //
            // Round 5: when /Components contains any name not present
            // in /Names, the /Process attribution is malformed per
            // §8.6.6.5 ('leading prefix' requirement). Treat /Process
            // as inert in that case — no filtering — so the spot
            // extractor returns the same result it would for a DeviceN
            // without /Process attribution. This matches the
            // `extract_process_paint_cmyk` policy (which returns None
            // and falls through). HONEST_GAP_DEVICEN_PROCESS_MISMATCHED
            // _NAMES documents the open question. ~keep
            let process_names: std::collections::HashSet<String> = process_names_if_valid_prefix(arr, names, &deref);

            let mut out = Vec::with_capacity(names.len());
            for (i, ink_obj) in names.iter().enumerate() {
                let Some(ink) = ink_obj.as_name() else {
                    continue;
                };
                if ink == "All" || ink == "None" {
                    continue;
                }
                if process_names.contains(ink) {
                    continue;
                }
                // Pair the colorant with its index-matched component.
                // If components vector is short the source is malformed
                // — pin tint 0 (no ink) for the missing position. ~keep
                let tint = components.get(i).copied().unwrap_or(0.0);
                out.push((ink.to_string(), tint));
            }
            out
        }
        _ => Vec::new(),
    }
}

/// Return the set of /Process /Components names ONLY when /Components
/// is a valid leading-prefix subset of /Names (§8.6.6.5). When any
/// /Components name is absent from /Names the attribution is
/// malformed; round 5 treats it as inert and returns an empty set so
/// the spot extractor surfaces every /Names entry as a spot colorant
/// — matching the no-/Process behaviour and keeping the dispatcher's
/// later RGB-inverse fallback (`extract_process_paint_cmyk` also
/// returns None on mismatched names) symmetric.
fn process_names_if_valid_prefix(
    cs_arr: &[Object],
    names: &[Object],
    deref: &impl Fn(&Object) -> Object,
) -> std::collections::HashSet<String> {
    let proc_components = cs_arr
        .get(4)
        .map(deref)
        .as_ref()
        .and_then(Object::as_dict)
        .and_then(|attrs| attrs.get("Process"))
        .map(deref)
        .as_ref()
        .and_then(Object::as_dict)
        .and_then(|proc_dict| proc_dict.get("Components"))
        .map(deref)
        .as_ref()
        .and_then(Object::as_array)
        .map(|comps| {
            comps
                .iter()
                .filter_map(|o| o.as_name().map(str::to_string))
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();
    if proc_components.is_empty() {
        return std::collections::HashSet::new();
    }
    let names_set: std::collections::HashSet<String> =
        names.iter().filter_map(|o| o.as_name().map(str::to_string)).collect();
    if proc_components.iter().all(|c| names_set.contains(c)) {
        proc_components.into_iter().collect()
    } else {
        // Malformed /Process /Components: at least one name absent
        // from /Names. Treat /Process as inert per
        // HONEST_GAP_DEVICEN_PROCESS_MISMATCHED_NAMES. ~keep
        std::collections::HashSet::new()
    }
}

/// Process-colour reconstruction for a DeviceN paint that declares
/// `/Process` attribution (ISO 32000-1:2008 §8.6.6.5 / Table 71 + Table 72).
///
/// A DeviceN colour space may carry an `/Attributes` sub-dictionary
/// whose `/Process` entry routes a prefix of the source colorants
/// through a declared process colour space (`/DeviceCMYK`,
/// `/DeviceRGB`, `/DeviceGray`, or `/ICCBased`). For overprint /
/// transparency compositing, those process-attributed tints establish
/// the §11.7.4.3 source CMYK directly — the paint's tint transform
/// (which targets the DeviceN alternate space) is irrelevant for the
/// process attribution path because §8.6.6.5 explicitly states that
/// process components are "interpreted directly as process values by
/// consumers making use of the process dictionary".
///
/// Returns `Some((c, m, y, k))` when `space` is a `DeviceN` array with
/// a `/Process` attribute and the process colour space evaluates
/// successfully. Returns `None` for:
///  - non-`DeviceN` colour spaces (callers should handle Separation /
///    Device-family / ICC / CalGray / CalRGB explicitly),
///  - DeviceN without `/Process` attribution (the paint is a pure spot
///    paint; the process-side overprint rule is "preserve backdrop" per
///    Table 149 row 3, handled by the `SeparationOrDeviceN` class),
///  - DeviceN with a `/Process /ColorSpace` whose array form is neither
///    `/ICCBased` (N=1/3/4) nor `/Cal*` (the latter falls through to
///    the §10.3.5 RGB inverse). Real PDFs use the four device-family
///    names and `/ICCBased` overwhelmingly; the rare CalRGB/CalGray
///    cases keep the existing fallback path.
///  - DeviceN with a `/Process /Components` entry that is not present
///    in `/Names` (malformed source per §8.6.6.5; logged + None per
///    `HONEST_GAP_DEVICEN_PROCESS_MISMATCHED_NAMES`).
///
/// `/Process /ColorSpace [/ICCBased <stream>]` with N=4 takes the
/// source tints as destination CMYK directly per §8.6.6.5's "natural
/// form" wording. N=3 and N=1 follow the same shape as the named
/// `/DeviceRGB` / `/DeviceGray` arms (§10.3.5 inverse). The
/// alternate reading — round-tripping through the embedded profile's
/// CMM into sRGB and then back to destination CMYK via §10.3.5 — is
/// declined as lossy (it destroys K) and qcms 0.3.0 does not support
/// CMYK→CMYK transforms anyway. See
/// `HONEST_GAP_DEVICEN_PROCESS_ICC_PROFILE_MISMATCH` for the
/// embedded-vs-OutputIntent divergence question.
///
/// The component pairing follows §8.6.6.5: `/Process /Components`
/// entries map name-by-position to the channels of the process
/// colour space; each name's index in the parent `/Names` array picks
/// the source tint. This handles both the "all-process" case (every
/// colorant in /Names is in /Components, in canonical order) and the
/// "mixed" case (process prefix + spot tail, where the process
/// position in /Names need not be index 0 for a /DeviceN — only
/// /NChannel constrains the names to appear "sequentially").
pub(crate) fn extract_process_paint_cmyk(
    space: &Object,
    components: &[f32],
    doc: &PdfDocument,
    rendering_intent: crate::color::RenderingIntent,
    retarget_cache: Option<&crate::rendering::resolution::context::IccTransformCache>,
) -> Option<(f32, f32, f32, f32)> {
    let arr = space.as_array()?;
    if arr.first().and_then(Object::as_name)? != "DeviceN" {
        return None;
    }
    let deref = |obj: &Object| -> Object { doc.resolve_object(obj).unwrap_or_else(|_| obj.clone()) };

    // Parent /Names array — every colorant name appears here in source
    // declaration order. The source tints (`components`) index into
    // this array. ~keep
    let names_obj = deref(arr.get(1)?);
    let names = names_obj.as_array()?;
    let name_index = |target: &str| -> Option<usize> {
        names.iter().enumerate().find_map(|(i, o)| match o.as_name() {
            Some(n) if n == target => Some(i),
            _ => None,
        })
    };

    let attrs_obj = deref(arr.get(4)?);
    let attrs = attrs_obj.as_dict()?;
    let process_obj = deref(attrs.get("Process")?);
    let process = process_obj.as_dict()?;
    let cs_obj = deref(process.get("ColorSpace")?);
    let proc_components_obj = deref(process.get("Components")?);
    let proc_components = proc_components_obj.as_array()?;

    // Pull the source tint corresponding to each /Process /Components
    // entry by looking the name up in the parent /Names array.
    //
    // §8.6.6.5 mandates that /Components names appear in /Names as a
    // leading prefix; a name absent from /Names violates the spec and
    // is unspecified reader behaviour. Round 5 fails closed (returns
    // None, the call site falls through to the §10.3.5 RGB inverse)
    // and emits a warning so downstream tooling can flag the
    // malformed source. The matching gap constant is
    // HONEST_GAP_DEVICEN_PROCESS_MISMATCHED_NAMES in
    // `tests/test_46_round5_devicen_process_polish.rs`. ~keep
    let mut proc_tints: Vec<f32> = Vec::with_capacity(proc_components.len());
    for c in proc_components {
        let name = c.as_name()?;
        let Some(idx) = name_index(name) else {
            tracing::warn!(
                "DeviceN /Process /Components entry {:?} is not present in /Names; \
                 source violates ISO 32000-1 §8.6.6.5 ('leading prefix' requirement). \
                 Falling through to the §10.3.5 RGB-inverse path. See \
                 HONEST_GAP_DEVICEN_PROCESS_MISMATCHED_NAMES.",
                name
            );
            return None;
        };
        // Malformed sources with short component vectors pin missing
        // positions to 0 (no ink) — same conservative rule the spot
        // extractor uses. ~keep
        proc_tints.push(components.get(idx).copied().unwrap_or(0.0));
    }

    // Resolve the process /ColorSpace into a CMYK quadruple per
    // §10.3.5 / §8.6.4. Names may be a direct name (e.g. /DeviceCMYK)
    // or an array form (e.g. [/ICCBased <indirect-ref>]); handle the
    // four named device-family cases plus /ICCBased N=4 directly, and
    // route the rest to the caller's fallback. ~keep
    if let Some(name) = cs_obj.as_name() {
        return match name {
            "DeviceCMYK" | "CMYK" => {
                // §8.6.4.4: subtractive (c, m, y, k) — the source tints
                // ARE the source CMYK in their natural form per §8.6.6.5
                // ("values associated with the process components shall
                // be stored in their natural form"). ~keep
                if proc_tints.len() < 4 {
                    return None;
                }
                Some((proc_tints[0], proc_tints[1], proc_tints[2], proc_tints[3]))
            }
            "DeviceRGB" | "RGB" => {
                // §10.3.5 additive-clamp inverse: C = 1-R, M = 1-G,
                // Y = 1-B, K = 0. Per §8.6.6.5 the process tints are
                // stored in their natural (additive) form for RGB,
                // matching §10.3.5's input convention. ~keep
                if proc_tints.len() < 3 {
                    return None;
                }
                let c = (1.0 - proc_tints[0]).clamp(0.0, 1.0);
                let m = (1.0 - proc_tints[1]).clamp(0.0, 1.0);
                let y = (1.0 - proc_tints[2]).clamp(0.0, 1.0);
                Some((c, m, y, 0.0))
            }
            "DeviceGray" | "G" => {
                // Gray → CMYK convention used by every device-space arm
                // in the renderer: K = 1 − g, C = M = Y = 0. ~keep
                if proc_tints.is_empty() {
                    return None;
                }
                let k = (1.0 - proc_tints[0]).clamp(0.0, 1.0);
                Some((0.0, 0.0, 0.0, k))
            }
            _ => None,
        };
    }

    // Array-form /Process /ColorSpace. /ICCBased is the case round 4
    // explicitly deferred (HONEST_GAP_DEVICEN_PROCESS_ICC_OVERPRINT).
    // Round 5 wires the ICCBased N=4 path: per §8.6.6.5, the process
    // tints are stored "in their natural form" — for an ICCBased CMYK
    // (N=4) process colour space the tints are subtractive CMYK in
    // the profile's CMYK space. The §11.7.4.3 dispatcher consumes
    // those tints under Table 149 row 2 ("any other process colour
    // space"). The natural-form reading preserves K and matches the
    // common production case where the embedded process profile IS
    // the document OutputIntent profile.
    //
    // The alternate reading — round-tripping through sRGB via the
    // embedded profile to recover destination CMYK via §10.3.5 —
    // destroys K and only fires when the embedded profile genuinely
    // differs from the OutputIntent. qcms 0.3.0 does not support
    // CMYK→CMYK transforms (CMYK→RGB only), so a profile-to-profile
    // retargeting is not currently available through the linked
    // CMM. HONEST_GAP_DEVICEN_PROCESS_ICC_PROFILE_MISMATCH names the
    // open question.
    //
    // N=3 (ICCBased RGB) and N=1 (ICCBased Gray) process colour
    // spaces follow the analogous device-family paths: tints in the
    // profile's source space are converted by §10.3.5 (R→C=1-R for
    // N=3; G→K=1-G for N=1). The embedded profile's tone-curve
    // adjustments are NOT applied because the round-5 reading
    // accepts tints as natural-form — exactly the spec text. This is
    // the same simplification the named /DeviceRGB and /DeviceGray
    // arms make above. ~keep
    if let Some(cs_arr) = cs_obj.as_array()
        && cs_arr.first().and_then(Object::as_name) == Some("ICCBased")
    {
        let n_components = cs_arr
            .get(1)
            .map(deref)
            .as_ref()
            .and_then(Object::as_dict)
            .and_then(|d| d.get("N"))
            .and_then(Object::as_integer)
            .unwrap_or(0);
        return match n_components {
            4 => {
                if proc_tints.len() < 4 {
                    return None;
                }
                // Round 7 ICC retargeting: when the active CMM
                // backend supports CMYK→CMYK retargeting AND the
                // embedded profile is genuinely different from the
                // document OutputIntent profile, retarget the
                // source tints through the destination profile's
                // BToA. The result is the same colour the press
                // (the OutputIntent's modelled press) would produce
                // for the source paint, with BPC applied for the
                // relative-colorimetric press default.
                //
                // Falls through to the round-5 "natural form"
                // reading when:
                //   - the backend can't do CMYK→CMYK (qcms 0.3),
                //   - no OutputIntent CMYK profile is declared,
                //   - the embedded profile parses but the
                //     destination profile fails to parse,
                //   - the two profiles compile to byte-identical
                //     bytes (same press, same paint — no
                //     conversion needed).
                //
                // See HONEST_GAP_DEVICEN_PROCESS_ICC_PROFILE_MISMATCH
                // for the three-state matrix. ~keep
                if let Some(retargeted) =
                    try_retarget_cmyk_via_embedded_profile(cs_arr, &proc_tints, doc, rendering_intent, retarget_cache)
                {
                    return Some(retargeted);
                }
                Some((proc_tints[0], proc_tints[1], proc_tints[2], proc_tints[3]))
            }
            3 => {
                if proc_tints.len() < 3 {
                    return None;
                }
                let c = (1.0 - proc_tints[0]).clamp(0.0, 1.0);
                let m = (1.0 - proc_tints[1]).clamp(0.0, 1.0);
                let y = (1.0 - proc_tints[2]).clamp(0.0, 1.0);
                Some((c, m, y, 0.0))
            }
            1 => {
                if proc_tints.is_empty() {
                    return None;
                }
                let k = (1.0 - proc_tints[0]).clamp(0.0, 1.0);
                Some((0.0, 0.0, 0.0, k))
            }
            _ => None,
        };
    }

    // CalRGB / CalGray / other array-form. These are uncommon
    // in DeviceN /Process attribution; routing them through the
    // proper colour transform is out of scope. The call site falls
    // back to the §10.3.5 inverse from the rasterised RGB. ~keep
    None
}

/// Closes `HONEST_GAP_DEVICEN_PROCESS_ICC_PROFILE_MISMATCH` for the
/// embedded /Process /ColorSpace [/ICCBased N=4] case.
///
/// Parses both the embedded process profile (from the /ICCBased
/// stream in `cs_arr`) and the document OutputIntent CMYK profile,
/// then runs the source tints through a `CmykRetargetTransform`
/// (which lcms2 builds as CMYK → Lab PCS → CMYK with BPC on for the
/// press default). The returned tuple is the destination-CMYK colour
/// the press would produce.
///
/// Returns `None` (so the caller falls back to the round-5 natural-
/// form reading) when:
///   - the active backend can't compile a CMYK→CMYK transform
///     (qcms 0.3 baseline — no CMYK output path),
///   - the document declares no OutputIntent CMYK profile,
///   - either profile fails to parse / cross-check the /N entry,
///   - the embedded profile's bytes match the OutputIntent profile's
///     bytes (identity retarget — no conversion needed).
///
/// The three-state HONEST_GAP_DEVICEN_PROCESS_ICC_PROFILE_MISMATCH
/// matrix in `tests/test_46_round5_devicen_process_polish.rs`
/// documents which state each (backend, profile-mismatch) tuple
/// resolves to.
fn try_retarget_cmyk_via_embedded_profile(
    cs_arr: &[Object],
    proc_tints: &[f32],
    doc: &PdfDocument,
    rendering_intent: crate::color::RenderingIntent,
    retarget_cache: Option<&crate::rendering::resolution::context::IccTransformCache>,
) -> Option<(f32, f32, f32, f32)> {
    if !crate::color::active_backend_supports_cmyk_retarget() {
        return None;
    }
    if proc_tints.len() < 4 {
        return None;
    }

    // The destination profile MUST come from the document
    // OutputIntents. Without it there's no defined target gamut to
    // retarget into, and the natural-form reading is the only
    // sensible fallback. `doc.output_intent_cmyk_profile()` already
    // performs the §14.11.5 lookup (first /GTS_PDFX or /GTS_PDFA
    // entry with a /N=4 /DestOutputProfile) and parses it through
    // IccProfile::parse, so we get a vetted Arc back. ~keep
    let dst_profile = doc.output_intent_cmyk_profile()?;

    // The embedded /Process /ColorSpace [/ICCBased N 0 R] stream
    // is at index 1 of cs_arr. Resolve the indirect reference,
    // decode the stream bytes, parse through IccProfile::parse
    // (which cross-checks N=4 against the ICC header CMYK
    // signature). ~keep
    let stream_obj = cs_arr.get(1)?;
    let resolved_stream = doc.resolve_object(stream_obj).ok()?;
    let dict = resolved_stream.as_dict()?;
    let declared_n: u8 = dict
        .get("N")
        .and_then(Object::as_integer)
        .filter(|n| *n == 4)
        .map(|n| n as u8)?;
    let bytes = resolved_stream.decode_stream_data().ok()?;
    let src_profile = std::sync::Arc::new(crate::color::IccProfile::parse(bytes, declared_n)?);

    // Identity retarget — both profiles are byte-identical, so any
    // transform we built would round-trip the input through Lab and
    // produce essentially the same bytes back (the natural-form
    // reading IS the identity retarget on byte-identical profiles).
    // Skip the transform-build cost and emit the natural form. ~keep
    if src_profile.content_hash() == dst_profile.content_hash() {
        return None;
    }

    // §10.7.3: the live `ri` operator (and any prior /RI ExtGState
    // entry) declares the rendering intent for the operator that
    // follows. The dispatcher reads `gs.rendering_intent` at paint
    // time and threads it here through `extract_process_paint_cmyk`,
    // so a `/Perceptual ri` before a /DeviceN /Process /ICCBased
    // paint retargets with the perceptual BToA tag. §8.6.5.8 pins
    // `RelativeColorimetric` as the fallback when the gs intent is
    // unset or unrecognised — that mapping is in
    // `RenderingIntent::from_pdf_name`, applied at the call site
    // before threading into here. BPC stays on for the press
    // default `TransformFlags::press_default()`.
    // Look up (or build, on miss) the compiled CMYK→CMYK retarget
    // transform through the per-renderer cache when available. Without
    // the cache, every paint re-parses both ICC profiles AND rebuilds
    // the lcms2 CLUT — for a page with thousands of process-attributed
    // DeviceN paints this is the dominant render cost. With the cache
    // the build runs once per unique (src, dst, intent) tuple and
    // every subsequent paint is a single `Arc<…>` clone. The
    // no-cache path stays around for non-rendering callers (e.g.
    // initial-colour evaluation in colour-space setup). ~keep
    let transform: Arc<crate::color::CmykRetargetTransform> = match retarget_cache {
        Some(cache) => cache.get_or_build_cmyk_retarget(&src_profile, &dst_profile, rendering_intent)?,
        None => Arc::new(crate::color::CmykRetargetTransform::new(
            src_profile,
            dst_profile,
            rendering_intent,
        )?),
    };
    let out = transform.retarget_pixel([
        proc_tints[0].clamp(0.0, 1.0),
        proc_tints[1].clamp(0.0, 1.0),
        proc_tints[2].clamp(0.0, 1.0),
        proc_tints[3].clamp(0.0, 1.0),
    ]);
    Some((
        out[0].clamp(0.0, 1.0),
        out[1].clamp(0.0, 1.0),
        out[2].clamp(0.0, 1.0),
        out[3].clamp(0.0, 1.0),
    ))
}

/// Initial colour values for a colour space per ISO 32000-1 §8.6.8
/// ("The `CS`/`cs` operator shall also set the current colour to its
/// initial value, which depends on the colour space").
///
/// Carries every field a paint operator's downstream state cares about:
/// the raw component vector, the derived RGB triple (used by the
/// rasteriser for the default colour fallback), and the spot-ink
/// identity (Separation / non-process DeviceN).
pub(crate) struct InitialColour {
    /// The §8.6.8 component vector for the new space.
    pub components: Vec<f32>,
    /// The derived (r, g, b) triple stored on `fill_color_rgb` /
    /// `stroke_color_rgb` so the rasteriser has a default RGB even
    /// before an explicit `scn` lands.
    pub rgb: (f32, f32, f32),
    /// `Some(cmyk)` only when the new space is DeviceCMYK; cleared
    /// otherwise so a stale prior CMYK identity does not leak into
    /// overprint / compose-first paths.
    pub cmyk: Option<(f32, f32, f32, f32)>,
    /// Spot identity for the new space. For /Separation this is a
    /// single entry at the spec's initial tint 1.0; for non-process
    /// /DeviceN it is one entry per non-process colorant, each at
    /// tint 1.0. Every other space clears the spot identity.
    pub spot_inks: Vec<(String, f32)>,
}

/// Compute the per-§8.6.8 initial colour state for the colour space
/// named `space_name`. `resolved_space` is the `Object` resolved from
/// the page's `/Resources/ColorSpace` subdictionary (or `None` for the
/// inline device-family names DeviceGray / DeviceRGB / DeviceCMYK /
/// Pattern that never appear in the resource dict).
///
/// Spec text (ISO 32000-1 §8.6.8):
///  - DeviceGray / CalGray / Indexed: 0.0
///  - DeviceRGB / CalRGB / Lab: (0, 0, 0)
///  - DeviceCMYK: (0, 0, 0, 1)  (pure black)
///  - ICCBased: all-zeros unless clamped to /Range
///  - Separation: tint 1.0  (§8.6.6.4 explicitly: "The initial value
///    for both the stroking and nonstroking colour in the graphics
///    state shall be 1.0.")
///  - DeviceN: tint 1.0 per colorant
///  - Pattern: a nothing-painted pattern object (we represent this as
///    an empty component vector — the rasteriser already treats the
///    Pattern space as "no fill" until an `scn` lands)
pub(crate) fn initial_colour_for_space(
    space_name: &str,
    resolved_space: Option<&Object>,
    doc: &PdfDocument,
    rendering_intent: crate::color::RenderingIntent,
    retarget_cache: Option<&crate::rendering::resolution::context::IccTransformCache>,
) -> InitialColour {
    let deref = |obj: &Object| -> Object { doc.resolve_object(obj).unwrap_or_else(|_| obj.clone()) };

    match space_name {
        "DeviceGray" | "G" | "CalGray" => {
            return InitialColour {
                components: vec![0.0],
                rgb: (0.0, 0.0, 0.0),
                cmyk: None,
                spot_inks: Vec::new(),
            };
        }
        "DeviceRGB" | "RGB" | "CalRGB" => {
            return InitialColour {
                components: vec![0.0, 0.0, 0.0],
                rgb: (0.0, 0.0, 0.0),
                cmyk: None,
                spot_inks: Vec::new(),
            };
        }
        "DeviceCMYK" | "CMYK" => {
            // Initial CMYK is (0, 0, 0, 1) — pure black per §8.6.8. ~keep
            let (r, g, b) = (0.0_f32, 0.0_f32, 0.0_f32);
            return InitialColour {
                components: vec![0.0, 0.0, 0.0, 1.0],
                rgb: (r, g, b),
                cmyk: Some((0.0, 0.0, 0.0, 1.0)),
                spot_inks: Vec::new(),
            };
        }
        "Pattern" => {
            return InitialColour {
                components: Vec::new(),
                rgb: (0.0, 0.0, 0.0),
                cmyk: None,
                spot_inks: Vec::new(),
            };
        }
        _ => {}
    }

    let arr = match resolved_space.and_then(Object::as_array) {
        Some(a) => a,
        None => {
            return InitialColour {
                components: Vec::new(),
                rgb: (0.0, 0.0, 0.0),
                cmyk: None,
                spot_inks: Vec::new(),
            };
        }
    };
    let type_name = arr.first().and_then(Object::as_name).unwrap_or("");
    match type_name {
        "CalGray" => InitialColour {
            components: vec![0.0],
            rgb: (0.0, 0.0, 0.0),
            cmyk: None,
            spot_inks: Vec::new(),
        },
        "CalRGB" | "Lab" => InitialColour {
            components: vec![0.0, 0.0, 0.0],
            rgb: (0.0, 0.0, 0.0),
            cmyk: None,
            spot_inks: Vec::new(),
        },
        "ICCBased" => {
            let n = arr
                .get(1)
                .map(deref)
                .as_ref()
                .and_then(Object::as_dict)
                .and_then(|d| d.get("N"))
                .and_then(Object::as_integer)
                .unwrap_or(3);
            // §8.6.8: ICCBased initial colour is all-zeros unless the
            // /Range entry clamps. We assume 0.0 is in-range (the
            // common case); a custom /Range that excludes 0 is rare
            // and the rasteriser will clamp downstream anyway. ~keep
            //
            // `/N` is document-declared and otherwise unbounded (`as_integer`
            // returns an i64): `project_iccbased_to_rgb` in page_renderer.rs
            // already clamps it to `0..=255` before trusting it
            // (`n.clamp(0, 255) as u8`) rather than letting a single `/N
            // 999999999999` dictionary entry size a `Vec<f32>` allocation. ~keep
            let components = vec![0.0_f32; n.clamp(1, 255) as usize];
            let cmyk = if n == 4 { Some((0.0, 0.0, 0.0, 0.0)) } else { None };
            InitialColour {
                components,
                rgb: (0.0, 0.0, 0.0),
                cmyk,
                spot_inks: Vec::new(),
            }
        }
        "Indexed" => InitialColour {
            components: vec![0.0],
            rgb: (0.0, 0.0, 0.0),
            cmyk: None,
            spot_inks: Vec::new(),
        },
        "Separation" => {
            // §8.6.8 + §8.6.6.4: initial tint 1.0 for the colorant. ~keep
            let name_obj = arr.get(1).map(deref);
            let ink = name_obj
                .as_ref()
                .and_then(Object::as_name)
                .map(str::to_string)
                .unwrap_or_default();
            let spot_inks = if !ink.is_empty() && ink != "All" && ink != "None" {
                vec![(ink, 1.0)]
            } else {
                // /All and /None branch in §8.6.6.3; both are handled
                // at paint time, not via spot identity. ~keep
                Vec::new()
            };
            InitialColour {
                components: vec![1.0],
                rgb: (0.0, 0.0, 0.0),
                cmyk: None,
                spot_inks,
            }
        }
        "DeviceN" => {
            let names_obj = arr.get(1).map(deref);
            let names = names_obj
                .as_ref()
                .and_then(Object::as_array)
                .map(|names| {
                    names
                        .iter()
                        .filter_map(|o| o.as_name().map(str::to_string))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            // Filter /Process channels from the spot set, same as the
            // paint-time extractor does. ~keep
            let process_names: std::collections::HashSet<String> = arr
                .get(4)
                .map(deref)
                .as_ref()
                .and_then(Object::as_dict)
                .and_then(|attrs| attrs.get("Process"))
                .map(deref)
                .as_ref()
                .and_then(Object::as_dict)
                .and_then(|proc_dict| proc_dict.get("Components"))
                .map(deref)
                .as_ref()
                .and_then(Object::as_array)
                .map(|comps| comps.iter().filter_map(|o| o.as_name().map(str::to_string)).collect())
                .unwrap_or_default();
            let spot_inks: Vec<(String, f32)> = names
                .iter()
                .filter(|n| n.as_str() != "All" && n.as_str() != "None")
                .filter(|n| !process_names.contains(*n))
                .map(|n| (n.clone(), 1.0_f32))
                .collect();
            // §8.6.8: initial tint is 1.0 for every colorant. ~keep
            let components = vec![1.0_f32; names.len().max(1)];
            // §8.6.6.5 + §11.7.4.3: when /Process attribution is
            // declared, the initial-tint vector feeds the process
            // /ColorSpace exactly like an `scn` would. The overprint
            // dispatcher reads `cmyk` for the §11.7.4.3 source CMYK;
            // without this population the initial-colour CMYK would
            // be lost (the call site would fall through to the
            // §10.3.5 RGB inverse from `fill_color_rgb = (0,0,0)`,
            // producing source CMYK (1, 1, 1, 0) — K dropped). Run
            // the same evaluator the paint-time path uses so the
            // mapping (named device families + ICCBased N=1/3/4) is
            // identical to the post-`scn` behaviour. ~keep
            let cmyk = extract_process_paint_cmyk(
                resolved_space.unwrap(),
                &components,
                doc,
                rendering_intent,
                retarget_cache,
            );
            InitialColour {
                components,
                rgb: (0.0, 0.0, 0.0),
                cmyk,
                spot_inks,
            }
        }
        "Pattern" => InitialColour {
            components: Vec::new(),
            rgb: (0.0, 0.0, 0.0),
            cmyk: None,
            spot_inks: Vec::new(),
        },
        _ => InitialColour {
            components: Vec::new(),
            rgb: (0.0, 0.0, 0.0),
            cmyk: None,
            spot_inks: Vec::new(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_normal_is_separable_white_preserving() {
        assert_eq!(
            BlendModeClass::from_name("Normal"),
            BlendModeClass::SeparableWhitePreserving
        );
    }

    #[test]
    fn classify_luminosity_is_non_separable() {
        assert_eq!(BlendModeClass::from_name("Luminosity"), BlendModeClass::NonSeparable);
    }

    #[test]
    fn classify_difference_is_separable_non_white_preserving() {
        assert_eq!(
            BlendModeClass::from_name("Difference"),
            BlendModeClass::SeparableNonWhitePreserving
        );
    }

    #[test]
    fn classify_unknown_falls_back_to_normal_class() {
        // ISO 32000-1 §11.6.3: unknown blend mode names render as
        // /Normal. The classifier reflects that by returning the same
        // class /Normal itself belongs to. ~keep
        assert_eq!(
            BlendModeClass::from_name("MarketingInventedMode"),
            BlendModeClass::SeparableWhitePreserving
        );
    }

    #[test]
    fn spot_dispatch_substitutes_normal_for_non_sep_and_non_wp() {
        // §11.7.4.2: only separable AND white-preserving modes apply
        // to spot lanes; every other class substitutes /Normal. ~keep
        assert_eq!(
            BlendModeClass::SeparableWhitePreserving.spot_dispatch(),
            SpotBlendDispatch::UseRequested
        );
        assert_eq!(
            BlendModeClass::SeparableNonWhitePreserving.spot_dispatch(),
            SpotBlendDispatch::SubstituteNormal
        );
        assert_eq!(
            BlendModeClass::NonSeparable.spot_dispatch(),
            SpotBlendDispatch::SubstituteNormal
        );
    }

    #[test]
    fn process_dispatch_is_identity_for_every_class() {
        // §11.7.4.2: process lanes always honour the requested BM. ~keep
        for class in &[
            BlendModeClass::SeparableWhitePreserving,
            BlendModeClass::SeparableNonWhitePreserving,
            BlendModeClass::NonSeparable,
        ] {
            assert_eq!(class.process_dispatch(), ProcessBlendDispatch::UseRequested);
        }
    }

    /// `/N` on an `ICCBased` colour-space dictionary is document-declared
    /// and otherwise unbounded; `initial_colour_for_space` used to size a
    /// `Vec<f32>` directly from it (`vec![0.0_f32; n.max(1) as usize]`).
    ///
    /// Before the fix: a declared `/N i64::MAX` allocates
    /// `i64::MAX * size_of::<f32>()` bytes, which panics with "capacity
    /// overflow" (the byte count exceeds `isize::MAX`) instead of being
    /// rejected as malformed input. After the fix, `/N` is clamped to
    /// `1..=255` before it ever reaches the allocation.
    #[test]
    fn iccbased_initial_colour_clamps_huge_declared_n() {
        let doc = crate::rendering::resolution::test_support::fixture_doc();

        let mut icc_dict = HashMap::new();
        icc_dict.insert("N".to_string(), Object::Integer(i64::MAX));
        let resolved_space = Object::Array(vec![Object::Name("ICCBased".to_string()), Object::Dictionary(icc_dict)]);

        let initial = initial_colour_for_space(
            "ICCBased",
            Some(&resolved_space),
            &doc,
            crate::color::RenderingIntent::default(),
            None,
        );

        assert!(
            initial.components.len() <= 255,
            "declared /N must be clamped rather than trusted directly: got {} components",
            initial.components.len()
        );
    }

    #[test]
    fn sidecar_allocates_cmyk_and_spot_planes() {
        let s = CmykSidecar::new(10, 5, vec!["PMS 185 C".into(), "Dieline".into()]);
        assert_eq!(s.dims(), (10, 5));
        assert_eq!(s.cmyk().len(), 4 * 10 * 5);
        assert!(s.cmyk().iter().all(|&b| b == 0));
        assert_eq!(s.spot_names(), &["PMS 185 C".to_string(), "Dieline".to_string()]);
        let p0 = s.spot_plane(0).unwrap();
        let p1 = s.spot_plane(1).unwrap();
        assert_eq!(p0.len(), 10 * 5);
        assert_eq!(p1.len(), 10 * 5);
        assert!(p0.iter().all(|&b| b == 0) && p1.iter().all(|&b| b == 0));
        assert!(s.spot_plane(2).is_none());
    }

    #[test]
    fn sidecar_no_spots_has_zero_length_spot_stack() {
        let s = CmykSidecar::new(7, 3, vec![]);
        assert_eq!(s.dims(), (7, 3));
        assert_eq!(s.cmyk().len(), 4 * 7 * 3);
        assert!(s.spot_names().is_empty());
        assert!(s.spot_plane(0).is_none());
    }

    /// `process_plate` decomposes the four `DeviceCMYK` channels from
    /// the interleaved `(C, M, Y, K)` plane. ISO 32000-1 §10.5: the
    /// plate's pixel value equals the subtractive tint of that ink at
    /// the pixel. Probe pins per-channel extraction with a synthetic
    /// interleaved fill.
    #[test]
    fn sidecar_process_plate_extracts_named_channel() {
        let mut s = CmykSidecar::new(2, 2, vec![]);
        // Pixel 0: C=10, M=20, Y=30, K=40
        // Pixel 1: C=50, M=60, Y=70, K=80
        // Pixel 2: C=90, M=100, Y=110, K=120
        // Pixel 3: C=130, M=140, Y=150, K=160 ~keep
        let plane = s.cmyk_mut();
        for (i, v) in plane.iter_mut().enumerate() {
            *v = (i + 10) as u8;
        }
        assert_eq!(
            s.process_plate("Cyan").unwrap(),
            vec![10, 14, 18, 22],
            "Cyan = byte 0 of every interleaved quad starting at 10, +4 per pixel"
        );
        assert_eq!(s.process_plate("Magenta").unwrap(), vec![11, 15, 19, 23]);
        assert_eq!(s.process_plate("Yellow").unwrap(), vec![12, 16, 20, 24]);
        assert_eq!(s.process_plate("Black").unwrap(), vec![13, 17, 21, 25]);
        // Unknown / spot name returns None — spot inks go through
        // spot_plate. ~keep
        assert!(s.process_plate("PANTONE 185 C").is_none());
        assert!(s.process_plate("cyan").is_none(), "case-sensitive");
    }

    /// `spot_plate` borrows the requested spot lane by name. Returns
    /// `None` when the ink was not in the discovered spot set.
    #[test]
    fn sidecar_spot_plate_returns_named_lane() {
        let mut s = CmykSidecar::new(3, 1, vec!["InkA".into(), "InkB".into()]);
        let plane_a = s.spot_plane_mut(0).unwrap();
        plane_a.copy_from_slice(&[10, 20, 30]);
        let plane_b = s.spot_plane_mut(1).unwrap();
        plane_b.copy_from_slice(&[40, 50, 60]);
        assert_eq!(s.spot_plate("InkA").unwrap(), &[10, 20, 30]);
        assert_eq!(s.spot_plate("InkB").unwrap(), &[40, 50, 60]);
        // Not-discovered → None (the §8.6.6.3 "no plate" semantic at
        // the caller). ~keep
        assert!(s.spot_plate("InkC").is_none());
    }

    /// `restore_cmyk` and `restore_spots` overwrite the sidecar's
    /// process and spot buffers. Used by the knockout-group cumulative
    /// replay to reset lane state to the group's backdrop between
    /// element compositions (ISO 32000-1 §11.4.6.2).
    #[test]
    fn sidecar_restore_cmyk_and_spots_overwrites_buffers() {
        let mut s = CmykSidecar::new(2, 1, vec!["InkA".into()]);
        s.cmyk_mut().copy_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
        s.spot_plane_mut(0).unwrap().copy_from_slice(&[9, 10]);
        let backdrop_cmyk = vec![100u8; 8];
        let backdrop_spots = vec![50u8; 2];
        s.restore_cmyk(&backdrop_cmyk);
        s.restore_spots(&backdrop_spots);
        assert_eq!(s.cmyk(), backdrop_cmyk.as_slice());
        assert_eq!(s.spots_all(), backdrop_spots.as_slice());
    }

    /// A test-only `tracing_subscriber::Layer` that captures every
    /// WARN-or-above event's stable identity and fields.
    ///
    /// Installed per-call via `tracing::subscriber::with_default`, which
    /// scopes the subscriber to the current thread for the duration of a
    /// closure — unlike a process-global, idempotent-once logger install,
    /// this needs no shared `OnceLock` gate and no snapshot-the-prior-length
    /// dance: each test gets its own fresh buffer. ~keep
    struct CapturingLayer {
        buf: std::sync::Arc<std::sync::Mutex<Vec<CapturedEvent>>>,
    }

    #[derive(Clone, Debug)]
    struct CapturedEvent {
        level: tracing::Level,
        target: String,
        fields: std::collections::BTreeMap<String, String>,
    }

    impl<S> tracing_subscriber::Layer<S> for CapturingLayer
    where
        S: tracing::Subscriber,
    {
        fn on_event(&self, event: &tracing::Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
            if *event.metadata().level() > tracing::Level::WARN {
                return;
            }
            let mut visitor = FieldVisitor::default();
            event.record(&mut visitor);
            self.buf.lock().unwrap().push(CapturedEvent {
                level: *event.metadata().level(),
                target: event.metadata().target().to_string(),
                fields: visitor.fields,
            });
        }
    }

    #[derive(Default)]
    struct FieldVisitor {
        fields: std::collections::BTreeMap<String, String>,
    }

    impl tracing::field::Visit for FieldVisitor {
        fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
            self.fields.insert(field.name().to_string(), format!("{value:?}"));
        }

        fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
            self.fields.insert(field.name().to_string(), value.to_string());
        }

        fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
            self.fields.insert(field.name().to_string(), value.to_string());
        }
    }

    /// Round-1 QA — surface, don't swallow, the deep-walk error.
    ///
    /// `discover_page_spot_inks` previously called
    /// `get_page_inks_deep(...).unwrap_or_default()`, silently mapping
    /// every error to an empty vec. A page that genuinely has spots
    /// but whose deep walk trips (parse error, recursion bound, page
    /// lookup miss) would then allocate a zero-length spot stack — and
    /// any downstream paint-op writes to those lanes would quietly
    /// drop on the floor.
    ///
    /// The fix emits `tracing::warn!` on the error path AND returns the
    /// empty vec (matching how the separation renderer handles the
    /// same `get_page_inks_deep` failure). This probe pins both halves
    /// of the contract: empty-vec return, AND a warn record surfaces.
    #[test]
    fn discover_page_spot_inks_warns_on_deep_walk_error() {
        use tracing_subscriber::layer::SubscriberExt as _;

        let buf: std::sync::Arc<std::sync::Mutex<Vec<CapturedEvent>>> =
            std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let subscriber = tracing_subscriber::registry().with(CapturingLayer { buf: buf.clone() });

        // Single-page synthetic PDF. We will then ask for page 42 — out
        // of range — so `get_page_inks_deep` returns Err on the page
        // tree walk. ~keep
        let pdf = b"%PDF-1.4\n\
                    1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
                    2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
                    3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>\nendobj\n\
                    xref\n0 4\n\
                    0000000000 65535 f \n\
                    0000000010 00000 n \n\
                    0000000059 00000 n \n\
                    0000000110 00000 n \n\
                    trailer\n<< /Size 4 /Root 1 0 R >>\nstartxref\n175\n%%EOF\n"
            .to_vec();
        let doc = PdfDocument::from_bytes(pdf).expect("synthetic PDF parses");

        let spots = tracing::subscriber::with_default(subscriber, || discover_page_spot_inks(&doc, 42));
        assert!(
            spots.is_empty(),
            "discover_page_spot_inks must return an empty vec on \
             deep-walk error (not panic, not propagate); got {:?}",
            spots
        );

        let all_records = buf.lock().unwrap().clone();
        let records: Vec<_> = all_records
            .iter()
            .filter(|event| event.target == format!("{}::rendering::sidecar", crate::LOG_TARGET_ROOT))
            .collect();
        assert_eq!(
            records.len(),
            1,
            "expected exactly one sidecar warning: {all_records:?}"
        );
        assert_eq!(records[0].level, tracing::Level::WARN);
        assert_eq!(
            records[0].target,
            format!("{}::rendering::sidecar", crate::LOG_TARGET_ROOT)
        );
        assert_eq!(records[0].fields.get("page_index").map(String::as_str), Some("42"));
        assert_eq!(
            records[0].fields.get("error_code").map(String::as_str),
            Some("invalid_pdf")
        );
        assert!(!records[0].fields.contains_key("error"));
    }

    /// Round 5 / B2: `extract_paint_spot_inks` for a Pattern colour
    /// space with a /Separation underlying. The Pattern array form is
    /// `[/Pattern <underlying-cs>]`; the underlying may be any colour
    /// space (uncoloured Tiling). When the underlying is a /Separation
    /// or /DeviceN with a spot colorant, the spot identity MUST
    /// propagate to the dispatcher so the spot mirror writes the
    /// correct lane.
    ///
    /// Spec citations:
    ///  - §8.7.3.1 — Pattern colour space (uncoloured Tiling carries
    ///    the underlying colour space's tints)
    ///  - §8.6.6.3 — /Separation spot identity
    ///  - §11.7.3   — single shape/opacity per pixel across lanes
    #[test]
    fn extract_paint_spot_inks_pattern_with_separation_underlying() {
        // Build the colour-space object: [/Pattern [/Separation
        // /PMS185 /DeviceCMYK <stub tint fn>]]. The stub tint fn is a
        // bare dict — the extractor does not consult it; the
        // dispatcher only reads /Separation's index-1 name and uses
        // the components vector for the tint. ~keep
        let tint_fn = Object::Dictionary(
            [
                ("FunctionType".to_string(), Object::Integer(2)),
                (
                    "Domain".to_string(),
                    Object::Array(vec![Object::Integer(0), Object::Integer(1)]),
                ),
                ("C0".to_string(), Object::Array(vec![Object::Integer(0); 4])),
                ("C1".to_string(), Object::Array(vec![Object::Integer(1); 4])),
                ("N".to_string(), Object::Integer(1)),
            ]
            .into_iter()
            .collect(),
        );
        let underlying = Object::Array(vec![
            Object::Name("Separation".to_string()),
            Object::Name("PMS185".to_string()),
            Object::Name("DeviceCMYK".to_string()),
            tint_fn,
        ]);
        let pattern_cs = Object::Array(vec![Object::Name("Pattern".to_string()), underlying]);

        // Minimal PDF for the doc context. The extractor only calls
        // resolve_object on indirect refs; the inline objects above
        // need no resolution. ~keep
        let pdf: Vec<u8> = b"%PDF-1.4\n\
                             1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
                             2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
                             3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>\nendobj\n\
                             xref\n0 4\n\
                             0000000000 65535 f \n\
                             0000000010 00000 n \n\
                             0000000059 00000 n \n\
                             0000000110 00000 n \n\
                             trailer\n<< /Size 4 /Root 1 0 R >>\nstartxref\n175\n%%EOF\n"
            .to_vec();
        let doc = PdfDocument::from_bytes(pdf).expect("synthetic PDF parses");

        // Components: the underlying /Separation expects one tint. ~keep
        let components = [0.6_f32];
        let spots = extract_paint_spot_inks(&pattern_cs, &components, &doc);

        assert_eq!(
            spots.len(),
            1,
            "ISO 32000-1 §8.7.3.1: Pattern[/Separation /PMS185 …] must \
             surface PMS185 via the underlying-space recursion. Got \
             {} entries; expected 1.",
            spots.len()
        );
        assert_eq!(spots[0].0, "PMS185", "spot identity propagation");
        assert_eq!(spots[0].1, 0.6_f32, "spot tint propagation (0.6_f32 is exact in f32)");
    }

    /// Round 5 / A5: the `process_names_if_valid_prefix` helper
    /// returns the /Components set ONLY when every name appears in
    /// /Names; otherwise it returns empty (treating the /Process
    /// attribution as inert per
    /// HONEST_GAP_DEVICEN_PROCESS_MISMATCHED_NAMES). Probe pins both
    /// arms.
    #[test]
    fn process_names_if_valid_prefix_returns_set_for_valid_prefix() {
        let deref = |o: &Object| -> Object { o.clone() };
        let names = vec![
            Object::Name("Cyan".to_string()),
            Object::Name("Magenta".to_string()),
            Object::Name("Yellow".to_string()),
            Object::Name("Black".to_string()),
            Object::Name("PMS185".to_string()),
        ];
        let attrs = Object::Dictionary(
            [(
                "Process".to_string(),
                Object::Dictionary(
                    [
                        ("ColorSpace".to_string(), Object::Name("DeviceCMYK".to_string())),
                        (
                            "Components".to_string(),
                            Object::Array(vec![
                                Object::Name("Cyan".to_string()),
                                Object::Name("Magenta".to_string()),
                                Object::Name("Yellow".to_string()),
                                Object::Name("Black".to_string()),
                            ]),
                        ),
                    ]
                    .into_iter()
                    .collect(),
                ),
            )]
            .into_iter()
            .collect(),
        );
        let cs_arr = vec![
            Object::Name("DeviceN".to_string()),
            Object::Array(names.clone()),
            Object::Name("DeviceCMYK".to_string()),
            // tint transform placeholder ~keep
            Object::Null,
            attrs,
        ];
        let result = process_names_if_valid_prefix(&cs_arr, &names, &deref);
        let expected: std::collections::HashSet<String> = ["Cyan", "Magenta", "Yellow", "Black"]
            .into_iter()
            .map(str::to_string)
            .collect();
        assert_eq!(result, expected, "valid prefix returns the /Components set");
    }

    #[test]
    fn process_names_if_valid_prefix_returns_empty_for_invalid_prefix() {
        let deref = |o: &Object| -> Object { o.clone() };
        let names = vec![
            Object::Name("Cyan".to_string()),
            Object::Name("Magenta".to_string()),
            Object::Name("Yellow".to_string()),
            Object::Name("Black".to_string()),
        ];
        let attrs = Object::Dictionary(
            [(
                "Process".to_string(),
                Object::Dictionary(
                    [
                        ("ColorSpace".to_string(), Object::Name("DeviceCMYK".to_string())),
                        (
                            "Components".to_string(),
                            Object::Array(vec![
                                Object::Name("Cyan".to_string()),
                                Object::Name("Magenta".to_string()),
                                Object::Name("Yellow".to_string()),
                                // /Iridescent NOT in /Names → malformed ~keep
                                Object::Name("Iridescent".to_string()),
                            ]),
                        ),
                    ]
                    .into_iter()
                    .collect(),
                ),
            )]
            .into_iter()
            .collect(),
        );
        let cs_arr = vec![
            Object::Name("DeviceN".to_string()),
            Object::Array(names.clone()),
            Object::Name("DeviceCMYK".to_string()),
            Object::Null,
            attrs,
        ];
        let result = process_names_if_valid_prefix(&cs_arr, &names, &deref);
        assert!(
            result.is_empty(),
            "ISO 32000-1 §8.6.6.5 violation (one name not in /Names) \
             must return empty per HONEST_GAP_DEVICEN_PROCESS_MISMATCHED\
             _NAMES. Got {:?}.",
            result
        );
    }

    // ============================================================
    // Detection-helper indirect-ref + nested-form regressions (M3).
    // ============================================================
    //
    // `page_declares_transparency_or_overprint` /
    // `page_declares_transparency` previously read `/CA /ca /SMask /BM`
    // straight off the ExtGState dict and only inspected the page-
    // level resource scope. Two PDF shapes silently routed through
    // the per-plate walker:
    //
    //   1. ExtGState whose `/CA /ca /BM` value is an indirect
    //      reference (the resolved name / number triggers transparency
    //      but the raw Reference variant fell through the `match` to
    //      `_ => 1.0` / unrecognised mode).
    //   2. Form XObject whose own `/Resources/ExtGState` declares a
    //      transparent entry, with the page-level ExtGState empty.
    //
    // The probes below construct minimal synthetic PDFs that
    // surface each case and assert the detection helper now returns
    // `true`. Sensitivity verification: stash the corresponding fix
    // → assertion flips to false. ~keep

    /// Build a single-page PDF whose page-level Resources dict carries
    /// the literal text in `resources_inner` (e.g.
    /// `"/ExtGState << /T << /Type /ExtGState /ca 6 0 R >> >>"`) and
    /// whose object table includes the verbatim `extra_objs` after the
    /// page-content stream. Returns the parsed `PdfDocument` and the
    /// page's `/Resources` dictionary so callers can hand both to
    /// `page_declares_transparency_*`.
    fn build_doc_with_resources_and_objs(resources_inner: &str, extra_objs: &[&str]) -> (PdfDocument, Object) {
        let mut buf: Vec<u8> = Vec::new();
        buf.extend_from_slice(b"%PDF-1.4\n");
        let cat_off = buf.len();
        buf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
        let pages_off = buf.len();
        buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
        let page_off = buf.len();
        let page = format!(
            "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
             /Resources << {} >> /Contents 4 0 R >>\nendobj\n",
            resources_inner
        );
        buf.extend_from_slice(page.as_bytes());
        let stream_off = buf.len();
        let body = b"% no content\n";
        let stream_hdr = format!("4 0 obj\n<< /Length {} >>\nstream\n", body.len());
        buf.extend_from_slice(stream_hdr.as_bytes());
        buf.extend_from_slice(body);
        buf.extend_from_slice(b"\nendstream\nendobj\n");

        let mut extra_offs: Vec<usize> = Vec::new();
        for obj in extra_objs {
            extra_offs.push(buf.len());
            buf.extend_from_slice(obj.as_bytes());
        }

        let xref_off = buf.len();
        let total_objs = 4 + extra_objs.len();
        buf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", total_objs + 1).as_bytes());
        for off in [cat_off, pages_off, page_off, stream_off] {
            buf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
        }
        for off in extra_offs {
            buf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
        }
        buf.extend_from_slice(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
                total_objs + 1,
                xref_off
            )
            .as_bytes(),
        );

        let doc = PdfDocument::from_bytes(buf).expect("synthetic PDF parses");
        let resources = doc.get_page_resources(0).expect("page resources");
        (doc, resources)
    }

    #[test]
    fn detection_resolves_indirect_ca() {
        // `/ca 6 0 R` where 6 0 obj is `Real(0.6)`. Pre-fix: the
        // `match v` arm on `Object::Reference` fell to `_ => 1.0`,
        // alpha stayed 1.0, the helper missed the trigger. ~keep
        let resources_inner = "/ExtGState << /T << /Type /ExtGState /ca 6 0 R >> >>";
        let extras = ["6 0 obj\n0.6\nendobj\n"];
        let (doc, resources) = build_doc_with_resources_and_objs(resources_inner, &extras);
        assert!(
            page_declares_transparency_or_overprint(&doc, &resources),
            "page_declares_transparency_or_overprint must dereference \
             `/ca 6 0 R` and recognise the resolved Real(0.6) < 1.0 \
             as transparent."
        );
        assert!(
            page_declares_transparency(&doc, &resources),
            "page_declares_transparency must dereference `/ca 6 0 R` \
             and recognise the resolved Real(0.6) < 1.0 as transparent."
        );
    }

    #[test]
    fn detection_resolves_indirect_ca_upper() {
        // /CA mirror of /ca. ~keep
        let resources_inner = "/ExtGState << /T << /Type /ExtGState /CA 6 0 R >> >>";
        let extras = ["6 0 obj\n0.7\nendobj\n"];
        let (doc, resources) = build_doc_with_resources_and_objs(resources_inner, &extras);
        assert!(
            page_declares_transparency_or_overprint(&doc, &resources),
            "page_declares_transparency_or_overprint must dereference \
             `/CA 6 0 R` and recognise the resolved Real(0.7) < 1.0 \
             as transparent."
        );
    }

    #[test]
    fn detection_resolves_indirect_bm() {
        // `/BM 6 0 R` where 6 0 obj is `Name("Multiply")`. Pre-fix:
        // `bm_is_non_normal` matched against `Object::Reference` and
        // returned `false`, missing the trigger. ~keep
        let resources_inner = "/ExtGState << /T << /Type /ExtGState /BM 6 0 R >> >>";
        let extras = ["6 0 obj\n/Multiply\nendobj\n"];
        let (doc, resources) = build_doc_with_resources_and_objs(resources_inner, &extras);
        assert!(
            page_declares_transparency_or_overprint(&doc, &resources),
            "page_declares_transparency_or_overprint must dereference \
             `/BM 6 0 R` and recognise the resolved /Multiply name as \
             non-/Normal."
        );
    }

    #[test]
    fn detection_recurses_into_form_xobject_extgstate() {
        // Form XObject (object 6) whose own /Resources/ExtGState
        // declares a transparent state (/ca 0.6). Page-level
        // ExtGState is empty. Pre-fix: the XObject loop checked only
        // /Group and /SMask on the form dict, missing the nested
        // transparency entirely. ~keep
        let form_obj = "6 0 obj\n\
            << /Type /XObject /Subtype /Form /FormType 1 \
               /BBox [0 0 100 100] \
               /Resources << /ExtGState << /Half << /Type /ExtGState /ca 0.6 >> >> >> \
               /Length 14 >>\n\
            stream\n% no paint\n\nendstream\nendobj\n";
        let resources_inner = "/XObject << /F 6 0 R >>";
        let (doc, resources) = build_doc_with_resources_and_objs(resources_inner, &[form_obj]);
        assert!(
            page_declares_transparency_or_overprint(&doc, &resources),
            "page_declares_transparency_or_overprint must recurse into \
             Form-XObject /Resources/ExtGState. The form's /Half \
             ExtGState declares /ca 0.6; the page must route through \
             composite-then-decompose."
        );
        assert!(
            page_declares_transparency(&doc, &resources),
            "narrower page_declares_transparency must also recurse \
             into nested-form ExtGState."
        );
    }

    #[test]
    fn detection_no_trigger_returns_false() {
        // Sanity: a page with neither ExtGState nor XObject still
        // reports false (no regressions from the recursion shape). ~keep
        let resources_inner = "/ColorSpace << /CS [/Separation /InkA /DeviceCMYK << >>] >>";
        let (doc, resources) = build_doc_with_resources_and_objs(resources_inner, &[]);
        assert!(
            !page_declares_transparency_or_overprint(&doc, &resources),
            "no ExtGState or XObject → no transparency / overprint trigger."
        );
        assert!(
            !page_declares_transparency(&doc, &resources),
            "no ExtGState or XObject → no transparency-only trigger."
        );
    }

    // Characterization of the ExtGState transparency decision. Each case
    // states the (transparency-only, transparency-or-overprint) result.

    fn detect(state_body: &str, extras: &[&str]) -> (bool, bool) {
        let inner = format!("/ExtGState << /T << /Type /ExtGState {state_body} >> >>");
        let (doc, resources) = build_doc_with_resources_and_objs(&inner, extras);
        (
            page_declares_transparency(&doc, &resources),
            page_declares_transparency_or_overprint(&doc, &resources),
        )
    }

    #[test]
    fn detect_opacity_below_one_fires() {
        assert_eq!(detect("/ca 0.5", &[]), (true, true));
        assert_eq!(detect("/CA 0", &[]), (true, true));
        assert_eq!(detect("/CA -0.5", &[]), (true, true));
    }

    #[test]
    fn detect_opaque_or_out_of_range_high_or_missing_opacity_does_not_fire() {
        assert_eq!(detect("/ca 1.0", &[]), (false, false));
        assert_eq!(detect("/CA 2.5", &[]), (false, false));
        assert_eq!(detect("", &[]), (false, false));
        assert_eq!(detect("/ca /Half", &[]), (false, false));
    }

    #[test]
    fn detect_nan_opacity_does_not_fire() {
        let doc = build_doc_with_resources_and_objs("", &[]).0;
        for key in ["CA", "ca"] {
            let state = Object::Dictionary(HashMap::from([(key.to_string(), Object::Real(f64::NAN))]));
            let states = HashMap::from([("T".to_string(), state)]);
            assert!(!ext_g_states_signal_transparency_only(&doc, &states), "{key}");
            assert!(!ext_g_states_signal_transparency(&doc, &states), "{key}");
        }
    }

    #[test]
    fn detect_state_behind_indirect_reference() {
        let inner = "/ExtGState << /T 6 0 R >>";
        let extras = ["6 0 obj\n<< /Type /ExtGState /ca 0.4 >>\nendobj\n"];
        let (doc, resources) = build_doc_with_resources_and_objs(inner, &extras);
        assert!(page_declares_transparency(&doc, &resources));
        assert!(page_declares_transparency_or_overprint(&doc, &resources));
    }

    #[test]
    fn detect_smask_none_does_not_fire() {
        assert_eq!(detect("/SMask /None", &[]), (false, false));
    }

    #[test]
    fn detect_smask_none_behind_reference_does_not_fire() {
        assert_eq!(detect("/SMask 6 0 R", &["6 0 obj\n/None\nendobj\n"]), (false, false));
    }

    #[test]
    fn detect_smask_dict_behind_reference_fires() {
        // Positive control for the test above: the same object number resolves to a
        // soft-mask dict, so the reference is followed.
        let dict = "6 0 obj\n<< /S /Alpha /G 7 0 R >>\nendobj\n";
        assert_eq!(detect("/SMask 6 0 R", &[dict]), (true, true));
    }

    #[test]
    fn detect_smask_dict_with_form_reference_fires() {
        assert_eq!(detect("/SMask << /S /Alpha /G 6 0 R >>", &[]), (true, true));
    }

    #[test]
    fn detect_smask_dict_without_form_reference() {
        // Shared-parser rule (ext_gstate.rs parse_smask): a soft-mask dict
        // without a /G reference parses to no mask, as it does for the renderers.
        assert_eq!(detect("/SMask << /S /Alpha >>", &[]), (false, false));
    }

    #[test]
    fn detect_smask_dict_with_direct_form_value() {
        // Shared-parser rule (ext_gstate.rs parse_smask): /G must be a
        // reference; anything else parses to no mask.
        assert_eq!(detect("/SMask << /S /Alpha /G 7 >>", &[]), (false, false));
    }

    #[test]
    fn detect_smask_non_name_non_dict() {
        // Shared-parser rule (ext_gstate.rs parse_smask): only /None and a
        // dict with /G classify; any other value leaves the mask untouched.
        assert_eq!(detect("/SMask 3", &[]), (false, false));
    }

    #[test]
    fn detect_smask_unresolvable_reference() {
        // Shared-parser rule (ext_gstate.rs parse_smask): a reference
        // that does not resolve is not a mask.
        assert_eq!(detect("/SMask 99 0 R", &[]), (false, false));
    }

    #[test]
    fn detect_blend_mode_names() {
        assert_eq!(detect("/BM /Multiply", &[]), (true, true));
        assert_eq!(detect("/BM /Normal", &[]), (false, false));
    }

    #[test]
    fn detect_blend_mode_unrecognised_name() {
        // Shared-parser rule (ext_gstate.rs parse_blend_mode): a name that is not a
        // standard blend mode, /Compatible included, reads as /Normal, as in the
        // array form.
        assert_eq!(detect("/BM /NoSuchMode", &[]), (false, false));
        assert_eq!(detect("/BM /Compatible", &[]), (false, false));
    }

    #[test]
    fn detect_blend_mode_array_takes_first_recognised_name() {
        assert_eq!(detect("/BM [/NoSuchMode /Multiply]", &[]), (true, true));
        assert_eq!(detect("/BM [/NoSuchMode /Normal /Multiply]", &[]), (false, false));
        assert_eq!(detect("/BM [/NoSuchMode]", &[]), (false, false));
    }

    #[test]
    fn detect_blend_mode_array_with_reference_element() {
        // Shared-parser rule (ext_gstate.rs parse_blend_mode): array elements are
        // resolved before the first recognised name is picked.
        assert_eq!(detect("/BM [6 0 R]", &["6 0 obj\n/Multiply\nendobj\n"]), (true, true));
    }

    #[test]
    fn detect_blend_mode_unresolvable_reference_does_not_fire() {
        assert_eq!(detect("/BM 99 0 R", &[]), (false, false));
    }

    #[test]
    fn detect_overprint_counts_only_for_the_overprint_variant() {
        assert_eq!(detect("/OP true", &[]), (false, true));
        assert_eq!(detect("/op true", &[]), (false, true));
        assert_eq!(detect("/OP false /op true", &[]), (false, true));
        assert_eq!(detect("/OP true /op false", &[]), (false, true));
        assert_eq!(detect("/OP false /op false", &[]), (false, false));
        assert_eq!(detect("/OP 6 0 R", &["6 0 obj\ntrue\nendobj\n"]), (false, true));
    }

    fn opaque_state() -> Object {
        Object::Dictionary(HashMap::from([("ca".to_string(), Object::Real(1.0))]))
    }

    fn transparent_state() -> Object {
        Object::Dictionary(HashMap::from([("ca".to_string(), Object::Real(0.5))]))
    }

    /// A document whose object 4 has an xref offset past the end of the file, so loading it
    /// fails. Returns the document and a reference to that object, which the shared parser refuses.
    fn doc_with_unreadable_state() -> (PdfDocument, Object) {
        let objects = [
            "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n",
            "2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n",
            "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> >>\nendobj\n",
            "4 0 obj\n<< /Type /ExtGState /ca 0.5 >>\nendobj\n",
        ];
        let mut buf = b"%PDF-1.4\n".to_vec();
        let mut offsets = Vec::new();
        for object in objects {
            offsets.push(buf.len());
            buf.extend_from_slice(object.as_bytes());
        }
        let unreadable = buf.len() + 100_000;
        let xref_offset = buf.len();
        buf.extend_from_slice(b"xref\n0 5\n0000000000 65535 f \n");
        for offset in [offsets[0], offsets[1], offsets[2], unreadable] {
            buf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        buf.extend_from_slice(
            format!("trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n").as_bytes(),
        );
        let doc = PdfDocument::from_bytes(buf).expect("synthetic PDF parses");
        let state = Object::Reference(crate::object::ObjectRef { id: 4, generation: 0 });
        assert!(
            parse_ext_g_state_inner(&state, &doc).is_err(),
            "fixture must fail to parse"
        );
        (doc, state)
    }

    #[test]
    fn detect_state_that_fails_to_parse_does_not_fire() {
        let (doc, broken) = doc_with_unreadable_state();
        let states = HashMap::from([("T".to_string(), broken.clone()), ("U".to_string(), opaque_state())]);
        assert!(!ext_g_states_signal_transparency_only(&doc, &states));
        assert!(!ext_g_states_signal_transparency(&doc, &states));
        let with_transparent = HashMap::from([("T".to_string(), broken), ("U".to_string(), transparent_state())]);
        assert!(ext_g_states_signal_transparency_only(&doc, &with_transparent));
    }

    #[test]
    fn detect_one_transparent_state_among_opaque_ones_fires() {
        let doc = build_doc_with_resources_and_objs("", &[]).0;
        let states = HashMap::from([
            ("A".to_string(), opaque_state()),
            ("B".to_string(), transparent_state()),
            ("C".to_string(), opaque_state()),
        ]);
        assert!(ext_g_states_signal_transparency_only(&doc, &states));
        assert!(ext_g_states_signal_transparency(&doc, &states));
        let all_opaque = HashMap::from([("A".to_string(), opaque_state()), ("C".to_string(), opaque_state())]);
        assert!(!ext_g_states_signal_transparency_only(&doc, &all_opaque));
    }
}
