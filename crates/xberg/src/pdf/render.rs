//! PDF page rendering using xberg_native_pdf.

use crate::Result;
use crate::core::diagnostics::{push_warning_deduped, warning};
use crate::error::XbergError;
use crate::types::ProcessingWarning;
use std::cell::RefCell;
use std::sync::Once;
use std::sync::atomic::{AtomicBool, Ordering};

/// `ProcessingWarning::source` used for glyphs the rasterizer could not paint.
///
/// See [`take_xberg_native_pdf_render_warnings`] for why this exists and where the
/// gap is upstream vs. xberg-side.
const PDF_RENDER_WARNING_SOURCE: &str = "pdf-render";

#[cfg(any(feature = "ocr", feature = "ocr-pipeline", feature = "layout-detection"))]
const ROTATED_PNG_ENCODE_BYTES_PER_PIXEL: u64 = 4;
#[cfg(any(feature = "ocr", feature = "ocr-pipeline", feature = "layout-detection"))]
const ROTATED_PNG_ENCODE_FIXED_BYTES: u64 = 256 * 1024;

thread_local! {
    /// Buffer for `xberg_native_pdf`'s `tracing::warn!` records emitted while a render
    /// call made by this thread is in flight. `None` when no render call is
    /// currently capturing (the default, and the state between calls).
    static ENGINE_LOG_CAPTURE: RefCell<Option<Vec<EngineWarning>>> = const { RefCell::new(None) };
    /// Deduped warnings drained from completed render calls on this thread,
    /// awaiting collection by [`take_xberg_native_pdf_render_warnings`].
    static ENGINE_PENDING_WARNINGS: RefCell<Vec<ProcessingWarning>> = const { RefCell::new(Vec::new()) };
}

static ENGINE_LOGGER_INIT: Once = Once::new();

/// Whether [`install_pdf_render_diagnostics`] actually won the process's
/// global `tracing` [`Subscriber`](tracing::Subscriber) slot. Capture is
/// skipped entirely while this is false, so the render path costs nothing for
/// the overwhelming majority of embedders who never opt in.
static ENGINE_CAPTURE_ACTIVE: AtomicBool = AtomicBool::new(false);

/// A [`tracing::Subscriber`] that captures `xberg_native_pdf`'s warning-level events
/// into the calling thread's [`ENGINE_LOG_CAPTURE`] buffer.
///
/// # Why this exists (#1364)
///
/// `xberg_native_pdf`'s rasterizer can silently drop a glyph — no font resolves for
/// the current run (`text_rasterizer.rs`: "No font found for '{}'..."),
/// parsing an embedded font fails (`page_renderer.rs`'s `load_resources` logs the
/// sanitized static message "rendering text with fallback font data", carrying the
/// actual diagnosis in structured tracing fields instead), the CJK predefined-CIDFont
/// substitution face is unavailable, or
/// direct CID/CFF glyph-outline rendering errors mid-run. In every one of
/// those cases `xberg_native_pdf` still returns `Ok(RenderedImage { .. })` — the
/// page just has a gap where the glyph should be, with the text-space cursor
/// advanced as if it painted. `RenderedImage` carries no diagnostic field, so
/// none of this is visible to callers through the return value.
///
/// `xberg_native_pdf` *does* report every one of these cases through `tracing::warn!`,
/// but this crate never installed a [`tracing::Subscriber`], so — independent
/// of this fix — those records went to whatever the process's default
/// dispatcher was (typically none) and were dropped a second time. That is
/// the exact upstream-plus-local gap #1364 describes: xberg_native_pdf's own
/// diagnostic channel existed but nothing was listening.
///
/// # The migration off `log` (xberg_native_pdf 1.0.1, fork commit `0aed9f1b`)
///
/// This module used to install a [`log::Log`] backend instead, because at the
/// time `xberg_native_pdf` reported these through `log::warn!`. As of xberg_native_pdf
/// 1.0.1 the fork migrated its diagnostics off the `log` facade entirely
/// ("refactor!: migrate from the log facade to tracing") — its `Cargo.toml`
/// carries no `log` dependency at all any more, and every site this module
/// cares about is now `tracing::warn!(target: "xberg_xberg_native_pdf::...", "{}",
/// ...)`. A `log::Log` backend receives nothing from that, which is why the
/// old capture silently went dark on this upgrade. This struct is the
/// tracing-side replacement, installed the same way as before: opt-in, once
/// per process, whichever side wins the single global slot keeps it.
///
/// This is the xberg-side fix: install a capturing subscriber (once per
/// process; if the host application has already claimed the `tracing`
/// dispatcher for its own subscriber, [`install_pdf_render_diagnostics`]
/// leaves it alone and this capture path silently yields nothing — no worse
/// than today), and during each render call collect `xberg_native_pdf`'s own
/// target-prefixed warnings into a `ProcessingWarning`. The actual *decision*
/// about which glyph gets dropped and why remains entirely inside
/// `xberg_native_pdf`/`ttf-parser` — that part is upstream and is not touched here.
/// The `tracing` target root of the native PDF engine, re-exported so consumers -- notably
/// `xberg-cli`'s log filter -- match on a value rather than on a copied string literal.
///
/// This is the engine's own `module_path!()` evaluated at its crate root, so it tracks the
/// engine's `[lib] name` automatically and cannot go stale.
pub const ENGINE_LOG_TARGET_ROOT: &str = xberg_native_pdf::LOG_TARGET_ROOT;

/// Whether a `tracing` target belongs to the PDF engine whose warnings this module captures.
///
/// Matches on [`ENGINE_LOG_TARGET_ROOT`] rather than a literal. That distinction is the whole
/// point: in #697 this predicate held a hardcoded prefix, the engine's crate name moved, every
/// record was rejected, and for twelve days no warning about unparseable fonts reached a
/// caller. Nothing failed to compile, the test asserting warnings ARE captured went red, and
/// its sibling asserting NO warnings kept passing vacuously. Deriving the prefix from the
/// engine itself removes the class.
fn is_pdf_engine_target(target: &str) -> bool {
    target.starts_with(ENGINE_LOG_TARGET_ROOT)
}

struct EngineWarningCapture;

impl EngineWarningCapture {
    fn interested(metadata: &tracing::Metadata<'_>) -> bool {
        *metadata.level() <= tracing::Level::WARN && is_pdf_engine_target(metadata.target())
    }
}

/// Pulls the `message` field out of a `tracing::Event`.
///
/// A `tracing` event's message is a *field* (named `"message"`), not
/// something `Event` exposes as a plain string. `tracing::warn!("{}", x)` —
/// the form xberg_native_pdf uses at `text_rasterizer.rs:502` — records it via
/// [`record_debug`](tracing::field::Visit::record_debug): the value handed in
/// is the formatted `fmt::Arguments`, and `fmt::Arguments`'s `Debug` impl
/// forwards to its `Display` impl, so `format!("{value:?}")` below is the
/// plain formatted text, not a debug-quoted string. A bare string-literal
/// message (`tracing::warn!("literal")`, used by some other sites in the
/// fork) instead reaches [`record_str`](tracing::field::Visit::record_str).
/// Both are implemented so either form is captured. Same pattern as the
/// `MessageVisitor` already used for tracing-capture tests elsewhere in this
/// crate (`tests/gpu_acceleration.rs`).
///
/// The `operation` field is captured beside the message: it names what the engine was
/// doing, which is what decides how the warning is worded (GH#1794). ~keep
#[derive(Default)]
struct MessageVisitor(EngineWarning);

/// One captured engine warning: its message and the engine's `operation` field, if any.
#[derive(Debug, Default)]
struct EngineWarning {
    message: String,
    operation: Option<String>,
}

impl tracing::field::Visit for MessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        match field.name() {
            "message" => self.0.message = format!("{value:?}"),
            "operation" => self.0.operation = Some(format!("{value:?}")),
            _ => {}
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        match field.name() {
            "message" => self.0.message = value.to_string(),
            "operation" => self.0.operation = Some(value.to_string()),
            _ => {}
        }
    }
}

impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for EngineWarningCapture {
    /// Capture the engine's warnings into this thread's buffer; ignore every other event.
    ///
    /// This is deliberately the **only** trait method overridden. `Layer::enabled` and
    /// `Layer::max_level_hint` are filters over the *whole subscriber stack*, not per-layer
    /// ones: narrowing either here would silence the host application's own `fmt` layer for
    /// every non-`xberg_native_pdf` event, and cap the process at `WARN`. A library must not make
    /// that trade on its embedder's behalf. Filtering inside the callback instead costs one
    /// level check and one target comparison per event and affects nobody else.
    ///
    /// Re-emitting a captured record through another facade is the obvious instinct and must
    /// not be reintroduced. The `tracing/log` feature is enabled in this build (pulled in
    /// through `tower`), which makes a `tracing::warn!` also emit a `log::Record` whenever no
    /// global `tracing` dispatcher is set. If this method forwarded what it captured by
    /// calling `log::warn!`, that record would round-trip back into `tracing` through the same
    /// bridge, back to this layer, forever. That is not hypothetical: forwarding between the
    /// two facades is exactly what produced the #1364 regression test's `fatal runtime error:
    /// stack overflow`. This method only ever appends to a thread-local `Vec<String>`.
    ///
    /// The buffer is thread-local *by design*, and the layout pass runs the render inside
    /// `tokio::task::spawn_blocking`. That works because a layer's `on_event` runs
    /// synchronously on whichever thread emitted the event, so the warning lands in the same
    /// thread's buffer that `render_page_capturing_glyph_drops` armed and will drain.
    /// `glyph_drop_warnings_survive_the_spawn_blocking_layout_pass` exists to pin that.
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        if !Self::interested(event.metadata()) {
            return;
        }
        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);
        ENGINE_LOG_CAPTURE.with(|cell| {
            if let Some(buffer) = cell.borrow_mut().as_mut() {
                buffer.push(visitor.0);
            }
        });
    }
}

/// A `tracing` [`Layer`](tracing_subscriber::Layer) that captures the PDF engine's glyph-drop
/// warnings, for composing into an application's existing subscriber stack.
///
/// Prefer this over [`install_pdf_render_diagnostics`] whenever the application installs a
/// subscriber of its own, because `tracing` has exactly **one** global dispatcher slot and
/// composing shares it instead of racing for it:
///
/// ```ignore
/// tracing_subscriber::fmt()
///     .with_env_filter(env_filter)
///     .finish()
///     .with(xberg::pdf::render::glyph_drop_capture_layer())
///     .try_init();
/// ```
///
/// Constructing the layer arms the capture — [`render_page_capturing_glyph_drops`] is a no-op
/// until something does, so that the render path costs nothing for the majority of embedders
/// who never opt in. Building a layer you then discard leaves the render path arming and
/// draining an empty buffer: harmless, but pointless.
#[cfg_attr(alef, alef(skip))]
pub fn glyph_drop_capture_layer<S: tracing::Subscriber>() -> impl tracing_subscriber::Layer<S> {
    ENGINE_CAPTURE_ACTIVE.store(true, Ordering::Release);
    EngineWarningCapture
}

/// Install the glyph-drop capture as the process-wide `tracing`
/// [`Subscriber`](tracing::Subscriber), exactly once.
///
/// **Prefer [`glyph_drop_capture_layer`] if the application installs a subscriber of its own.**
/// `tracing` has exactly one global default dispatcher slot per process, and this function
/// claims it. If something else already holds it — an application wiring
/// `tracing_subscriber::fmt()...try_init()`, which is what `xberg-cli` does in `main()` —
/// `set_global_default` fails and this is a no-op, so the engine's glyph-drop records go to
/// that other subscriber and [`take_xberg_native_pdf_render_warnings`] stays empty. This function is
/// the fallback for embedders that have no subscriber at all; composing the layer is what
/// works when they do.
///
/// ★ That distinction is not theoretical, and it does not show up in tests. A test binary
/// installs no `fmt` subscriber, so the capture always wins the slot there and every test
/// passes — while the CLI, which claims the slot first, captures nothing. The port that
/// introduced this function in its `set_global_default`-only form would have gone green on all
/// three glyph-drop tests with the real CLI path still dead. Warnings arriving in a test are
/// not evidence that they arrive in production; the contested resource only exists once
/// something else has claimed it.
///
/// **Opt-in.** Nothing calls this automatically: a library that seizes the global dispatcher on
/// its own behalf breaks its embedder, because a host that later calls
/// `tracing_subscriber::fmt()...init()` panics (`.try_init()` instead returns `Err`). That
/// decision belongs to the application.
///
/// Returns whether the capture is active — `true` if this call installed it, an earlier call
/// did, or a [`glyph_drop_capture_layer`] was composed into someone else's stack; `false` if
/// some other component owns the dispatcher slot and no layer was composed.
///
/// Without one of the two opt-ins the #1364 warnings are not produced. The glyph drop itself is
/// decided inside `xberg_native_pdf`, which reports it only through `tracing::warn!`; there is no
/// return-value channel to read instead.
pub fn install_pdf_render_diagnostics() -> bool {
    ENGINE_LOGGER_INIT.call_once(|| {
        use tracing_subscriber::layer::SubscriberExt as _;

        // A bare `Registry` carrying only this layer: it neither filters nor caps the level for
        // anything else, so the earlier form's side effect — every event in the process below
        // `WARN` silently discarded, via a `max_level_hint` that applied stack-wide — is gone.
        let subscriber = tracing_subscriber::registry().with(EngineWarningCapture);
        if tracing::subscriber::set_global_default(subscriber).is_ok() {
            ENGINE_CAPTURE_ACTIVE.store(true, Ordering::Release);
            // Re-resolve any callsite whose interest was already cached (as
            // `never()`, from being reached with no global dispatcher
            // installed at all) before this call. This is safe here
            // specifically *because* `set_global_default` just above
            // succeeded: by the time this runs, the process has a global
            // default (this subscriber) for every thread to resolve interest
            // against. Calling this with no global default set at all is the
            // known hazard that disables `tracing` process-wide.
            tracing::callsite::rebuild_interest_cache();
        }
    });
    ENGINE_CAPTURE_ACTIVE.load(Ordering::Acquire)
}

/// Engine `operation`s whose warning means glyph ink is missing from the render: a
/// dropped glyph, a font that could not be found or loaded for rendering, and the CJK
/// substitution face being unavailable. Every other engine warning (a Type 3 glyph-name
/// fallback while loading a font, an embedded font replaced by a system font, a shading
/// or mask skipped) keeps its own cause and is not reported as missing ink (GH#1794). ~keep
const GLYPH_INK_LOSS_OPERATIONS: [&str; 4] = ["render_glyph", "resolve_font", "load_render_font", "load_cjk_fallback"];

/// Turn one captured `xberg_native_pdf` log line into a `(page, message)`
/// [`ProcessingWarning`], naming the page so a multi-page document does not
/// read as "somewhere in this PDF, something happened".
fn glyph_drop_warning(page_index: usize, cause: &str) -> ProcessingWarning {
    warning(
        PDF_RENDER_WARNING_SOURCE,
        format!(
            "Page {} rendering could not paint one or more glyphs and continued anyway \
             (advance-only, so layout is preserved but the glyph ink is missing): {cause}",
            page_index + 1
        ),
    )
}

/// Whether a captured engine warning means a whole image XObject failed to rasterize
/// (e.g. its stream failed to decode, tripped a decompression-bomb guard, or carried an
/// unsupported colour space), as opposed to a single glyph being dropped.
///
/// Matches the exact wording `xberg_native_pdf::rendering::page_renderer::render_xobject` logs
/// at its `render_image`/`render_image_mask` catch sites ("Skipping unrenderable image
/// XObject '{name}': {e}" / "Skipping unrenderable ImageMask XObject '{name}': {e}") when
/// `render_image` returns `Err` and the page is left with a blank region instead. That is a
/// materially different defect from a dropped glyph — a whole picture is missing, not one
/// character's ink — and reporting it as "glyph ink is missing" would misdescribe the cause
/// to anyone reading `processing_warnings`.
fn indicates_unrenderable_image(cause: &str) -> bool {
    cause.contains("unrenderable image XObject") || cause.contains("unrenderable ImageMask XObject")
}

fn image_render_failure_warning(page_index: usize, cause: &str) -> ProcessingWarning {
    warning(
        PDF_RENDER_WARNING_SOURCE,
        format!(
            "Page {} could not rasterize one or more images and left that region blank: {cause}",
            page_index + 1
        ),
    )
}

fn render_notice_warning(page_index: usize, cause: &str) -> ProcessingWarning {
    warning(
        PDF_RENDER_WARNING_SOURCE,
        format!(
            "Page {} rendering logged a warning and continued: {cause}",
            page_index + 1
        ),
    )
}

/// Word one captured engine warning by what it reports: a whole image left blank,
/// glyph ink missing, or any other warning passed through with its own cause.
fn classify_engine_warning(page_index: usize, captured: &EngineWarning) -> ProcessingWarning {
    let cause = captured.message.as_str();
    if indicates_unrenderable_image(cause) {
        image_render_failure_warning(page_index, cause)
    } else if captured
        .operation
        .as_deref()
        .is_some_and(|operation| GLYPH_INK_LOSS_OPERATIONS.contains(&operation))
    {
        glyph_drop_warning(page_index, cause)
    } else {
        render_notice_warning(page_index, cause)
    }
}

/// Render a page while capturing any `xberg_native_pdf` render-degradation warnings it
/// logs during the call — dropped glyphs and unrenderable image XObjects alike — deduping
/// them into [`ENGINE_PENDING_WARNINGS`] for later collection via
/// [`take_xberg_native_pdf_render_warnings`].
///
/// Capture is opt-in: unless the application called
/// [`install_pdf_render_diagnostics`], this arms nothing and is exactly
/// equivalent to calling `render` directly, at no cost.
fn render_page_capturing_glyph_drops(
    page_index: usize,
    render: impl FnOnce() -> std::result::Result<xberg_native_pdf::rendering::RenderedImage, xberg_native_pdf::Error>,
) -> std::result::Result<xberg_native_pdf::rendering::RenderedImage, xberg_native_pdf::Error> {
    if !ENGINE_CAPTURE_ACTIVE.load(Ordering::Acquire) {
        return render();
    }
    ENGINE_LOG_CAPTURE.with(|cell| *cell.borrow_mut() = Some(Vec::new()));
    let result = render();
    let captured = ENGINE_LOG_CAPTURE.with(|cell| cell.borrow_mut().take().unwrap_or_default());
    if !captured.is_empty() {
        ENGINE_PENDING_WARNINGS.with(|pending| {
            let mut pending = pending.borrow_mut();
            // GH#1548: this used to also filter out any cause containing "PDF spec compliant",
            // to exclude the Latin-1-fallback message. That message now stays below the
            // capture threshold (TRACE, gated on <= WARN by `EngineWarningCapture::interested`)
            // so it can never reach here, and the substring match was a trap for whoever next
            // wrote a genuinely actionable warning that happened to share the phrase. Every
            // captured cause is classified below instead of pre-filtered. ~keep
            for engine_warning in captured.iter() {
                push_warning_deduped(&mut pending, classify_engine_warning(page_index, engine_warning));
            }
        });
    }
    result
}

/// Drain the glyph-drop [`ProcessingWarning`]s accumulated on this thread by
/// render calls since the last call to this function.
///
/// Callers that render pages as part of extraction should call this after
/// their render pass and merge the result into
/// `InternalDocument::processing_warnings` (see the module-level convention
/// in `crate::core::diagnostics`) so a page with missing glyphs is never
/// returned to the user without a signal. Warnings are already deduped
/// per-thread across all pages rendered before this call.
///
/// `pub` (rather than `pub(crate)`) so both in-tree render-consumers and the
/// regression test for #1364 can observe capture without depending on any
/// one extractor's internal state.
///
/// As of #340, `crate::extractors::pdf::mod` drains this unconditionally right
/// after assembling a document's `processing_warnings`, so every PDF
/// extraction that renders at least one page picks up any captured
/// glyph-drop warnings for free. ~keep: that drain only ever observes
/// warnings from render calls that happened on the *same OS thread* before it
/// ran, because [`ENGINE_PENDING_WARNINGS`] is thread-local. OCR page
/// rendering runs inline on the extracting task's thread, so it is covered.
/// Layout-detection rasterization runs inside `tokio::task::spawn_blocking`,
/// which always executes on a different OS thread, so this function alone
/// would never see those warnings. As of #353,
/// `extractors::pdf::layout_runner::run_layout_for_pdf_pages_async` drains
/// this function itself from inside its `spawn_blocking` closure — the only
/// place that can observe the blocking-pool thread's thread-local buffer —
/// and threads the drained warnings back through its return value for the
/// caller in `extractors::pdf::mod` to merge, so layout-path glyph drops are
/// no longer silently lost.
pub fn take_xberg_native_pdf_render_warnings() -> Vec<ProcessingWarning> {
    ENGINE_PENDING_WARNINGS.with(|pending| std::mem::take(&mut *pending.borrow_mut()))
}

/// Reasonable max pixel dimension (on either axis) for a rendered page before we
/// force a lower DPI. This prevents Pixmap allocation failures or OOM for
/// extremely wide/tall technical diagrams, CAD exports, etc. while still
/// producing a usable raster for OCR/VLM (which are robust to moderate downscaling).
///
/// Chosen as 16384px because a 20000pt-wide page at the default 150 DPI produces
/// ~41667px on the long axis (20000 * 150 / 72), which triggers Pixmap creation
/// or rasterization failures inside xberg_native_pdf/tiny-skia for real vector-heavy
/// content. 16384 is high enough for normal documents (A3 landscape at 300dpi ~
/// 3500px) but catches the extreme cases reported in #1078. See the regression
/// test in this module for the exact repro input that previously failed.
const MAX_RENDER_DIMENSION_PX: f32 = 16384.0;

/// Compute a safe DPI for the given page MediaBox so that the rendered pixel
/// size stays within practical limits for the underlying rasterizer (tiny-skia
/// Pixmap + path/text rasterization in xberg_native_pdf).
///
/// Falls back to 72 DPI minimum. Returns the (possibly reduced) DPI to use.
fn choose_safe_dpi(w_pt: f32, h_pt: f32, base_dpi: u32) -> u32 {
    if w_pt <= 0.0 || h_pt <= 0.0 {
        return base_dpi.max(72);
    }
    let scale = base_dpi as f32 / 72.0;
    let w_px = w_pt * scale;
    let h_px = h_pt * scale;
    let max_dim = w_px.max(h_px);
    if max_dim <= MAX_RENDER_DIMENSION_PX {
        return base_dpi;
    }
    let factor = MAX_RENDER_DIMENSION_PX / max_dim;
    (base_dpi as f32 * factor).max(72.0) as u32
}

/// Fetch page MediaBox (in points) with a sane Letter fallback.
pub(crate) fn get_page_dimensions_pt(doc: &xberg_native_pdf::PdfDocument, page_index: usize) -> (f32, f32) {
    doc.get_page_media_box(page_index)
        .map(|(llx, lly, urx, ury)| ((urx - llx).abs(), (ury - lly).abs()))
        .unwrap_or((612.0, 792.0))
}

/// Derive the true resolution, in DPI, of a page raster produced by
/// [`render_page_with_safeguards`].
///
/// The renderer does not necessarily honour the DPI it is asked for: [`choose_safe_dpi`]
/// silently reduces it whenever the MediaBox would rasterize past
/// [`MAX_RENDER_DIMENSION_PX`], and the effective value it picked is then discarded — the
/// `RenderedImage` it returns carries only `data`, `width`, `height` and `format`. Recovering
/// the resolution from the raster's own pixel width against the page's MediaBox width is
/// exact whether or not that reduction fired, so nothing has to be threaded back out of the
/// renderer. It also stays correct per page in a document that mixes page sizes, which is why
/// this is derived per call rather than carried on a config.
///
/// The raster must be the MediaBox-oriented one [`normalize_rendered_page_for_ocr`] produces
/// (its axes align with the MediaBox, see [`pixel_bbox_to_pdf_points`]), not a raster that has
/// since been rotated upright — after a 90/270 degree rotation the width no longer corresponds
/// to `page_width_pt`. The resolution itself is rotation-invariant, so deriving it before any
/// such rotation and carrying the scalar forward is safe.
///
/// Returns `None` for a degenerate page box or an empty raster so callers keep their
/// "resolution unknown" behaviour instead of adopting a fabricated one.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(crate) fn rendered_page_dpi(rendered_width_px: u32, page_width_pt: f32) -> Option<f64> {
    /// Points per inch in PDF user space: a MediaBox is expressed in these units, so a page's
    /// width in inches is its width in points divided by this.
    const POINTS_PER_INCH: f64 = 72.0;

    let page_width_pt = f64::from(page_width_pt);
    if rendered_width_px == 0 || !page_width_pt.is_finite() || page_width_pt <= 0.0 {
        return None;
    }
    Some(f64::from(rendered_width_px) * POINTS_PER_INCH / page_width_pt)
}

/// Map a bounding box from OCR-image pixel space (origin top-left, y down)
/// to PDF point space (origin bottom-left, y up).
///
/// `rendered_w`/`rendered_h` are the dimensions of the image the OCR backend
/// saw. `xberg_native_pdf` renders in display orientation (`/Rotate` applied) and
/// [`normalize_rendered_page_for_ocr`] rotates that back to user-space
/// orientation, so the OCR image axes already align with the MediaBox: the
/// mapping is a pure scale plus a y flip. Scaling from the actual rendered
/// size keeps it correct when [`choose_safe_dpi`] reduced the render DPI.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(crate) fn pixel_bbox_to_pdf_points(
    bbox: crate::types::BoundingBox,
    rendered_w: u32,
    rendered_h: u32,
    page_w_pt: f32,
    page_h_pt: f32,
) -> crate::types::BoundingBox {
    if rendered_w == 0 || rendered_h == 0 {
        return bbox;
    }
    let sx = page_w_pt as f64 / rendered_w as f64;
    let sy = page_h_pt as f64 / rendered_h as f64;

    let x_pts = [bbox.x0 * sx, bbox.x1 * sx];
    let y_pts = [page_h_pt as f64 - bbox.y0 * sy, page_h_pt as f64 - bbox.y1 * sy];

    crate::types::BoundingBox {
        x0: x_pts[0].min(x_pts[1]),
        y0: y_pts[0].min(y_pts[1]),
        x1: x_pts[0].max(x_pts[1]),
        y1: y_pts[0].max(y_pts[1]),
    }
}

#[cfg(all(test, any(feature = "ocr", feature = "ocr-pipeline")))]
mod pixel_bbox_tests {
    use super::pixel_bbox_to_pdf_points;
    use crate::types::BoundingBox;

    #[test]
    fn scales_and_flips_y_to_bottom_left_origin() {
        // 1275x1650 px render of a 612x792 pt Letter page (150 DPI).
        let px = BoundingBox {
            x0: 127.5,
            y0: 165.0,
            x1: 255.0,
            y1: 330.0,
        };
        let pt = pixel_bbox_to_pdf_points(px, 1275, 1650, 612.0, 792.0);
        assert!((pt.x0 - 61.2).abs() < 1e-6);
        assert!((pt.x1 - 122.4).abs() < 1e-6);
        // Top of the box in image space is the HIGH y in point space.
        assert!((pt.y1 - (792.0 - 79.2)).abs() < 1e-6);
        assert!((pt.y0 - (792.0 - 158.4)).abs() < 1e-6);
        assert!(pt.y0 < pt.y1);
    }

    #[test]
    fn adapts_to_reduced_render_dpi() {
        // The same page rendered at half resolution maps to the same points.
        let px = BoundingBox {
            x0: 63.75,
            y0: 82.5,
            x1: 127.5,
            y1: 165.0,
        };
        let pt = pixel_bbox_to_pdf_points(px, 637, 825, 612.0, 792.0);
        assert!((pt.x0 - 61.25).abs() < 0.2);
        assert!((pt.y1 - 712.8).abs() < 0.5);
    }

    #[test]
    fn zero_dimension_render_returns_input() {
        let px = BoundingBox {
            x0: 1.0,
            y0: 2.0,
            x1: 3.0,
            y1: 4.0,
        };
        assert_eq!(pixel_bbox_to_pdf_points(px, 0, 100, 612.0, 792.0), px);
    }
}

/// Read per-page /Rotate values for a whole document, normalized to
/// 0/90/180/270.
///
/// Delegates to `xberg_native_pdf::PdfDocument::get_page_rotation`, which walks the
/// page tree's `/Parent`-inheritance chain per ISO 32000-1 §7.7.3.4 (a page
/// without its own `/Rotate` inherits from its `/Pages` ancestors) — the
/// same resolution this function used to hand-roll via a second, separate
/// `lopdf::Document::load_mem` parse of the same bytes. Every current caller
/// already holds the `xberg_native_pdf::PdfDocument` passed in here open for
/// rendering, so taking `&xberg_native_pdf::PdfDocument` instead of raw bytes
/// removes that second parse entirely.
///
/// A per-page lookup that errors (e.g. an encrypted document whose page tree
/// could not be decrypted, or a corrupt page-tree node that neither the tree
/// walk nor its scanning fallback can resolve) defaults that page's rotation
/// to `0`, matching this function's previous contract of defaulting to `0`
/// on any parse failure — the difference is this now happens per page
/// instead of for the whole document at once, since a `xberg_native_pdf::PdfDocument`
/// reaching this function has by definition already opened successfully.
///
/// # Deliberate choice: non-multiple-of-90 `/Rotate` values are folded to 0
///
/// ISO 32000-1 §7.7.3.3 requires `/Rotate` to be a multiple of 90; a PDF that
/// sets a non-multiple (e.g. `135`) is out of spec. The previous hand-rolled
/// implementation stored such a value verbatim (after only a
/// `rem_euclid(360)` fold for sign), which mattered only as a *value*:
/// [`rotate_dynamic_image`] below only rotates on an exact 90/180/270 match
/// and no-ops everything else, so a stored `135` never rotated a single
/// pixel. Its one live effect was being forwarded unchanged into
/// `ocr_config_with_page_rotation_hint`'s `page_rotation_degrees` backend
/// hint (see `extractors::pdf::ocr`), telling an OCR backend the page is
/// rotated by a degree count nothing in this pipeline can ever act on.
/// `xberg_native_pdf::PdfDocument::get_page_rotation` instead folds any
/// non-multiple-of-90 value to `0` at the source, treating an out-of-spec
/// `/Rotate` exactly like a *missing* one. That is the behaviour kept here:
/// it is strictly more correct (spec-conformant) than the old
/// preserve-and-forward-garbage behaviour, and no test anywhere in this
/// crate asserted the old value was ever used for anything, so nothing
/// relies on it. See `get_page_rotations`' test module for the pinning test.
///
/// See `get_page_rotations_from_bytes` for callers that hold only the raw bytes.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline", feature = "layout-detection"))]
pub(crate) fn get_page_rotations(doc: &xberg_native_pdf::PdfDocument, page_count: usize) -> Vec<u32> {
    (0..page_count)
        .map(|page_index| match doc.get_page_rotation(page_index) {
            // `get_page_rotation`'s own contract guarantees a value in
            // {0, 90, 180, 270}, always non-negative.
            Ok(degrees) => degrees as u32,
            Err(error) => {
                tracing::warn!(
                    page = page_index + 1,
                    %error,
                    "could not resolve /Rotate for page; defaulting to 0 (no rotation)"
                );
                0
            }
        })
        .collect()
}

/// Prefer the document-taking form wherever a `PdfDocument` is already open — three call
/// sites were re-parsing the same bytes a second time purely to read `/Rotate`. This exists
/// for the markdown-layout reuse gate, which genuinely has no document in scope, and it opens
/// one so the extra parse is at least explicit at the call site rather than hidden inside the
/// lookup.
///
/// Returns all-zero rotations if the document cannot be opened: this is a rendering hint,
/// and a document that will not open fails for better reasons elsewhere.
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "layout-detection"))]
pub(crate) fn get_page_rotations_from_bytes(content: &[u8], page_count: usize) -> Vec<u32> {
    match xberg_native_pdf::PdfDocument::from_bytes(content.to_vec()) {
        Ok(doc) => get_page_rotations(&doc, page_count),
        Err(error) => {
            tracing::warn!(%error, "failed to open PDF to read page rotations; assuming none");
            vec![0; page_count]
        }
    }
}

/// Rotate a decoded page image per the page's normalized /Rotate value.
/// No-op for 0 or non-quarter-turn values.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline", feature = "layout-detection"))]
pub(crate) fn rotate_dynamic_image(img: image::DynamicImage, rotation_degrees: u32) -> image::DynamicImage {
    match rotation_degrees % 360 {
        90 => img.rotate90(),
        180 => img.rotate180(),
        270 => img.rotate270(),
        _ => img,
    }
}

/// Return the correction needed to make a page raster upright after
/// `xberg_native_pdf` has applied the PDF page's `/Rotate` value while rendering.
/// ~keep
#[cfg(any(feature = "ocr", feature = "ocr-pipeline", feature = "layout-detection"))]
pub(crate) fn ocr_page_correction_degrees(rotation_degrees: u32) -> u32 {
    (360 - rotation_degrees % 360) % 360
}

/// Rotate PNG-encoded page bytes per the page's /Rotate value.
///
/// Fast path: rotation 0 returns the input unchanged (no decode). Rotated
/// pages pay one decode + re-encode, which only happens for documents that
/// actually carry /Rotate. Returns the (possibly new) PNG bytes with the
/// post-rotation width and height.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline", feature = "layout-detection"))]
#[cfg(test)]
pub(crate) fn rotate_png_page_if_needed(
    png_data: Vec<u8>,
    width: u32,
    height: u32,
    rotation_degrees: u32,
) -> Result<(Vec<u8>, u32, u32)> {
    rotate_png_page_if_needed_with_security_limits(
        png_data,
        width,
        height,
        rotation_degrees,
        &crate::extractors::security::SecurityLimits::default(),
    )
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline", feature = "layout-detection"))]
pub(crate) fn rotate_png_page_if_needed_with_security_limits(
    png_data: Vec<u8>,
    width: u32,
    height: u32,
    rotation_degrees: u32,
    security_limits: &crate::extractors::security::SecurityLimits,
) -> Result<(Vec<u8>, u32, u32)> {
    if rotation_degrees.is_multiple_of(360) {
        return Ok((png_data, width, height));
    }
    let pixel_count = u64::from(width)
        .checked_mul(u64::from(height))
        .ok_or_else(|| crate::extraction::image_decode::image_dimension_error(width, height, u64::MAX, u64::MAX))?;
    let additional_live_bytes = pixel_count
        .checked_mul(3 + ROTATED_PNG_ENCODE_BYTES_PER_PIXEL)
        .and_then(|bytes| bytes.checked_add(ROTATED_PNG_ENCODE_FIXED_BYTES))
        .ok_or_else(|| crate::extraction::image_decode::image_dimension_error(width, height, u64::MAX, u64::MAX))?;
    let rgb = crate::extraction::image_decode::decode_standard_rgb8_with_additional_live_bytes_and_security_limits(
        &png_data,
        security_limits,
        additional_live_bytes,
    )
    .map_err(|error| match error {
        error @ XbergError::Validation { .. } => error,
        error => XbergError::Parsing {
            message: format!("failed to decode rendered page for rotation correction: {error}"),
            source: Some(Box::new(error)),
        },
    })?;
    let img = image::DynamicImage::ImageRgb8(rgb);
    let rotated = rotate_dynamic_image(img, rotation_degrees);
    let (w, h) = (rotated.width(), rotated.height());
    let mut buf = Vec::new();
    rotated
        .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
        .map_err(|e| XbergError::Parsing {
            message: format!("failed to re-encode rotated page: {e}"),
            source: None,
        })?;
    Ok((buf, w, h))
}

/// Normalize a `xberg_native_pdf` page raster for OCR.
///
/// `xberg_native_pdf` already applies `/Rotate` to the rendered page. OCR needs the
/// inverse transform exactly once so text is upright before layout and OCR
/// inference consume the shared raster. ~keep
#[cfg(any(feature = "ocr", feature = "ocr-pipeline", feature = "layout-detection"))]
#[cfg(test)]
pub(crate) fn normalize_rendered_page_for_ocr(
    png_data: Vec<u8>,
    width: u32,
    height: u32,
    rotation_degrees: u32,
) -> Result<(Vec<u8>, u32, u32)> {
    rotate_png_page_if_needed(png_data, width, height, ocr_page_correction_degrees(rotation_degrees))
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline", feature = "layout-detection"))]
pub(crate) fn normalize_rendered_page_for_ocr_with_security_limits(
    png_data: Vec<u8>,
    width: u32,
    height: u32,
    rotation_degrees: u32,
    security_limits: &crate::extractors::security::SecurityLimits,
) -> Result<(Vec<u8>, u32, u32)> {
    rotate_png_page_if_needed_with_security_limits(
        png_data,
        width,
        height,
        ocr_page_correction_degrees(rotation_degrees),
        security_limits,
    )
}

/// Contain a panic raised while rasterizing one page, turning it into an
/// ordinary `xberg_native_pdf::Error` for that page.
///
/// Rasterization runs third-party code over attacker-controlled geometry, and a
/// malformed page can violate an invariant the rasterizer only asserts: on a PDF
/// whose content streams fail to inflate (`FlateDecode` recovery exhausted),
/// tiny-skia 0.12.0 unwraps an `AlphaRun` in `break_run` (`alpha_runs.rs:170`)
/// that is `None`, and panics. There is no newer tiny-skia to upgrade to —
/// 0.12.0 is the current release.
///
/// Because render calls run synchronously on a Tokio worker, that panic unwinds
/// through the async boundary and fails the entire request with a 500, so a
/// single bad page costs the caller every other page's text as well. Containing
/// it here is the same treatment the text path gives the total-order sort panic
/// (#1198): the page becomes an `Err`, callers fall back to their existing
/// render-failure handling, and the rest of the document still extracts.
fn guard_render_panic(
    page_index: usize,
    render: impl FnOnce() -> std::result::Result<xberg_native_pdf::rendering::RenderedImage, xberg_native_pdf::Error>,
) -> std::result::Result<xberg_native_pdf::rendering::RenderedImage, xberg_native_pdf::Error> {
    super::native::guard_native_panic(render, |message| {
        xberg_native_pdf::Error::InvalidPdf(format!(
            "page {} could not be rasterized: the rasterizer panicked and was contained ({message})",
            page_index + 1
        ))
    })
}

/// Render a page using safeguards for extreme dimensions (wide vector diagrams,
/// CAD sheets, etc.). This is the root-cause fix for render failures on such
/// inputs during force_ocr / VLM / layout paths.
///
/// Uses the opened document (so callers that batch multiple pages only parse once).
pub(crate) fn render_page_with_safeguards(
    doc: &xberg_native_pdf::PdfDocument,
    page_index: usize,
    base_dpi: u32,
) -> std::result::Result<xberg_native_pdf::rendering::RenderedImage, xberg_native_pdf::Error> {
    let (w_pt, h_pt) = get_page_dimensions_pt(doc, page_index);
    let safe_dpi = choose_safe_dpi(w_pt, h_pt, base_dpi);
    if safe_dpi != base_dpi {
        tracing::warn!(
            page = page_index + 1,
            original_dpi = base_dpi,
            effective_dpi = safe_dpi,
            width_pt = w_pt,
            height_pt = h_pt,
            "reducing render DPI for page due to extreme dimensions (wide vector-heavy PDF or similar)"
        );
    }
    let options = xberg_native_pdf::rendering::RenderOptions::with_dpi(safe_dpi);
    // The panic guard sits inside the capture wrapper, not around it, so a
    // panicking page still lets the wrapper take its thread-local buffer back
    // instead of leaving it armed on a pooled thread.
    render_page_capturing_glyph_drops(page_index, || {
        guard_render_panic(page_index, || {
            xberg_native_pdf::rendering::render_page(doc, page_index, &options)
        })
    })
}

/// Open (and optionally authenticate) a PDF document from raw bytes.
///
/// Parsing the cross-reference table and trailer is the expensive part of
/// working with a PDF; rendering a page only reads the already-parsed
/// structures. Callers that need several pages should open the document once
/// with this helper and reuse the returned handle across
/// [`render_open_pdf_page_to_png`] calls rather than re-opening per page.
///
/// # Errors
///
/// Returns `XbergError::Parsing` if the PDF cannot be opened or authenticated.
pub(crate) fn open_pdf_document(pdf_bytes: &[u8], password: Option<&str>) -> Result<xberg_native_pdf::PdfDocument> {
    let doc = xberg_native_pdf::PdfDocument::from_bytes(pdf_bytes.to_vec()).map_err(|e| XbergError::Parsing {
        message: format!("Failed to open PDF: {e}"),
        source: None,
    })?;

    if let Some(pwd) = password {
        doc.authenticate(pwd.as_bytes()).map_err(|e| XbergError::Parsing {
            message: format!("Failed to authenticate PDF: {e}"),
            source: None,
        })?;
    }

    Ok(doc)
}

/// Read the page count from an already-open document.
///
/// # Errors
///
/// Returns `XbergError::Parsing` if the page count cannot be read.
pub(crate) fn document_page_count(doc: &xberg_native_pdf::PdfDocument) -> Result<usize> {
    doc.page_count().map_err(|e| XbergError::Parsing {
        message: format!("Failed to read page count: {e}"),
        source: None,
    })
}

/// Render one page of an already-open document to PNG bytes via the
/// extreme-dimension DPI safeguard.
///
/// This is the per-page primitive shared by [`render_pdf_page_to_png`] (which
/// opens the document, then delegates) and batch callers that open once and
/// render every page from a single parsed handle. `page_index` is assumed to be
/// in range; out-of-range indices surface as the underlying rasterizer error.
///
/// # Errors
///
/// Returns `XbergError::Parsing` if the page cannot be rendered.
pub(crate) fn render_open_pdf_page_to_png(
    doc: &xberg_native_pdf::PdfDocument,
    page_index: usize,
    dpi: Option<i32>,
) -> Result<Vec<u8>> {
    let render_dpi = dpi.unwrap_or(150).max(1) as u32;
    let rendered = render_page_with_safeguards(doc, page_index, render_dpi).map_err(|e| XbergError::Parsing {
        message: format!("Failed to render page {page_index}: {e}"),
        source: None,
    })?;

    Ok(rendered.data)
}

/// An open PDF document that can render multiple pages without reparsing the file.
///
/// This Rust-only session keeps the native PDF engine private while exposing the
/// efficient open-once rendering path. Use [`render_pdf_page_to_png`] for a single
/// page and this type when rendering several pages from the same document.
#[cfg_attr(alef, alef(skip))]
pub struct PdfRenderSession {
    document: xberg_native_pdf::PdfDocument,
    page_count: usize,
}

#[cfg_attr(alef, alef(skip))]
impl PdfRenderSession {
    /// Open and optionally authenticate a PDF document.
    ///
    /// # Errors
    ///
    /// Returns `XbergError::Parsing` if the PDF cannot be opened, authenticated,
    /// or its page count cannot be read.
    pub fn open(pdf_bytes: &[u8], password: Option<&str>) -> Result<Self> {
        let document = open_pdf_document(pdf_bytes, password)?;
        let page_count = document_page_count(&document)?;
        Ok(Self { document, page_count })
    }

    /// Return the number of pages in the open document.
    #[must_use]
    pub fn page_count(&self) -> usize {
        self.page_count
    }

    /// Render a zero-based page index to PNG bytes.
    ///
    /// # Errors
    ///
    /// Returns `XbergError::Parsing` if `page_index` is out of range or the page
    /// cannot be rendered.
    pub fn render_page_to_png(&self, page_index: usize, dpi: Option<i32>) -> Result<Vec<u8>> {
        if page_index >= self.page_count {
            return Err(XbergError::Parsing {
                message: format!(
                    "Page index {page_index} out of range (document has {} pages)",
                    self.page_count
                ),
                source: None,
            });
        }

        render_open_pdf_page_to_png(&self.document, page_index, dpi)
    }
}

/// Render a single PDF page to PNG bytes.
///
/// Returns raw PNG-encoded bytes for the specified page at the given DPI.
/// Uses xberg_native_pdf with tiny-skia for pure-Rust rendering.
///
/// For pages with extreme dimensions (very wide vector diagrams, etc.) the
/// effective DPI may be automatically reduced to avoid rasterizer failure.
/// A warning is logged when this happens.
///
/// # Arguments
///
/// * `pdf_bytes` - Raw PDF file bytes
/// * `page_index` - Zero-based page index
/// * `dpi` - Resolution in dots per inch (default: 150)
/// * `password` - Optional password for encrypted PDFs
///
/// # Errors
///
/// Returns `XbergError::Parsing` if the PDF cannot be opened, authenticated,
/// or rendered, or if `page_index` is out of range.
pub fn render_pdf_page_to_png(
    pdf_bytes: &[u8],
    page_index: usize,
    dpi: Option<i32>,
    password: Option<&str>,
) -> Result<Vec<u8>> {
    PdfRenderSession::open(pdf_bytes, password)?.render_page_to_png(page_index, dpi)
}

/// Count the pages in a PDF without rendering any of them.
///
/// Opens the document and returns its page count from the PDF structure. No page
/// is rasterized, so this is cheap relative to `render_pdf_page_to_png` — use it
/// when you only need the count (e.g. to drive a render loop over the pages).
///
/// # Arguments
///
/// * `pdf_bytes` - Raw PDF file bytes
/// * `password` - Optional password for encrypted PDFs
///
/// # Errors
///
/// Returns `XbergError::Parsing` if the PDF cannot be opened, authenticated,
/// or its page count read.
pub fn pdf_page_count(pdf_bytes: &[u8], password: Option<&str>) -> Result<usize> {
    Ok(PdfRenderSession::open(pdf_bytes, password)?.page_count())
}

/// Build a minimal valid single-page PDF with the given MediaBox (in points).
/// Used to test the wide-page / extreme-dimension safeguard in the renderer.
/// Note: the generated PDF has no content stream or /Resources. It is sufficient
/// to exercise the MediaBox-based DPI guard, but real-world wide vector diagrams
/// with complex paths may exercise additional failure modes in the rasterizer.
/// This is a known limitation of the in-memory test; a real repro PDF from #1078
/// was used during manual verification.
#[cfg(all(test, feature = "pdf"))]
pub(crate) fn build_minimal_pdf_with_mediabox(w: f32, h: f32) -> Vec<u8> {
    let mut buf = Vec::<u8>::new();
    buf.extend_from_slice(b"%PDF-1.4\n");

    let obj1_offset = buf.len();
    buf.extend_from_slice(b"1 0 obj\n<</Type /Catalog /Pages 2 0 R>>\nendobj\n");

    let obj2_offset = buf.len();
    buf.extend_from_slice(b"2 0 obj\n<</Type /Pages /Kids [3 0 R] /Count 1>>\nendobj\n");

    let obj3_offset = buf.len();
    let mb = format!("[0 0 {} {}]", w, h);
    buf.extend_from_slice(format!("3 0 obj\n<</Type /Page /MediaBox {} /Parent 2 0 R>>\nendobj\n", mb).as_bytes());

    let xref_offset = buf.len();

    buf.extend_from_slice(b"xref\n");
    buf.extend_from_slice(b"0 4\n");
    buf.extend_from_slice(b"0000000000 65535 f \n");
    buf.extend_from_slice(format!("{:010} 00000 n \n", obj1_offset).as_bytes());
    buf.extend_from_slice(format!("{:010} 00000 n \n", obj2_offset).as_bytes());
    buf.extend_from_slice(format!("{:010} 00000 n \n", obj3_offset).as_bytes());

    buf.extend_from_slice(b"trailer\n<</Size 4 /Root 1 0 R>>\n");
    buf.extend_from_slice(format!("startxref\n{}\n%%EOF\n", xref_offset).as_bytes());

    buf
}

/// Build a single-page PDF embedding a synthetic Type 1C (CFF) font whose
/// dot-bearing glyphs carry the deprecated `dotsection` operator, mirroring
/// Adobe's Type 1 to Type 2 converter output that surfaced the bug. The font
/// is generated with fontTools (no third-party font data); stock ttf-parser
/// 0.25.1 drops all seven dotsection glyphs from it while the controls keep
/// their outlines, so the patched parser (which carries the fix) is what makes
/// this render. Used by the dotsection regression test below.
///
/// Layout (48pt glyphs, one per 72pt-wide cell starting at x=72):
///   row 1, baseline y=650: i j . : ; ! ?   (all carry dotsection)
///   row 2, baseline y=450: l n ,           (controls, no dotsection)
///
/// Each glyph is drawn with its own `Td`, so ink positions are independent of
/// advance widths. Poppler renders every cell of this exact layout.
#[cfg(all(test, feature = "pdf"))]
pub(crate) fn build_dotsection_cff_pdf() -> Vec<u8> {
    const CFF: &[u8] = include_bytes!("testdata/dotsection_test_font.cff");
    const DIFFERENCES: &str = "[ 33 /exclam 44 /comma 46 /period 58 /colon 59 /semicolon \
                               63 /question 105 /i 106 /j 108 /l 110 /n ]";

    let mut content = String::new();
    for (codes, y) in [(DOTSECTION_ROW, 650u32), (CONTROL_ROW, 450u32)] {
        for (k, code) in codes.iter().enumerate() {
            let x = 72 + k * 72;
            content.push_str(&format!("BT /F1 48 Tf {x} {y} Td <{code:02X}> Tj ET\n"));
        }
    }

    let widths = (33..=110).map(|_| "500").collect::<Vec<_>>().join(" ");
    let objects: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
           /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>"
            .to_vec(),
        format!(
            "<< /Type /Font /Subtype /Type1 /BaseFont /XbergDotsectionTest \
             /FirstChar 33 /LastChar 110 /Widths [ {widths} ] \
             /FontDescriptor 6 0 R /Encoding 7 0 R >>"
        )
        .into_bytes(),
        {
            let mut o = format!("<< /Length {} >>\nstream\n", content.len()).into_bytes();
            o.extend_from_slice(content.as_bytes());
            o.extend_from_slice(b"endstream");
            o
        },
        b"<< /Type /FontDescriptor /FontName /XbergDotsectionTest /Flags 32 \
           /FontBBox [-200 -250 1000 1000] /ItalicAngle 0 /Ascent 800 \
           /Descent -200 /CapHeight 700 /StemV 80 /FontFile3 8 0 R >>"
            .to_vec(),
        format!("<< /Type /Encoding /BaseEncoding /WinAnsiEncoding /Differences {DIFFERENCES} >>").into_bytes(),
        {
            let mut o = format!("<< /Subtype /Type1C /Length {} >>\nstream\n", CFF.len()).into_bytes();
            o.extend_from_slice(CFF);
            o.extend_from_slice(b"\nendstream");
            o
        },
    ];

    let mut buf = Vec::<u8>::new();
    buf.extend_from_slice(b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n");
    let mut offsets = Vec::with_capacity(objects.len());
    for (i, body) in objects.iter().enumerate() {
        offsets.push(buf.len());
        buf.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        buf.extend_from_slice(body);
        buf.extend_from_slice(b"\nendobj\n");
    }
    let xref_offset = buf.len();
    buf.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
    buf.extend_from_slice(b"0000000000 65535 f \n");
    for off in offsets {
        buf.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    buf.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R >>\n", objects.len() + 1).as_bytes());
    buf.extend_from_slice(format!("startxref\n{xref_offset}\n%%EOF\n").as_bytes());
    buf
}

/// Char codes of the dotsection-carrying glyphs in [`build_dotsection_cff_pdf`]:
/// i, j, period, colon, semicolon, exclam, question.
#[cfg(all(test, feature = "pdf"))]
pub(crate) const DOTSECTION_ROW: &[u8] = &[105, 106, 46, 58, 59, 33, 63];

/// Char codes of the control glyphs (no dotsection): l, n, comma.
#[cfg(all(test, feature = "pdf"))]
pub(crate) const CONTROL_ROW: &[u8] = &[108, 110, 44];

#[cfg(all(test, feature = "pdf"))]
mod tests {
    use super::*;

    #[test]
    fn test_choose_safe_dpi_normal_page_unchanged() {
        let dpi = choose_safe_dpi(612.0, 792.0, 150);
        assert_eq!(dpi, 150);
    }

    #[test]
    fn test_choose_safe_dpi_extreme_wide_reduced() {
        let dpi = choose_safe_dpi(20000.0, 200.0, 150);
        assert_eq!(dpi, 72);
    }

    /// The raster's own pixel width is the only honest record of the resolution a page was
    /// rendered at, because `render_page_with_safeguards` throws `choose_safe_dpi`'s effective
    /// value away. A Letter page rendered at an arbitrary requested 150 DPI (any DPI works;
    /// the actual default the PDF OCR route requests is `effective_pdf_render_dpi`, #1577) is
    /// 1275px wide, and that must read back as 150 — not as the 72 the preprocessor assumes
    /// when nobody tells it otherwise.
    #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
    #[test]
    fn should_derive_render_dpi_from_raster_width_and_mediabox() {
        assert_eq!(rendered_page_dpi(1275, 612.0), Some(150.0));
    }

    /// The same derivation on a page `choose_safe_dpi` reduced: a 20000pt-wide sheet asked for
    /// at 150 DPI comes back at 72 (see `test_choose_safe_dpi_extreme_wide_reduced`), i.e.
    /// 20000px, and must read back as the 72 it really is rather than the 150 that was asked
    /// for.
    #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
    #[test]
    fn should_derive_reduced_render_dpi_when_safe_dpi_clamped_the_page() {
        assert_eq!(rendered_page_dpi(20000, 20000.0), Some(72.0));
    }

    /// A degenerate MediaBox or an empty raster yields no resolution at all, so the caller
    /// keeps its "unknown" behaviour instead of dividing by zero into an infinite DPI.
    #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
    #[test]
    fn should_return_none_when_page_box_or_raster_is_degenerate() {
        assert_eq!(rendered_page_dpi(1275, 0.0), None);
        assert_eq!(rendered_page_dpi(1275, -612.0), None);
        assert_eq!(rendered_page_dpi(1275, f32::NAN), None);
        assert_eq!(rendered_page_dpi(0, 612.0), None);
    }

    #[test]
    fn test_render_pdf_page_to_png_very_wide_does_not_panic_or_hard_fail() {
        let wide_pdf = build_minimal_pdf_with_mediabox(20000.0, 300.0);
        let res = render_pdf_page_to_png(&wide_pdf, 0, None, None);
        assert!(
            res.is_ok(),
            "wide page render should succeed thanks to safeguard, got: {:?}",
            res.err()
        );
    }

    // A rasterizer panic used to unwind through the Tokio worker and fail the
    // whole extraction with a 500, so the other pages' text was lost with it.
    #[test]
    fn test_guard_render_panic_contains_panic_as_page_error() {
        // Matched rather than `expect_err`, which would require RenderedImage: Debug.
        let message = match guard_render_panic(3, || panic!("simulated tiny-skia unwrap")) {
            Ok(_) => panic!("panic must not escape the guard"),
            Err(error) => error.to_string(),
        };
        assert!(
            message.contains("page 4"),
            "error should name the 1-based page: {message}"
        );
        assert!(
            message.contains("simulated tiny-skia unwrap"),
            "error should carry the panic message: {message}"
        );
    }

    #[test]
    fn test_pdf_page_count_single_page() {
        let pdf = build_minimal_pdf_with_mediabox(612.0, 792.0);
        let count = pdf_page_count(&pdf, None).expect("page count should succeed for a valid PDF");
        assert_eq!(count, 1, "minimal single-page PDF must report 1 page");
    }

    #[test]
    fn pdf_render_session_reports_count_and_renders_without_reopening() {
        fn assert_send_sync<T: Send + Sync>() {}

        assert_send_sync::<PdfRenderSession>();
        let pdf = build_minimal_pdf_with_mediabox(612.0, 792.0);
        let session = PdfRenderSession::open(&pdf, None).expect("valid PDF should open");

        assert_eq!(session.page_count(), 1);
        let png = session
            .render_page_to_png(0, Some(72))
            .expect("open session should render its page");
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");

        let error = session
            .render_page_to_png(1, Some(72))
            .expect_err("out-of-range page must fail");
        assert!(error.to_string().contains("document has 1 pages"));
    }

    #[test]
    fn test_pdf_page_count_invalid_pdf_errors() {
        let err = pdf_page_count(b"not a pdf", None).expect_err("invalid PDF bytes must error");
        assert!(
            matches!(err, XbergError::Parsing { .. }),
            "expected a Parsing error, got: {err:?}"
        );
    }

    #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
    #[test]
    fn test_rotate_dynamic_image_0_degrees_is_noop() {
        let img = image::DynamicImage::new_rgb8(100, 150);
        let rotated = rotate_dynamic_image(img, 0);
        assert_eq!((rotated.width(), rotated.height()), (100, 150));
    }

    #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
    #[test]
    fn test_rotate_dynamic_image_90_degrees_swaps_dimensions() {
        let img = image::DynamicImage::new_rgb8(100, 150);
        let rotated = rotate_dynamic_image(img, 90);
        assert_eq!((rotated.width(), rotated.height()), (150, 100));
    }

    #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
    #[test]
    fn test_rotate_dynamic_image_180_degrees_keeps_dimensions() {
        let img = image::DynamicImage::new_rgb8(100, 150);
        let rotated = rotate_dynamic_image(img, 180);
        assert_eq!((rotated.width(), rotated.height()), (100, 150));
    }

    #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
    #[test]
    fn test_rotate_dynamic_image_270_degrees_swaps_dimensions() {
        let img = image::DynamicImage::new_rgb8(100, 150);
        let rotated = rotate_dynamic_image(img, 270);
        assert_eq!((rotated.width(), rotated.height()), (150, 100));
    }

    #[cfg(any(feature = "ocr", feature = "ocr-pipeline", feature = "layout-detection"))]
    #[test]
    fn rotated_png_rejects_request_limit_before_peak_allocation() {
        let image = image::RgbImage::from_pixel(2, 2, image::Rgb([255, 255, 255]));
        let mut encoded = Vec::new();
        image::DynamicImage::ImageRgb8(image)
            .write_to(&mut std::io::Cursor::new(&mut encoded), image::ImageFormat::Png)
            .expect("encode rotation fixture");
        let limits = crate::extractors::security::SecurityLimits {
            max_content_size: 1_000,
            ..Default::default()
        };

        let error = rotate_png_page_if_needed_with_security_limits(encoded, 2, 2, 90, &limits)
            .expect_err("rotation, PNG encoder workspace, and output must honor request limits");

        assert!(matches!(error, XbergError::Validation { .. }));
    }

    /// Count pixels darker than mid-gray inside one glyph cell of the
    /// dotsection fixture. Cells are 72pt wide starting at x=72; the y band
    /// covers ascender through descender around the row's baseline.
    fn dark_pixels_in_cell(img: &image::GrayImage, cell: usize, baseline_pt: f32) -> u32 {
        const SCALE: f32 = 150.0 / 72.0;
        const PAGE_H_PT: f32 = 792.0;
        let x0 = (((72 + cell * 72) as f32 - 4.0) * SCALE).max(0.0) as u32;
        let x1 = ((((72 + cell * 72) + 56) as f32) * SCALE).min(img.width() as f32) as u32;
        let y0 = ((PAGE_H_PT - (baseline_pt + 40.0)) * SCALE).max(0.0) as u32;
        let y1 = ((PAGE_H_PT - (baseline_pt - 14.0)) * SCALE).min(img.height() as f32) as u32;
        let mut dark = 0u32;
        for y in y0..y1 {
            for x in x0..x1 {
                if img.get_pixel(x, y).0[0] < 128 {
                    dark += 1;
                }
            }
        }
        dark
    }

    /// Regression test for CFF fonts whose charstrings carry the deprecated
    /// `dotsection` operator (12 0). ttf-parser 0.25.1 aborted the whole
    /// charstring with `UnsupportedOperator`, so xberg_native_pdf painted nothing for
    /// i, j, period, colon, semicolon, exclam and question while still
    /// advancing the cursor: OCR received page images with those letters
    /// silently missing. Exercises the full render path against the parser the
    /// workspace `[patch.crates-io]` routes to — `xberg-ttf-parser`, which
    /// carries the fix (upstream #228).
    #[test]
    fn test_render_paints_cff_glyphs_that_use_dotsection() {
        let names_row1 = ["i", "j", "period", "colon", "semicolon", "exclam", "question"];
        let names_row2 = ["l", "n", "comma"];

        let pdf = build_dotsection_cff_pdf();
        let png = render_pdf_page_to_png(&pdf, 0, Some(150), None).expect("fixture page must render");
        let img = image::load_from_memory(&png)
            .expect("rendered PNG must decode")
            .to_luma8();

        for (names, codes, baseline) in [
            (&names_row1[..], DOTSECTION_ROW, 650.0),
            (&names_row2[..], CONTROL_ROW, 450.0),
        ] {
            for (cell, name) in names.iter().enumerate() {
                let dark = dark_pixels_in_cell(&img, cell, baseline);
                assert!(
                    dark >= 3,
                    "glyph '{name}' (char code {}) rendered no ink: {dark} dark pixels in its cell; \
                     dotsection charstrings must produce outlines",
                    codes[cell],
                );
            }
        }
    }

    #[cfg(any(feature = "ocr", feature = "ocr-pipeline", feature = "layout-detection"))]
    #[test]
    fn ocr_page_correction_inverts_pdf_rotation() {
        assert_eq!(ocr_page_correction_degrees(0), 0);
        assert_eq!(ocr_page_correction_degrees(90), 270);
        assert_eq!(ocr_page_correction_degrees(180), 180);
        assert_eq!(ocr_page_correction_degrees(270), 90);
        assert_eq!(ocr_page_correction_degrees(360), 0);
    }

    #[cfg(any(feature = "ocr", feature = "ocr-pipeline", feature = "layout-detection"))]
    #[test]
    fn rendered_page_ocr_normalization_applies_inverse_quarter_turns() {
        let marker = image::Rgb([17, 31, 47]);
        let mut source = image::RgbImage::new(3, 2);
        source.put_pixel(0, 0, marker);
        let mut encoded = std::io::Cursor::new(Vec::new());
        source
            .write_to(&mut encoded, image::ImageFormat::Png)
            .expect("test image should encode");
        let encoded = encoded.into_inner();

        let cases = [
            (0, (3, 2), (0, 0)),
            (90, (2, 3), (0, 2)),
            (180, (3, 2), (2, 1)),
            (270, (2, 3), (1, 0)),
        ];
        for (pdf_rotation, expected_dimensions, marker_position) in cases {
            let (normalized, width, height) = normalize_rendered_page_for_ocr(encoded.clone(), 3, 2, pdf_rotation)
                .expect("in-memory OCR normalization should succeed");
            let image = image::load_from_memory(&normalized)
                .expect("normalized PNG should decode")
                .to_rgb8();

            assert_eq!((width, height), expected_dimensions, "PDF rotation {pdf_rotation}");
            assert_eq!(image.dimensions(), expected_dimensions, "PDF rotation {pdf_rotation}");
            assert_eq!(
                *image.get_pixel(marker_position.0, marker_position.1),
                marker,
                "PDF rotation {pdf_rotation}"
            );
        }
    }

    #[cfg(any(feature = "ocr", feature = "ocr-pipeline", feature = "layout-detection"))]
    #[test]
    fn test_get_page_rotations_no_rotate_attribute_yields_zeroes() {
        // Behavior-preserving: passes against both the old lopdf-based
        // implementation and the new xberg_native_pdf-delegating one (a missing
        // /Rotate defaults to 0 either way). Only the call mechanics changed
        // (an already-open `PdfDocument` instead of raw bytes), to match the
        // new `get_page_rotations` signature.
        let pdf = build_minimal_pdf_with_mediabox(612.0, 792.0);
        let doc = xberg_native_pdf::PdfDocument::from_bytes(pdf).expect("fixture PDF must open");
        assert_eq!(get_page_rotations(&doc, 1), vec![0]);
    }

    /// Build a single-page PDF whose `/Rotate` is set either on the page
    /// itself (`inherited = false`) or on its parent `/Pages` node
    /// (`inherited = true`), for pinning [`get_page_rotations`]' inheritance
    /// and value-normalization contract. Mirrors the equivalent fixture
    /// builder in `extractors::pdf::layout_runner`'s test module, which is
    /// private to that module and out of scope to reuse from here.
    #[cfg(any(feature = "ocr", feature = "ocr-pipeline", feature = "layout-detection"))]
    fn build_pdf_with_rotate(rotation: i64, inherited: bool) -> Vec<u8> {
        use lopdf::{Document, Object, Stream, dictionary};

        let mut document = Document::with_version("1.5");
        let pages_id = document.new_object_id();
        let page_id = document.new_object_id();
        let content_id = document.add_object(Stream::new(dictionary! {}, Vec::new()));

        let mut page = dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "MediaBox" => vec![0.into(), 0.into(), 200.into(), 100.into()],
            "Resources" => dictionary! {},
            "Contents" => content_id,
        };
        if !inherited {
            page.set("Rotate", rotation);
        }
        document.objects.insert(page_id, Object::Dictionary(page));

        let mut pages = dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
        };
        if inherited {
            pages.set("Rotate", rotation);
        }
        document.objects.insert(pages_id, Object::Dictionary(pages));

        let catalog_id = document.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        document.trailer.set("Root", catalog_id);

        let mut bytes = Vec::new();
        document.save_to(&mut bytes).expect("fixture PDF must serialize");
        bytes
    }

    #[cfg(any(feature = "ocr", feature = "ocr-pipeline", feature = "layout-detection"))]
    #[test]
    fn test_get_page_rotations_own_rotate_is_read() {
        // Behavior-preserving for a normal, in-spec /Rotate: both the old
        // and new implementations resolve a page's own /Rotate to itself.
        let pdf = build_pdf_with_rotate(90, false);
        let doc = xberg_native_pdf::PdfDocument::from_bytes(pdf).expect("fixture PDF must open");
        assert_eq!(get_page_rotations(&doc, 1), vec![90]);
    }

    #[cfg(any(feature = "ocr", feature = "ocr-pipeline", feature = "layout-detection"))]
    #[test]
    fn test_get_page_rotations_inherited_rotate_is_resolved_from_parent() {
        // Behavior-preserving: both the old /Parent-walking implementation
        // and the new xberg_native_pdf delegate correctly resolve a /Rotate set
        // only on the ancestor /Pages node (ISO 32000-1 SS7.7.3.4).
        let pdf = build_pdf_with_rotate(90, true);
        let doc = xberg_native_pdf::PdfDocument::from_bytes(pdf).expect("fixture PDF must open");
        assert_eq!(get_page_rotations(&doc, 1), vec![90]);
    }

    #[cfg(any(feature = "ocr", feature = "ocr-pipeline", feature = "layout-detection"))]
    #[test]
    fn test_get_page_rotations_negative_rotate_normalizes_to_positive_equivalent() {
        // Behavior-preserving: the old `rem_euclid(360)` fold and the new
        // `((raw % 360) + 360) % 360` fold both normalize -90 to 270.
        let pdf = build_pdf_with_rotate(-90, false);
        let doc = xberg_native_pdf::PdfDocument::from_bytes(pdf).expect("fixture PDF must open");
        assert_eq!(get_page_rotations(&doc, 1), vec![270]);
    }

    #[cfg(any(feature = "ocr", feature = "ocr-pipeline", feature = "layout-detection"))]
    #[test]
    fn test_get_page_rotations_non_multiple_of_90_folds_to_zero() {
        // Pins the deliberate behavior CHANGE documented on `get_page_rotations`:
        // ISO 32000-1 SS7.7.3.3 requires /Rotate to be a multiple of 90, so 135 is
        // out-of-spec. The old lopdf-based code stored it verbatim as 135 (only
        // `rem_euclid`-folded for sign), which this test would NOT have caught
        // under the old signature since 135 % 360 == 135 either way -- had that
        // code been adapted to this fixture it would assert `vec![135]`, not
        // `vec![0]`. This test only compiles and passes against the new,
        // xberg_native_pdf-delegating implementation, which folds any non-multiple of 90
        // to 0 (treating it the same as a missing /Rotate) rather than forwarding
        // a degree value nothing downstream can act on.
        let pdf = build_pdf_with_rotate(135, false);
        let doc = xberg_native_pdf::PdfDocument::from_bytes(pdf).expect("fixture PDF must open");
        assert_eq!(get_page_rotations(&doc, 1), vec![0]);
    }

    #[cfg(any(feature = "ocr", feature = "ocr-pipeline", feature = "layout-detection"))]
    #[test]
    fn test_get_page_rotations_page_index_beyond_tree_defaults_to_zero() {
        // Replaces the old `test_get_page_rotations_unparsable_bytes_yield_zeroes`:
        // that test fed raw unparsable bytes straight into `get_page_rotations`,
        // which no longer accepts raw bytes at all -- a document that fails to
        // open now errors at `xberg_native_pdf::PdfDocument::from_bytes` (already
        // covered by `test_pdf_page_count_invalid_pdf_errors`), before this
        // function is ever reached. What this function itself can still fail to
        // resolve, per page, is a page index the tree (and its scanning
        // fallback) cannot find -- exercised here by asking for 3 pages from a
        // document that only has 1. Each failing lookup must default to 0,
        // matching this function's previous contract of defaulting to 0 on any
        // per-page resolution failure.
        let pdf = build_minimal_pdf_with_mediabox(612.0, 792.0);
        let doc = xberg_native_pdf::PdfDocument::from_bytes(pdf).expect("fixture PDF must open");
        assert_eq!(get_page_rotations(&doc, 3), vec![0, 0, 0]);
    }

    /// #697: the PDF engine's `log` target follows the fork's own `[lib] name`, so when it was
    /// republished as `xberg-native` every glyph-drop warning silently stopped being captured
    /// -- for twelve days, in production, not just in tests. The prefix this filter matches is
    /// therefore load-bearing and easy to break invisibly, so it is pinned directly here rather
    /// than only through the render-path tests that depend on it.
    #[test]
    fn engine_log_targets_are_captured_under_both_the_current_and_former_crate_names() {
        assert!(
            is_pdf_engine_target(&format!("{ENGINE_LOG_TARGET_ROOT}::rendering::page_renderer")),
            "the CURRENT engine crate name must be captured -- this is the assertion that was \
             failing in production"
        );
        assert!(
            is_pdf_engine_target(&format!("{ENGINE_LOG_TARGET_ROOT}::fonts")),
            "explicit-target sites inside the engine must be captured too"
        );
        assert!(
            is_pdf_engine_target(ENGINE_LOG_TARGET_ROOT),
            "the FORMER crate name must stay accepted so a revert or a rename back is not a \
             silent regression"
        );
        assert!(
            !is_pdf_engine_target("tower::buffer::worker"),
            "unrelated crates must not be captured: this sink owns the process's only log \
             backend, so anything it accepts is diverted from every other consumer"
        );
        assert!(
            !is_pdf_engine_target("xberg::extractors::pdf"),
            "xberg's own records must not be captured"
        );
    }

    /// GH#1548: `render_page_capturing_glyph_drops` used to exclude any captured cause
    /// containing the literal substring "PDF spec compliant" -- originally meant only to skip
    /// the Latin-1-fallback message, which is now emitted at TRACE and never reaches this
    /// capture at all (see `EngineWarningCapture::interested`, gated on `<= Level::WARN`).
    /// A genuine WARN-level engine event whose text incidentally shares that phrase must still
    /// surface as a `ProcessingWarning` rather than being silently dropped by a substring trap.
    #[test]
    fn genuine_warning_sharing_the_old_substring_is_not_silently_excluded() {
        assert!(
            install_pdf_render_diagnostics(),
            "no other component should own the tracing dispatcher in this test binary"
        );
        let _ = take_xberg_native_pdf_render_warnings();

        let result = render_page_capturing_glyph_drops(0, || {
            tracing::warn!(
                target: "xberg_native_pdf::fonts",
                "a genuine problem that happens to mention PDF spec compliant wording"
            );
            Ok(xberg_native_pdf::rendering::RenderedImage {
                data: vec![0u8; 4],
                width: 1,
                height: 1,
                format: xberg_native_pdf::rendering::ImageFormat::RawRgba8,
            })
        });
        assert!(result.is_ok(), "capturing a warning must not change the render outcome");

        let warnings = take_xberg_native_pdf_render_warnings();
        assert_eq!(
            warnings.len(),
            1,
            "a captured WARN-level engine event must be reported even when its text incidentally \
             contains the retired Latin-1-fallback substring; got: {warnings:?}"
        );
    }

    fn blank_render() -> std::result::Result<xberg_native_pdf::rendering::RenderedImage, xberg_native_pdf::Error> {
        Ok(xberg_native_pdf::rendering::RenderedImage {
            data: vec![0u8; 4],
            width: 1,
            height: 1,
            format: xberg_native_pdf::rendering::ImageFormat::RawRgba8,
        })
    }

    /// GH#1794: only an engine warning about a glyph the render could not paint is
    /// reported as missing glyph ink. A font-load notice keeps its own cause.
    #[test]
    fn only_a_dropped_glyph_is_reported_as_missing_glyph_ink() {
        assert!(
            install_pdf_render_diagnostics(),
            "no other component should own the tracing dispatcher in this test binary"
        );
        let _ = take_xberg_native_pdf_render_warnings();

        let result = render_page_capturing_glyph_drops(0, || {
            tracing::warn!(
                target: "xberg_native_pdf::fonts",
                operation = "load_font",
                error_code = "type3_font",
                "using Type 3 glyph-name fallback"
            );
            tracing::warn!(
                target: "xberg_native_pdf::fonts",
                operation = "render_glyph",
                error_code = "glyph_dropped",
                "glyph rendering omitted content"
            );
            blank_render()
        });
        assert!(result.is_ok(), "capturing a warning must not change the render outcome");

        let warnings = take_xberg_native_pdf_render_warnings();
        assert_eq!(warnings.len(), 2, "both warnings are reported; got: {warnings:?}");
        let notice = warnings
            .iter()
            .find(|w| w.message.contains("using Type 3 glyph-name fallback"))
            .expect("the font-load notice is reported with its cause");
        assert!(
            !notice.message.contains("glyph ink is missing"),
            "a font-load notice drops no glyph, got: {}",
            notice.message
        );
        assert!(notice.message.starts_with("Page 1 "), "got: {}", notice.message);
        let dropped = warnings
            .iter()
            .find(|w| w.message.contains("glyph rendering omitted content"))
            .expect("the dropped glyph is reported");
        assert!(
            dropped.message.contains("glyph ink is missing"),
            "a dropped glyph is missing ink, got: {}",
            dropped.message
        );
    }

    /// The engine end to end: a page set in a Type 3 font logs the glyph-name fallback
    /// while loading the font, and every glyph still renders, so no warning may say the
    /// ink is missing.
    #[test]
    fn a_type3_font_page_reports_no_missing_glyph_ink() {
        assert!(
            install_pdf_render_diagnostics(),
            "no other component should own the tracing dispatcher in this test binary"
        );
        let _ = take_xberg_native_pdf_render_warnings();

        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/pdf/regressions/type3/gh1782-2-readable-lines.pdf");
        let pdf = std::fs::read(&path).expect("read the Type 3 fixture");
        let png = render_pdf_page_to_png(&pdf, 0, Some(72), None).expect("the Type 3 page renders");
        assert!(!png.is_empty());

        let warnings = take_xberg_native_pdf_render_warnings();
        assert!(
            warnings
                .iter()
                .any(|w| w.message.contains("using Type 3 glyph-name fallback")),
            "the fixture must log the Type 3 notice for this test to mean anything; got: {warnings:?}"
        );
        assert!(
            warnings.iter().all(|w| !w.message.contains("glyph ink is missing")),
            "no glyph was dropped, so no warning may report missing ink; got: {warnings:?}"
        );
    }
}
