#[cfg(all(feature = "ocr", feature = "pdf", not(target_arch = "wasm32")))]
#[path = "pipeline/sparse_glyph_read.rs"]
mod sparse_glyph_read;

#[cfg(all(
    feature = "pdf",
    feature = "ocr",
    feature = "tokio-runtime",
    not(paddle_ocr),
    not(target_arch = "wasm32")
))]
#[path = "pipeline/direct_rgb_budget_recovery.rs"]
mod direct_rgb_budget_recovery;

#[cfg(all(
    any(feature = "ocr", feature = "ocr-pipeline"),
    feature = "pdf",
    any(
        paddle_ocr,
        all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32"))
    )
))]
#[path = "pipeline/per_page_flat_metadata.rs"]
mod per_page_flat_metadata;

#[cfg(all(any(feature = "ocr", feature = "ocr-wasm"), feature = "layout-detection"))]
use super::document::assemble_ocr_page_paragraphs;
#[cfg(all(
    any(feature = "ocr", feature = "ocr-pipeline"),
    any(
        feature = "ocr",
        feature = "ocr-wasm",
        feature = "pdf",
        not(feature = "layout-detection")
    )
))]
use super::document::resolved_ocr_layout_dimensions;
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
#[path = "pipeline/mixed_plain.rs"]
mod mixed_plain;
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[path = "pipeline/page_ocr/mod.rs"]
mod page_ocr;
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
use super::document::{
    accepted_ocr_page_replacements, apply_ocr_page_replacements, apply_ocr_text_list_fallback,
    fill_unstructured_ocr_pages, heuristically_restructured_ocr_pages,
};
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(in crate::extractors::pdf) use mixed_plain::render_mixed_plain_pages;
// Read only by the two OCR-paragraph assembly blocks below -- one gated on
// `layout-detection` *with* `ocr`/`ocr-wasm`, the other on `not(layout-detection)`. With
// `layout-detection` and neither OCR frontend (the `formula-recognition,pdf` CI leg) both
// blocks compile out, so importing these under the enclosing function's plain
// `any(ocr, ocr-pipeline)` gate is an unused import. ~keep
#[cfg(all(
    any(feature = "ocr", feature = "ocr-pipeline"),
    feature = "pdf",
    any(
        paddle_ocr,
        all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32"))
    )
))]
use super::document::merge_structured_ocr_pages_into_internal_document;
#[cfg(all(
    any(feature = "ocr", feature = "ocr-pipeline"),
    any(feature = "ocr", feature = "ocr-wasm", not(feature = "layout-detection"))
))]
use super::document::{apply_ocr_layout_content_filter, ocr_points_per_pixel};
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
use super::document::{
    build_mixed_ocr_page_document, build_pipeline_ocr_page_document, carry_page_ocr_payload_forward,
    formula_bbox_to_page_points, ocr_margin_filter_capability_warning, public_ocr_elements_for_pdf_page,
    rescale_ocr_bboxes_to_page_points, should_use_document_processing, split_document_global_ocr_structure_by_page,
    undo_auto_rotate_point,
};
#[cfg(all(
    any(feature = "ocr", feature = "ocr-pipeline"),
    feature = "pdf",
    feature = "layout-detection"
))]
use super::document::{detection_for_mixed_route_page, single_stage_pipeline_for_layout};
// Same two blocks as `apply_ocr_layout_content_filter` above, but their uses sit inside
// the blocks' nested `#[cfg(feature = "pdf")]` margin-filter scopes. ~keep
#[cfg(all(
    any(feature = "ocr", feature = "ocr-pipeline"),
    feature = "pdf",
    any(feature = "ocr", feature = "ocr-wasm", not(feature = "layout-detection"))
))]
use super::document::{filter_ocr_paragraphs_by_page_margins, ocr_paragraphs_plain_text};
#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
use super::document::{
    recognized_table_to_public_table, scale_detection_to_dimensions, scale_detection_to_ocr_coordinates,
    transform_ocr_elements_to_render_space,
};
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
use super::rendering::EncodedPage;
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
use super::rendering::{
    OCR_PNG_ENCODE_BYTES_PER_PIXEL, OCR_PNG_ENCODE_FIXED_BYTES, PreRenderedPageGeometry, XObjectRecoveryOutcome,
    clone_rgb_for_png_encode, fallback_render_document, open_pdf_for_full_ocr, open_pdf_for_page_ocr,
    page_dimensions_pt, page_needs_xobject_fallback, pre_rendered_page_geometry, pre_rendered_page_source_dpi,
    recover_page_text_from_bounded_image_xobjects, recover_page_text_from_image_xobjects,
    render_full_pdf_ocr_batch_with_dpi, render_selected_pages_with_image_config, share_rendered_page_images,
    single_block_for_ocr_page, valid_page_indices, validate_png_encode_pages_individually,
    whole_page_raster_for_ocr_page,
};
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
use super::scoring::{
    NativeTextStats, OcrPageNoiseVerdict, accept_or_reject_ocr_page, compute_quality_score, mean_text_conf_of,
    ocr_content_is_repairable_prose, page_ocr_confidence, pipeline_stage_score, repair_ocr_list_markers,
    repair_ocr_numeric_tokens, word_count_of,
};
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
use std::borrow::Cow;

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
use crate::core::config::ExtractionConfig;
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
use crate::core::config::OcrQualityThresholds;
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
use image::ImageEncoder;
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
use image::codecs::png::PngEncoder;
#[cfg(all(
    any(feature = "ocr", feature = "ocr-pipeline"),
    feature = "pdf",
    feature = "tokio-runtime",
    not(target_arch = "wasm32")
))]
use rayon::prelude::*;
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
use std::io::Cursor;

/// The native layer of a page when it holds no lost-glyph marker of either kind. ~keep
#[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
fn clean_ink_layer_of<'a>(layers: Option<&'a ahash::AHashMap<u32, String>>, page: &u32) -> Option<&'a str> {
    layers
        .and_then(|layers| layers.get(page))
        .map(String::as_str)
        .filter(|layer| !layer.contains(super::super::unmapped_glyphs::is_lost_glyph))
}

/// Whether `OcrConfig::numeric_repair` (GH#1789) is turned on for this extraction. Split out
/// from the call site as its own function so the config-wiring path can be exercised in a unit
/// test without running a real OCR page (see `pipeline_tests::numeric_repair_enabled_reads_the_
/// ocr_config_flag` and its sibling `..._defaults_to_disabled`).
///
/// ~keep Also false when the OCR run returns markup instead of prose: on this route the repair's
/// only target is each page's OCR text, which under `output_format = "hocr"` or `"tsv"` *is* the
/// markup, and the separator rule re-punctuates its bare coordinate integers (GH#1836). See
/// [`ocr_content_is_repairable_prose`].
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(crate) fn numeric_repair_enabled(config: &ExtractionConfig) -> bool {
    config
        .ocr
        .as_ref()
        .is_some_and(|ocr| ocr.numeric_repair && ocr_content_is_repairable_prose(ocr))
}

/// Whether the force-OCR PDF route may run TATR table recognition.
///
/// ~keep `TableModel::Disabled` is the only value that turns it off here. The SLANet variants are
/// wired only on the native PDF route (`pdf::structure::pipeline`), so treating them as "not TATR"
/// on this route would remove table recognition outright rather than substitute a model. Before
/// this the route gated on `layout_detections.is_some()` alone, and `table_model = "disabled"` was
/// silently ignored on every force-OCR extraction (xberg-io/xberg#1813).
// ~keep The cfg is the single call site's: the `tatr_model` checkout inside
// `extract_with_ocr_for_page` (`any(ocr, ocr-pipeline)`) under its `layout-detection` gate.
// `layout-detection` alone would leave this dead on the `formula-recognition,pdf` leg.
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "layout-detection"))]
pub(super) fn pdf_ocr_table_recognition_enabled(config: &ExtractionConfig) -> bool {
    config
        .layout
        .as_ref()
        .is_none_or(|layout| layout.table_model != crate::core::config::layout::TableModel::Disabled)
}

/// Whether the force-OCR PDF route needs a TATR model: table recognition is on and layout
/// detection found a table region on some page. TATR reads only `Table` regions, so a document
/// without one would load the model for nothing.
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "layout-detection"))]
pub(super) fn pdf_ocr_needs_table_model(
    config: &ExtractionConfig,
    layout_detections: Option<&[crate::layout::DetectionResult]>,
) -> bool {
    let has_table_region = layout_detections.is_some_and(|pages| {
        pages
            .iter()
            .flat_map(|page| &page.detections)
            .any(|detection| detection.class_name == crate::layout::LayoutClass::Table)
    });
    has_table_region && pdf_ocr_table_recognition_enabled(config)
}

/// ~keep The warning for a force-OCR extraction whose table model could not be checked out although
/// layout detection found table regions and table recognition is on. `None` when a model is held,
/// recognition is off, or no page has a table region.
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "layout-detection"))]
pub(super) fn tatr_unavailable_warning(
    config: &ExtractionConfig,
    layout_detections: Option<&[crate::layout::DetectionResult]>,
    model_held: bool,
) -> Option<crate::types::ProcessingWarning> {
    (!model_held && pdf_ocr_needs_table_model(config, layout_detections))
        .then(|| crate::layout::table_model_unavailable_warning("pdf_tables", "TATR"))
}

/// GH#1789's numeric-token repair, applied to every string representation of an OCR'd page that
/// `merge_structured_ocr_pages_into_internal_document` carries into the final document.
///
/// The flat per-page strings (`ocr_results`) are all a Plain-format extraction renders. Every other
/// renderer rebuilds each OCR'd page from these per-page documents instead
/// (`extractors::pdf::select_pdf_document`), so a repair confined to the flat string is discarded
/// for exactly the output formats that carry tables (GH#1840). Same argument already written down
/// for the standalone-image route on
/// `extractors::image::apply_numeric_repair_to_standalone_image_ocr`.
///
/// `Table::columns` is repaired alongside `cells` because it is a copy of the header row, stamped
/// by `assign_deterministic_table_ids` inside the document-global heuristic's own
/// `extract_document_structure_from_segments` call -- repairing `cells` alone would leave the two
/// disagreeing. `prebuilt_ocr_elements` is deliberately not repaired: those are the backend's raw
/// per-word boxes, and inserting a separator into one word's text would put it out of step with the
/// box measured for it. ~keep
///
/// ~keep Must run *after* the document-global heading/list block, not beside the flat-string
/// repair: that block rebuilds every page still in the map from `ocr_page_paragraphs` and would
/// discard anything written into `elements` earlier.
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) fn apply_numeric_repair_to_structured_ocr_pages(
    structured_ocr_pages: &mut ahash::AHashMap<u32, crate::types::internal::InternalDocument>,
) {
    for page_document in structured_ocr_pages.values_mut() {
        for element in &mut page_document.elements {
            repair_numeric_tokens_in_element(element);
        }
        for table in &mut page_document.tables {
            repair_numeric_tokens_in_table(table);
        }
    }
}

/// GH#1789's numeric-token repair for the whole-document OCR route.
///
/// The counterpart of [`apply_numeric_repair_to_structured_ocr_pages`] for the route `force_ocr`,
/// the near-empty `Auto` fallback, and the OCR gate's whole-document fallback all take
/// (`extractors::pdf::run_ocr_with_layout`). That route yields one document rather than a per-page
/// map and had no repair at all, so `{numeric_repair: true, force_ocr_pages: [1, 2]}` repaired those
/// pages while `{numeric_repair: true, force_ocr: true}` repaired nothing, from the same flag.
/// GH#1892.
///
/// `document.tables` and `tables` are repaired separately because they are separate objects by the
/// time this runs, not two views of one: `run_ocr_with_layout` returns the table list alongside the
/// document and `select_pdf_document` can carry either forward.
///
/// ~keep Must run BEFORE formula recognition, not after: `recognize_pdf_formula_regions` rewrites
/// formula regions into LaTeX, and the separator rule re-punctuates bare integers in markup exactly
/// as it does in an hOCR `bbox` (GH#1836).
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(crate) fn apply_numeric_repair_to_whole_document_ocr(
    text: &mut String,
    page_texts: &mut [String],
    document: Option<&mut crate::types::internal::InternalDocument>,
    tables: &mut [crate::types::Table],
    config: &ExtractionConfig,
) {
    let boundaries =
        super::super::ocr_page_body_spans(text, page_texts, config).filter(|boundaries| !boundaries.is_empty());
    if boundaries.is_none()
        && let Cow::Owned(repaired) = repair_ocr_numeric_tokens(text)
    {
        *text = repaired;
    }
    // Repair matching page spans from the end so length changes preserve earlier offsets. ~keep
    for (index, page_text) in page_texts.iter_mut().enumerate().rev() {
        if let Cow::Owned(repaired) = repair_ocr_numeric_tokens(page_text) {
            if let Some(boundaries) = &boundaries {
                let boundary = &boundaries[index];
                text.replace_range(boundary.byte_start..boundary.byte_end, &repaired);
            }
            *page_text = repaired;
        }
    }
    if let Some(document) = document {
        for element in &mut document.elements {
            repair_numeric_tokens_in_element(element);
        }
        for table in &mut document.tables {
            repair_numeric_tokens_in_table(table);
        }
    }
    for table in tables {
        repair_numeric_tokens_in_table(table);
    }
}

/// Repair one element's text without letting a byte-range annotation slide off the words it marks.
///
/// ~keep `repair_ocr_numeric_tokens` changes byte lengths and reports no offset map, so an
/// annotation covering only part of the text cannot be re-anchored across it; such an element keeps
/// its unrepaired text rather than silently mismarking. A whole-text span is exact, and is the shape
/// `pdf::structure::assembly` emits for an OCR paragraph (`para.is_bold`, and a Caption's italic).
/// The standalone-image route's equivalent has no such guard because its OCR elements carry no
/// annotations at all (see `extractors::image`); these do.
// ~keep cfg widened from `all(any(ocr, ocr-pipeline), pdf)` to the union of its call sites'
// cfgs: `apply_numeric_repair_to_whole_document_ocr` is reached from `run_ocr_with_layout`, which
// is gated on `any(ocr, ocr-pipeline)` alone.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn repair_numeric_tokens_in_element(element: &mut crate::types::internal::InternalElement) {
    let Cow::Owned(repaired) = repair_ocr_numeric_tokens(&element.text) else {
        return;
    };
    let original_len = element.text.len() as u32;
    if element
        .annotations
        .iter()
        .any(|annotation| annotation.start != 0 || annotation.end != original_len)
    {
        return;
    }
    let repaired_len = repaired.len() as u32;
    for annotation in &mut element.annotations {
        annotation.end = repaired_len;
    }
    element.text = repaired;
}

/// Repair every string a reconstructed OCR table carries: the cells, the header-row copy in
/// `columns`, and the `markdown` baked from the cells.
// ~keep cfg widened for the same reason as `repair_numeric_tokens_in_element` above.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn repair_numeric_tokens_in_table(table: &mut crate::types::Table) {
    for row in &mut table.cells {
        for cell in row.iter_mut() {
            if let Cow::Owned(repaired) = repair_ocr_numeric_tokens(cell.as_str()) {
                *cell = repaired;
            }
        }
    }
    if let Some(columns) = table.columns.as_mut() {
        for column in columns.iter_mut() {
            if let Cow::Owned(repaired) = repair_ocr_numeric_tokens(column.as_str()) {
                *column = repaired;
            }
        }
    }
    if let Cow::Owned(repaired) = repair_ocr_numeric_tokens(&table.markdown) {
        table.markdown = repaired;
    }
}

/// Page-failure text for `error`: its display plus the variant names of its `source()` chain.
///
/// Only leading identifiers of each source's `Debug` output are kept, so the typed cause stays
/// distinguishable in the returned error and failure record.
/// The outer error message is supplied by the backend. ~keep
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(crate) fn describe_page_failure(error: &crate::XbergError) -> String {
    use std::error::Error as _;
    const MAX_DEPTH: usize = 8;
    const MAX_IDENT: usize = 64;
    let mut causes: Vec<String> = Vec::new();
    let mut current = error.source();
    while let Some(source) = current {
        if causes.len() == MAX_DEPTH {
            break;
        }
        let debug = format!("{source:?}");
        let ident: String = debug
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .take(MAX_IDENT)
            .collect();
        if ident.chars().next().is_some_and(|c| c.is_ascii_uppercase()) {
            causes.push(ident);
        } else {
            causes.push("<unnamed>".to_string());
        }
        current = source.source();
    }
    if causes.is_empty() {
        error.to_string()
    } else {
        format!("{error} [cause chain: {}]", causes.join(" > "))
    }
}

#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
fn push_mixed_ocr_page_failure_warning(
    warnings: &mut Vec<crate::types::ProcessingWarning>,
    failures: &mut Vec<crate::types::OcrPageFailure>,
    page_idx: usize,
    error: &crate::XbergError,
    recovered: bool,
) {
    crate::core::diagnostics::push_warning_deduped(
        warnings,
        crate::types::ProcessingWarning {
            source: std::borrow::Cow::Borrowed("ocr"),
            message: std::borrow::Cow::Owned(if recovered {
                format!(
                    "OCR of page {} failed ({error}); the page's native text was kept.",
                    page_idx + 1
                )
            } else {
                format!(
                    "OCR of page {} failed ({error}); the page had no native text to retain.",
                    page_idx + 1
                )
            }),
        },
    );
    merge_ocr_page_failures(
        failures,
        vec![crate::types::OcrPageFailure {
            page: (page_idx + 1) as u32,
            error: error.to_string(),
            recovered,
        }],
    );
}

/// An OCR run that failed, with the record of each page it failed on.
///
/// The list is empty when the run failed before it read a page.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[derive(Debug)]
pub(crate) struct FailedOcrRun {
    pub(crate) error: crate::XbergError,
    pub(crate) page_failures: Vec<crate::types::OcrPageFailure>,
    /// Count from the single-page native-ink rejection, when that caused the run to fail.
    pub(crate) unrecovered_glyphs_on_page_one: usize,
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(crate) type OcrRunResult<T> = Result<T, FailedOcrRun>;

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
impl From<crate::XbergError> for FailedOcrRun {
    fn from(error: crate::XbergError) -> Self {
        Self {
            error,
            page_failures: Vec::new(),
            unrecovered_glyphs_on_page_one: 0,
        }
    }
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
impl From<FailedOcrRun> for crate::XbergError {
    fn from(run: FailedOcrRun) -> Self {
        run.error
    }
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
impl std::fmt::Display for FailedOcrRun {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.error.fmt(formatter)
    }
}

/// The error recorded for a page the extraction time limit stopped before it was read.
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(in crate::extractors::pdf) const TIME_LIMIT_UNREAD_PAGE_ERROR: &str =
    "the extraction time limit was reached before the page was read";

/// How long pages already in flight may keep running once the extraction time limit has asked
/// the page loop to stop: a third of the grace the caller gets for the pages it holds, which leaves
/// the rest of it for assembling the result. A batch is one OCR task per worker, and under load a
/// task can take far longer than the grace that is left, so the batch is not awaited to its end.
///
/// Abandoning a page stops waiting for it, not its blocking thread. Native Tesseract recognition
/// polls the stop between words and returns early, but the page's other native work, such as layout
/// analysis, runs to its end on that thread. No page is started after the stop, so the work left over
/// is bounded by one batch.
#[cfg(all(
    any(feature = "ocr", feature = "ocr-pipeline"),
    feature = "pdf",
    feature = "tokio-runtime",
    not(target_arch = "wasm32")
))]
const IN_FLIGHT_PAGE_GRACE: std::time::Duration =
    std::time::Duration::from_millis((crate::core::extractor::time_limit::PARTIAL_RESULT_GRACE.as_millis() / 3) as u64);

/// How often a batch that is being awaited looks at the stop request.
#[cfg(all(
    any(feature = "ocr", feature = "ocr-pipeline"),
    feature = "pdf",
    feature = "tokio-runtime",
    not(target_arch = "wasm32")
))]
const STOP_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(100);

/// What a task of the batch's page task set returns: the read of a page, a sparse-glyph trial
/// whose page is read at normal density when the trial is declined, or the render that requeues
/// such a page.
#[cfg(all(
    any(feature = "ocr", feature = "ocr-pipeline"),
    feature = "pdf",
    feature = "tokio-runtime",
    not(target_arch = "wasm32")
))]
enum PageTaskOutput {
    Read(
        usize,
        Box<(
            crate::Result<OcrPipelineOutput>,
            Option<crate::cancellation::CancellationToken>,
        )>,
    ),
    #[cfg(all(paddle_ocr, feature = "ocr"))]
    Trial(usize, crate::Result<Option<sparse_glyph_read::AcceptedTrial>>),
    #[cfg(all(paddle_ocr, feature = "ocr"))]
    RetryRender(usize, Box<sparse_glyph_read::RetryRenderOutcome>),
}

/// What awaiting the next page task of a batch produced.
#[cfg(all(
    any(feature = "ocr", feature = "ocr-pipeline"),
    feature = "pdf",
    feature = "tokio-runtime",
    not(target_arch = "wasm32")
))]
enum NextPageTask<T> {
    Finished(Result<T, tokio::task::JoinError>),
    /// The in-flight grace passed after the stop request: the tasks still running were aborted.
    Abandoned,
    Cancelled,
    Drained,
}

/// Await the next finished page task. Once `token` has a stop requested, the tasks still running
/// get [`IN_FLIGHT_PAGE_GRACE`] from the moment the request was first seen, then they are aborted
/// and the caller records their pages as unread. Explicit cancellation aborts the wait at the
/// next poll.
#[cfg(all(
    any(feature = "ocr", feature = "ocr-pipeline"),
    feature = "pdf",
    feature = "tokio-runtime",
    not(target_arch = "wasm32")
))]
async fn next_page_task<T: 'static>(
    join_set: &mut tokio::task::JoinSet<T>,
    token: Option<&crate::cancellation::CancellationToken>,
    abandon_at: &mut Option<tokio::time::Instant>,
) -> NextPageTask<T> {
    loop {
        let joined = tokio::time::timeout(STOP_POLL_INTERVAL, join_set.join_next()).await;
        if token.is_some_and(crate::cancellation::CancellationToken::is_cancelled) {
            join_set.abort_all();
            return NextPageTask::Cancelled;
        }
        match joined {
            Ok(Some(joined)) => return NextPageTask::Finished(joined),
            Ok(None) => return NextPageTask::Drained,
            Err(_elapsed) => {
                if token.is_some_and(crate::cancellation::CancellationToken::stop_requested) {
                    let deadline =
                        *abandon_at.get_or_insert_with(|| tokio::time::Instant::now() + IN_FLIGHT_PAGE_GRACE);
                    if tokio::time::Instant::now() >= deadline {
                        join_set.abort_all();
                        return NextPageTask::Abandoned;
                    }
                }
            }
        }
    }
}

#[cfg(all(
    any(feature = "ocr", feature = "ocr-pipeline"),
    feature = "pdf",
    feature = "tokio-runtime",
    not(target_arch = "wasm32")
))]
fn capture_ocr_completion<T>(
    result: crate::Result<T>,
    token: Option<&crate::cancellation::CancellationToken>,
) -> (crate::Result<T>, Option<crate::cancellation::CancellationToken>) {
    if token.is_some_and(|token| token.is_cancelled()) {
        return (Err(crate::XbergError::Cancelled), None);
    }
    // A stop can suppress the retry without making the completed backend error a timeout. ~keep
    let expired = token.filter(|token| {
        matches!(&result, Err(error) if !matches!(error, crate::XbergError::Cancelled)) && token.deadline_elapsed()
    });
    (result, expired.cloned())
}

/// Record `pages` (0-based) as unread: the extraction time limit stopped the run before they were
/// read. The first of them is reported on `token`; `recovered` says whether a page's native text
/// still gives it content.
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
fn mark_pages_unread(
    token: Option<&crate::cancellation::CancellationToken>,
    pages: impl Iterator<Item = usize>,
    page_failures: &mut Vec<crate::types::OcrPageFailure>,
    failed_page_errors: &mut ahash::AHashMap<usize, crate::XbergError>,
    recovered: impl Fn(usize) -> bool,
) {
    for page_idx in pages {
        if let Some(token) = token {
            token.note_unread_page((page_idx + 1) as u32);
        }
        merge_ocr_page_failures(
            page_failures,
            vec![crate::types::OcrPageFailure {
                page: (page_idx + 1) as u32,
                error: TIME_LIMIT_UNREAD_PAGE_ERROR.to_string(),
                recovered: recovered(page_idx),
            }],
        );
        failed_page_errors.insert(
            page_idx,
            crate::XbergError::Timeout {
                elapsed_ms: 0,
                limit_ms: 0,
            },
        );
    }
}

/// The error of a run in which no page was read. Pages the time limit left unread make it the
/// time limit, whatever failed earlier; otherwise it is the error of the first failed page.
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
fn run_error_for_failed_pages(
    page_indices: &[usize],
    failed_page_errors: &mut ahash::AHashMap<usize, crate::XbergError>,
) -> crate::XbergError {
    let stopped_by_limit = page_indices.iter().find(|page_idx| {
        matches!(
            failed_page_errors.get(page_idx),
            Some(crate::XbergError::Timeout { .. })
        )
    });
    let chosen = stopped_by_limit
        .or_else(|| {
            page_indices
                .iter()
                .find(|page_idx| failed_page_errors.contains_key(page_idx))
        })
        .copied();
    chosen
        .and_then(|page_idx| failed_page_errors.remove(&page_idx))
        .unwrap_or_else(|| crate::XbergError::Ocr {
            message: "OCR failed for every requested page".to_string(),
            source: None,
        })
}

/// Add `incoming` records to `failures`. A page that already has a record keeps its first
/// error and is recovered when any of its records is.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(super) fn merge_ocr_page_failures(
    failures: &mut Vec<crate::types::OcrPageFailure>,
    incoming: Vec<crate::types::OcrPageFailure>,
) {
    for failure in incoming {
        if let Some(existing) = failures.iter_mut().find(|existing| existing.page == failure.page) {
            existing.recovered |= failure.recovered;
        } else {
            failures.push(failure);
        }
    }
}

#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
fn mixed_native_page_has_content(
    native_text: &str,
    boundaries: &[crate::types::PageBoundary],
    page_idx: usize,
) -> bool {
    boundaries
        .iter()
        .find(|boundary| boundary.page_number == (page_idx + 1) as u32)
        .and_then(|boundary| native_text.get(boundary.byte_start..boundary.byte_end))
        .is_some_and(|text| !text.trim().is_empty())
}

#[cfg(all(feature = "ocr", feature = "pdf", not(target_arch = "wasm32")))]
fn preceding_native_khmer_page(
    native_text: &str,
    boundaries: &[crate::types::PageBoundary],
    ocr_pages: &std::collections::HashSet<u32>,
    page_idx: usize,
) -> bool {
    let Ok(previous) = u32::try_from(page_idx) else {
        return false;
    };
    if previous == 0
        || ocr_pages.contains(&previous)
        || mixed_native_page_has_content(native_text, boundaries, page_idx)
    {
        return false;
    }
    boundaries
        .iter()
        .find(|boundary| boundary.page_number == previous)
        .and_then(|boundary| native_text.get(boundary.byte_start..boundary.byte_end))
        .is_some_and(super::super::unreadable_script::is_complete_khmer_retry_witness)
}

#[cfg(all(test, feature = "ocr", feature = "pdf", not(target_arch = "wasm32")))]
#[test]
fn khmer_retry_hint_stays_with_the_immediate_raster_only_neighbor() {
    let text = include_str!("../../../../tests/fixtures/pdf/khmer-cluster-2536/layers/complete.txt");
    let boundaries = [
        crate::types::PageBoundary {
            page_number: 1,
            byte_start: 0,
            byte_end: text.len(),
        },
        crate::types::PageBoundary {
            page_number: 2,
            byte_start: text.len(),
            byte_end: text.len(),
        },
    ];
    assert!(preceding_native_khmer_page(
        text,
        &boundaries,
        &std::collections::HashSet::from([2]),
        1
    ));
    assert!(!preceding_native_khmer_page(
        text,
        &boundaries,
        &std::collections::HashSet::from([2, 3]),
        2
    ));
    assert!(!preceding_native_khmer_page(
        text,
        &boundaries,
        &std::collections::HashSet::from([1, 2]),
        1
    ));
    assert!(!preceding_native_khmer_page(
        text,
        &boundaries,
        &std::collections::HashSet::from([1]),
        0
    ));
}

/// Add to `failures` the records of a pipeline run that the route for selected pages got back.
///
/// The pipeline does not see the native text, so its record says only what its stages read.
/// A page that has native text keeps it when the run returns the page blank, so its record is
/// recovered: the rule for a page whose run failed.
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
fn merge_returned_page_failures(
    failures: &mut Vec<crate::types::OcrPageFailure>,
    returned: &mut crate::types::internal::InternalDocument,
    page_text: &str,
    native_text: &str,
    boundaries: &[crate::types::PageBoundary],
) {
    for failure in &mut returned.ocr_page_failures {
        let native_kept =
            mixed_native_page_has_content(native_text, boundaries, failure.page.saturating_sub(1) as usize);
        if native_kept && !failure.recovered && page_text.trim().is_empty() {
            update_discarded_xobject_retry_warning(&mut returned.processing_warnings, failure);
        }
        failure.recovered |= native_kept;
    }
    merge_ocr_page_failures(failures, std::mem::take(&mut returned.ocr_page_failures));
}

#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
fn update_discarded_xobject_retry_warning(
    warnings: &mut [crate::types::ProcessingWarning],
    failure: &crate::types::OcrPageFailure,
) -> bool {
    let discarded = discarded_xobject_retry_warning(failure, false);
    let Some(warning) = warnings
        .iter_mut()
        .find(|warning| warning.source == discarded.source && warning.message == discarded.message)
    else {
        return false;
    };
    *warning = discarded_xobject_retry_warning(failure, true);
    true
}

#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
fn discarded_xobject_retry_warning(
    failure: &crate::types::OcrPageFailure,
    native_kept: bool,
) -> crate::types::ProcessingWarning {
    let outcome = if native_kept {
        "its embedded-image retry was discarded and the page's native text was kept"
    } else {
        "its embedded-image retry was discarded"
    };
    crate::core::diagnostics::warning(
        "ocr",
        format!("OCR of page {} failed ({}); {outcome}.", failure.page, failure.error),
    )
}

#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(in crate::extractors::pdf) fn report_native_page_recovery(
    warnings: &mut Vec<crate::types::ProcessingWarning>,
    failure: &crate::types::OcrPageFailure,
) {
    if update_discarded_xobject_retry_warning(warnings, failure) {
        return;
    }
    let no_retry = unread_page_failure_message(failure.page, &failure.error, false);
    let empty_retry = unread_page_failure_message(failure.page, &failure.error, true);
    let previous = warnings
        .iter_mut()
        .find(|warning| warning.source == "ocr" && (warning.message == no_retry || warning.message == empty_retry));
    let retry_detail = match &previous {
        Some(warning) if warning.message == empty_retry => {
            "the retry on the page's embedded image XObjects also returned no text, and "
        }
        _ => "",
    };
    let updated = crate::core::diagnostics::warning(
        "ocr",
        format!(
            "OCR of page {} failed ({}); {retry_detail}the page's native text was kept.",
            failure.page, failure.error
        ),
    );
    if let Some(warning) = previous {
        *warning = updated;
    } else {
        crate::core::diagnostics::push_warning_deduped(warnings, updated);
    }
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn unread_page_failure_message(page: u32, error: &str, empty_retry: bool) -> String {
    if empty_retry {
        format!(
            "OCR of page {page} failed ({error}); the retry on the page's embedded image XObjects also returned no text."
        )
    } else {
        format!("OCR of page {page} failed and could not be recovered: {error}")
    }
}

/// Records for an OCR run that failed as a whole: one for each page the run was asked to read.
///
/// `recovered` is true for a page that has native text.
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(crate) fn route_level_ocr_page_failures(
    pages: &[u32],
    error: &crate::XbergError,
    native_text: &str,
    boundaries: &[crate::types::PageBoundary],
) -> Vec<crate::types::OcrPageFailure> {
    let error = error.to_string();
    let mut failures: Vec<crate::types::OcrPageFailure> = Vec::with_capacity(pages.len());
    for &page in pages {
        if page == 0 || failures.iter().any(|failure| failure.page == page) {
            continue;
        }
        failures.push(crate::types::OcrPageFailure {
            page,
            error: error.clone(),
            recovered: mixed_native_page_has_content(native_text, boundaries, (page - 1) as usize),
        });
    }
    failures
}

/// Records for an OCR run of the whole document that failed, when the native text is kept.
///
/// Only a page without native text gets a record. The run read every page because the quality
/// check failed on each of them, so the check does not say which pages needed OCR. The native
/// text does: a page that has it keeps it in the result. The run's own records are used when
/// it has them, and a run without records gives one for each page. Without page boundaries the
/// pages that have native text are not known, so there is no record. A record is not
/// recovered: the output of the run is not in the result.
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(crate) fn failed_run_page_failures(
    run: &FailedOcrRun,
    page_count: u32,
    native_text: &str,
    boundaries: &[crate::types::PageBoundary],
) -> Vec<crate::types::OcrPageFailure> {
    if boundaries.is_empty() {
        return Vec::new();
    }
    let mut failures = if run.page_failures.is_empty() {
        let pages: Vec<u32> = (1..=page_count).collect();
        route_level_ocr_page_failures(&pages, &run.error, native_text, boundaries)
    } else {
        run.page_failures.clone()
    };
    failures.retain(|failure| {
        !mixed_native_page_has_content(native_text, boundaries, failure.page.saturating_sub(1) as usize)
    });
    for failure in &mut failures {
        failure.recovered = false;
    }
    failures.sort_by_key(|failure| failure.page);
    failures
}

/// Build mixed text from native extraction and per-page OCR results.
///
/// For each page boundary, if the page is in `ocr_page_numbers` (1-indexed),
/// use the OCR result; otherwise use the native text slice.
///
/// Page numbers must be >= 1 (invalid values are filtered out with a warning).
/// An `ocr` config is recommended but not required; defaults are used if absent.
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
type MixedOcrOutput = (
    String,
    ahash::AHashMap<u32, String>,
    ahash::AHashMap<u32, crate::types::internal::InternalDocument>,
    Vec<crate::types::LlmUsage>,
    Option<Vec<crate::types::ExtractedImage>>,
    Vec<crate::types::Formula>,
    ahash::AHashMap<u32, crate::types::ImagePreprocessingMetadata>,
    ahash::AHashMap<u32, crate::types::page::PageOcrConfidence>,
    Vec<crate::types::ProcessingWarning>,
    Vec<crate::types::OcrPageFailure>,
);

#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(crate) struct MixedOcrRun {
    pub(crate) output: MixedOcrOutput,
    /// The route's layout inputs for the native formula pass, keeping only the replaced pages that
    /// hold a formula region. `None` without a formula model, without layout data, on the early
    /// returns and the `Prepared` path, and when no replaced page holds a formula region.
    #[cfg(feature = "formula-recognition")]
    pub(crate) formula_layout_inputs: Option<PreparedLayoutInputs>,
}

#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
#[derive(Clone, Copy, PartialEq, Eq)]
enum AllPagesFailedPolicy {
    PreserveNative,
    ReturnError,
}

#[cfg(all(
    any(feature = "ocr", feature = "ocr-pipeline"),
    feature = "pdf",
    feature = "layout-detection"
))]
// ~keep `Prepared` is only constructed by the paddle-OCR-gated whole-document route, so the
// variant is dead on a `layout-detection + ocr-pipeline` leg without `paddle_ocr`; a
// union-of-consumers `cfg` here is what drifted and failed the 1.3.0 publish (GH#1951).
#[allow(dead_code)]
#[derive(Clone, Copy)]
enum MixedLayoutInputs<'a> {
    Resolve(Option<&'a crate::core::layout_pages::LayoutPageSelection>),
    Prepared(Option<&'a PreparedLayoutInputs>),
}

#[cfg(all(
    any(feature = "ocr", feature = "ocr-pipeline"),
    feature = "pdf",
    feature = "layout-detection"
))]
type PreparedLayoutInputs = (Vec<image::DynamicImage>, Vec<crate::layout::DetectionResult>);

#[cfg(all(
    any(feature = "ocr", feature = "ocr-pipeline"),
    feature = "pdf",
    feature = "layout-detection"
))]
type MixedLayoutOutcome = (
    Option<PreparedLayoutInputs>,
    Option<crate::types::ProcessingWarning>,
    Vec<crate::types::ProcessingWarning>,
);

#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
struct MixedOcrPageSelection<'a> {
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    probe_reads: Option<std::sync::Arc<super::read_reuse::ProbeReads>>,
    ocr: &'a [u32],
    raster_dpis: Option<std::sync::Arc<crate::pdf::render::OcrRasterDpis>>,
    single_block: &'a [u32],
    preserve_native: &'a [u32],
    /// Pages whose native layer is in a script that OCR must produce again: see
    /// `unreadable_script::ocr_dropped_native_script`.
    native_script: &'a [u32],
    /// Readable text layers a whole-document caller computed; `None` lets a selected-page run
    /// read them from its own native text.
    native_layers: Option<std::sync::Arc<ahash::AHashMap<u32, String>>>,
    hidden_layer_backend: Option<std::sync::Arc<dyn crate::plugins::OcrBackend>>,
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    single_native_type0_page: Option<&'a str>,
}

#[cfg(all(test, feature = "ocr", feature = "pdf"))]
// ~keep Test-only compatibility wrapper for the classical OCR regression suite.
pub(crate) async fn extract_mixed_ocr_native(
    native_text: &str,
    boundaries: &[crate::types::PageBoundary],
    ocr_page_numbers: &[u32],
    content: &[u8],
    config: &ExtractionConfig,
    _path: Option<&std::path::Path>,
) -> crate::Result<MixedOcrOutput> {
    extract_mixed_ocr_native_with_single_block_pages(
        native_text,
        boundaries,
        ocr_page_numbers,
        &[],
        content,
        config,
        _path,
    )
    .await
    .map(|run| run.output)
}

#[cfg(all(test, feature = "ocr", feature = "pdf"))]
pub(crate) async fn extract_mixed_ocr_native_with_single_block_pages(
    native_text: &str,
    boundaries: &[crate::types::PageBoundary],
    ocr_page_numbers: &[u32],
    single_block_page_numbers: &[u32],
    content: &[u8],
    config: &ExtractionConfig,
    _path: Option<&std::path::Path>,
) -> crate::Result<MixedOcrRun> {
    extract_mixed_ocr_native_with_layout_pages(
        native_text,
        boundaries,
        (ocr_page_numbers, single_block_page_numbers),
        content,
        config,
        None,
        (&[], &[]),
    )
    .await
}

#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(crate) async fn extract_mixed_ocr_native_with_layout_pages(
    native_text: &str,
    boundaries: &[crate::types::PageBoundary],
    selected_pages: (&[u32], &[u32]),
    content: &[u8],
    config: &ExtractionConfig,
    layout_pages: Option<&crate::core::layout_pages::LayoutPageSelection>,
    (preserve_native, native_script): (&[u32], &[u32]),
) -> crate::Result<MixedOcrRun> {
    let (ocr_page_numbers, single_block_page_numbers) = selected_pages;
    let _ = &layout_pages;
    extract_mixed_ocr_native_with_layout_inputs(
        native_text,
        boundaries,
        MixedOcrPageSelection {
            #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
            probe_reads: None,
            ocr: ocr_page_numbers,
            raster_dpis: None,
            single_block: single_block_page_numbers,
            preserve_native,
            native_script,
            native_layers: None,
            hidden_layer_backend: None,
            #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
            single_native_type0_page: None,
        },
        content,
        config,
        if config.force_ocr_pages.is_some() {
            AllPagesFailedPolicy::ReturnError
        } else {
            AllPagesFailedPolicy::PreserveNative
        },
        #[cfg(feature = "layout-detection")]
        MixedLayoutInputs::Resolve(layout_pages),
    )
    .await
    .map_err(crate::XbergError::from)
}

#[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
pub(crate) async fn extract_mixed_ocr_native_with_probe_reads(
    native_text: &str,
    boundaries: &[crate::types::PageBoundary],
    selected_pages: (&[u32], &[u32]),
    content: &[u8],
    config: &ExtractionConfig,
    layout_pages: Option<&crate::core::layout_pages::LayoutPageSelection>,
    (preserve_native, native_script, probe_reads): (
        &[u32],
        &[u32],
        Option<std::sync::Arc<super::read_reuse::ProbeReads>>,
    ),
) -> crate::Result<MixedOcrRun> {
    let _release_reads = probe_reads.as_ref().and_then(|reads| reads.release_after_final_read());
    let _ = &layout_pages;
    extract_mixed_ocr_native_with_layout_inputs(
        native_text,
        boundaries,
        MixedOcrPageSelection {
            ocr: selected_pages.0,
            raster_dpis: None,
            single_block: selected_pages.1,
            preserve_native,
            native_script,
            native_layers: None,
            hidden_layer_backend: None,
            single_native_type0_page: None,
            probe_reads,
        },
        content,
        config,
        if config.force_ocr_pages.is_some() {
            AllPagesFailedPolicy::ReturnError
        } else {
            AllPagesFailedPolicy::PreserveNative
        },
        #[cfg(feature = "layout-detection")]
        MixedLayoutInputs::Resolve(layout_pages),
    )
    .await
    .map_err(crate::XbergError::from)
}

#[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
pub(in crate::extractors::pdf) async fn probe_incomplete_hidden_layers(
    native_text: &str,
    boundaries: &[crate::types::PageBoundary],
    scanned_pages: &[u32],
    content: &[u8],
    config: &ExtractionConfig,
) -> crate::Result<Option<MixedOcrRun>> {
    if boundaries.is_empty()
        || scanned_pages.is_empty()
        || content.len() > super::hidden_layer::preview_byte_limit(config)
    {
        return Ok(None);
    }
    let Some(backend) = super::hidden_layer::stock_backend(config) else {
        return Ok(None);
    };
    let result = Box::pin(extract_mixed_ocr_native_with_layout_inputs(
        native_text,
        boundaries,
        MixedOcrPageSelection {
            #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
            probe_reads: None,
            ocr: scanned_pages,
            raster_dpis: None,
            single_block: &[],
            preserve_native: &[],
            native_script: &[],
            native_layers: None,
            hidden_layer_backend: Some(backend),
            #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
            single_native_type0_page: None,
        },
        content,
        config,
        AllPagesFailedPolicy::PreserveNative,
        #[cfg(feature = "layout-detection")]
        MixedLayoutInputs::Resolve(None),
    ))
    .await;
    super::super::ensure_pdf_not_cancelled(config)?;
    match result {
        Ok(run) if !run.output.1.is_empty() => Ok(Some(run)),
        Ok(_) => Ok(None),
        Err(failure) => {
            tracing::debug!(error = %failure.error.pdf_ocr_trace_category(), "Hidden PDF text layer probe preserved native text");
            Ok(None)
        }
    }
}

#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
fn mixed_layer_lines_to_put_first(
    layer: &str,
    independent_read: &str,
    rendered_read: &str,
    noise_verdict: bool,
    thresholds: &crate::core::config::OcrQualityThresholds,
    boxes: Option<&super::document::PageLineBoxes>,
    plain_rows: super::document::PlainOcrRows,
) -> Option<super::document::LayerFirst> {
    if !noise_verdict
        && independent_read != rendered_read
        && !super::document::ocr_confirms_layer(layer, independent_read, thresholds)
    {
        return None;
    }
    super::document::layer_lines_to_put_first(layer, rendered_read, noise_verdict, thresholds, boxes, plain_rows)
}

#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
#[cfg(all(feature = "layout-detection", feature = "tokio-runtime", not(target_arch = "wasm32")))]
type MixedPageDetection = crate::layout::DetectionResult;
// Defined only where its `not(layout-detection)` users compile: the alias itself otherwise
// outlived the `ocr`-gated users and warned as dead code on a `pdf`-only build. ~keep
#[cfg(all(
    any(feature = "ocr", feature = "ocr-pipeline"),
    feature = "pdf",
    not(feature = "layout-detection"),
    feature = "tokio-runtime",
    not(target_arch = "wasm32")
))]
type MixedPageDetection = ();

/// Which of the two direct-backend routes recorded a page read. The sequential route never ran
/// the geometry and language passes on main, so the shared recorder keeps that difference typed.
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
#[derive(Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum GeometryEvidence {
    Run,
    Skip,
}

#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
#[allow(dead_code)]
impl GeometryEvidence {
    fn concurrent() -> Self {
        Self::Run
    }

    fn sequential() -> Self {
        Self::Skip
    }

    fn runs(self) -> bool {
        matches!(self, Self::Run)
    }
}

/// Whether the layer-occurrence map may be captured for a page. A scale-only geometry read is
/// the only mapping `finish_layer_prepend` can trust, so any refusal leaves the map empty.
#[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
fn captures_matched_layer_occurrences(observed_scale_only: bool, alternative_raster: bool) -> bool {
    observed_scale_only && !alternative_raster
}

#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
struct MixedOcrContext<'a> {
    // Caller inputs.
    native_text: &'a str,
    boundaries: &'a [crate::types::PageBoundary],
    pages: MixedOcrPageSelection<'a>,
    content: &'a [u8],
    config: &'a ExtractionConfig,
    all_pages_failed_policy: AllPagesFailedPolicy,
    #[cfg(feature = "layout-detection")]
    layout_inputs: MixedLayoutInputs<'a>,

    // The selected pages and the document they are rendered from.
    #[cfg_attr(not(all(feature = "ocr", not(target_arch = "wasm32"))), allow(dead_code))]
    ocr_set: std::collections::HashSet<u32>,
    page_indices: Vec<usize>,
    render_doc: std::sync::Arc<xberg_native_pdf::PdfDocument>,
    page_count: usize,
    page_rotations: Vec<u32>,
    single_block_pages: std::sync::Arc<std::collections::HashSet<u32>>,
    probing_hidden_layer: bool,

    // Resolved routing and render configuration.
    ocr_config_resolved: crate::core::config::ocr::OcrConfig,
    ocr_config_owned: crate::core::config::ocr::OcrConfig,
    preprocessing_render_dpi: Option<i32>,
    batch_size: usize,
    capture_rasters: bool,
    effective_pipeline: Option<crate::core::config::OcrPipelineConfig>,
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    automatic_mixed_prose: bool,
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    whole_document_scan: bool,
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    orientation_retry: Option<std::sync::Arc<super::orientation_retry::OrientationRetry>>,
    pipeline_stage_config: ExtractionConfig,
    backend: Option<std::sync::Arc<dyn crate::plugins::OcrBackend>>,
    #[cfg(all(auto_rotate, feature = "ocr", not(target_arch = "wasm32")))]
    direct_prose_retry: Option<std::sync::Arc<super::orientation_retry::OrientationRetry>>,
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    allow_khmer_hint: bool,
    ocr_output_thresholds: crate::core::config::OcrQualityThresholds,
    page_layers: Option<std::sync::Arc<ahash::AHashMap<u32, String>>>,
    #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
    number_layers: Option<std::sync::Arc<ahash::AHashMap<u32, String>>>,
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    native_coverage_layers: Option<std::sync::Arc<ahash::AHashMap<u32, String>>>,
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    sparse_glyph_pages: ahash::AHashMap<usize, sparse_glyph_read::MarkedWords>,
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    sparse_glyph_images: crate::core::config::ImageExtractionConfig,
    #[cfg(all(paddle_ocr, feature = "ocr", not(target_arch = "wasm32")))]
    implicit_sparse_glyph_pages: ahash::AHashMap<usize, sparse_glyph_read::MarkedWords>,
    #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
    mixed_number_recovery: std::sync::Arc<super::number_recovery::Run>,
    #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
    texture_work: super::hidden_layer::PreviewWorkBudget,
    #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
    certified_background: bool,
    #[cfg(feature = "layout-detection")]
    resolved_layout_inputs: Option<PreparedLayoutInputs>,
    #[cfg_attr(
        not(any(feature = "ocr", all(feature = "tokio-runtime", not(target_arch = "wasm32")))),
        allow(dead_code)
    )]
    mixed_route_layout_active: bool,
    total: usize,
    raster_dpis: crate::pdf::render::OcrRasterDpis,

    // Per-page evidence and verdicts.
    noise_verdict_pages: std::collections::HashSet<u32>,
    #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
    texture_noise_by_page: ahash::AHashMap<u32, super::symbol_noise_lines::texture::Evidence>,
    #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
    paint_retention_by_page: ahash::AHashMap<u32, super::symbol_noise_lines::paint::Retention>,
    #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
    background_noise_by_page: ahash::AHashMap<u32, super::symbol_noise_background::Evidence>,
    low_confidence_pages: std::collections::HashSet<u32>,
    dictionary_noise_pages: std::collections::HashSet<u32>,

    // Per-page OCR output and the evidence that decides whether to keep it.
    #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
    khmer_supplements: ahash::AHashMap<u32, String>,
    #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
    leading_khmer_rows: ahash::AHashMap<u32, super::layer_token_recovery::LeadingKhmerRows>,
    #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
    number_primary_texts: ahash::AHashMap<u32, String>,
    ocr_results: ahash::AHashMap<u32, String>,
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    original_tesseract_raster_pages: ahash::AHashSet<u32>,
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    native_placed_words_pages: ahash::AHashMap<u32, std::collections::HashSet<String>>,
    #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
    matched_layer_occurrences: ahash::AHashMap<u32, super::document::MatchedLayerOccurrences>,
    #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
    paragraphs_in_page_points: ahash::AHashSet<u32>,
    #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
    caption_rewrites: ahash::AHashMap<u32, super::caption_conversion::CaptionRewrite>,
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    native_words_covered_pages: ahash::AHashSet<u32>,
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    native_ink_owned_pages: ahash::AHashSet<u32>,
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    alternative_raster_pages: ahash::AHashSet<u32>,
    page_mean_confidence: ahash::AHashMap<u32, f64>,
    page_dictionary_invalid_word_ratio: ahash::AHashMap<u32, f64>,
    page_word_count: ahash::AHashMap<u32, u32>,
    ocr_confidence_by_page: ahash::AHashMap<u32, crate::types::page::PageOcrConfidence>,
    structured_ocr_pages: ahash::AHashMap<u32, crate::types::internal::InternalDocument>,
    table_only_xobject_recovery_pages: ahash::AHashSet<u32>,
    ocr_page_paragraphs: ahash::AHashMap<u32, Vec<crate::pdf::structure::types::PdfParagraph>>,
    #[cfg(all(
        feature = "ocr",
        feature = "layout-detection",
        feature = "tokio-runtime",
        not(target_arch = "wasm32")
    ))]
    column_lanes_by_page: ahash::AHashMap<u32, super::column_retry::ColumnLanes>,
    preserve_ordered_layout: ahash::AHashSet<u32>,
    ocr_raster_by_page: ahash::AHashMap<u32, (u32, u32)>,
    #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
    reflow_scales: ahash::AHashMap<u32, (f32, f32)>,

    // Document-level accumulators.
    accumulated_llm_usage: Vec<crate::types::LlmUsage>,
    accumulated_formulas: Vec<crate::types::Formula>,
    accumulated_warnings: Vec<crate::types::ProcessingWarning>,
    recovery_warnings: Vec<(u32, crate::types::ProcessingWarning)>,
    discarded_recovery_pages: ahash::AHashSet<u32>,
    page_failures: Vec<crate::types::OcrPageFailure>,
    failed_page_errors: ahash::AHashMap<usize, crate::XbergError>,
    captured_rasters: Vec<crate::types::ExtractedImage>,
    preprocessing_by_page: ahash::AHashMap<u32, crate::types::ImagePreprocessingMetadata>,
    xobject_recovery_budget: crate::extractors::security::SecurityBudget,
}

#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
struct SelectedMixedOcrPages {
    ocr_set: std::collections::HashSet<u32>,
    page_indices: Vec<usize>,
    render_doc: std::sync::Arc<xberg_native_pdf::PdfDocument>,
    page_count: usize,
    page_rotations: Vec<u32>,
    single_block_pages: std::sync::Arc<std::collections::HashSet<u32>>,
    probing_hidden_layer: bool,
}

#[cfg(all(
    any(feature = "ocr", feature = "ocr-pipeline"),
    feature = "pdf",
    feature = "layout-detection"
))]
struct ResolvedMixedLayout {
    raster_dpis: crate::pdf::render::OcrRasterDpis,
    inputs: Option<PreparedLayoutInputs>,
    warning: Option<crate::types::ProcessingWarning>,
    glyph_drop_warnings: Vec<crate::types::ProcessingWarning>,
}

#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
struct MixedOcrRouting {
    ocr_config_resolved: crate::core::config::ocr::OcrConfig,
    ocr_config_owned: crate::core::config::ocr::OcrConfig,
    preprocessing_render_dpi: Option<i32>,
    capture_rasters: bool,
    effective_pipeline: Option<crate::core::config::OcrPipelineConfig>,
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    automatic_mixed_prose: bool,
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    whole_document_scan: bool,
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    orientation_retry: Option<std::sync::Arc<super::orientation_retry::OrientationRetry>>,
    pipeline_stage_config: ExtractionConfig,
    backend: Option<std::sync::Arc<dyn crate::plugins::OcrBackend>>,
    #[cfg(all(auto_rotate, feature = "ocr", not(target_arch = "wasm32")))]
    direct_prose_retry: Option<std::sync::Arc<super::orientation_retry::OrientationRetry>>,
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    allow_khmer_hint: bool,
}

/// Run the cross-page furniture pass over the mixed route's fallback paragraphs in one
/// frame: page points.
///
/// `ocr_page_paragraphs` still carries raster-pixel geometry for every page recognition
/// read from a raster (`ocr_raster_by_page`), while `page_heights` and the horizontal
/// normalizer that `strip_repeating_text_from_pages` derives from the boxes are in points.
/// Each such page is scaled into points before the strip and back afterwards, so the
/// caller's rebuild keeps the frame it already expects.
#[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
fn strip_fallback_furniture_in_page_points(
    pages: &mut [Vec<crate::pdf::structure::types::PdfParagraph>],
    page_numbers: &[u32],
    raster_by_page: &ahash::AHashMap<u32, (u32, u32)>,
    reads_sideways: impl Fn(u32) -> bool,
    page_size_pt: impl Fn(u32) -> (f32, f32),
    page_heights: &[f32],
    filter: &crate::core::config::ContentFilterConfig,
) {
    let mut scales: Vec<(u32, (f32, f32))> = Vec::new();
    for &page_number in page_numbers {
        if reads_sideways(page_number) {
            continue;
        }
        let Some(&(width_px, height_px)) = raster_by_page.get(&page_number) else {
            continue;
        };
        if width_px == 0 || height_px == 0 {
            continue;
        }
        let (width_pt, height_pt) = page_size_pt(page_number);
        if ![width_pt, height_pt]
            .into_iter()
            .all(|value| value.is_finite() && value > 0.0)
        {
            continue;
        }
        let Some(slot) = pages.get_mut((page_number - 1) as usize) else {
            continue;
        };
        let scale = (width_pt / width_px as f32, height_pt / height_px as f32);
        detached_mixed_frame::scale_paragraph_geometry(slot, scale);
        scales.push((page_number, scale));
    }

    crate::pdf::structure::pipeline::strip_repeating_text_from_pages(
        pages,
        page_heights,
        filter.include_headers,
        filter.include_footers,
    );

    for (page_number, (sx, sy)) in scales {
        if let Some(slot) = pages.get_mut((page_number - 1) as usize) {
            detached_mixed_frame::scale_paragraph_geometry(slot, (1.0 / sx, 1.0 / sy));
        }
    }
}

#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
async fn extract_mixed_ocr_native_with_layout_inputs(
    native_text: &str,
    boundaries: &[crate::types::PageBoundary],
    pages: MixedOcrPageSelection<'_>,
    content: &[u8],
    config: &ExtractionConfig,
    all_pages_failed_policy: AllPagesFailedPolicy,
    #[cfg(feature = "layout-detection")] layout_inputs: MixedLayoutInputs<'_>,
) -> OcrRunResult<MixedOcrRun> {
    let Some(mut context) = MixedOcrContext::prepare(
        native_text,
        boundaries,
        pages,
        content,
        config,
        all_pages_failed_policy,
        #[cfg(feature = "layout-detection")]
        layout_inputs,
    )
    .await?
    else {
        return Ok(MixedOcrRun::native(native_text));
    };
    context.run_page_batches().await?;
    context.finish()
}

#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
impl<'a> MixedOcrContext<'a> {
    async fn prepare(
        native_text: &'a str,
        boundaries: &'a [crate::types::PageBoundary],
        pages: MixedOcrPageSelection<'a>,
        content: &'a [u8],
        config: &'a ExtractionConfig,
        all_pages_failed_policy: AllPagesFailedPolicy,
        #[cfg(feature = "layout-detection")] layout_inputs: MixedLayoutInputs<'a>,
    ) -> OcrRunResult<Option<Self>> {
        let Some(selected) = select_mixed_ocr_pages(native_text, boundaries, &pages, content, config).await? else {
            return Ok(None);
        };

        #[cfg(feature = "layout-detection")]
        let layout = resolve_mixed_ocr_layout(layout_inputs, content, config).await;
        #[cfg(feature = "layout-detection")]
        let mixed_route_layout_active = match layout_inputs {
            MixedLayoutInputs::Prepared(inputs) => inputs.is_some(),
            MixedLayoutInputs::Resolve(_) => layout.inputs.is_some(),
        };
        #[cfg(not(feature = "layout-detection"))]
        let mixed_route_layout_active = false;

        let routing = resolve_mixed_ocr_routing(
            config,
            &selected.ocr_set,
            selected.page_count,
            selected.probing_hidden_layer,
            mixed_route_layout_active,
            pages.hidden_layer_backend.clone(),
        )?;

        #[cfg_attr(not(feature = "layout-detection"), allow(unused_mut))]
        let mut raster_dpis = pages.raster_dpis.as_deref().cloned().unwrap_or_default();
        #[cfg(feature = "layout-detection")]
        raster_dpis.extend(layout.raster_dpis);
        let total = selected.page_indices.len();
        let batch_size = resolve_mixed_batch_size(
            &selected.page_indices,
            &selected.render_doc,
            content,
            config,
            routing.preprocessing_render_dpi,
            match routing.effective_pipeline.as_ref() {
                Some(pipeline) => pipeline_route_recognition_sessions(pipeline),
                None => direct_route_recognition_sessions(None, routing.backend.as_ref()),
            },
        )
        .await;

        let ocr_output_thresholds = route_noise_thresholds(config);
        let page_layers = match pages.native_layers.clone() {
            Some(layers) => Some(layers),
            None if !selected.probing_hidden_layer
                && all_pages_failed_policy == AllPagesFailedPolicy::PreserveNative =>
            {
                let layers = super::document::clean_native_page_layers(
                    native_text,
                    boundaries,
                    Some(&selected.ocr_set),
                    &ocr_output_thresholds,
                );
                (!layers.is_empty()).then(|| std::sync::Arc::new(layers))
            }
            None => None,
        };
        #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
        let native_coverage_layers = if marker_layers_allowed(
            all_pages_failed_policy,
            selected.probing_hidden_layer,
            mixed_route_layout_active,
            &config.ocr_strategy,
            config.ocr.as_ref(),
        ) {
            let markers = super::document::marker_native_ink_layers(
                native_text,
                boundaries,
                &selected.ocr_set,
                selected.page_count,
                &ocr_output_thresholds,
            );
            if markers.is_empty() {
                page_layers.clone()
            } else {
                let mut layers = page_layers.as_deref().cloned().unwrap_or_default();
                layers.extend(markers);
                Some(std::sync::Arc::new(layers))
            }
        } else {
            None
        };
        #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
        let sparse_glyph_pages = sparse_glyph_read::eligible_pages(
            config,
            &selected.render_doc,
            native_coverage_layers.as_deref(),
            routing.backend.as_ref(),
        );
        #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
        let sparse_glyph_images = sparse_glyph_read::images_config();
        #[cfg(any(
            all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")),
            all(paddle_ocr, feature = "ocr")
        ))]
        let number_layers = pages.native_layers.clone();
        #[cfg(all(paddle_ocr, feature = "ocr", not(target_arch = "wasm32")))]
        let implicit_sparse_glyph_pages = sparse_glyph_read::without_number_recovery_pages(
            sparse_glyph_read::implicit_pipeline_pages(
                config,
                &routing.ocr_config_owned,
                routing.effective_pipeline.as_ref(),
                &selected.render_doc,
                native_coverage_layers.as_deref(),
                !selected.probing_hidden_layer && !mixed_route_layout_active && !routing.capture_rasters,
            ),
            number_layers.as_deref(),
        );

        #[cfg_attr(not(feature = "layout-detection"), allow(unused_mut))]
        let mut accumulated_warnings: Vec<crate::types::ProcessingWarning> = Vec::new();
        #[cfg(feature = "layout-detection")]
        {
            if let Some(warning) = layout.warning {
                crate::core::diagnostics::push_warning_deduped(&mut accumulated_warnings, warning);
            }
            for warning in layout.glyph_drop_warnings {
                crate::core::diagnostics::push_warning_deduped(&mut accumulated_warnings, warning);
            }
        }

        Ok(Some(Self {
            native_text,
            boundaries,
            pages,
            content,
            config,
            all_pages_failed_policy,
            #[cfg(feature = "layout-detection")]
            layout_inputs,
            ocr_set: selected.ocr_set,
            page_indices: selected.page_indices,
            render_doc: selected.render_doc,
            page_count: selected.page_count,
            page_rotations: selected.page_rotations,
            single_block_pages: selected.single_block_pages,
            probing_hidden_layer: selected.probing_hidden_layer,
            ocr_config_resolved: routing.ocr_config_resolved,
            ocr_config_owned: routing.ocr_config_owned,
            preprocessing_render_dpi: routing.preprocessing_render_dpi,
            batch_size,
            capture_rasters: routing.capture_rasters,
            effective_pipeline: routing.effective_pipeline,
            #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
            automatic_mixed_prose: routing.automatic_mixed_prose,
            #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
            whole_document_scan: routing.whole_document_scan,
            #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
            orientation_retry: routing.orientation_retry,
            pipeline_stage_config: routing.pipeline_stage_config,
            backend: routing.backend,
            #[cfg(all(auto_rotate, feature = "ocr", not(target_arch = "wasm32")))]
            direct_prose_retry: routing.direct_prose_retry,
            #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
            allow_khmer_hint: routing.allow_khmer_hint,
            ocr_output_thresholds,
            page_layers,
            #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
            number_layers,
            #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
            native_coverage_layers,
            #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
            sparse_glyph_pages,
            #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
            sparse_glyph_images,
            #[cfg(all(paddle_ocr, feature = "ocr", not(target_arch = "wasm32")))]
            implicit_sparse_glyph_pages,
            #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
            mixed_number_recovery: std::sync::Arc::new(super::number_recovery::Run::default()),
            #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
            texture_work: super::hidden_layer::PreviewWorkBudget::new(config),
            #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
            certified_background: super::symbol_noise_background::certified_source(content),
            #[cfg(feature = "layout-detection")]
            resolved_layout_inputs: layout.inputs,
            mixed_route_layout_active,
            total,
            raster_dpis,
            noise_verdict_pages: std::collections::HashSet::new(),
            #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
            texture_noise_by_page: ahash::AHashMap::new(),
            #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
            paint_retention_by_page: ahash::AHashMap::new(),
            #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
            background_noise_by_page: ahash::AHashMap::new(),
            low_confidence_pages: std::collections::HashSet::new(),
            dictionary_noise_pages: std::collections::HashSet::new(),
            #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
            khmer_supplements: ahash::AHashMap::new(),
            #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
            leading_khmer_rows: ahash::AHashMap::new(),
            #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
            number_primary_texts: ahash::AHashMap::new(),
            ocr_results: ahash::AHashMap::with_capacity(total),
            #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
            original_tesseract_raster_pages: ahash::AHashSet::new(),
            #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
            native_placed_words_pages: ahash::AHashMap::new(),
            #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
            matched_layer_occurrences: ahash::AHashMap::new(),
            #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
            paragraphs_in_page_points: ahash::AHashSet::new(),
            #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
            caption_rewrites: ahash::AHashMap::new(),
            #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
            native_words_covered_pages: ahash::AHashSet::new(),
            #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
            native_ink_owned_pages: ahash::AHashSet::new(),
            #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
            alternative_raster_pages: ahash::AHashSet::new(),
            page_mean_confidence: ahash::AHashMap::new(),
            page_dictionary_invalid_word_ratio: ahash::AHashMap::new(),
            page_word_count: ahash::AHashMap::new(),
            ocr_confidence_by_page: ahash::AHashMap::new(),
            structured_ocr_pages: ahash::AHashMap::with_capacity(total),
            table_only_xobject_recovery_pages: ahash::AHashSet::new(),
            ocr_page_paragraphs: ahash::AHashMap::with_capacity(total),
            #[cfg(all(
                feature = "ocr",
                feature = "layout-detection",
                feature = "tokio-runtime",
                not(target_arch = "wasm32")
            ))]
            column_lanes_by_page: ahash::AHashMap::new(),
            preserve_ordered_layout: ahash::AHashSet::new(),
            ocr_raster_by_page: ahash::AHashMap::with_capacity(total),
            #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
            reflow_scales: ahash::AHashMap::new(),
            accumulated_llm_usage: Vec::new(),
            accumulated_formulas: Vec::new(),
            accumulated_warnings,
            recovery_warnings: Vec::new(),
            discarded_recovery_pages: ahash::AHashSet::new(),
            page_failures: Vec::new(),
            failed_page_errors: ahash::AHashMap::new(),
            captured_rasters: Vec::new(),
            preprocessing_by_page: ahash::AHashMap::new(),
            xobject_recovery_budget: crate::extractors::security::SecurityBudget::from_config(config),
        }))
    }

    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    fn use_khmer_hint(&self, page_idx: usize, whole_page_raster: bool) -> bool {
        self.allow_khmer_hint
            && whole_page_raster
            && super::document::native_preservation_is_automatic(self.config, (page_idx + 1) as u32)
            && preceding_native_khmer_page(self.native_text, self.boundaries, &self.ocr_set, page_idx)
            && !crate::pdf::scan_detect::carries_scan_raster(&self.render_doc, page_idx - 1)
    }

    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    fn mixed_page_allows_prose(&self, page_idx: usize, whole_page_raster: bool) -> bool {
        self.automatic_mixed_prose
            && whole_page_raster
            && !mixed_native_page_has_content(self.native_text, self.boundaries, page_idx)
            && !self.use_khmer_hint(page_idx, whole_page_raster)
    }

    #[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
    fn hints_for_pipeline_page(&self, idx: usize, raster: Option<crate::pdf::render::OcrRasterDpi>) -> PageOcrHints {
        #[cfg(not(feature = "ocr"))]
        let _ = raster;
        let whole_page_raster = crate::pdf::scan_detect::full_page_raster_density(&self.render_doc, idx).is_some();
        PageOcrHints {
            #[cfg(all(
                feature = "pdf",
                feature = "ocr",
                feature = "tokio-runtime",
                not(target_arch = "wasm32")
            ))]
            number_source: (!self.probing_hidden_layer
                && !self.mixed_route_layout_active
                && self
                    .config
                    .ocr
                    .as_ref()
                    .is_some_and(|ocr| !ocr.has_tuning_backend_options())
                && self
                    .number_layers
                    .as_ref()
                    .is_some_and(|layers| layers.contains_key(&((idx + 1) as u32))))
            .then(|| super::number_recovery::DetachedSource {
                document: std::sync::Arc::clone(&self.render_doc),
                run: std::sync::Arc::clone(&self.mixed_number_recovery),
                raster,
                layers: self.number_layers.clone().unwrap_or_default(),
                config: self.config.clone(),
            }),
            #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
            probe_reads: self.pages.probe_reads.clone(),
            known_full_page_scan: whole_page_raster,
            #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
            detached_recognition_recovery: Some(if self.whole_document_scan {
                super::orientation_retry::RecognitionRecovery::TableOrProse
            } else if self.automatic_mixed_prose {
                super::orientation_retry::RecognitionRecovery::MixedPage {
                    allow_prose: self.mixed_page_allows_prose(idx, whole_page_raster),
                }
            } else {
                super::orientation_retry::RecognitionRecovery::Table
            }),
            single_block_pages: Some(std::sync::Arc::clone(&self.single_block_pages)),
            #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
            decision_layers: self.page_layers.clone(),
            #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
            native_coverage_layers: self
                .native_coverage_layers
                .as_ref()
                .filter(|_| super::document::native_preservation_is_automatic(self.config, (idx + 1) as u32))
                .cloned(),
            #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
            single_native_type0_page: self
                .pages
                .single_native_type0_page
                .filter(|_| self.page_count == 1 && idx == 0)
                .map(std::sync::Arc::from),
            ..Default::default()
        }
    }

    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    fn images_for_page(&self, page_index: usize) -> Option<&crate::core::config::ImageExtractionConfig> {
        if self.sparse_glyph_pages.contains_key(&page_index) {
            return Some(&self.sparse_glyph_images);
        }
        self.config.images.as_ref()
    }

    #[cfg(not(all(feature = "ocr", not(target_arch = "wasm32"))))]
    fn images_for_page(&self, _page_index: usize) -> Option<&crate::core::config::ImageExtractionConfig> {
        self.config.images.as_ref()
    }

    #[cfg(feature = "layout-detection")]
    fn layout_inputs_for_mixed(&self) -> Option<&PreparedLayoutInputs> {
        match self.layout_inputs {
            MixedLayoutInputs::Prepared(inputs) => inputs,
            MixedLayoutInputs::Resolve(_) => self.resolved_layout_inputs.as_ref(),
        }
    }

    #[cfg(feature = "layout-detection")]
    fn layout_detections_for_mixed(&self) -> Option<&[crate::layout::DetectionResult]> {
        self.layout_inputs_for_mixed()
            .map(|(_, detections)| detections.as_slice())
    }

    #[cfg(feature = "layout-detection")]
    fn layout_images_for_mixed(&self) -> Option<&[image::DynamicImage]> {
        self.layout_inputs_for_mixed().map(|(images, _)| images.as_slice())
    }
}

/// Filter the requested OCR pages, open the document, run the hidden-layer probe, and keep the
/// pages that still need a read. `None` means the run keeps the native text unchanged.
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
#[cfg_attr(not(all(feature = "ocr", not(target_arch = "wasm32"))), allow(unused_variables))]
async fn select_mixed_ocr_pages(
    native_text: &str,
    boundaries: &[crate::types::PageBoundary],
    pages: &MixedOcrPageSelection<'_>,
    content: &[u8],
    config: &ExtractionConfig,
) -> OcrRunResult<Option<SelectedMixedOcrPages>> {
    #[cfg(not(all(feature = "ocr", not(target_arch = "wasm32"))))]
    let _ = (pages.preserve_native, pages.native_script);
    let ocr_set: std::collections::HashSet<u32> = pages
        .ocr
        .iter()
        .copied()
        .filter(|&p| {
            if p == 0 {
                tracing::warn!("force_ocr_pages contains 0; page numbers are 1-indexed, ignoring");
                false
            } else {
                true
            }
        })
        .collect();

    if ocr_set.is_empty() {
        return Ok(None);
    }

    let mut page_indices: Vec<usize> = ocr_set.iter().map(|&p| (p - 1) as usize).collect();
    page_indices.sort_unstable();
    let (render_doc, page_count, page_rotations) = open_pdf_for_page_ocr(content)?;
    // Shared with each spawned pipeline task below, whose embedded-image retry reads it (#1912).
    let render_doc = std::sync::Arc::new(render_doc);
    // A listed page that carries a scan keeps the scan's segmentation mode; see
    // `single_block_for_ocr_page`.
    let single_block_pages = std::sync::Arc::new(
        pages
            .single_block
            .iter()
            .copied()
            .filter(|page| ocr_set.contains(page))
            .filter(|&page| !crate::pdf::scan_detect::carries_scan_raster(&render_doc, (page - 1) as usize))
            .collect::<std::collections::HashSet<u32>>(),
    );
    page_indices = valid_page_indices(&page_indices, page_count);
    let probing_hidden_layer = pages.hidden_layer_backend.is_some();
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    if probing_hidden_layer {
        let byte_limit = super::hidden_layer::preview_byte_limit(config);
        let probe_doc = std::sync::Arc::clone(&render_doc);
        let cancel = config.cancel_token.clone();
        let page_limit = super::hidden_layer::preview_page_limit(config);
        let work_budget = super::hidden_layer::PreviewWorkBudget::new(config);
        // An audited page whose invisible layer covers its picture is read only when the
        // language check did not judge that layer plausible prose: it abstained or flagged it.
        // The check runs inside the blocking closure, and only for such pages, on the same page
        // text and thresholds as `record_implausible_text_pages`. With plausibility routing
        // disabled it judges nothing, so every covered layer is read. ~keep
        let candidates: std::collections::HashSet<usize> = page_indices.iter().copied().collect();
        let layer_texts: std::collections::HashMap<usize, String> = boundaries
            .iter()
            .filter_map(|boundary| {
                let index = usize::try_from(boundary.page_number).ok()?.checked_sub(1)?;
                candidates.contains(&index).then_some(())?;
                let text = native_text.get(boundary.byte_start..boundary.byte_end)?;
                Some((index, text.to_owned()))
            })
            .collect();
        let plausibility_thresholds = config
            .ocr
            .as_ref()
            .map(|ocr| ocr.effective_thresholds())
            .unwrap_or_default();
        super::super::ensure_pdf_not_cancelled(config)?;
        let decisions = super::super::spawn_pdf_blocking(move || {
            let is_cancelled = || cancel.as_ref().is_some_and(|token| token.is_cancelled());
            let mut work_budget = work_budget;
            let mut decisions = Vec::new();
            for index in page_indices.into_iter().take(page_limit) {
                if is_cancelled() {
                    break;
                }
                let needed = work_budget.probe_page(|page_work| {
                    super::hidden_layer::needs_ocr(&probe_doc, index, byte_limit, page_work, is_cancelled)
                });
                let plausible = super::hidden_layer::layer_judged_plausible(
                    needed,
                    layer_texts.get(&index).map(String::as_str),
                    &plausibility_thresholds,
                );
                decisions.push((index, needed, plausible));
            }
            Ok(decisions)
        })
        .await?;
        page_indices = super::hidden_layer::pages_to_probe(decisions);
        super::super::ensure_pdf_not_cancelled(config)?;
    }
    if page_indices.is_empty() {
        return Ok(None);
    }

    Ok(Some(SelectedMixedOcrPages {
        ocr_set,
        page_indices,
        render_doc,
        page_count,
        page_rotations,
        single_block_pages,
        probing_hidden_layer,
    }))
}

/// Run the layout pass for a selected-page mixed OCR call, or reuse the inputs a whole-document
/// caller prepared (#665).
#[cfg(all(
    any(feature = "ocr", feature = "ocr-pipeline"),
    feature = "pdf",
    feature = "layout-detection"
))]
async fn resolve_mixed_ocr_layout(
    layout_inputs: MixedLayoutInputs<'_>,
    content: &[u8],
    config: &ExtractionConfig,
) -> ResolvedMixedLayout {
    let mut layout_raster_dpis = crate::pdf::render::OcrRasterDpis::new();
    let (inputs, warning, glyph_drop_warnings): MixedLayoutOutcome = if let MixedLayoutInputs::Prepared(_) =
        layout_inputs
    {
        (None, None, Vec::new())
    } else if let Some(layout_config) = config.resolved_layout_config() {
        let layout_thread_budget = crate::core::config::concurrency::resolve_thread_budget(config.concurrency.as_ref());
        let default_security_limits = crate::extractors::security::SecurityLimits::default();
        let security_limits = config.security_limits.as_ref().unwrap_or(&default_security_limits);
        let selected_layout_pages = match layout_inputs {
            MixedLayoutInputs::Resolve(pages) => pages,
            MixedLayoutInputs::Prepared(_) => None,
        };
        match super::super::layout_runner::run_layout_for_ocr_with_pages(
            content,
            layout_config.as_ref(),
            layout_thread_budget,
            security_limits,
            config.images.as_ref(),
            selected_layout_pages,
        )
        .await
        {
            Ok((
                super::super::layout_runner::LayoutAttempt {
                    output:
                        super::super::layout_runner::LayoutRunOutput {
                            data: Some((images, _, _, detections)),
                            raster_dpis,
                            ..
                        },
                    warning,
                    ..
                },
                glyph_drop_warnings,
            )) => {
                layout_raster_dpis = raster_dpis;
                (
                    Some(super::super::prepare_ocr_layout_inputs(images, detections)),
                    warning,
                    glyph_drop_warnings,
                )
            }
            Ok((
                super::super::layout_runner::LayoutAttempt {
                    output: super::super::layout_runner::LayoutRunOutput { data: None, .. },
                    warning,
                    ..
                },
                glyph_drop_warnings,
            )) => {
                tracing::info!(
                    "OCR layout (mixed route): layout gate skipped every page, continuing without layout assembly"
                );
                (None, warning, glyph_drop_warnings)
            }
            Err(error) => {
                tracing::warn!(
                    error = %error,
                    "OCR layout detection failed for mixed OCR route; continuing without layout assembly"
                );
                (
                    None,
                    Some(super::super::layout_runner::layout_failure_warning(&error)),
                    Vec::new(),
                )
            }
        }
    } else {
        (None, None, Vec::new())
    };
    ResolvedMixedLayout {
        raster_dpis: layout_raster_dpis,
        inputs,
        warning,
        glyph_drop_warnings,
    }
}

/// Resolve the OCR configuration, the staffed pipeline/backend, and the per-document retry
/// policies this route uses. Every decision here is a pure function of the input config and the
/// selected pages.
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
#[cfg_attr(not(all(feature = "ocr", not(target_arch = "wasm32"))), allow(unused_variables))]
fn resolve_mixed_ocr_routing(
    config: &ExtractionConfig,
    ocr_set: &std::collections::HashSet<u32>,
    page_count: usize,
    probing_hidden_layer: bool,
    mixed_route_layout_active: bool,
    hidden_layer_backend: Option<std::sync::Arc<dyn crate::plugins::OcrBackend>>,
) -> crate::Result<MixedOcrRouting> {
    let mut ocr_config_resolved = super::resolve_ocr_config(config).into_owned();
    if probing_hidden_layer {
        ocr_config_resolved.tesseract_config = Some(crate::types::TesseractConfig {
            output_format: "text".to_owned(),
            enable_table_detection: false,
            ..Default::default()
        });
    }
    if ocr_config_resolved.acceleration.is_none() {
        ocr_config_resolved.acceleration = config.acceleration.clone();
    }
    // GH#1554: mirrors `acceleration` above so this mixed native/OCR route inherits the
    // caller's configured decode limits instead of always falling back to
    // `SecurityLimits::default()`. ~keep
    if ocr_config_resolved.security_limits.is_none() {
        ocr_config_resolved.security_limits = config.security_limits.clone();
    }

    #[cfg(all(feature = "ocr", feature = "pdf", not(target_arch = "wasm32")))]
    let preprocessing_render_dpi =
        mixed_route_promotes_preprocessing_dpi(mixed_route_layout_active, probing_hidden_layer)
            .then(|| registered_preprocessing_render_dpi(config, &ocr_config_resolved))
            .flatten();
    #[cfg(not(all(feature = "ocr", feature = "pdf", not(target_arch = "wasm32"))))]
    let preprocessing_render_dpi: Option<i32> = None;

    let capture_rasters = config.images.as_ref().is_some_and(|c| c.include_page_rasters);
    let ocr_config_owned = ensure_elements_enabled(&ocr_config_resolved);
    // When any effective pipeline is configured or synthesized, each page must run through
    // the shared pipeline runner so its fallback decision stays page-local. This includes
    // the implicit Tesseract-to-Paddle pipeline as well as `vlm_fallback` and an explicit pipeline;
    // otherwise selected-page routes silently collapse the implicit pipeline to Tesseract
    // alone (#1908). A build/configuration with no effective pipeline keeps the fast path. ~keep
    //
    // Layout detections (#665, `mixed_route_layout_active`) are threaded the same way: the
    // pipeline route is the only one that hands `layout_detections` down to
    // `extract_with_ocr_for_page`'s classification. Wrapping the single configured backend
    // in a one-stage pipeline (`single_stage_pipeline_for_layout`) reuses that exact,
    // already-tested code path instead of duplicating pixel-space layout assembly here. This
    // only fires when a real detection is available for this call, so `--layout` producing
    // nothing (gate skipped every page, or no `config.layout`) leaves the fast path untouched.
    let effective_pipeline = if probing_hidden_layer {
        None
    } else if let Some(pipeline) = ocr_config_owned.effective_pipeline() {
        Some(pipeline)
    } else if mixed_route_layout_active {
        #[cfg(feature = "layout-detection")]
        {
            Some(single_stage_pipeline_for_layout(&ocr_config_owned))
        }
        #[cfg(not(feature = "layout-detection"))]
        {
            None
        }
    } else {
        None
    };
    // The sparse glyph regression cases exercise the single-backend raster path. ~keep
    #[cfg(all(test, feature = "ocr", feature = "bundle-tessdata-eng"))]
    let effective_pipeline =
        effective_pipeline.filter(|_| !sparse_glyph_read::test_uses_single_backend(&ocr_config_owned));
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    let automatic_mixed_prose = cfg!(auto_rotate)
        && !probing_hidden_layer
        && !mixed_route_layout_active
        && config.force_ocr_pages.is_none()
        && matches!(config.ocr_strategy, crate::core::config::OcrStrategy::Auto)
        && !capture_rasters
        && uses_default_ocr_pipeline(&ocr_config_owned)
        && config
            .ocr
            .as_ref()
            .is_some_and(super::orientation_retry::recognition_retry_eligible);
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    let whole_document_scan = (1..=page_count as u32).all(|page| ocr_set.contains(&page));
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    let orientation_retry = effective_pipeline
        .as_ref()
        .filter(|_| super::orientation_retry::OrientationRetry::allows(config) && !mixed_route_layout_active)
        .map(|pipeline| {
            std::sync::Arc::new(super::orientation_retry::OrientationRetry::new(
                config,
                pipeline.quality_thresholds.clone(),
            ))
        });

    // Each invocation below owns exactly one page. Page markers belong to the outer document
    // assembly; leaving them enabled here both stamps every detached page as page 1 and lets a
    // marker make an empty primary stage look good enough to suppress its fallback (#1931). ~keep
    let pipeline_stage_config = {
        let mut stage_config = config.clone();
        if let Some(pages) = stage_config.pages.as_mut() {
            pages.insert_page_markers = false;
        }
        stage_config
    };

    // The top-level `backend` registry lookup is only needed by the single-backend
    // route below; the pipeline route resolves each of its own stage backends
    // internally via `run_ocr_pipeline`. Resolving it eagerly meant a
    // `vlm_fallback = Always` config (or an explicit `pipeline`) that never touches
    // this top-level backend still failed if it happened to be unregistered
    // (review follow-up to #1341).
    let backend = if let Some(backend) = hidden_layer_backend {
        Some(backend)
    } else if effective_pipeline.is_none() {
        let registry = crate::plugins::registry::get_ocr_backend_registry();
        let registry = registry.read();
        Some(registry.get(&ocr_config_owned.backend)?)
    } else {
        None
    };

    #[cfg(all(auto_rotate, feature = "ocr", not(target_arch = "wasm32")))]
    let direct_prose_retry = (automatic_mixed_prose
        && backend.as_ref().is_some_and(|backend| {
            crate::plugins::registry::get_ocr_backend_registry()
                .read()
                .is_builtin_tesseract(backend)
        }))
    .then(|| {
        std::sync::Arc::new(super::orientation_retry::OrientationRetry::new(
            config,
            ocr_config_owned.effective_thresholds(),
        ))
    });

    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    let allow_khmer_hint = !probing_hidden_layer
        && crate::ocr::processor::script_retry::may_retry_khmer(
            &ocr_config_owned.effective_tesseract_language().join("+"),
        )
        && matches!(config.ocr_strategy, crate::core::config::OcrStrategy::Auto);

    Ok(MixedOcrRouting {
        ocr_config_resolved,
        ocr_config_owned,
        preprocessing_render_dpi,
        capture_rasters,
        effective_pipeline,
        #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
        automatic_mixed_prose,
        #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
        whole_document_scan,
        #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
        orientation_retry,
        pipeline_stage_config,
        backend,
        #[cfg(all(auto_rotate, feature = "ocr", not(target_arch = "wasm32")))]
        direct_prose_retry,
        #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
        allow_khmer_hint,
    })
}

/// How many pages this route renders, encodes and recognises at once.
///
/// `max_content_size` limits the text one extraction returns; it says nothing about the rasters a
/// batch holds while it works. Sizing the batch from it pinned the width at four pages on an
/// ordinary 150 DPI document, so no stage of this route could ever use more than four threads and
/// the thread budget changed nothing (#1666, #1716).
///
/// What a batch really costs is this document's own page cost times the batch width, so that
/// product is what the width is bounded by, against the memory the host has free. Measuring the
/// page rather than assuming a fixed per-page figure is what keeps the bound honest at a high
/// render DPI, where one page costs an order of magnitude more than the assumption does. Each
/// page's own peak is still checked against `max_content_size`, one page at a time, before that
/// page is encoded. ~keep
///
/// A batch renders every page before any of them is read, so a page the recognizer cannot admit
/// yet only waits with its raster resident. `recognition_sessions` caps the width at what
/// recognition admits; see [`bound_batch_to_recognition`].
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
async fn resolve_mixed_batch_size(
    page_indices: &[usize],
    render_doc: &xberg_native_pdf::PdfDocument,
    content: &[u8],
    config: &ExtractionConfig,
    preprocessing_render_dpi: Option<i32>,
    recognition_sessions: Option<usize>,
) -> usize {
    let configured_batch_size = crate::core::config::concurrency::resolve_thread_budget(config.concurrency.as_ref());
    let render_page_cap = config
        .security_limits
        .as_ref()
        .cloned()
        .unwrap_or_default()
        .max_content_size;
    let batch_size = match page_indices.first() {
        Some(&first_page_idx) => {
            let (page_width_pt, page_height_pt) = page_dimensions_pt(render_doc, first_page_idx);
            adapt_batch_size_to_memory(
                configured_batch_size,
                content.len(),
                mixed_route_batch_working_set(
                    ocr_page_working_set_bytes(
                        f64::from(page_width_pt),
                        f64::from(page_height_pt),
                        config.images.as_ref(),
                    ),
                    preprocessing_render_dpi,
                    render_page_cap,
                ),
            )
            .await
        }
        None => configured_batch_size,
    };
    if batch_size < configured_batch_size {
        tracing::info!(
            configured = configured_batch_size,
            adapted = batch_size,
            "Reduced the per-page OCR batch to fit available memory"
        );
    }
    bound_batch_to_recognition(batch_size, recognition_sessions)
}

/// The widest batch the recognizer can read at once.
///
/// The direct built-in Tesseract route admits `recognition_sessions` reads at a time
/// (`max_concurrent_ocr`, or the memory-bounded default). Each page beyond that is rendered and
/// encoded with the rest of its batch, then holds its raster until a session frees, so its
/// render buys no throughput and only raises the peak. With `max_concurrent_ocr: 1`, an
/// eight-page batch of a 300 DPI scan rendered all eight pages (about 1 GB resident) to read
/// them one at a time. `None` leaves the width alone: a backend without that admission limit
/// reads every page of its batch at once. ~keep
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) fn bound_batch_to_recognition(batch_size: usize, recognition_sessions: Option<usize>) -> usize {
    let Some(sessions) = recognition_sessions.map(|sessions| sessions.max(1)) else {
        return batch_size;
    };
    if sessions >= batch_size {
        return batch_size;
    }
    tracing::debug!(
        batch_size,
        recognition_sessions = sessions,
        "Narrowed the per-page OCR batch to the recognition sessions that read it"
    );
    sessions
}

/// Recognition sessions that bound this route's batch width, or `None` when nothing does.
///
/// Only the direct route to the built-in Tesseract backend has an admission limit here: a
/// custom backend reads every page it is handed at once. The pipeline route is bounded by
/// [`pipeline_route_recognition_sessions`].
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) fn direct_route_recognition_sessions(
    effective_pipeline: Option<&crate::core::config::OcrPipelineConfig>,
    backend: Option<&std::sync::Arc<dyn crate::plugins::OcrBackend>>,
) -> Option<usize> {
    if effective_pipeline.is_some() {
        return None;
    }
    let backend = backend?;
    #[cfg(test)]
    {
        let pinned = TEST_RECOGNITION_SESSIONS.load(std::sync::atomic::Ordering::SeqCst);
        if pinned != 0 {
            return Some(pinned);
        }
    }
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    {
        crate::plugins::registry::get_ocr_backend_registry()
            .read()
            .is_builtin_tesseract(backend)
            .then(crate::core::config::concurrency::recognition_concurrency)
    }
    #[cfg(not(all(feature = "ocr", not(target_arch = "wasm32"))))]
    {
        let _ = backend;
        None
    }
}

/// Recognition sessions to report for any direct-route backend instead of asking whether it is
/// the built-in Tesseract one, so a test can drive the batch bound with a probe backend. The
/// process-wide limit itself is latched by the first extraction and cannot be set per test.
///
/// Zero means "not set". Process-wide, so every test that sets it is `#[serial]`.
/// Recognition sessions that bound the pipeline route's batch width, or `None` when nothing does.
///
/// Every stage must be the built-in Tesseract backend, so every read of every page waits on the
/// same admission limit. A pipeline with any other stage (a VLM, a custom backend) reads pages
/// that Tesseract's sessions do not gate, and narrowing its batch would only serialise them.
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) fn pipeline_route_recognition_sessions(pipeline: &crate::core::config::OcrPipelineConfig) -> Option<usize> {
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    {
        let registry = crate::plugins::registry::get_ocr_backend_registry().read();
        pipeline_stages_all_match(pipeline, |name| {
            registry
                .get(name)
                .is_ok_and(|backend| registry.is_builtin_tesseract(&backend))
        })
        .then(crate::core::config::concurrency::recognition_concurrency)
    }
    #[cfg(not(all(feature = "ocr", not(target_arch = "wasm32"))))]
    {
        let _ = pipeline;
        None
    }
}

/// Whether the pipeline has stages and `is_tesseract` accepts the backend of every one.
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) fn pipeline_stages_all_match(
    pipeline: &crate::core::config::OcrPipelineConfig,
    is_tesseract: impl Fn(&str) -> bool,
) -> bool {
    !pipeline.stages.is_empty() && pipeline.stages.iter().all(|stage| is_tesseract(&stage.backend))
}

#[cfg(all(test, any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) static TEST_RECOGNITION_SESSIONS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
impl MixedOcrContext<'_> {
    async fn run_page_batches(&mut self) -> OcrRunResult<()> {
        let page_loop = self
            .config
            .cancel_token
            .as_ref()
            .map(crate::cancellation::CancellationToken::enter_page_loop);
        for batch_start in (0..self.total).step_by(self.batch_size) {
            if !self.run_page_batch(batch_start).await? {
                break;
            }
        }
        drop(page_loop);
        Ok(())
    }

    fn stop_token(&self) -> Option<crate::cancellation::CancellationToken> {
        self.config
            .cancel_token
            .as_ref()
            .filter(|token| token.stop_requested())
            .cloned()
    }

    /// The backend of the direct single-backend route, or the same structured error every caller
    /// reports when the route reached a page without one.
    fn direct_backend(&self) -> crate::Result<std::sync::Arc<dyn crate::plugins::OcrBackend>> {
        self.backend.clone().ok_or_else(|| crate::XbergError::Ocr {
            message: "OCR backend was not resolved for the direct page route".to_string(),
            source: None,
        })
    }

    #[cfg(all(paddle_ocr, feature = "ocr", not(target_arch = "wasm32")))]
    fn time_limit_token(&self) -> Option<crate::cancellation::CancellationToken> {
        self.config
            .cancel_token
            .as_ref()
            .filter(|token| token.time_limit_reached())
            .cloned()
    }

    fn mark_pages_unread(
        &mut self,
        token: Option<crate::cancellation::CancellationToken>,
        pages: impl Iterator<Item = usize>,
    ) {
        let native_text = self.native_text;
        let boundaries = self.boundaries;
        mark_pages_unread(
            token.as_ref(),
            pages,
            &mut self.page_failures,
            &mut self.failed_page_errors,
            |page_idx| mixed_native_page_has_content(native_text, boundaries, page_idx),
        );
    }

    fn record_page_failure(&mut self, page_idx: usize, error: crate::XbergError) {
        let recovered = mixed_native_page_has_content(self.native_text, self.boundaries, page_idx);
        push_mixed_ocr_page_failure_warning(
            &mut self.accumulated_warnings,
            &mut self.page_failures,
            page_idx,
            &error,
            recovered,
        );
        self.failed_page_errors.insert(page_idx, error);
    }

    /// Render and read one batch. Returns `false` when the caller asked the loop to stop.
    async fn run_page_batch(&mut self, batch_start: usize) -> OcrRunResult<bool> {
        // The extraction time limit asks for the pages read so far: the pages of the batches
        // that finished stay in the result and every later page is recorded as unread.
        if let Some(token) = self.stop_token() {
            let pending: Vec<usize> = self.page_indices[batch_start..].to_vec();
            self.mark_pages_unread(Some(token), pending.into_iter());
            return Ok(false);
        }
        let batch_end = (batch_start + self.batch_size).min(self.total);
        let batch_pages: Vec<usize> = self.page_indices[batch_start..batch_end].to_vec();
        let default_security_limits = crate::extractors::security::SecurityLimits::default();
        let security_limits = self
            .config
            .security_limits
            .as_ref()
            .unwrap_or(&default_security_limits)
            .clone();

        #[cfg(all(paddle_ocr, feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
        let trial_pages: Vec<usize> = self
            .effective_pipeline
            .as_ref()
            .map(|_| {
                batch_pages
                    .iter()
                    .copied()
                    .filter(|idx| self.implicit_sparse_glyph_pages.contains_key(idx))
                    .collect()
            })
            .unwrap_or_default();
        #[cfg(all(paddle_ocr, feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
        let without_trials: Option<Vec<usize>> = (!trial_pages.is_empty()).then(|| {
            batch_pages
                .iter()
                .copied()
                .filter(|idx| !trial_pages.contains(idx))
                .collect::<Vec<_>>()
        });
        #[cfg(all(paddle_ocr, feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
        let render_pages: &[usize] = without_trials.as_deref().unwrap_or(&batch_pages);
        #[cfg(not(all(paddle_ocr, feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32"))))]
        let render_pages: &[usize] = &batch_pages;

        #[cfg(feature = "layout-detection")]
        let (page_images, render_failures, batch_raster_dpis) = {
            if let Some(images) = self.layout_images_for_mixed() {
                let mut page_images = Vec::with_capacity(batch_end - batch_start);
                let mut render_failures = Vec::new();
                for &page_idx in render_pages {
                    match images.get(page_idx) {
                        Some(image) => page_images.push((page_idx, image.clone())),
                        None => render_failures.push((
                            page_idx,
                            crate::XbergError::Ocr {
                                message: format!("prepared layout image missing for page {}", page_idx + 1),
                                source: None,
                            },
                        )),
                    }
                }
                (page_images, render_failures, super::rendering::PageRasterDpis::new())
            } else {
                render_selected_pages_with_image_config(
                    &self.render_doc,
                    &self.page_rotations,
                    render_pages,
                    &security_limits,
                    |idx| self.images_for_page(idx),
                    self.preprocessing_render_dpi,
                )
            }
        };
        #[cfg(not(feature = "layout-detection"))]
        let (page_images, render_failures, batch_raster_dpis) = render_selected_pages_with_image_config(
            &self.render_doc,
            &self.page_rotations,
            render_pages,
            &security_limits,
            |idx| self.images_for_page(idx),
            self.preprocessing_render_dpi,
        );
        self.raster_dpis.extend(batch_raster_dpis);
        let mut budget_refused: Vec<(usize, crate::XbergError)> = Vec::new();
        for (page_idx, error) in render_failures {
            if render_refused_by_source_image_budget(&error) {
                budget_refused.push((page_idx, error));
                continue;
            }
            self.record_page_failure(page_idx, error);
        }

        if self.effective_pipeline.is_some() {
            self.run_pipeline_batch(
                page_images,
                budget_refused,
                #[cfg(all(paddle_ocr, feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
                trial_pages,
                #[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
                &security_limits,
            )
            .await?;
        } else {
            self.run_single_backend_batch(page_images, budget_refused, &security_limits)
                .await?;
        }
        Ok(true)
    }

    /// Multi-stage pipeline route (#1341): drive each page through `run_ocr_pipeline`
    /// so `vlm_fallback` / explicit-pipeline stages apply here, mirroring the image
    /// extractor's per-image pipeline path.
    async fn run_pipeline_batch(
        &mut self,
        page_images: Vec<(usize, image::DynamicImage)>,
        budget_refused: Vec<(usize, crate::XbergError)>,
        #[cfg(all(paddle_ocr, feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
        trial_pages: Vec<usize>,
        #[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
        security_limits: &crate::extractors::security::SecurityLimits,
    ) -> OcrRunResult<()> {
        self.recover_pipeline_budget_refused(budget_refused).await?;
        let pipeline = self.effective_pipeline.clone().ok_or_else(|| crate::XbergError::Ocr {
            message: "the pipeline route was entered without an effective pipeline".to_string(),
            source: None,
        })?;
        let page_images = share_rendered_page_images(page_images);
        // Rasters are released as each page task finishes; only an explicit capture keeps a
        // second reference for the batch to encode afterwards.
        let page_images_for_capture = self.capture_rasters.then(|| page_images.clone());
        #[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
        self.run_pipeline_pages_async(
            page_images,
            pipeline,
            #[cfg(all(paddle_ocr, feature = "ocr", not(target_arch = "wasm32")))]
            trial_pages,
            security_limits,
        )
        .await?;
        #[cfg(any(not(feature = "tokio-runtime"), target_arch = "wasm32"))]
        self.run_pipeline_pages_sequential(&page_images, pipeline).await?;
        if let Some(page_images) = page_images_for_capture {
            self.capture_pipeline_rasters(&page_images)?;
        }
        Ok(())
    }

    /// A page whose source image the render budget refused has no raster to inspect: its
    /// embedded images are the only place its text can come from.
    async fn recover_pipeline_budget_refused(
        &mut self,
        budget_refused: Vec<(usize, crate::XbergError)>,
    ) -> OcrRunResult<()> {
        for (page_idx, error) in budget_refused {
            let backend = crate::plugins::registry::get_ocr_backend_registry()
                .read()
                .get(&self.ocr_config_owned.backend)
                .ok();
            let recovery = match backend {
                Some(backend) => {
                    recover_page_text_from_image_xobjects(
                        &backend,
                        &self.render_doc,
                        page_idx,
                        self.page_rotations.len(),
                        &self.ocr_config_owned,
                        XObjectRecoveryContext {
                            budget: &mut self.xobject_recovery_budget,
                            blank_raster_warnings: Some(&mut self.accumulated_warnings),
                            no_admitted_images: None,
                            stored_jpegs: None,
                            cancel_token: self.config.cancel_token.as_ref(),
                            use_proven_source_dpi: true,
                            proven_read: Some(ProvenReadPolicy {
                                thresholds: &self.ocr_output_thresholds,
                                warn_when_kept: true,
                            }),
                            emit_progress: true,
                        },
                    )
                    .await?
                }
                None => None,
            };
            let page_number = (page_idx + 1) as u32;
            let mut recovered = false;
            let mut retry_causes = Vec::new();
            if let Some(XObjectRecoveryOutcome {
                text,
                text_outside_tables,
                attempted,
                images,
                mut llm_usage,
                tables,
                mut formulas,
                image_preprocessing,
                tesseract_psm,
                failure_causes,
                warnings,
            }) = recovery
            {
                retry_causes = failure_causes;
                self.accumulated_warnings.extend(warnings);
                recovered = should_adopt_xobject_retry_text("", &text) || !tables.is_empty();
                if should_adopt_xobject_retry_text("", &text) {
                    self.structured_ocr_pages.insert(
                        page_number,
                        super::document::flat_ocr_page_document(&text_outside_tables),
                    );
                    self.ocr_results.insert(page_number, text);
                }
                if !tables.is_empty() {
                    let current_text = self.ocr_results.get(&page_number).map_or("", String::as_str);
                    let page_doc = self
                        .structured_ocr_pages
                        .entry(page_number)
                        .or_insert_with(|| super::document::flat_ocr_page_document(current_text));
                    super::document::attach_page_ocr_payload(page_doc, tables, Vec::new(), page_number);
                    if !self.ocr_results.contains_key(&page_number) {
                        self.table_only_xobject_recovery_pages.insert(page_number);
                    }
                }
                if let Some(psm) = tesseract_psm
                    && let Some(page_doc) = self.structured_ocr_pages.get_mut(&page_number)
                {
                    attach_page_segmentation_modes(page_doc, [(page_number, psm)]);
                }
                self.accumulated_llm_usage.append(&mut llm_usage);
                self.accumulated_formulas.append(&mut formulas);
                if let Some(metadata) = image_preprocessing {
                    self.preprocessing_by_page.insert(page_number, metadata);
                }
                if self.capture_rasters {
                    self.captured_rasters.extend(images);
                }
                self.accumulated_warnings
                    .push(xobject_fallback_warning(page_idx, attempted, true));
            }
            if recovered {
                let recovered_kind = if self
                    .ocr_results
                    .get(&page_number)
                    .is_some_and(|text| !text.trim().is_empty())
                {
                    "text"
                } else {
                    "content"
                };
                self.accumulated_warnings.push(crate::types::ProcessingWarning {
                    source: std::borrow::Cow::Borrowed("ocr"),
                    message: std::borrow::Cow::Owned(format!(
                        "OCR of page {page_number} failed ({error}); its {recovered_kind} was recovered from the \
                         page's embedded image XObjects instead."
                    )),
                });
                merge_ocr_page_failures(
                    &mut self.page_failures,
                    vec![crate::types::OcrPageFailure {
                        page: page_number,
                        error: error.to_string(),
                        recovered: true,
                    }],
                );
            } else {
                let error = with_retry_causes(error, &retry_causes);
                self.record_page_failure(page_idx, error);
            }
        }
        Ok(())
    }

    #[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
    async fn run_pipeline_pages_async(
        &mut self,
        page_images: Vec<(usize, std::sync::Arc<image::DynamicImage>)>,
        pipeline: crate::core::config::OcrPipelineConfig,
        #[cfg(all(paddle_ocr, feature = "ocr", not(target_arch = "wasm32")))] trial_pages: Vec<usize>,
        security_limits: &crate::extractors::security::SecurityLimits,
    ) -> OcrRunResult<()> {
        let render_doc = std::sync::Arc::clone(&self.render_doc);
        let stage_config = self.pipeline_stage_config.clone();
        let config = self.config;
        let page_rotations = self.page_rotations.clone();
        #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
        let orientation_retry = self.orientation_retry.clone();
        let capture_rasters = self.capture_rasters;
        let mixed_route_layout_active = self.mixed_route_layout_active;
        #[cfg(not(all(paddle_ocr, feature = "ocr")))]
        let _ = (capture_rasters, mixed_route_layout_active);
        #[cfg(not(all(paddle_ocr, feature = "ocr", not(target_arch = "wasm32"))))]
        let _ = security_limits;

        let mut join_set = tokio::task::JoinSet::new();
        let mut pending_pages: ahash::AHashSet<usize> = ahash::AHashSet::new();
        #[cfg_attr(not(paddle_ocr), allow(unused_mut))]
        let mut dimensions: ahash::AHashMap<_, _> = page_images
            .iter()
            .map(|(index, image)| (*index, (image.width(), image.height())))
            .collect();
        #[cfg(all(paddle_ocr, feature = "ocr", not(target_arch = "wasm32")))]
        let panicked_trials = sparse_glyph_read::PanickedTrials::default();
        let spawn_page = |join_set: &mut tokio::task::JoinSet<PageTaskOutput>,
                          page_idx: usize,
                          image_arc: std::sync::Arc<image::DynamicImage>,
                          hints: PageOcrHints,
                          page_detection: Option<MixedPageDetection>| {
            let render_doc_clone = std::sync::Arc::clone(&render_doc);
            let pipeline_clone = pipeline.clone();
            let config_clone = stage_config.clone();
            #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
            let orientation_retry = orientation_retry.clone();
            let idx = page_idx;
            // This page's own known `/Rotate` value, already resolved by
            // `open_pdf_for_page_ocr` above -- the sibling single-backend route
            // below reads the same `page_rotations` slice the same way. Passed
            // explicitly rather than via `content` because the pipeline call below
            // hands `run_ocr_pipeline` a single detached image (not the full page
            // list `extract_with_ocr`'s own content-based auto-detection expects
            // index-aligned to page 0), so an index-based content lookup there would
            // resolve the wrong page's rotation (or none at all) for anything but the
            // document's first OCR'd page (#651).
            let page_rotation_degrees = page_rotations.get(page_idx).copied().unwrap_or(0);
            let mut page_ocr_hints = hints;
            page_ocr_hints.source_dpi = rendered_page_source_dpi(&render_doc, idx, image_arc.width());
            // See `extract_with_ocr_for_page`'s doc comment on
            // `points_per_pixel_override`: this call hands the stage a single
            // detached image with `content: None`, so its own pixel -> point lookup
            // cannot resolve this page's real MediaBox height and would silently fall
            // back to `1.0`. Computed from the same `render_doc` / raster height the
            // sibling single-backend route below rescales bboxes with.
            let points_per_pixel_override = {
                let (_, page_height_pt) = page_dimensions_pt(&render_doc, page_idx);
                let image_height_px = image_arc.height();
                (image_height_px > 0).then(|| page_height_pt / image_height_px as f32)
            };
            #[cfg(all(paddle_ocr, feature = "ocr"))]
            let allow_stored_retry = !capture_rasters
                && !mixed_route_layout_active
                && page_ocr_hints.known_full_page_scan
                && stored_scan_retry::eligible(config, (idx + 1) as u32);
            let task = crate::engine::seams::inherit_progress(async move {
                #[cfg(all(test, feature = "bundle-tessdata-eng", paddle_ocr, feature = "ocr"))]
                sparse_glyph_read::parallel_tests::test_page_started(&config_clone, idx);
                if config_clone.cancel_token.as_ref().is_some_and(|t| t.is_cancelled()) {
                    return PageTaskOutput::Read(idx, Box::new((Err(crate::XbergError::Cancelled), None)));
                }
                #[cfg(feature = "layout-detection")]
                let page_detection_slice = page_detection.as_ref().map(std::slice::from_ref);
                #[cfg(not(feature = "layout-detection"))]
                let _ = page_detection;
                let source = DetachedOcrPage {
                    document: render_doc_clone.as_ref(),
                    #[cfg(all(paddle_ocr, feature = "ocr", not(target_arch = "wasm32")))]
                    stored_scan: None,
                    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
                    orientation_retry: orientation_retry.as_ref(),
                };
                let result = Box::pin(run_ocr_pipeline_for_page(
                    None,
                    Some(std::slice::from_ref(image_arc.as_ref())),
                    #[cfg(feature = "layout-detection")]
                    page_detection_slice,
                    &config_clone,
                    &pipeline_clone,
                    None,
                    page_rotation_degrees,
                    true,
                    points_per_pixel_override,
                    Some(page_ocr_hints),
                    idx,
                    Some(source),
                ))
                .await;
                #[cfg(all(paddle_ocr, feature = "ocr"))]
                let result = stored_scan_retry::maybe_retry(
                    source.document,
                    idx,
                    image_arc,
                    &config_clone,
                    result,
                    allow_stored_retry,
                )
                .await;
                PageTaskOutput::Read(
                    idx,
                    Box::new(capture_ocr_completion(
                        result.map_err(crate::XbergError::from),
                        config_clone.cancel_token.as_ref(),
                    )),
                )
            });
            #[cfg(test)]
            let task = crate::plugins::registry::inherit_test_ocr_backend(task);
            join_set.spawn(crate::core::extractor::cacheability::inherit(task));
        };
        {
            #[cfg(all(paddle_ocr, feature = "ocr", not(target_arch = "wasm32")))]
            {
                let trial_hints: ahash::AHashMap<usize, PageOcrHints> = trial_pages
                    .iter()
                    .map(|&idx| (idx, self.hints_for_pipeline_page(idx, None)))
                    .collect();
                sparse_glyph_read::spawn_trials(
                    &mut join_set,
                    &panicked_trials,
                    &sparse_glyph_read::TrialContext {
                        pdf: &render_doc,
                        config: &stage_config,
                        pipeline: &pipeline,
                    },
                    trial_pages
                        .iter()
                        .filter_map(|&idx| self.implicit_sparse_glyph_pages.get(&idx).map(|words| (idx, words))),
                    |idx| sparse_glyph_read::TrialPage {
                        rotation: page_rotations.get(idx).copied().unwrap_or(0),
                        hints: trial_hints[&idx].clone(),
                        orientation_retry: orientation_retry.clone(),
                    },
                );
                pending_pages.extend(trial_pages.iter().copied());
            }

            for (page_idx, image_arc) in page_images {
                if config.cancel_token.as_ref().is_some_and(|t| t.is_cancelled()) {
                    break;
                }
                pending_pages.insert(page_idx);
                let hints = self.hints_for_pipeline_page(page_idx, self.raster_dpis.get(&page_idx).copied());
                #[cfg(feature = "layout-detection")]
                let page_detection =
                    detection_for_mixed_route_page(self.layout_detections_for_mixed(), page_idx).cloned();
                #[cfg(not(feature = "layout-detection"))]
                let page_detection: Option<MixedPageDetection> = None;
                spawn_page(&mut join_set, page_idx, image_arc, hints, page_detection);
            }
        }

        self.collect_pipeline_reads(
            &mut join_set,
            &mut pending_pages,
            &mut dimensions,
            #[cfg(all(paddle_ocr, feature = "ocr", not(target_arch = "wasm32")))]
            &panicked_trials,
            spawn_page,
            security_limits,
        )
        .await
    }

    /// Drain the pipeline tasks of one batch. The spawn closure is the caller's, so a trial
    /// retry render uses the same page configuration.
    #[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
    async fn collect_pipeline_reads<F>(
        &mut self,
        join_set: &mut tokio::task::JoinSet<PageTaskOutput>,
        pending_pages: &mut ahash::AHashSet<usize>,
        dimensions: &mut ahash::AHashMap<usize, (u32, u32)>,
        #[cfg(all(paddle_ocr, feature = "ocr", not(target_arch = "wasm32")))]
        panicked_trials: &sparse_glyph_read::PanickedTrials,
        spawn_page: F,
        security_limits: &crate::extractors::security::SecurityLimits,
    ) -> OcrRunResult<()>
    where
        F: Fn(
            &mut tokio::task::JoinSet<PageTaskOutput>,
            usize,
            std::sync::Arc<image::DynamicImage>,
            PageOcrHints,
            Option<MixedPageDetection>,
        ),
    {
        let config = self.config;
        let render_doc = std::sync::Arc::clone(&self.render_doc);
        let page_rotations = self.page_rotations.clone();
        let preprocessing_render_dpi = self.preprocessing_render_dpi;
        #[cfg(not(all(paddle_ocr, feature = "ocr", not(target_arch = "wasm32"))))]
        let _ = (
            &spawn_page,
            security_limits,
            &render_doc,
            &page_rotations,
            preprocessing_render_dpi,
        );
        let mut abandon_at = None;
        loop {
            let join_result = match next_page_task(join_set, config.cancel_token.as_ref(), &mut abandon_at).await {
                NextPageTask::Finished(join_result) => join_result,
                NextPageTask::Drained => break,
                NextPageTask::Cancelled => return Err(crate::XbergError::Cancelled.into()),
                NextPageTask::Abandoned => {
                    let pending: Vec<usize> = pending_pages.drain().collect();
                    self.mark_pages_unread(config.cancel_token.clone(), pending.into_iter());
                    break;
                }
            };
            let output = match join_result {
                Ok(output) => output,
                Err(error) => {
                    #[cfg(all(paddle_ocr, feature = "ocr", not(target_arch = "wasm32")))]
                    let panicked_trial = panicked_trials.take().map(|idx| {
                        tracing::debug!(page = idx + 1, "Sparse glyph OCR task did not finish");
                        PageTaskOutput::Trial(idx, Ok(None))
                    });
                    #[cfg(not(all(paddle_ocr, feature = "ocr", not(target_arch = "wasm32"))))]
                    let panicked_trial: Option<PageTaskOutput> = None;
                    match panicked_trial {
                        Some(output) => output,
                        None => {
                            return Err(crate::XbergError::Plugin {
                                message: format!("OCR pipeline task panicked: {}", error),
                                plugin_name: "ocr".to_string(),
                            }
                            .into());
                        }
                    }
                }
            };
            let (page_idx, result, expired, sparse_dimensions) = match output {
                PageTaskOutput::Read(page_idx, read) => {
                    let (result, expired) = *read;
                    (page_idx, result, expired, None)
                }
                #[cfg(all(paddle_ocr, feature = "ocr", not(target_arch = "wasm32")))]
                PageTaskOutput::Trial(idx, Ok(Some(read))) => {
                    crate::engine::seams::emit_ocr_page(idx + 1, self.page_count, "tesseract");
                    self.raster_dpis.insert(idx, read.dpi);
                    (idx, Ok(*read.output), None, Some(read.dimensions))
                }
                #[cfg(all(paddle_ocr, feature = "ocr", not(target_arch = "wasm32")))]
                PageTaskOutput::Trial(_, Err(error)) => return Err(error.into()),
                #[cfg(all(paddle_ocr, feature = "ocr", not(target_arch = "wasm32")))]
                PageTaskOutput::Trial(idx, Ok(None)) => {
                    if let Some(token) = self.time_limit_token() {
                        pending_pages.remove(&idx);
                        self.mark_pages_unread(Some(token), std::iter::once(idx));
                        continue;
                    }
                    // Trial pages have no XObject, Pattern or Shading resources (`marked_words`
                    // rejects them), so the source-image budget refusal that gets XObject
                    // recovery above cannot occur here.
                    sparse_glyph_read::spawn_retry_render(
                        join_set,
                        sparse_glyph_read::RetryRenderRequest {
                            pdf: std::sync::Arc::clone(&render_doc),
                            index: idx,
                            rotations: page_rotations.clone(),
                            limits: security_limits.clone(),
                            images_config: self.images_for_page(idx).cloned(),
                            preprocessing_dpi: preprocessing_render_dpi,
                        },
                    );
                    continue;
                }
                #[cfg(all(paddle_ocr, feature = "ocr", not(target_arch = "wasm32")))]
                PageTaskOutput::RetryRender(idx, rendered) => {
                    // The render is a task of this set, not an inline await, so a stalled
                    // render cannot keep the collector from draining finished reads or
                    // honouring the stop request. Its warnings were drained on the
                    // blocking worker that rendered them.
                    let (retry_images, retry_failures, retry_dpis, retry_warnings) = match *rendered {
                        Ok(rendered) => rendered,
                        Err(error) => {
                            pending_pages.remove(&idx);
                            self.record_page_failure(idx, error);
                            continue;
                        }
                    };
                    crate::core::diagnostics::dedup_extend_warnings(&mut self.accumulated_warnings, retry_warnings);
                    self.raster_dpis.extend(retry_dpis);
                    for (failed_idx, error) in retry_failures {
                        pending_pages.remove(&failed_idx);
                        self.record_page_failure(failed_idx, error);
                    }
                    if config.cancel_token.as_ref().is_some_and(|token| token.is_cancelled()) {
                        return Err(crate::XbergError::Cancelled.into());
                    }
                    if let Some(token) = self.time_limit_token() {
                        pending_pages.remove(&idx);
                        self.mark_pages_unread(Some(token), std::iter::once(idx));
                        continue;
                    }
                    let retry_images = share_rendered_page_images(retry_images);
                    if retry_images.is_empty() {
                        pending_pages.remove(&idx);
                        continue;
                    }
                    for (retry_idx, image_arc) in retry_images {
                        dimensions.insert(retry_idx, (image_arc.width(), image_arc.height()));
                        let hints = self.hints_for_pipeline_page(retry_idx, self.raster_dpis.get(&retry_idx).copied());
                        #[cfg(feature = "layout-detection")]
                        let page_detection =
                            detection_for_mixed_route_page(self.layout_detections_for_mixed(), retry_idx).cloned();
                        #[cfg(not(feature = "layout-detection"))]
                        let page_detection: Option<MixedPageDetection> = None;
                        spawn_page(join_set, retry_idx, image_arc, hints, page_detection);
                    }
                    continue;
                }
            };
            pending_pages.remove(&page_idx);
            if config.cancel_token.as_ref().is_some_and(|token| token.is_cancelled())
                || matches!(&result, Err(crate::XbergError::Cancelled))
            {
                return Err(crate::XbergError::Cancelled.into());
            }
            if let Some(token) = expired {
                token.request_stop();
                self.mark_pages_unread(Some(token), std::iter::once(page_idx));
                continue;
            }
            let result = match result {
                Ok(result) => result,
                Err(error) => {
                    self.record_page_failure(page_idx, error);
                    continue;
                }
            };
            let page_dims = sparse_dimensions.or_else(|| dimensions.get(&page_idx).copied());
            self.record_pipeline_page_read(page_idx, result, page_dims);
        }
        Ok(())
    }

    #[cfg(any(not(feature = "tokio-runtime"), target_arch = "wasm32"))]
    async fn run_pipeline_pages_sequential(
        &mut self,
        page_images: &[(usize, std::sync::Arc<image::DynamicImage>)],
        pipeline: crate::core::config::OcrPipelineConfig,
    ) -> OcrRunResult<()> {
        for (page_idx, image) in page_images {
            if self.config.cancel_token.as_ref().is_some_and(|t| t.is_cancelled()) {
                break;
            }
            // See the matching comments on the sibling `JoinSet` branch above (#651,
            // and `points_per_pixel_override` re. `extract_with_ocr_for_page`'s doc
            // comment).
            let page_rotation_degrees = self.page_rotations.get(*page_idx).copied().unwrap_or(0);
            let page_ocr_hints = PageOcrHints {
                #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
                probe_reads: self.pages.probe_reads.clone(),
                source_dpi: rendered_page_source_dpi(&self.render_doc, *page_idx, image.width()),
                known_full_page_scan: crate::pdf::scan_detect::full_page_raster_density(&self.render_doc, *page_idx)
                    .is_some(),
                single_block_pages: Some(std::sync::Arc::clone(&self.single_block_pages)),
                native_page_layers: None,
                #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
                decision_layers: self.page_layers.clone(),
                #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
                native_coverage_layers: self
                    .native_coverage_layers
                    .as_ref()
                    .filter(|_| super::document::native_preservation_is_automatic(self.config, (*page_idx + 1) as u32))
                    .cloned(),
                #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
                single_native_type0_page: self
                    .pages
                    .single_native_type0_page
                    .filter(|_| self.page_count == 1 && *page_idx == 0)
                    .map(std::sync::Arc::from),
                ..Default::default()
            };
            let points_per_pixel_override = {
                let (_, page_height_pt) = page_dimensions_pt(&self.render_doc, *page_idx);
                let image_height_px = image.height();
                (image_height_px > 0).then(|| page_height_pt / image_height_px as f32)
            };
            // See the matching comment on the sibling `JoinSet` branch above (#665).
            #[cfg(feature = "layout-detection")]
            let page_detection: Option<&crate::layout::DetectionResult> =
                detection_for_mixed_route_page(self.layout_detections_for_mixed(), *page_idx);
            let source = DetachedOcrPage {
                document: self.render_doc.as_ref(),
                #[cfg(all(paddle_ocr, feature = "ocr", not(target_arch = "wasm32")))]
                stored_scan: None,
                #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
                orientation_retry: None,
            };
            let result = Box::pin(run_ocr_pipeline_for_page(
                None,
                Some(std::slice::from_ref(image.as_ref())),
                #[cfg(feature = "layout-detection")]
                page_detection.map(std::slice::from_ref),
                &self.pipeline_stage_config,
                &pipeline,
                None,
                page_rotation_degrees,
                true,
                points_per_pixel_override,
                Some(page_ocr_hints),
                *page_idx,
                Some(source),
            ))
            .await;
            let result = match result.map_err(crate::XbergError::from) {
                Ok(result) => result,
                Err(error @ crate::XbergError::Cancelled) => return Err(error.into()),
                Err(error) => {
                    self.record_page_failure(*page_idx, error);
                    continue;
                }
            };
            self.record_pipeline_page_read(*page_idx, result, Some((image.width(), image.height())));
        }
        Ok(())
    }

    /// Store one page read produced by `run_ocr_pipeline_for_page`, shared by the concurrent and
    /// sequential pipeline loops.
    fn record_pipeline_page_read(&mut self, page_idx: usize, result: OcrPipelineOutput, page_dims: Option<(u32, u32)>) {
        let (
            text,
            tables,
            elements,
            doc,
            usage,
            page_texts,
            _rasters,
            formulas,
            mut page_raw_paragraphs,
            preprocessing,
            page_ocr_confidences,
            noise_verdicts,
            read_evidence,
        ) = result;
        #[cfg(all(
            test,
            feature = "pdf",
            feature = "ocr",
            feature = "tokio-runtime",
            not(target_arch = "wasm32")
        ))]
        record_region_replay(&read_evidence);
        self.accumulated_llm_usage.extend(usage);
        self.ocr_confidence_by_page.extend(page_ocr_confidences);
        let page_number = (page_idx + 1) as u32;
        #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
        if read_evidence.original_tesseract_raster {
            self.original_tesseract_raster_pages.insert(page_number);
            if read_evidence.complete_native_words {
                self.native_words_covered_pages.insert(page_number);
            }
            if read_evidence.native_ink_owned {
                self.native_ink_owned_pages.insert(page_number);
            }
            self.native_placed_words_pages
                .insert(page_number, read_evidence.native_placed_words);
        }
        #[cfg(not(all(feature = "ocr", not(target_arch = "wasm32"))))]
        let _ = read_evidence;
        #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
        if let Some(matched) = read_evidence.matched_layer_occurrences {
            self.matched_layer_occurrences.insert(page_number, matched);
        }
        if noise_verdicts.iter().any(|verdict| !verdict.discarded) {
            self.noise_verdict_pages.insert(page_number);
        }
        if let Some(metadata) = preprocessing.into_values().next() {
            self.preprocessing_by_page.insert(page_number, metadata);
        }
        // `run_ocr_pipeline_for_page` drove exactly one detached image
        // (`std::slice::from_ref` above), so its own per-page paragraph vec has
        // at most one entry -- this page's own, still bare/unclassified because
        // `skip_document_global_heuristic = true` was passed above.
        let mut paragraph_page = page_raw_paragraphs.pop().unwrap_or_default();
        #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
        if let Some(rows) = paragraph_page.leading_khmer_rows.take() {
            self.leading_khmer_rows.insert(page_number, rows);
        }
        let paragraphs_in_points = paragraph_page.place_detached_in_points();
        #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
        if paragraphs_in_points {
            self.paragraphs_in_page_points.insert(page_number);
        }
        #[cfg(all(
            feature = "ocr",
            feature = "layout-detection",
            feature = "tokio-runtime",
            not(target_arch = "wasm32")
        ))]
        if let Some(lanes) = paragraph_page.column_lanes.take() {
            self.column_lanes_by_page.insert(page_number, lanes);
        }
        if !paragraph_page.paragraphs.is_empty() {
            if paragraph_page.preserve_ordered_layout {
                self.preserve_ordered_layout.insert(page_number);
            }
            self.ocr_page_paragraphs.insert(page_number, paragraph_page.paragraphs);
        }
        for mut formula in formulas {
            formula.page = Some(page_number);
            if let Some((w, h)) = page_dims {
                formula_bbox_to_page_points(&mut formula, &self.render_doc, page_idx, None, w, h);
            }
            self.accumulated_formulas.push(formula);
        }
        // `run_ocr_pipeline`/`extract_with_ocr` assemble `text` as if this
        // lone image were page 0 of the document, so a configured page marker
        // is stamped "page 1" regardless of the real page number. The raw
        // `page_texts` entry has no marker injected at that layer, so prefer
        // fall back to `text` only if the backend returned no page_texts.
        let page_text = page_texts.into_iter().next().unwrap_or(text);
        // The pipeline's tables and OCR elements used to be dropped here (#60);
        // they now ride along on the page's structured document. ~keep
        // This route also skipped the pixel -> point bbox conversion entirely,
        // so its bboxes stayed in raster pixels after #1423 fixed the
        // single-backend route; `build_pipeline_ocr_page_document` applies it.
        let raster_size_px = page_dims.unwrap_or((0, 0));
        if !paragraphs_in_points && let Some(raster_px) = paragraph_page.raster_px {
            self.ocr_raster_by_page.insert(page_number, raster_px);
        }
        let page_size_pt = page_dimensions_pt(&self.render_doc, page_idx);
        #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
        if let Some(rewrite) = read_evidence
            .caption
            .and_then(|rewrite| rewrite.bind_page(raster_size_px, page_size_pt))
        {
            self.caption_rewrites.insert(page_number, rewrite);
        }
        if let Some(mut d) = build_pipeline_ocr_page_document(
            doc,
            tables,
            elements,
            &page_text,
            page_number,
            raster_size_px,
            page_size_pt,
        ) {
            merge_returned_page_failures(
                &mut self.page_failures,
                &mut d,
                &page_text,
                self.native_text,
                self.boundaries,
            );
            crate::core::diagnostics::dedup_extend_warnings(
                &mut self.accumulated_warnings,
                std::mem::take(&mut d.processing_warnings),
            );
            self.structured_ocr_pages.insert(page_number, d);
        }
        #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
        if let Some(primary) = read_evidence.number_primary_text {
            self.number_primary_texts.insert(page_number, primary);
        }
        self.ocr_results.insert(page_number, page_text);
    }

    fn capture_pipeline_rasters(
        &mut self,
        page_images: &[(usize, std::sync::Arc<image::DynamicImage>)],
    ) -> OcrRunResult<()> {
        let default_security_limits = crate::extractors::security::SecurityLimits::default();
        let security_limits = self.config.security_limits.as_ref().unwrap_or(&default_security_limits);
        // One page at a time, as on the single-backend path below. `max_content_size`
        // bounds what a single page costs to render and encode; summing a whole batch
        // against it makes that ceiling a function of the thread budget, so a Letter
        // page at the 300 DPI an `images` config implies rejects the whole batch, and
        // every page in it, from a width of two upwards. The batch's own footprint is
        // bounded by `batch_size` above, which free memory decides. ~keep
        validate_png_encode_pages_individually(page_images.iter().map(|(_, image)| image.as_ref()), security_limits)?;
        for (page_idx, image) in page_images {
            let rgb = clone_rgb_for_png_encode(image, security_limits)?;
            let (w, h) = rgb.dimensions();
            let mut buf = Cursor::new(Vec::new());
            PngEncoder::new(&mut buf)
                .write_image(&rgb, w, h, image::ColorType::Rgb8.into())
                .map_err(|e| crate::XbergError::Parsing {
                    message: format!("Failed to encode page {} raster: {}", page_idx + 1, e),
                    source: None,
                })?;
            self.captured_rasters.push(build_page_raster_image(
                *page_idx,
                bytes::Bytes::from(buf.into_inner()),
                w,
                h,
            ));
        }
        Ok(())
    }
}

#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
impl MixedOcrContext<'_> {
    /// The direct single-backend route: encode this batch's rasters, read every page through the
    /// resolved backend, then retry the pages whose read was blank or failed on their embedded
    /// image XObjects.
    async fn run_single_backend_batch(
        &mut self,
        page_images: Vec<(usize, image::DynamicImage)>,
        budget_refused: Vec<(usize, crate::XbergError)>,
        security_limits: &crate::extractors::security::SecurityLimits,
    ) -> OcrRunResult<()> {
        // Reached only when `effective_pipeline` is `None`, so `backend` was resolved above.
        let backend = self.direct_backend()?;

        // One page at a time. `max_content_size` bounds what a single page may cost to render
        // and encode; summing a whole batch against it made that ceiling a function of the
        // thread budget, so a wide batch rejected every page in it (#1665) and the narrow
        // batch that hid the defect in #1666 was the workaround. The batch's own footprint is
        // bounded by `batch_size` above, which free memory decides. ~keep
        validate_png_encode_pages_individually(page_images.iter().map(|(_, image)| image), security_limits)?;
        let encoded = encode_selected_pages(&page_images, security_limits)?;
        drop(page_images);
        // A page whose backend call failed waits here for the embedded-image retry below.
        let mut failed_pages: ahash::AHashMap<usize, crate::XbergError> = ahash::AHashMap::new();
        #[cfg_attr(not(all(feature = "tokio-runtime", not(target_arch = "wasm32"))), allow(unused_mut))]
        let mut abandoned_pages: ahash::AHashSet<usize> = ahash::AHashSet::new();
        let budget_refused_pages: Vec<usize> = budget_refused.iter().map(|(page_idx, _)| *page_idx).collect();
        failed_pages.extend(budget_refused);

        // `tokio::task::JoinSet::spawn` requires `Send` futures, but extractor/backend futures
        // are `!Send` on wasm32 (async_trait(?Send), see plugins/extractor/trait.rs) — and
        // wasm32 has no OS threads to run them on regardless. Fall back to the sequential path
        // there even though `tokio-runtime` is active (it's pulled in by
        // `chunking-tokenizers`/`static-embeddings`, not concurrency support). ~keep
        #[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
        self.run_single_backend_pages_async(
            &encoded,
            &backend,
            security_limits,
            &mut failed_pages,
            &mut abandoned_pages,
        )
        .await?;
        #[cfg(any(not(feature = "tokio-runtime"), target_arch = "wasm32"))]
        self.run_single_backend_pages_sequential(&encoded, &backend, &mut failed_pages)
            .await?;

        self.retry_single_backend_xobjects(
            &encoded,
            &budget_refused_pages,
            &abandoned_pages,
            &mut failed_pages,
            security_limits,
        )
        .await?;

        if self.capture_rasters {
            for (page_idx, png_arc, w, h) in &encoded {
                let png_bytes = bytes::Bytes::copy_from_slice(png_arc.as_ref());
                self.captured_rasters
                    .push(build_page_raster_image(*page_idx, png_bytes, *w, *h));
            }
        }
        Ok(())
    }

    #[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
    #[allow(clippy::too_many_arguments)]
    #[cfg_attr(not(all(feature = "ocr", not(target_arch = "wasm32"))), allow(unused_variables))]
    async fn run_single_backend_pages_async(
        &mut self,
        encoded: &[EncodedPage],
        backend: &std::sync::Arc<dyn crate::plugins::OcrBackend>,
        security_limits: &crate::extractors::security::SecurityLimits,
        failed_pages: &mut ahash::AHashMap<usize, crate::XbergError>,
        abandoned_pages: &mut ahash::AHashSet<usize>,
    ) -> OcrRunResult<()> {
        let orientation_handling = backend.page_orientation_handling();
        let config = self.config;
        let ocr_config_resolved = self.ocr_config_resolved.clone();
        let ocr_output_thresholds = self.ocr_output_thresholds.clone();
        let render_doc = std::sync::Arc::clone(&self.render_doc);
        let mut join_set = tokio::task::JoinSet::new();
        let mut pending_pages: ahash::AHashSet<usize> = ahash::AHashSet::new();
        for (page_idx, data, width, height) in encoded {
            if config.cancel_token.as_ref().is_some_and(|t| t.is_cancelled()) {
                break;
            }
            let backend_clone = std::sync::Arc::clone(backend);
            let page_rotation_degrees = self.page_rotations.get(*page_idx).copied().unwrap_or(0);
            // Derived from the MediaBox-oriented raster, before `upright_raster_for_backend`
            // may swap its axes.
            let source_dpi = rendered_page_source_dpi(&self.render_doc, *page_idx, *width);
            let whole_page_raster =
                crate::pdf::scan_detect::full_page_raster_density(&self.render_doc, *page_idx).is_some();
            #[cfg(feature = "layout-detection")]
            let layout_assembly =
                detection_for_mixed_route_page(self.layout_detections_for_mixed(), *page_idx).is_some();
            #[cfg(not(feature = "layout-detection"))]
            let layout_assembly = false;
            let config_clone = ocr_config_with_page_rotation_hint(
                &self.ocr_config_owned,
                page_rotation_degrees,
                source_dpi,
                whole_page_raster,
                self.single_block_pages.contains(&(*page_idx as u32 + 1)),
                layout_assembly,
            );
            #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
            let khmer_hint = crate::plugins::registry::get_ocr_backend_registry()
                .read()
                .is_builtin_tesseract(backend)
                && self.use_khmer_hint(*page_idx, whole_page_raster);
            let config_clone = config_clone.into_owned();
            #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
            let normal_page_config = self
                .sparse_glyph_pages
                .contains_key(page_idx)
                .then(|| config_clone.clone());
            #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
            let config_clone = sparse_glyph_read::ocr_config(config_clone, normal_page_config.is_some());
            let (upright_data, upright_width, upright_height, correction_degrees) = match upright_raster_for_backend(
                data,
                *width,
                *height,
                page_rotation_degrees,
                orientation_handling,
                config.security_limits.as_ref(),
            ) {
                Ok(upright) => upright,
                Err(error) => {
                    failed_pages.insert(*page_idx, error);
                    continue;
                }
            };
            let idx = *page_idx;
            let cancel_token = config.cancel_token.clone();
            #[cfg(all(auto_rotate, feature = "ocr", not(target_arch = "wasm32")))]
            let prose_retry = self
                .direct_prose_retry
                .as_ref()
                .filter(|_| self.mixed_page_allows_prose(idx, whole_page_raster))
                .map(|retry| (std::sync::Arc::clone(retry), std::sync::Arc::clone(&upright_data)));
            let original_config = ocr_config_resolved.clone();
            let retained_bytes = retained_ocr_source_bytes(data, &upright_data, 0);
            #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
            let probe_reads = self.pages.probe_reads.clone();
            #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
            let observe_source_geometry = correction_degrees == 0
                && !self.mixed_route_layout_active
                && crate::plugins::registry::get_ocr_backend_registry()
                    .read()
                    .is_builtin_tesseract(backend)
                && (ocr_output_thresholds.prune_symbol_noise_lines
                    || (self.page_layers.as_ref().is_some_and(|layers| {
                        layers.contains_key(&((idx + 1) as u32))
                            && super::document::native_preservation_is_automatic(config, (idx + 1) as u32)
                    })));
            #[cfg(feature = "ocr")]
            let sparse_glyph_retry = self
                .sparse_glyph_pages
                .get(page_idx)
                .cloned()
                .zip(normal_page_config)
                .map(|(words, normal)| {
                    (
                        words,
                        std::sync::Arc::clone(&render_doc),
                        security_limits.clone(),
                        normal,
                        ocr_config_resolved.clone(),
                        ocr_output_thresholds.clone(),
                    )
                });
            let task = crate::engine::seams::inherit_progress(async move {
                if cancel_token.as_ref().is_some_and(|t| t.is_cancelled()) {
                    return (
                        idx,
                        correction_degrees,
                        upright_width,
                        upright_height,
                        (Err(crate::XbergError::Cancelled), None),
                        false,
                        None,
                    );
                }
                let (original_config, config_clone) = match crate::extractors::image::reserve_retained_ocr_raster(
                    &backend_clone,
                    (original_config, config_clone),
                    retained_bytes,
                ) {
                    Ok(configs) => configs,
                    Err(error) => {
                        return (
                            idx,
                            correction_degrees,
                            upright_width,
                            upright_height,
                            capture_ocr_completion(Err(error), cancel_token.as_ref()),
                            false,
                            None,
                        );
                    }
                };
                #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
                let scale_only = std::sync::atomic::AtomicBool::new(false);
                #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
                let read = super::read_reuse::read(
                    if khmer_hint { None } else { probe_reads.as_ref() },
                    super::read_reuse::ReadIdentity {
                        page: (idx + 1) as u32,
                        backend: &backend_clone,
                        configs: (&original_config, &config_clone),
                        whole_page_raster,
                        correction_degrees,
                    },
                    upright_data,
                    cancel_token.as_ref(),
                    |data| {
                        let read = crate::extractors::image::process_whole_image_ocr_with_sparse_retry(
                            &backend_clone,
                            data,
                            &original_config,
                            &config_clone,
                            crate::extractors::image::SparseRetryPolicy {
                                whole_page_raster,
                                allow_standard_retry: false,
                            },
                            cancel_token.as_ref(),
                        );
                        let scale_only = &scale_only;
                        async move {
                            if observe_source_geometry {
                                let (result, observed) =
                                    crate::ocr::preprocessing::geometry_evidence::observe(read).await;
                                scale_only.store(observed, std::sync::atomic::Ordering::Relaxed);
                                result
                            } else {
                                read.await
                            }
                        }
                    },
                );
                #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
                let result = if khmer_hint {
                    crate::ocr::processor::script_retry::MIXED_PDF_KHMER_RETRY
                        .scope(true, read)
                        .await
                } else {
                    read.await
                };
                #[cfg(all(auto_rotate, feature = "ocr", not(target_arch = "wasm32")))]
                let result = if let Some((retry, raster)) = prose_retry {
                    retry
                        .after_mixed_prose(&backend_clone, raster, &config_clone, result, cancel_token.as_ref())
                        .await
                } else {
                    result
                };
                #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
                let scale_only = scale_only.into_inner();
                #[cfg(not(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32"))))]
                let scale_only = false;
                #[cfg(not(all(feature = "ocr", not(target_arch = "wasm32"))))]
                let result = crate::extractors::image::process_whole_image_ocr_with_sparse_retry(
                    &backend_clone,
                    upright_data,
                    &original_config,
                    &config_clone,
                    crate::extractors::image::SparseRetryPolicy {
                        whole_page_raster,
                        allow_standard_retry: false,
                    },
                    cancel_token.as_ref(),
                )
                .await;
                #[cfg(all(test, feature = "ocr", feature = "bundle-tessdata-eng"))]
                let result = sparse_glyph_read::test_attempt(
                    result,
                    &original_config,
                    upright_width,
                    upright_height,
                    sparse_glyph_retry.as_ref().map(|(words, ..)| words),
                );
                #[cfg(feature = "ocr")]
                if let Some((words, pdf, limits, normal_config, original, thresholds)) = sparse_glyph_retry
                    && !matches!(&result, Err(crate::XbergError::Cancelled))
                    && !result.as_ref().is_ok_and(|read| {
                        words.accounted_for(read)
                            && !sparse_glyph_read::rejected_by_page_quality(
                                read,
                                &thresholds,
                                backend_clone.confidence_semantics(),
                            )
                    })
                {
                    return match sparse_glyph_read::retry(
                        pdf,
                        idx,
                        &backend_clone,
                        (&original, normal_config),
                        limits,
                        cancel_token.as_ref(),
                    )
                    .await
                    {
                        Ok((read, width, height, dpi)) => (idx, 0, width, height, (Ok(read), None), false, Some(dpi)),
                        Err(error) => (
                            idx,
                            0,
                            upright_width,
                            upright_height,
                            capture_ocr_completion(Err(error), cancel_token.as_ref()),
                            false,
                            None,
                        ),
                    };
                }
                (
                    idx,
                    correction_degrees,
                    upright_width,
                    upright_height,
                    capture_ocr_completion(result, cancel_token.as_ref()),
                    scale_only,
                    None,
                )
            });
            pending_pages.insert(*page_idx);
            join_set.spawn(crate::core::extractor::cacheability::inherit(task));
        }
        self.collect_single_backend_reads(
            &mut join_set,
            &mut pending_pages,
            abandoned_pages,
            failed_pages,
            encoded,
            backend,
        )
        .await
    }

    /// Drain the direct-backend tasks of one batch, recording failures and finishing each read.
    #[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
    async fn collect_single_backend_reads(
        &mut self,
        join_set: &mut tokio::task::JoinSet<SingleBackendTaskOutput>,
        pending_pages: &mut ahash::AHashSet<usize>,
        abandoned_pages: &mut ahash::AHashSet<usize>,
        failed_pages: &mut ahash::AHashMap<usize, crate::XbergError>,
        encoded: &[EncodedPage],
        backend: &std::sync::Arc<dyn crate::plugins::OcrBackend>,
    ) -> OcrRunResult<()> {
        let config = self.config;
        let mut abandon_at = None;
        loop {
            let join_result = match next_page_task(join_set, config.cancel_token.as_ref(), &mut abandon_at).await {
                NextPageTask::Finished(join_result) => join_result,
                NextPageTask::Drained => break,
                NextPageTask::Cancelled => return Err(crate::XbergError::Cancelled.into()),
                NextPageTask::Abandoned => {
                    let abandoned: Vec<usize> = pending_pages.drain().collect();
                    self.mark_pages_unread(config.cancel_token.clone(), abandoned.iter().copied());
                    abandoned_pages.extend(abandoned);
                    break;
                }
            };
            let (
                page_idx,
                correction_degrees,
                upright_width,
                upright_height,
                (result, expired),
                observed_scale_only,
                retry_dpi,
            ) = join_result.map_err(|e| crate::XbergError::Plugin {
                message: format!("OCR task panicked: {}", e),
                plugin_name: "ocr".to_string(),
            })?;
            if let Some(dpi) = retry_dpi {
                self.raster_dpis.insert(page_idx, dpi);
            }
            pending_pages.remove(&page_idx);
            if config.cancel_token.as_ref().is_some_and(|token| token.is_cancelled())
                || matches!(&result, Err(crate::XbergError::Cancelled))
            {
                return Err(crate::XbergError::Cancelled.into());
            }
            if let Some(token) = expired {
                token.request_stop();
                self.mark_pages_unread(Some(token), std::iter::once(page_idx));
                abandoned_pages.insert(page_idx);
                continue;
            }
            self.finish_single_backend_read(
                SingleBackendRead {
                    page_idx,
                    correction_degrees,
                    upright_width,
                    upright_height,
                    result,
                    observed_scale_only,
                    retry_dpi,
                },
                encoded,
                backend,
                failed_pages,
            )
            .await?;
        }
        Ok(())
    }

    #[cfg(any(not(feature = "tokio-runtime"), target_arch = "wasm32"))]
    async fn run_single_backend_pages_sequential(
        &mut self,
        encoded: &[EncodedPage],
        backend: &std::sync::Arc<dyn crate::plugins::OcrBackend>,
        failed_pages: &mut ahash::AHashMap<usize, crate::XbergError>,
    ) -> OcrRunResult<()> {
        let orientation_handling = backend.page_orientation_handling();
        let config = self.config;
        for (page_idx, data, width, height) in encoded {
            if config.cancel_token.as_ref().is_some_and(|t| t.is_cancelled()) {
                break;
            }
            let page_rotation_degrees = self.page_rotations.get(*page_idx).copied().unwrap_or(0);
            let source_dpi = rendered_page_source_dpi(&self.render_doc, *page_idx, *width);
            let whole_page_raster =
                crate::pdf::scan_detect::full_page_raster_density(&self.render_doc, *page_idx).is_some();
            #[cfg(feature = "layout-detection")]
            let layout_assembly =
                detection_for_mixed_route_page(self.layout_detections_for_mixed(), *page_idx).is_some();
            #[cfg(not(feature = "layout-detection"))]
            let layout_assembly = false;
            let config_for_page = ocr_config_with_page_rotation_hint(
                &self.ocr_config_owned,
                page_rotation_degrees,
                source_dpi,
                whole_page_raster,
                self.single_block_pages.contains(&(*page_idx as u32 + 1)),
                layout_assembly,
            );
            let (upright_data, upright_width, upright_height, correction_degrees) = match upright_raster_for_backend(
                data,
                *width,
                *height,
                page_rotation_degrees,
                orientation_handling,
                config.security_limits.as_ref(),
            ) {
                Ok(upright) => upright,
                Err(error) => {
                    failed_pages.insert(*page_idx, error);
                    continue;
                }
            };
            #[cfg(not(target_arch = "wasm32"))]
            let ocr_config_resolved = self.ocr_config_resolved.clone();
            #[cfg(not(target_arch = "wasm32"))]
            let result = async {
                let retained_bytes = retained_ocr_source_bytes(data, &upright_data, 0);
                let (original, effective) = crate::extractors::image::reserve_retained_ocr_raster(
                    backend,
                    (ocr_config_resolved, config_for_page.into_owned()),
                    retained_bytes,
                )?;
                crate::extractors::image::process_whole_image_ocr_with_sparse_retry(
                    backend,
                    upright_data,
                    &original,
                    &effective,
                    crate::extractors::image::SparseRetryPolicy {
                        whole_page_raster,
                        allow_standard_retry: false,
                    },
                    config.cancel_token.as_ref(),
                )
                .await
            }
            .await;
            #[cfg(target_arch = "wasm32")]
            let result = backend
                .process_image(upright_data.as_slice(), config_for_page.as_ref())
                .await;
            crate::engine::seams::emit_ocr_page(*page_idx + 1, self.page_rotations.len(), backend.name());
            let extraction_result = match result {
                Ok(extraction_result) => extraction_result,
                Err(error @ crate::XbergError::Cancelled) => return Err(error.into()),
                Err(error) => {
                    failed_pages.insert(*page_idx, error);
                    continue;
                }
            };
            self.record_single_backend_read(
                *page_idx,
                correction_degrees,
                upright_width,
                upright_height,
                None,
                extraction_result,
                false,
                GeometryEvidence::sequential(),
                encoded,
                backend,
            )
            .await?;
        }
        Ok(())
    }

    /// Apply the tall-source replacement (or record why it was declined) and hand a successful
    /// read to the page recorder.
    #[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
    #[allow(clippy::too_many_arguments)]
    async fn finish_single_backend_read(
        &mut self,
        read: SingleBackendRead,
        encoded: &[EncodedPage],
        backend: &std::sync::Arc<dyn crate::plugins::OcrBackend>,
        failed_pages: &mut ahash::AHashMap<usize, crate::XbergError>,
    ) -> OcrRunResult<()> {
        #[allow(unused_mut)]
        let SingleBackendRead {
            page_idx,
            correction_degrees,
            upright_width,
            upright_height,
            mut result,
            observed_scale_only,
            retry_dpi,
        } = read;
        #[cfg(all(
            feature = "ocr",
            feature = "pdf",
            feature = "tokio-runtime",
            not(target_arch = "wasm32")
        ))]
        let tall_source_candidate = !self.mixed_route_layout_active
            && crate::plugins::registry::get_ocr_backend_registry()
                .read()
                .is_builtin_tesseract(backend);
        #[cfg(all(
            feature = "ocr",
            feature = "pdf",
            feature = "tokio-runtime",
            not(target_arch = "wasm32")
        ))]
        let tall_source_unsupported = tall_source_candidate
            .then(|| {
                super::tall_source::unsupported_reason_for(self.config, &self.ocr_config_resolved, backend.as_ref())
            })
            .flatten();
        #[cfg(all(
            feature = "ocr",
            feature = "pdf",
            feature = "tokio-runtime",
            not(target_arch = "wasm32")
        ))]
        let try_tall_source = tall_source_candidate && tall_source_unsupported.is_none();
        #[cfg(all(
            feature = "ocr",
            feature = "pdf",
            feature = "tokio-runtime",
            not(target_arch = "wasm32")
        ))]
        if try_tall_source
            && correction_degrees == 0
            && let Ok(extraction_result) = &mut result
            && let Some(raster) = encoded.iter().find(|(index, ..)| *index == page_idx)
            && super::tall_source::try_replace(
                extraction_result,
                backend,
                self.config,
                &self.ocr_config_resolved,
                (&self.render_doc, page_idx),
                raster,
            )
            .await?
        {
            self.alternative_raster_pages.insert((page_idx + 1) as u32);
        } else if let Some(reason) = tall_source_unsupported
            && correction_degrees == 0
            && let Ok(extraction_result) = &mut result
            && let Some(raster) = encoded.iter().find(|(index, ..)| *index == page_idx)
        {
            super::tall_source::note_unrecovered(
                extraction_result,
                &self.ocr_config_resolved,
                (&self.render_doc, page_idx),
                (raster.2, raster.3),
                super::tall_source::Unrecovered::Config(reason),
            );
        }
        crate::engine::seams::emit_ocr_page(page_idx + 1, self.page_rotations.len(), backend.name());
        #[allow(unused_mut)]
        let mut extraction_result = match result {
            Ok(extraction_result) => extraction_result,
            Err(error) => {
                failed_pages.insert(page_idx, error);
                return Ok(());
            }
        };
        #[cfg(all(
            feature = "pdf",
            feature = "ocr",
            feature = "tokio-runtime",
            not(target_arch = "wasm32")
        ))]
        if correction_degrees == 0
            && !self.alternative_raster_pages.contains(&((page_idx + 1) as u32))
            && let Some(raster) = encoded.iter().find(|(index, ..)| *index == page_idx)
        {
            super::tall_source::probe_anchored_occurrences(
                &mut extraction_result,
                backend,
                self.config,
                &self.ocr_config_resolved,
                (&self.render_doc, page_idx),
                raster,
            )
            .await?;
        }
        self.record_single_backend_read(
            page_idx,
            correction_degrees,
            upright_width,
            upright_height,
            retry_dpi,
            extraction_result,
            observed_scale_only,
            GeometryEvidence::concurrent(),
            encoded,
            backend,
        )
        .await?;
        Ok(())
    }

    /// Enrich one direct-backend read with the language and native-layer evidence, then store its
    /// structured page, confidence summary and flat text.
    #[allow(clippy::too_many_arguments)]
    #[cfg_attr(
        not(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32"))),
        allow(unused_variables)
    )]
    async fn record_single_backend_read(
        &mut self,
        page_idx: usize,
        correction_degrees: u32,
        upright_width: u32,
        upright_height: u32,
        retry_dpi: Option<crate::pdf::render::OcrRasterDpi>,
        mut extraction_result: crate::types::ExtractedDocument,
        observed_scale_only: bool,
        geometry_evidence: GeometryEvidence,
        encoded: &[EncodedPage],
        backend: &std::sync::Arc<dyn crate::plugins::OcrBackend>,
    ) -> OcrRunResult<()> {
        let page_number = (page_idx + 1) as u32;
        #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
        if let Some(serde_json::Value::String(text)) = extraction_result
            .metadata
            .additional
            .remove(crate::ocr::processor::script_retry::KHMER_SUPPLEMENT_KEY)
        {
            self.khmer_supplements.insert(page_number, text);
        }
        // The sequential route never ran the geometry and language passes before this split, so
        // the shared recorder keeps them behind the caller's flag instead of changing page
        // decisions in a no-tokio build.
        #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
        if geometry_evidence.runs() {
            self.capture_native_coverage(page_idx, correction_degrees, observed_scale_only, &extraction_result);
        }
        crate::extraction::derive::adopt_public_ocr_document(
            &mut extraction_result,
            self.config.security_limits.as_ref().unwrap_or(&Default::default()),
        )?;
        #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
        if geometry_evidence.runs() {
            self.recover_language_evidence(page_idx, correction_degrees, &mut extraction_result);
        }
        if let Some(metadata) = extraction_result.metadata.image_preprocessing.clone() {
            self.preprocessing_by_page.insert(page_number, metadata);
        }
        undo_upright_raster_correction(
            &mut extraction_result,
            correction_degrees,
            upright_width,
            upright_height,
        );
        if let Some(usage) = extraction_result.llm_usage.take() {
            self.accumulated_llm_usage.extend(usage);
        }
        let page_dims = retry_dpi.map(|_| (upright_width, upright_height)).or_else(|| {
            encoded
                .iter()
                .find(|(encoded_page, ..)| *encoded_page == page_idx)
                .map(|(_, _, w, h)| (*w, *h))
        });
        for mut formula in std::mem::take(&mut extraction_result.formulas) {
            formula.page = Some(page_number);
            if let Some((w, h)) = page_dims {
                formula_bbox_to_page_points(
                    &mut formula,
                    &self.render_doc,
                    page_idx,
                    Some(&extraction_result.metadata),
                    w,
                    h,
                );
            }
            self.accumulated_formulas.push(formula);
        }
        // The backend's own warnings used to be dropped on this route (#60).
        crate::core::diagnostics::dedup_extend_warnings(
            &mut self.accumulated_warnings,
            std::mem::take(&mut extraction_result.processing_warnings),
        );
        #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
        self.capture_symbol_noise_evidence(
            page_idx,
            correction_degrees,
            &extraction_result,
            observed_scale_only,
            encoded,
        )
        .await;
        let (width, height) = retry_dpi.map(|_| (upright_width, upright_height)).unwrap_or_else(|| {
            encoded
                .iter()
                .find(|(encoded_page, ..)| *encoded_page == page_idx)
                .map_or((0, 0), |(_, _, w, h)| (*w, *h))
        });
        let (page_width_pt, page_height_pt) = page_dimensions_pt(&self.render_doc, page_idx);
        let page_psm = effective_tesseract_psm(&extraction_result.metadata, backend.name());
        if let Some((mut page_doc, paragraphs)) = build_mixed_ocr_page_document(
            &mut extraction_result,
            &self.ocr_config_resolved,
            page_number,
            width,
            height,
            page_width_pt,
            page_height_pt,
            crate::pdf::native::text::PageMarginFractions::from_extraction_config(Some(self.config)),
        ) {
            crate::core::diagnostics::dedup_extend_warnings(
                &mut self.accumulated_warnings,
                std::mem::take(&mut page_doc.processing_warnings),
            );
            if !paragraphs.is_empty() {
                self.ocr_page_paragraphs.insert(page_number, paragraphs);
                #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
                self.paragraphs_in_page_points.insert(page_number);
            }
            if let Some(psm) = page_psm {
                attach_page_segmentation_modes(&mut page_doc, [(page_number, psm)]);
            }
            self.structured_ocr_pages.insert(page_number, page_doc);
        }
        if let Some(conf) = mean_text_conf_of(&extraction_result.metadata.additional) {
            self.page_mean_confidence.insert(page_number, conf);
        }
        if let Some(words) = word_count_of(&extraction_result.metadata.additional) {
            self.page_word_count.insert(page_number, words);
        }
        if let Some(ratio) = extraction_result
            .metadata
            .additional
            .get(crate::ocr_metadata_keys::OCR_TESSERACT_DICT_INVALID_WORD_RATIO_METADATA_KEY)
            .and_then(serde_json::Value::as_f64)
        {
            self.page_dictionary_invalid_word_ratio.insert(page_number, ratio);
        }
        self.ocr_results.insert(page_number, extraction_result.content);
        Ok(())
    }

    /// Capture the native-coverage and layer-occurrence evidence the replacement decision reads.
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    fn capture_native_coverage(
        &mut self,
        page_idx: usize,
        correction_degrees: u32,
        observed_scale_only: bool,
        extraction_result: &crate::types::ExtractedDocument,
    ) {
        let page_number = (page_idx + 1) as u32;
        let is_tesseract = self.backend.as_ref().is_some_and(|backend| {
            crate::plugins::registry::get_ocr_backend_registry()
                .read()
                .is_builtin_tesseract(backend)
        });
        if correction_degrees == 0
            && !self.alternative_raster_pages.contains(&page_number)
            && let Some(layer) = self
                .native_coverage_layers
                .as_ref()
                .filter(|_| super::document::native_preservation_is_automatic(self.config, page_number))
                .and_then(|layers| layers.get(&page_number))
            && is_tesseract
        {
            self.native_placed_words_pages.insert(
                page_number,
                super::native_word_coverage::read_words_at_native_place(
                    self.render_doc.as_ref(),
                    page_idx,
                    extraction_result,
                ),
            );
            if super::native_word_coverage::covers_complete_native_page(
                self.render_doc.as_ref(),
                page_idx,
                layer,
                extraction_result,
            ) {
                self.native_words_covered_pages.insert(page_number);
            } else if super::native_word_coverage::native_text_paints_all_read_ink(
                self.render_doc.as_ref(),
                page_idx,
                layer,
                extraction_result,
                super::native_word_coverage::InkProofLimits {
                    thresholds: &self.ocr_output_thresholds,
                    max_working_bytes: super::hidden_layer::preview_byte_limit(self.config),
                    cancelled: &|| {
                        self.config
                            .cancel_token
                            .as_ref()
                            .is_some_and(|token| token.is_cancelled())
                    },
                },
            ) {
                self.native_ink_owned_pages.insert(page_number);
            }
        }
        #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
        if captures_matched_layer_occurrences(
            observed_scale_only,
            self.alternative_raster_pages.contains(&page_number),
        ) && let Some(layer) = self.page_layers.as_ref().and_then(|layers| layers.get(&page_number))
            && let Some(matched) =
                super::document::MatchedLayerOccurrences::capture(&self.render_doc, page_idx, layer, extraction_result)
        {
            self.matched_layer_occurrences.insert(page_number, matched);
        }
    }

    /// Apply the Latin-row, minority-Khmer and English-recovery passes to one direct read.
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    fn recover_language_evidence(
        &mut self,
        page_idx: usize,
        correction_degrees: u32,
        extraction_result: &mut crate::types::ExtractedDocument,
    ) {
        let page_number = (page_idx + 1) as u32;
        let is_tesseract = self.backend.as_ref().is_some_and(|backend| {
            crate::plugins::registry::get_ocr_backend_registry()
                .read()
                .is_builtin_tesseract(backend)
        });
        if correction_degrees == 0
            && !self.alternative_raster_pages.contains(&page_number)
            && !self.khmer_supplements.contains_key(&page_number)
            && is_tesseract
        {
            recover_automatic_latin_rows(
                &self.render_doc,
                (page_idx, page_number),
                extraction_result,
                self.page_layers
                    .as_ref()
                    .and_then(|layers| layers.get(&page_number))
                    .map(String::as_str),
                self.config,
                &self.ocr_output_thresholds,
            );
        }
        if correction_degrees == 0
            && self.config.ocr_strategy == crate::core::config::OcrStrategy::Auto
            && self
                .config
                .pdf_options
                .as_ref()
                .is_none_or(|pdf| !pdf.has_nonzero_raw_margins())
            && !self.khmer_supplements.contains_key(&page_number)
            && !self.alternative_raster_pages.contains(&page_number)
            && super::document::native_preservation_is_automatic(self.config, page_number)
            && super::document::resolved_ocr_correction_degrees(&extraction_result.metadata)
                .is_none_or(|angle| angle == 0)
            && is_tesseract
            && super::layer_token_recovery::is_english_read(extraction_result)
        {
            let limits = super::native_word_coverage::InkProofLimits {
                thresholds: &self.ocr_output_thresholds,
                max_working_bytes: super::hidden_layer::preview_byte_limit(self.config),
                cancelled: &|| {
                    self.config
                        .cancel_token
                        .as_ref()
                        .is_some_and(|token| token.is_cancelled())
                },
            };
            if let Some((additions, leading)) =
                super::layer_token_recovery::plan_minority_khmer(&self.render_doc, page_idx, extraction_result, &limits)
                && let Some(rows) = super::layer_token_recovery::apply_minority_khmer(
                    extraction_result,
                    &additions,
                    leading,
                    limits.cancelled,
                )
            {
                self.leading_khmer_rows.insert(page_number, rows);
            }
        }
    }

    /// Capture the symbol-noise evidence that the post-loop pruner reconciles against the read.
    #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
    async fn capture_symbol_noise_evidence(
        &mut self,
        page_idx: usize,
        correction_degrees: u32,
        extraction_result: &crate::types::ExtractedDocument,
        observed_scale_only: bool,
        encoded: &[EncodedPage],
    ) {
        if observed_scale_only
            && correction_degrees == 0
            && self.ocr_output_thresholds.prune_symbol_noise_lines
            && !self.alternative_raster_pages.contains(&((page_idx + 1) as u32))
            && extraction_result.tables.is_empty()
            && !extraction_result
                .metadata
                .additional
                .get(crate::ocr_metadata_keys::OCR_AUTO_ROTATED_METADATA_KEY)
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false)
            && let Some(raster) = encoded.iter().find(|(index, ..)| *index == page_idx)
        {
            if self.certified_background
                && let Some(evidence) = super::symbol_noise_background::Evidence::capture(extraction_result, page_idx)
            {
                self.background_noise_by_page.insert((page_idx + 1) as u32, evidence);
            }
            let planned = self.texture_work.probe_page(|work| {
                if let Some(candidates) =
                    super::symbol_noise_lines::texture::capture(raster, extraction_result, self.config, work)
                {
                    self.texture_noise_by_page.insert((page_idx + 1) as u32, candidates);
                }
                super::symbol_noise_lines::paint::plan(
                    &self.render_doc,
                    page_idx,
                    extraction_result,
                    self.config,
                    &self.ocr_config_resolved,
                    work,
                )
            });
            if let Some(planned) = planned
                && let Some(retained) = planned
                    .verify(
                        std::sync::Arc::clone(&self.render_doc),
                        self.config,
                        &self.ocr_config_resolved,
                    )
                    .await
            {
                self.paint_retention_by_page.insert((page_idx + 1) as u32, retained);
            }
        }
    }

    /// A page whose OCR came back blank or failed is retried on its embedded image XObjects, as
    /// the whole-document route does (#1355, #1444).
    async fn retry_single_backend_xobjects(
        &mut self,
        encoded: &[EncodedPage],
        budget_refused_pages: &[usize],
        abandoned_pages: &ahash::AHashSet<usize>,
        failed_pages: &mut ahash::AHashMap<usize, crate::XbergError>,
        security_limits: &crate::extractors::security::SecurityLimits,
    ) -> OcrRunResult<()> {
        let probing_hidden_layer = self.probing_hidden_layer;
        let retry_pages = encoded
            .iter()
            .map(|(page_idx, png, _, _)| (page_idx, Some(png.as_slice())))
            .chain(budget_refused_pages.iter().map(|page_idx| (page_idx, None)))
            .filter(|(page_idx, _)| !abandoned_pages.contains(*page_idx))
            .filter(|_| !probing_hidden_layer);
        for (page_idx, png) in retry_pages {
            let failure = failed_pages.remove(page_idx);
            if let Some(token) = self.stop_token() {
                if let Some(error) = failure {
                    let native_recovered = mixed_native_page_has_content(self.native_text, self.boundaries, *page_idx);
                    push_mixed_ocr_page_failure_warning(
                        &mut self.accumulated_warnings,
                        &mut self.page_failures,
                        *page_idx,
                        &error,
                        native_recovered,
                    );
                    // Keep the original failure record and classify the skipped retry as a time cut. ~keep
                    self.mark_pages_unread(Some(token), std::iter::once(*page_idx));
                }
                continue;
            }
            let page_number = (page_idx + 1) as u32;
            let page_text = self.ocr_results.get(&page_number).map_or("", String::as_str);
            let trigger = match png {
                Some(png) => page_needs_xobject_fallback(page_text, png, security_limits),
                None => super::rendering::XObjectFallbackTrigger {
                    needs_fallback: true,
                    draw_failed: true,
                },
            };
            let recovery = if trigger.needs_fallback {
                let backend = self.direct_backend()?;
                recover_page_text_from_image_xobjects(
                    &backend,
                    &self.render_doc,
                    *page_idx,
                    self.page_rotations.len(),
                    &self.ocr_config_owned,
                    XObjectRecoveryContext {
                        budget: &mut self.xobject_recovery_budget,
                        blank_raster_warnings: trigger.draw_failed.then_some(&mut self.accumulated_warnings),
                        no_admitted_images: None,
                        stored_jpegs: None,
                        cancel_token: self.config.cancel_token.as_ref(),
                        use_proven_source_dpi: true,
                        proven_read: Some(ProvenReadPolicy {
                            thresholds: &self.ocr_output_thresholds,
                            warn_when_kept: false,
                        }),
                        emit_progress: true,
                    },
                )
                .await?
            } else {
                None
            };
            let mut recovered = false;
            let mut retry_causes = Vec::new();
            if let Some(XObjectRecoveryOutcome {
                text,
                text_outside_tables,
                attempted,
                images,
                mut llm_usage,
                tables,
                mut formulas,
                image_preprocessing,
                tesseract_psm,
                failure_causes,
                warnings,
            }) = recovery
            {
                retry_causes = failure_causes;
                self.accumulated_warnings.extend(warnings);
                let recovered_tables = !tables.is_empty();
                #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
                if recovered_tables {
                    self.matched_layer_occurrences.remove(&page_number);
                }
                let recovered_text_is_empty = text.trim().is_empty();
                if should_adopt_xobject_retry_text(page_text, &text) {
                    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
                    self.alternative_raster_pages.insert(page_number);
                    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
                    self.native_words_covered_pages.remove(&page_number);
                    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
                    self.native_ink_owned_pages.remove(&page_number);
                    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
                    self.native_placed_words_pages.remove(&page_number);
                    // The render's paragraphs, confidence and word count describe the blank
                    // render, not the recovered text.
                    let recovered_paragraphs =
                        crate::pdf::structure::adapters::ocr_text_to_paragraphs(&text_outside_tables);
                    if recovered_paragraphs.is_empty() {
                        self.ocr_page_paragraphs.remove(&page_number);
                    } else {
                        self.ocr_page_paragraphs.insert(page_number, recovered_paragraphs);
                    }
                    self.page_mean_confidence.remove(&page_number);
                    self.page_word_count.remove(&page_number);
                    self.page_dictionary_invalid_word_ratio.remove(&page_number);
                    // The recovered tables print themselves beside these paragraphs (#2014).
                    self.structured_ocr_pages.insert(
                        page_number,
                        super::document::flat_ocr_page_document(&text_outside_tables),
                    );
                    self.ocr_results.insert(page_number, text);
                    recovered = true;
                }
                if !tables.is_empty() {
                    let current_text = self.ocr_results.get(&page_number).map_or("", String::as_str);
                    let page_doc = self
                        .structured_ocr_pages
                        .entry(page_number)
                        .or_insert_with(|| super::document::flat_ocr_page_document(current_text));
                    super::document::attach_page_ocr_payload(page_doc, tables, Vec::new(), page_number);
                    if recovered_text_is_empty {
                        self.table_only_xobject_recovery_pages.insert(page_number);
                    }
                }
                recovered |= recovered_tables;
                if recovered
                    && let Some(psm) = tesseract_psm
                    && let Some(page_doc) = self.structured_ocr_pages.get_mut(&page_number)
                {
                    attach_page_segmentation_modes(page_doc, [(page_number, psm)]);
                }
                self.accumulated_llm_usage.append(&mut llm_usage);
                self.accumulated_formulas.append(&mut formulas);
                if let Some(metadata) = image_preprocessing {
                    self.preprocessing_by_page.insert(page_number, metadata);
                }
                if self.capture_rasters {
                    self.captured_rasters.extend(images);
                }
                self.accumulated_warnings
                    .push(xobject_fallback_warning(*page_idx, attempted, trigger.draw_failed));
            }
            if let Some(error) = failure {
                if !recovered {
                    let error = with_retry_causes(error, &retry_causes);
                    self.record_page_failure(*page_idx, error);
                    continue;
                }
                let recovered_kind = if self
                    .ocr_results
                    .get(&page_number)
                    .is_some_and(|text| !text.trim().is_empty())
                {
                    "text"
                } else {
                    "content"
                };
                self.recovery_warnings.push((
                    page_number,
                    crate::types::ProcessingWarning {
                        source: std::borrow::Cow::Borrowed("ocr"),
                        message: std::borrow::Cow::Owned(format!(
                            "OCR of page {page_number} failed ({error}); its {recovered_kind} was recovered from the \
                             page's embedded image XObjects instead."
                        )),
                    },
                ));
                merge_ocr_page_failures(
                    &mut self.page_failures,
                    vec![crate::types::OcrPageFailure {
                        page: page_number,
                        error: error.to_string(),
                        recovered: true,
                    }],
                );
            }
        }
        Ok(())
    }
}

#[cfg(all(
    any(feature = "ocr", feature = "ocr-pipeline"),
    feature = "pdf",
    feature = "tokio-runtime",
    not(target_arch = "wasm32")
))]
type SingleBackendTaskOutput = (
    usize,
    u32,
    u32,
    u32,
    (
        crate::Result<crate::types::ExtractedDocument>,
        Option<crate::cancellation::CancellationToken>,
    ),
    bool,
    Option<crate::pdf::render::OcrRasterDpi>,
);

#[cfg(all(
    any(feature = "ocr", feature = "ocr-pipeline"),
    feature = "pdf",
    feature = "tokio-runtime",
    not(target_arch = "wasm32")
))]
struct SingleBackendRead {
    page_idx: usize,
    correction_degrees: u32,
    upright_width: u32,
    upright_height: u32,
    result: crate::Result<crate::types::ExtractedDocument>,
    observed_scale_only: bool,
    retry_dpi: Option<crate::pdf::render::OcrRasterDpi>,
}

/// Encode the batch's rasters for the OCR backend, in parallel where OS threads exist.
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
fn encode_selected_pages(
    page_images: &[(usize, image::DynamicImage)],
    security_limits: &crate::extractors::security::SecurityLimits,
) -> crate::Result<Vec<EncodedPage>> {
    #[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
    {
        page_images
            .par_iter()
            .map(|(page_idx, image)| encode_one_page(*page_idx, image, security_limits))
            .collect()
    }
    #[cfg(any(not(feature = "tokio-runtime"), target_arch = "wasm32"))]
    {
        page_images
            .iter()
            .map(|(page_idx, image)| encode_one_page(*page_idx, image, security_limits))
            .collect()
    }
}

#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
fn encode_one_page(
    page_idx: usize,
    image: &image::DynamicImage,
    security_limits: &crate::extractors::security::SecurityLimits,
) -> crate::Result<EncodedPage> {
    let rgb = clone_rgb_for_png_encode(image, security_limits)?;
    let (w, h) = rgb.dimensions();
    let mut buf = Cursor::new(Vec::new());
    PngEncoder::new(&mut buf)
        .write_image(&rgb, w, h, image::ColorType::Rgb8.into())
        .map_err(|e| crate::XbergError::Parsing {
            message: format!("Failed to encode page {} for OCR: {}", page_idx + 1, e),
            source: None,
        })?;
    Ok((page_idx, std::sync::Arc::new(buf.into_inner()), w, h))
}

#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
impl MixedOcrContext<'_> {
    /// Judge the collected reads, decide which pages they replace, and assemble the run.
    fn finish(&mut self) -> OcrRunResult<MixedOcrRun> {
        if self.all_pages_failed_policy == AllPagesFailedPolicy::ReturnError
            && !self.page_indices.is_empty()
            && self
                .page_indices
                .iter()
                .all(|page_idx| self.failed_page_errors.contains_key(page_idx))
        {
            let error = run_error_for_failed_pages(&self.page_indices, &mut self.failed_page_errors);
            return Err(FailedOcrRun {
                page_failures: std::mem::take(&mut self.page_failures),
                ..error.into()
            });
        }

        self.assess_direct_backend_reads();
        self.apply_recovery_claims();
        self.repair_page_texts();

        let native_preservation_enabled = self.native_preservation_enabled();
        #[cfg(all(feature = "ocr", not(target_arch = "wasm32"), feature = "layout-detection"))]
        let ink_proof_layout_free = self.resolved_layout_inputs.is_none();
        #[cfg(not(all(feature = "ocr", not(target_arch = "wasm32"), feature = "layout-detection")))]
        let ink_proof_layout_free = true;

        let mut accepted_replacements = self.decide_replacements();
        self.prune_noise_lines(&mut accepted_replacements);

        #[cfg_attr(
            not(all(feature = "ocr", not(target_arch = "wasm32"))),
            allow(unused_mut, unused_variables)
        )]
        let (mut complete_native_pages, clean_native_ink_pages, page_layers_first) = self.assemble_layer_prepend(
            &mut accepted_replacements,
            native_preservation_enabled,
            ink_proof_layout_free,
        );
        super::super::ensure_pdf_not_cancelled(self.config)?;
        self.structured_ocr_pages.retain(|page, _| {
            accepted_replacements.contains_key(page) || self.table_only_xobject_recovery_pages.contains(page)
        });
        retain_ocr_formulas_for_accepted_pages(&mut self.accumulated_formulas, &accepted_replacements);

        self.restructure_pages()?;

        #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
        let (type0_loss_pages, lost_glyph_ink_pages) = self.detect_type0_and_lost_glyph_pages(
            &accepted_replacements,
            &mut complete_native_pages,
            native_preservation_enabled,
            ink_proof_layout_free,
        );
        self.reject_native_yielding_pages(
            &mut accepted_replacements,
            complete_native_pages,
            #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
            &clean_native_ink_pages,
            #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
            &type0_loss_pages,
            #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
            &lost_glyph_ink_pages,
        )?;

        #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
        self.emit_type0_retention_warnings(&type0_loss_pages, &accepted_replacements);
        self.finish_layer_prepend(&mut accepted_replacements, page_layers_first);
        self.emit_empty_text_warnings();
        self.apply_caption_rewrites(&mut accepted_replacements);
        self.apply_khmer_supplements(&mut accepted_replacements);
        self.emit_unrecovered_glyph_warnings(&accepted_replacements);

        Ok(self.assemble_run(accepted_replacements))
    }

    fn native_preservation_enabled(&self) -> bool {
        let enabled = self.effective_pipeline.is_none() && !self.probing_hidden_layer;
        #[cfg(feature = "layout-detection")]
        let enabled = enabled && self.resolved_layout_inputs.is_none();
        enabled
    }

    /// Pipeline stages already assess their output in `extract_with_ocr_for_page` using the
    /// actual producing backend. Assess only the direct single-backend route here to avoid
    /// duplicate warnings and guessed pipeline confidence semantics. ~keep
    fn assess_direct_backend_reads(&mut self) {
        let Some(producing_backend) = self.backend.as_ref() else {
            return;
        };
        let confidence_semantics = producing_backend.confidence_semantics();
        let producing_backend_name = producing_backend.name().to_string();
        for (page_number, text) in &mut self.ocr_results {
            let confidence = self.page_mean_confidence.get(page_number).copied();
            let dictionary_ratio = self.page_dictionary_invalid_word_ratio.get(page_number).copied();
            tracing::debug!(page = *page_number, ?confidence, "OCR page mean confidence");
            // Recorded before the accept/reject call below, and never retracted afterwards: a
            // page the noise gate discards was still OCR'd, and its confidence is exactly the
            // evidence a caller needs to understand why it was discarded (#1568). ~keep
            if let Some(summary) = page_ocr_confidence(
                confidence_semantics,
                confidence,
                self.page_word_count.get(page_number).copied().unwrap_or(0),
                &producing_backend_name,
            ) {
                self.ocr_confidence_by_page.insert(*page_number, summary);
            }
            let acceptance = accept_or_reject_ocr_page(
                (*page_number as usize).saturating_sub(1),
                std::mem::take(text),
                &self.ocr_output_thresholds,
                &mut self.accumulated_warnings,
                dictionary_ratio,
                confidence_semantics,
                confidence,
            );
            if acceptance.discarded {
                self.discarded_recovery_pages.insert(*page_number);
            }
            // The verdict decides whether the page keeps its text layer (see below). A page
            // the noise gate discarded has blank text and keeps the layer already. ~keep
            if acceptance.verdict.is_some() && !acceptance.discarded {
                self.noise_verdict_pages.insert(*page_number);
            }
            if acceptance.verdict.is_some_and(|verdict| verdict.low_confidence) {
                self.low_confidence_pages.insert(*page_number);
            }
            if acceptance.verdict.is_some_and(|verdict| verdict.dictionary_noise) {
                self.dictionary_noise_pages.insert(*page_number);
            }
            *text = acceptance.content;
        }
    }

    /// Recovery claims describe retained reads, after the caller's noise policy ran. ~keep
    fn apply_recovery_claims(&mut self) {
        for (page, warning) in std::mem::take(&mut self.recovery_warnings) {
            if self.discarded_recovery_pages.contains(&page) {
                let native_recovered =
                    mixed_native_page_has_content(self.native_text, self.boundaries, (page - 1) as usize);
                let outcome = if native_recovered {
                    "the page's native text was kept"
                } else {
                    "the page had no native text to retain"
                };
                for failure in self.page_failures.iter_mut().filter(|failure| failure.page == page) {
                    failure.recovered = native_recovered;
                    crate::core::diagnostics::push_warning_deduped(
                        &mut self.accumulated_warnings,
                        crate::core::diagnostics::warning(
                            "ocr",
                            format!("OCR of page {page} failed ({}); {outcome}.", failure.error),
                        ),
                    );
                }
            } else {
                self.accumulated_warnings.push(warning);
            }
        }
    }

    fn repair_page_texts(&mut self) {
        for text in self.ocr_results.values_mut() {
            if let std::borrow::Cow::Owned(repaired) = repair_ocr_list_markers(text) {
                *text = repaired;
            }
        }
        // Opt-in (GH#1789): unlike the list-marker repair above, this can misjudge a genuine
        // European-locale decimal, so it only runs when the caller has asserted the source
        // documents use the US/UK number convention. See `OcrConfig::numeric_repair`'s doc comment.
        if numeric_repair_enabled(self.config) {
            for text in self.ocr_results.values_mut() {
                if let std::borrow::Cow::Owned(repaired) = repair_ocr_numeric_tokens(text) {
                    *text = repaired;
                }
            }
        }
    }

    /// Decide which pages the OCR reads replace, including the native-script and preserve-native
    /// vetoes.
    fn decide_replacements(&mut self) -> ahash::AHashMap<u32, String> {
        // The same page's ink proof (native text paints the ink OCR read, or covers its words) is what
        // lets a marked page be judged on the text it held. A lost Type0 glyph in the render is no such
        // proof: that case keeps its own check (`native_survives_type0_ink_loss`). ~keep
        #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
        let ink_corroborated =
            |page: u32| self.native_ink_owned_pages.contains(&page) || self.native_words_covered_pages.contains(&page);
        #[cfg(not(all(feature = "ocr", not(target_arch = "wasm32"))))]
        let ink_corroborated = |_page: u32| false;
        #[cfg_attr(not(all(feature = "ocr", not(target_arch = "wasm32"))), allow(unused_mut))]
        let mut accepted_replacements = accepted_ocr_page_replacements(
            self.native_text,
            self.boundaries,
            &self.ocr_results,
            &self.ocr_output_thresholds,
            |page| {
                super::document::marked_page_is_judged_on_readable_text(
                    self.config,
                    page,
                    self.probing_hidden_layer,
                    ink_corroborated(page),
                )
            },
        );
        #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
        for boundary in self.boundaries {
            let page = boundary.page_number;
            if self.pages.native_script.contains(&page)
                && let Some(text) = accepted_replacements.get(&page)
                && let Some(native) = self.native_text.get(boundary.byte_start..boundary.byte_end)
                && super::super::unreadable_script::ocr_dropped_native_script(native, text)
            {
                accepted_replacements.remove(&page);
                self.structured_ocr_pages.remove(&page);
                self.ocr_confidence_by_page.remove(&page);
                self.preprocessing_by_page.remove(&page);
                crate::core::diagnostics::push_warning_deduped(
                    &mut self.accumulated_warnings,
                    super::super::unreadable_script::ocr_dropped_script_warning(page, native),
                );
            }
        }
        #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
        if self.probing_hidden_layer || !self.pages.preserve_native.is_empty() {
            for boundary in self.boundaries {
                let page = boundary.page_number;
                if (self.probing_hidden_layer || self.pages.preserve_native.contains(&page))
                    && let Some(text) = accepted_replacements.get(&page)
                    && (self.noise_verdict_pages.contains(&page)
                        || !self
                            .native_text
                            .get(boundary.byte_start..boundary.byte_end)
                            .is_some_and(|native| {
                                let preserves = |candidate: &str| {
                                    super::hidden_layer::adds_text_preserving_native(native, candidate)
                                        && (self.probing_hidden_layer
                                            || super::hidden_layer::keeps_native_lines_in_order(native, candidate))
                                };
                                preserves(text)
                                    || (!self.probing_hidden_layer
                                        && self.structured_ocr_pages.get(&page).is_some_and(|doc| {
                                            super::hidden_layer::table_read_preserves_native(native, text, &doc.tables)
                                        }))
                            }))
                    // Every probed page passed the hidden-text audit: all of its native text is render
                    // mode 3. A confident read has a calibrated score that clears the default gate and
                    // no low-confidence or dictionary-noise verdict. The fragmentation signal does not
                    // veto it: a table of numbers is mostly short tokens. ~keep
                    && !(self.probing_hidden_layer
                        && self
                            .native_text
                            .get(boundary.byte_start..boundary.byte_end)
                            .is_some_and(|native| {
                                super::hidden_layer::confident_read_replaces_layer(
                                    self.low_confidence_pages.contains(&page),
                                    self.dictionary_noise_pages.contains(&page),
                                    self.ocr_confidence_by_page.get(&page).and_then(|confidence| confidence.score),
                                    native,
                                    text,
                                )
                            }))
                {
                    accepted_replacements.remove(&page);
                }
            }
        }
        accepted_replacements
    }

    /// The direct single-backend read has no per-page pipeline stage, so it prunes here, after the
    /// verdict and the replacement decision judged the unpruned text (#2312, #2921).
    fn prune_noise_lines(&mut self, accepted_replacements: &mut ahash::AHashMap<u32, String>) {
        if !self.ocr_output_thresholds.prune_symbol_noise_lines {
            return;
        }
        for page in &self.noise_verdict_pages {
            let Some(text) = accepted_replacements.get_mut(page) else {
                continue;
            };
            if self
                .structured_ocr_pages
                .get(page)
                .is_some_and(|doc| !doc.tables.is_empty() || !super::symbol_noise_lines::elements_are_prunable(doc))
                || !super::symbol_noise_lines::page_is_dominated_by_noise_lines(text.lines())
            {
                continue;
            }
            #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
            if !self.alternative_raster_pages.contains(page)
                && !self
                    .config
                    .cancel_token
                    .as_ref()
                    .is_some_and(|token| token.is_cancelled())
                && let Some(read) = self.ocr_results.get(page)
                && (self.texture_noise_by_page.contains_key(page)
                    || self.paint_retention_by_page.contains_key(page)
                    || self.background_noise_by_page.contains_key(page))
            {
                let background = self
                    .background_noise_by_page
                    .get(page)
                    .filter(|_| text == read)
                    .and_then(|evidence| evidence.reconcile(text));
                let pcb_row = self
                    .texture_noise_by_page
                    .get(page)
                    .and_then(|evidence| evidence.reconciled_pcb_row(text, read));
                let mut candidates = self
                    .texture_noise_by_page
                    .get(page)
                    .map(|evidence| evidence.reconciled_tokens(text, read))
                    .unwrap_or_default();
                if let Some(word) = background.filter(|word| !candidates.contains(word)) {
                    candidates.push(word);
                }
                let retained = self
                    .paint_retention_by_page
                    .get(page)
                    .filter(|_| text == read)
                    .and_then(|evidence| evidence.reconcile(text));
                let (removed, texture_removed, pcb_removed) = super::symbol_noise_lines::texture::prune_mixed(
                    text,
                    self.ocr_results.get_mut(page),
                    self.ocr_page_paragraphs.get_mut(page),
                    self.structured_ocr_pages.get_mut(page),
                    super::symbol_noise_lines::texture::MixedPruningEvidence {
                        candidates: &candidates,
                        retained,
                        pcb_row,
                    },
                );
                crate::core::diagnostics::push_warning_deduped(
                    &mut self.accumulated_warnings,
                    super::symbol_noise_lines::pruned_lines_warning(*page as usize, removed),
                );
                let background_removed = usize::from(
                    texture_removed != 0
                        && background.is_some_and(|word| !text.split_whitespace().any(|token| token == word)),
                );
                if background_removed != 0 {
                    crate::core::diagnostics::push_warning_deduped(
                        &mut self.accumulated_warnings,
                        super::symbol_noise_background::warning(*page, background_removed),
                    );
                }
                if texture_removed + pcb_removed > background_removed {
                    crate::core::diagnostics::push_warning_deduped(
                        &mut self.accumulated_warnings,
                        super::symbol_noise_lines::texture::warning(
                            *page,
                            texture_removed + pcb_removed - background_removed,
                        ),
                    );
                }
                continue;
            }
            let (pruned, removed) = super::symbol_noise_lines::prune_text(text);
            *text = pruned;
            // Later readers compare the read with the structured page, so both lose the same lines. ~keep
            if let Some(read) = self.ocr_results.get_mut(page) {
                *read = super::symbol_noise_lines::prune_text(read).0;
            }
            if let Some(paragraphs) = self.ocr_page_paragraphs.get_mut(page) {
                super::symbol_noise_lines::prune_paragraphs(paragraphs);
            }
            if let Some(document) = self.structured_ocr_pages.get_mut(page) {
                super::symbol_noise_lines::prune_document_elements(document);
            }
            crate::core::diagnostics::push_warning_deduped(
                &mut self.accumulated_warnings,
                super::symbol_noise_lines::pruned_lines_warning(*page as usize, removed),
            );
        }
    }

    /// The replacement is decided on the OCR text alone. A page that it replaces, whose OCR text
    /// got the noise verdict or confirms the layer, now gets the layer lines that its OCR text
    /// lacks first. A page it keeps native stays as it was. ~keep
    fn assemble_layer_prepend(
        &mut self,
        accepted_replacements: &mut ahash::AHashMap<u32, String>,
        native_preservation_enabled: bool,
        ink_proof_layout_free: bool,
    ) -> (
        std::collections::HashSet<u32>,
        std::collections::HashSet<u32>,
        ahash::AHashMap<u32, super::document::LayerFirst>,
    ) {
        #[cfg(not(all(feature = "ocr", not(target_arch = "wasm32"))))]
        let _ = ink_proof_layout_free;
        let mut page_layers_first: ahash::AHashMap<u32, super::document::LayerFirst> = ahash::AHashMap::new();
        let mut complete_native_pages = std::collections::HashSet::new();
        #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
        let mut clean_native_ink_pages = std::collections::HashSet::new();
        #[cfg(not(all(feature = "ocr", not(target_arch = "wasm32"))))]
        let clean_native_ink_pages = std::collections::HashSet::new();
        if let Some(layers) = self.page_layers.as_ref() {
            let mut geometry = MergeGeometrySession::new(self.content, self.config);
            for (page_number, text) in accepted_replacements.iter_mut() {
                if geometry.cancelled() {
                    break;
                }
                let ocr_scale = match self.ocr_raster_by_page.get(page_number) {
                    Some(&(width_px, height_px)) => {
                        let (width_pt, height_pt) =
                            page_dimensions_pt(&self.render_doc, page_number.saturating_sub(1) as usize);
                        (width_px > 0 && height_px > 0)
                            .then(|| (width_pt / width_px as f32, height_pt / height_px as f32))
                    }
                    None => Some((1.0, 1.0)),
                };
                let confirmation_text = text.as_str();
                #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
                let confirmation_text = self
                    .number_primary_texts
                    .get(page_number)
                    .map_or(confirmation_text, String::as_str);
                let boxes = layers.get(page_number).and_then(|layer| {
                    if self.noise_verdict_pages.contains(page_number)
                        || !super::document::ocr_confirms_layer(layer, confirmation_text, &self.ocr_output_thresholds)
                    {
                        return None;
                    }
                    geometry.page_boxes(*page_number, self.ocr_page_paragraphs.get(page_number), ocr_scale)
                });
                #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
                let clean_ink_layer = clean_ink_layer_of(self.native_coverage_layers.as_deref(), page_number);
                #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
                if self.native_ink_owned_pages.contains(page_number)
                    && ink_proof_layout_free
                    && super::document::native_preservation_is_automatic(self.config, *page_number)
                    && !self.noise_verdict_pages.contains(page_number)
                    && self.structured_ocr_pages.get(page_number).is_some_and(|doc| {
                        super::native_word_coverage::has_only_certified_tesseract_text(doc)
                            || super::native_word_coverage::ink_owned_table_structure_can_yield_to_native(
                                doc,
                                confirmation_text,
                                clean_ink_layer,
                            )
                    })
                    && layers.get(page_number).is_some_and(|layer| {
                        let mut stripped = layer.clone();
                        super::super::unmapped_glyphs::strip_native_fragment(&mut stripped);
                        super::document::ocr_confirms_layer(&stripped, confirmation_text, &self.ocr_output_thresholds)
                    })
                {
                    complete_native_pages.insert(*page_number);
                    // Marker-bearing candidates can own ink too; only this clean-layer admission permits omissions. ~keep
                    if clean_ink_layer.is_some() {
                        clean_native_ink_pages.insert(*page_number);
                    }
                }
                if native_preservation_enabled
                    && super::document::native_preservation_is_automatic(self.config, *page_number)
                    && !self.noise_verdict_pages.contains(page_number)
                    && self
                        .structured_ocr_pages
                        .get(page_number)
                        .is_some_and(super::document::ocr_page_has_only_plain_payload)
                    && layers.get(page_number).is_some_and(|layer| {
                        super::document::native_page_covers_ocr(
                            layer,
                            confirmation_text,
                            &self.ocr_output_thresholds,
                            boxes.as_ref(),
                        )
                    })
                {
                    complete_native_pages.insert(*page_number);
                }
                #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
                let remaining = self
                    .leading_khmer_rows
                    .get(page_number)
                    .zip(layers.get(page_number))
                    .map_or_else(Default::default, |(rows, layer)| {
                        rows.prepend_policy(layer, confirmation_text)
                    });
                #[cfg(not(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32"))))]
                let remaining = super::document::NativePrepend::Original;
                // A page whose native script the OCR read kept is replaced by that read: the script rule
                // decided it, so no native row is put first. ~keep
                if !self.pages.native_script.contains(page_number)
                    && let Some(layer) = layers.get(page_number)
                    && let Some((source, independent_read)) = remaining.inputs(layer, confirmation_text)
                    && let Some((_, read)) = remaining.inputs(layer, text)
                    && let Some(first) = mixed_layer_lines_to_put_first(
                        source,
                        independent_read,
                        read,
                        self.noise_verdict_pages.contains(page_number),
                        &self.ocr_output_thresholds,
                        boxes
                            .as_ref()
                            .filter(|_| matches!(remaining, super::document::NativePrepend::Original)),
                        super::document::PlainOcrRows::of_page(self.structured_ocr_pages.get(page_number), None),
                    )
                {
                    if let Some(warning) =
                        super::super::unmapped_glyphs::lost_glyph_warning(first.lost_glyphs, self.config)
                    {
                        crate::core::diagnostics::push_warning_deduped(&mut self.accumulated_warnings, warning);
                    }
                    if first.lines.is_empty() {
                        continue;
                    }
                    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
                    if self.native_words_covered_pages.contains(page_number)
                        && !self.noise_verdict_pages.contains(page_number)
                        && self
                            .structured_ocr_pages
                            .get(page_number)
                            .is_some_and(super::native_word_coverage::has_only_certified_tesseract_text)
                        && super::document::unique_literal_embedding(layer, confirmation_text)
                    {
                        complete_native_pages.insert(*page_number);
                    }
                    page_layers_first.insert(*page_number, first);
                }
            }
        }
        (complete_native_pages, clean_native_ink_pages, page_layers_first)
    }

    /// Document-global heading/list heuristic (same font-clustering pass the OCR-only
    /// route runs via `extract_with_ocr_for_page`, previously unreachable from this mixed
    /// route -- see `ocr_page_paragraphs`'s own comment above). Runs once, over every
    /// accepted OCR'd page's paragraphs at once: the heuristic clusters font sizes
    /// document-wide and needs several pages in hand, which is exactly what the per-page
    /// callers above could not give it on their own.
    fn restructure_pages(&mut self) -> OcrRunResult<()> {
        if self.structured_ocr_pages.is_empty() {
            return Ok(());
        }
        let mut pages_for_heuristic: Vec<Vec<crate::pdf::structure::types::PdfParagraph>> =
            vec![Vec::new(); self.page_count];
        for (&page_number, paragraphs) in &self.ocr_page_paragraphs {
            // `structured_ocr_pages` (not `ocr_page_paragraphs`) is the source of truth for
            // which pages are still in play: the destructive noise-filter opt-in above may
            // have dropped a page after its paragraphs were already collected.
            if self.structured_ocr_pages.contains_key(&page_number)
                && let Some(slot) = pages_for_heuristic.get_mut((page_number - 1) as usize)
            {
                slot.extend_from_slice(ocr_heuristic_input(
                    paragraphs,
                    self.preserve_ordered_layout.contains(&page_number),
                ));
            }
        }
        let tables_for_heuristic: Vec<crate::types::Table> = self
            .structured_ocr_pages
            .values()
            .flat_map(|doc| doc.tables.iter().cloned())
            .collect();
        let mut ocr_page_numbers: Vec<u32> = self.structured_ocr_pages.keys().copied().collect();
        ocr_page_numbers.sort_unstable();
        let page_heights_for_heuristic = (0..self.page_count)
            .map(|page_index| page_dimensions_pt(&self.render_doc, page_index).1)
            .collect::<Vec<_>>();
        let mut split_pages = heuristically_restructured_ocr_pages(
            &pages_for_heuristic,
            &page_heights_for_heuristic,
            &tables_for_heuristic,
            self.config,
        )
        .map(|combined_doc| split_document_global_ocr_structure_by_page(combined_doc, &ocr_page_numbers))
        .unwrap_or_default();
        // A page the heuristic did not classify (document declined, or this page dropped)
        // falls back to rebuilding from `ocr_page_paragraphs`, and that path never ran the
        // native route's cross-page furniture pass. Strip the paragraph lists once,
        // document-wide, so a running header repeating across scanned pages is detected:
        // stripping the assembled page documents instead would miss the repetition it
        // needs to compare. ~keep
        #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
        let filter = self.config.content_filter.clone().unwrap_or_default();
        #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
        let mut stripped_fallback_pages: Option<Vec<Vec<crate::pdf::structure::types::PdfParagraph>>> = None;
        #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
        if split_pages.len() < ocr_page_numbers.len() && filter.strip_repeating_text {
            let mut fallback_pages: Vec<Vec<crate::pdf::structure::types::PdfParagraph>> =
                vec![Vec::new(); self.page_count];
            for page_number in &ocr_page_numbers {
                if let (Some(paragraphs), Some(slot)) = (
                    self.ocr_page_paragraphs.get(page_number),
                    fallback_pages.get_mut((*page_number - 1) as usize),
                ) {
                    slot.extend_from_slice(paragraphs);
                }
            }
            strip_fallback_furniture_in_page_points(
                &mut fallback_pages,
                &ocr_page_numbers,
                &self.ocr_raster_by_page,
                |page_number| {
                    self.structured_ocr_pages
                        .get(&page_number)
                        .is_some_and(|existing| detached_mixed_frame::read_sideways(&existing.metadata))
                },
                |page_number| page_dimensions_pt(&self.render_doc, page_number.saturating_sub(1) as usize),
                &page_heights_for_heuristic,
                &filter,
            );
            stripped_fallback_pages = Some(fallback_pages);
        }
        for page_number in &ocr_page_numbers {
            let Some(existing) = self.structured_ocr_pages.get(page_number) else {
                continue;
            };
            if !self.ocr_page_paragraphs.contains_key(page_number) {
                continue;
            }
            let mut new_page_doc = match split_pages.remove(page_number) {
                // The heuristic's combined document has no notion of the backend's raw
                // per-word OCR elements, this page's earlier warnings, or its OCR coordinate
                // frame -- all three come from the fallback per-page document already built
                // above (`carry_page_ocr_payload_forward`); only the *structural* elements
                // (headings/paragraphs/list items/tables) come from the document-global pass.
                Some(mut new_page_doc) => {
                    #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
                    if self.config.output_format == crate::core::config::OutputFormat::Markdown {
                        match self.ocr_raster_by_page.get(page_number) {
                            Some(&(width, height)) if width > 0 && height > 0 => {
                                let media_box = self
                                    .render_doc
                                    .get_page_media_box(page_number.saturating_sub(1) as usize);
                                if let Some(scale) = super::document::MatchedLayerOccurrences::raster_to_page_scale(
                                    media_box.ok(),
                                    (width, height),
                                ) {
                                    self.reflow_scales.insert(*page_number, scale);
                                }
                            }
                            None if self.paragraphs_in_page_points.contains(page_number) => {
                                self.reflow_scales.insert(*page_number, (1.0, 1.0));
                            }
                            _ => {}
                        }
                    }
                    carry_page_ocr_payload_forward(existing, &mut new_page_doc);
                    new_page_doc
                }
                // The heuristic either didn't run at all for this document (Plain output, or
                // some other page was already ML-layout-classified -- `already_structured` is
                // document-wide) or dropped this specific page. Either way this page's
                // paragraphs never got ANY list-item classification (#713): apply a
                // text-marker-only fallback pass, which never touches `heading_level`, so it
                // cannot regress heading detection the way pre-classifying before the
                // heuristic call would.
                None => {
                    #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
                    let mut paragraphs = stripped_fallback_pages
                        .as_ref()
                        .and_then(|pages| pages.get((*page_number - 1) as usize))
                        .cloned()
                        .unwrap_or_else(|| self.ocr_page_paragraphs.get(page_number).cloned().unwrap_or_default());
                    #[cfg(not(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32"))))]
                    let mut paragraphs = self.ocr_page_paragraphs.get(page_number).cloned().unwrap_or_default();
                    apply_ocr_text_list_fallback(&mut paragraphs);
                    // #729: recover a bare marker no ML hint ever classified, e.g. a
                    // marker paragraph whose body landed several paragraphs away --
                    // see `adapters::reattach_detached_ocr_list_markers`'s doc comment.
                    // #760: threaded with this page's known PDF `/Rotate` so the
                    // reattachment's baseline/indent comparison runs in the rotation-
                    // corrected frame instead of the raw raster one.
                    let page_rotation_degrees = self
                        .page_rotations
                        .get((*page_number - 1) as usize)
                        .copied()
                        .unwrap_or(0);
                    crate::pdf::structure::adapters::reattach_detached_ocr_list_markers(
                        &mut paragraphs,
                        page_rotation_degrees,
                    );
                    // ~keep This rebuild publishes paragraph boxes in page points. Convert
                    // after the list passes, which measure marker geometry in raster pixels.
                    // The raster map only contains pages whose paragraphs still need scaling.
                    // Keep the sideways reading-order exception for heading/table placement.
                    if !detached_mixed_frame::read_sideways(&existing.metadata)
                        && let Some(&(width_px, height_px)) = self.ocr_raster_by_page.get(page_number)
                        && width_px > 0
                        && height_px > 0
                    {
                        let (width_pt, height_pt) =
                            page_dimensions_pt(&self.render_doc, page_number.saturating_sub(1) as usize);
                        detached_mixed_frame::scale_paragraph_geometry(
                            &mut paragraphs,
                            (width_pt / width_px as f32, height_pt / height_px as f32),
                        );
                    }
                    let mut new_page_doc = crate::pdf::structure::assemble_internal_document(
                        vec![paragraphs],
                        &existing.tables,
                        Some(&existing.images),
                        &[],
                        &Default::default(),
                    );
                    carry_page_ocr_payload_forward(existing, &mut new_page_doc);
                    #[cfg(all(
                        feature = "ocr",
                        feature = "layout-detection",
                        feature = "tokio-runtime",
                        not(target_arch = "wasm32")
                    ))]
                    if let Some(lanes) = self.column_lanes_by_page.remove(page_number) {
                        let page_size = page_dimensions_pt(&self.render_doc, page_number.saturating_sub(1) as usize);
                        apply_owned_column_order(
                            &mut new_page_doc,
                            lanes,
                            page_size,
                            self.config.cancel_token.as_ref(),
                        )?;
                    }
                    new_page_doc
                }
            };
            super::document::recertify_scan_html_paragraph(existing, &mut new_page_doc);
            self.structured_ocr_pages.insert(*page_number, new_page_doc);
        }
        // GH#1840: the flat-string repair above only reaches the rendered output for Plain format.
        // Every other renderer rebuilds each OCR'd page from `structured_ocr_pages`
        // (`select_pdf_document` -> `merge_structured_ocr_pages_into_internal_document`), and the block
        // above has just rebuilt every page in that map from `ocr_page_paragraphs`, so this has to run
        // here rather than beside the repair at the top. ~keep
        if numeric_repair_enabled(self.config) {
            apply_numeric_repair_to_structured_ocr_pages(&mut self.structured_ocr_pages);
        }
        Ok(())
    }

    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    fn detect_type0_and_lost_glyph_pages(
        &mut self,
        accepted_replacements: &ahash::AHashMap<u32, String>,
        complete_native_pages: &mut std::collections::HashSet<u32>,
        native_preservation_enabled: bool,
        ink_proof_layout_free: bool,
    ) -> (ahash::AHashSet<u32>, ahash::AHashMap<u32, usize>) {
        let mut type0_loss_pages = ahash::AHashSet::new();
        let mut lost_glyph_ink_pages = ahash::AHashMap::new();
        if self.probing_hidden_layer
            || self.mixed_route_layout_active
            || self.config.ocr_strategy != crate::core::config::OcrStrategy::Auto
            || !self.config.ocr.as_ref().is_some_and(uses_default_ocr_pipeline)
        {
            return (type0_loss_pages, lost_glyph_ink_pages);
        }
        let direct_tesseract = native_preservation_enabled
            && self.backend.as_ref().is_some_and(|backend| {
                crate::plugins::registry::get_ocr_backend_registry()
                    .read()
                    .is_builtin_tesseract(backend)
            });
        let no_placed_words = std::collections::HashSet::new();
        for boundary in self.boundaries {
            let page = boundary.page_number;
            let native = if self.page_count == 1 && page == 1 {
                self.pages.single_native_type0_page
            } else {
                None
            }
            .or_else(|| self.native_text.get(boundary.byte_start..boundary.byte_end));
            if (direct_tesseract || self.original_tesseract_raster_pages.contains(&page))
                && !self.alternative_raster_pages.contains(&page)
                && self
                    .raster_dpis
                    .get(&(page.saturating_sub(1) as usize))
                    .is_some_and(|raster| raster.type0_ink_loss)
                && let (Some(native), Some(read)) = (native, self.ocr_results.get(&page))
                && super::document::type0_native_is_corroborated(
                    native,
                    read,
                    &self.ocr_output_thresholds,
                    super::document::Type0ReadEvidence {
                        ink_owned: self.native_ink_owned_pages.contains(&page),
                        words_covered: self.native_words_covered_pages.contains(&page),
                        placed_words: self.native_placed_words_pages.get(&page).unwrap_or(&no_placed_words),
                    },
                )
            {
                complete_native_pages.insert(page);
                type0_loss_pages.insert(page);
            }
            if self.native_ink_owned_pages.contains(&page)
                && ink_proof_layout_free
                && native.is_some_and(|native| native.contains(super::super::unmapped_glyphs::is_lost_glyph))
                && accepted_replacements.contains_key(&page)
                && !self.noise_verdict_pages.contains(&page)
            {
                complete_native_pages.insert(page);
                let lost = native.map_or(0, |native| {
                    native.matches(super::super::unmapped_glyphs::is_lost_glyph).count()
                });
                lost_glyph_ink_pages.insert(page, lost);
            }
        }
        (type0_loss_pages, lost_glyph_ink_pages)
    }

    /// Reject only certified automatic replacements, preserving the original native page objects.
    /// Recheck the final OCR structure because the document-global pass can add rich payload. ~keep
    #[allow(clippy::too_many_arguments)]
    fn reject_native_yielding_pages(
        &mut self,
        accepted_replacements: &mut ahash::AHashMap<u32, String>,
        complete_native_pages: std::collections::HashSet<u32>,
        #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))] clean_native_ink_pages: &std::collections::HashSet<
            u32,
        >,
        #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))] type0_loss_pages: &ahash::AHashSet<u32>,
        #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))] lost_glyph_ink_pages: &ahash::AHashMap<u32, usize>,
    ) -> OcrRunResult<()> {
        for page in complete_native_pages {
            #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
            let plain = self
                .structured_ocr_pages
                .get(&page)
                .zip(self.ocr_results.get(&page))
                .is_some_and(|(doc, text)| {
                    let text = text.as_str();
                    #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
                    let text = self.number_primary_texts.get(&page).map_or(text, String::as_str);
                    super::native_word_coverage::final_ocr_structure_can_yield_to_native(
                        page,
                        doc,
                        text,
                        super::native_word_coverage::FinalOcrStructureYieldSets {
                            native_words_covered_pages: &self.native_words_covered_pages,
                            native_ink_owned_pages: &self.native_ink_owned_pages,
                            native_coverage_layers: self.native_coverage_layers.as_ref(),
                            clean_native_ink_pages,
                            type0_loss_pages,
                        },
                    )
                });
            #[cfg(not(all(feature = "ocr", not(target_arch = "wasm32"))))]
            let plain = self
                .structured_ocr_pages
                .get(&page)
                .zip(self.ocr_results.get(&page))
                .is_some_and(|(doc, text)| super::document::plain_ocr_page_can_yield_to_native(doc, text.as_str()));
            let has_formula = self
                .accumulated_formulas
                .iter()
                .any(|formula| formula.page.is_none_or(|owner| owner == page));
            if plain && !has_formula && super::document::native_preservation_is_automatic(self.config, page) {
                accepted_replacements.remove(&page);
                self.structured_ocr_pages.remove(&page);
                self.ocr_confidence_by_page.remove(&page);
                self.preprocessing_by_page.remove(&page);
                #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
                {
                    if self.all_pages_failed_policy == AllPagesFailedPolicy::ReturnError
                        && self.pages.single_native_type0_page.is_some()
                        && let Some(lost) = lost_glyph_ink_pages.get(&page)
                    {
                        return Err(lost_glyph_ocr_rejection(*lost));
                    }
                    if type0_loss_pages.contains(&page)
                        && self.all_pages_failed_policy == AllPagesFailedPolicy::ReturnError
                        && self.page_count == 1
                        && self.pages.single_native_type0_page.is_some()
                    {
                        return Err(type0_ocr_rejection());
                    }
                }
            }
        }
        Ok(())
    }

    /// A page whose OCR text the information-loss check already rejected never reaches the loop
    /// above, so the warning is raised here for every such page, whichever check kept it. ~keep
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    fn emit_type0_retention_warnings(
        &mut self,
        type0_loss_pages: &ahash::AHashSet<u32>,
        accepted_replacements: &ahash::AHashMap<u32, String>,
    ) {
        let mut retained: Vec<u32> = type0_loss_pages
            .iter()
            .copied()
            .filter(|page| !accepted_replacements.contains_key(page))
            .collect();
        retained.sort_unstable();
        for page in retained {
            crate::core::diagnostics::push_warning_deduped(
                &mut self.accumulated_warnings,
                crate::core::diagnostics::warning(
                    "ocr",
                    format!("Page {page} retained native text after its OCR render lost Type0 glyph ink."),
                ),
            );
        }
    }

    fn finish_layer_prepend(
        &mut self,
        accepted_replacements: &mut ahash::AHashMap<u32, String>,
        mut page_layers_first: ahash::AHashMap<u32, super::document::LayerFirst>,
    ) {
        for (page_number, first) in &mut page_layers_first {
            let Some(text) = accepted_replacements.get_mut(page_number) else {
                continue;
            };
            let page_doc = self.structured_ocr_pages.get_mut(page_number);
            #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
            let mut page_doc = page_doc;
            #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
            if !self.alternative_raster_pages.contains(page_number)
                && !self.table_only_xobject_recovery_pages.contains(page_number)
                && !self.caption_rewrites.contains_key(page_number)
                && let Some(matched) = self.matched_layer_occurrences.get(page_number)
                && let Some(doc) = page_doc.as_deref_mut()
            {
                matched.trim_prepend(
                    first,
                    text,
                    doc,
                    *page_number,
                    self.reflow_scales.get(page_number).copied(),
                );
                matched.replace_split_literals(first, text, doc);
                matched.replace_repeated_identifiers(first, text, doc);
            }
            super::document::finish_mixed_layer_prepend(text, page_doc, first);
        }
    }

    fn emit_empty_text_warnings(&mut self) {
        if self
            .config
            .cancel_token
            .as_ref()
            .is_some_and(|token| token.is_cancelled())
        {
            return;
        }
        let retained_formula_pages: ahash::AHashSet<_> = self
            .accumulated_formulas
            .iter()
            .filter_map(|formula| formula.page)
            .collect();
        for &page_idx in &self.page_indices {
            let page = (page_idx + 1) as u32;
            if (self.ocr_results.get(&page).is_none_or(|text| text.trim().is_empty())
                || self.noise_verdict_pages.contains(&page))
                && !self.table_only_xobject_recovery_pages.contains(&page)
                && !retained_formula_pages.contains(&page)
                && self
                    .structured_ocr_pages
                    .get(&page)
                    .is_none_or(|doc| doc.tables.is_empty())
                && let Some(dpi) = self.raster_dpis.get(&page_idx)
                && let Some(warning) = dpi.empty_text_warning(page_idx)
            {
                crate::core::diagnostics::push_warning_deduped(&mut self.accumulated_warnings, warning);
            }
        }
    }

    #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
    fn apply_caption_rewrites(&mut self, accepted_replacements: &mut ahash::AHashMap<u32, String>) {
        for (page, rewrite) in std::mem::take(&mut self.caption_rewrites) {
            if let Some(text) = accepted_replacements.get_mut(&page)
                && let Some(document) = self.structured_ocr_pages.get_mut(&page)
            {
                rewrite.apply(document, text, page);
            }
        }
    }

    #[cfg(not(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32"))))]
    fn apply_caption_rewrites(&mut self, _accepted_replacements: &mut ahash::AHashMap<u32, String>) {}

    #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
    fn apply_khmer_supplements(&mut self, accepted_replacements: &mut ahash::AHashMap<u32, String>) {
        for (page, supplement) in std::mem::take(&mut self.khmer_supplements) {
            if let Some(text) = accepted_replacements.get_mut(&page) {
                text.push_str("\n\n");
                text.push_str(&supplement);
                if let Some(document) = self.structured_ocr_pages.get_mut(&page) {
                    use crate::types::internal::{ElementKind, InternalElement};
                    let mut paragraph = InternalElement::text(ElementKind::Paragraph, supplement, 0);
                    paragraph.page = Some(page);
                    document.push_element(paragraph);
                }
            }
        }
    }

    #[cfg(not(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32"))))]
    fn apply_khmer_supplements(&mut self, _accepted_replacements: &mut ahash::AHashMap<u32, String>) {}

    fn emit_unrecovered_glyph_warnings(&mut self, accepted_replacements: &ahash::AHashMap<u32, String>) {
        for boundary in self.boundaries {
            let page = boundary.page_number;
            if !accepted_replacements.contains_key(&page)
                && self.ocr_results.contains_key(&page)
                && let Some(native) = self.native_text.get(boundary.byte_start..boundary.byte_end)
                && let Some(warning) = super::super::unmapped_glyphs::unrecovered_page_glyph_warning(
                    page,
                    native.matches(super::super::unmapped_glyphs::is_lost_glyph).count(),
                )
            {
                crate::core::diagnostics::push_warning_deduped(&mut self.accumulated_warnings, warning);
            }
        }
    }

    fn assemble_run(&mut self, accepted_replacements: ahash::AHashMap<u32, String>) -> MixedOcrRun {
        let result = apply_ocr_page_replacements(self.native_text, self.boundaries, &accepted_replacements);
        self.page_failures.sort_by_key(|failure| failure.page);
        #[cfg(feature = "formula-recognition")]
        let formula_layout_inputs = formula_layout_inputs_for_replaced_pages(
            self.resolved_layout_inputs.take(),
            &accepted_replacements,
            self.config.layout.as_ref(),
        );
        MixedOcrRun {
            output: (
                result,
                accepted_replacements,
                std::mem::take(&mut self.structured_ocr_pages),
                std::mem::take(&mut self.accumulated_llm_usage),
                if self.capture_rasters {
                    Some(std::mem::take(&mut self.captured_rasters))
                } else {
                    None
                },
                std::mem::take(&mut self.accumulated_formulas),
                std::mem::take(&mut self.preprocessing_by_page),
                std::mem::take(&mut self.ocr_confidence_by_page),
                std::mem::take(&mut self.accumulated_warnings),
                std::mem::take(&mut self.page_failures),
            ),
            #[cfg(feature = "formula-recognition")]
            formula_layout_inputs,
        }
    }
}

#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
impl MixedOcrRun {
    fn native(native_text: &str) -> Self {
        Self {
            output: (
                native_text.to_string(),
                ahash::AHashMap::new(),
                ahash::AHashMap::new(),
                Vec::new(),
                None,
                Vec::new(),
                ahash::AHashMap::new(),
                ahash::AHashMap::new(),
                Vec::new(),
                Vec::new(),
            ),
            #[cfg(feature = "formula-recognition")]
            formula_layout_inputs: None,
        }
    }
}

/// Keeps the layout inputs of the replaced pages that hold a formula region, so the formula pass
/// leaves native pages and rejected OCR pages as they were. Other pages keep an empty raster to
/// hold the page index without holding its memory.
#[cfg(all(
    any(feature = "ocr", feature = "ocr-pipeline"),
    feature = "pdf",
    feature = "formula-recognition"
))]
pub(super) fn formula_layout_inputs_for_replaced_pages(
    inputs: Option<PreparedLayoutInputs>,
    replaced_pages: &ahash::AHashMap<u32, String>,
    layout: Option<&crate::core::config::LayoutDetectionConfig>,
) -> Option<PreparedLayoutInputs> {
    layout?.formula_model.as_ref()?;
    let (mut images, mut detections) = inputs?;
    let mut any_formula_page = false;
    for (page_idx, (image, detection)) in images.iter_mut().zip(&mut detections).enumerate() {
        let has_formula = detection
            .detections
            .iter()
            .any(|region| matches!(region.class_name, crate::layout::LayoutClass::Formula));
        if has_formula && replaced_pages.contains_key(&(page_idx as u32 + 1)) {
            any_formula_page = true;
        } else {
            *image = image::DynamicImage::new_rgb8(0, 0);
            detection.detections.clear();
        }
    }
    any_formula_page.then_some((images, detections))
}

#[cfg(all(
    any(feature = "ocr", feature = "ocr-pipeline"),
    feature = "pdf",
    any(
        paddle_ocr,
        all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32"))
    )
))]
fn full_document_page_seed(page_count: usize, config: &ExtractionConfig) -> (String, Vec<crate::types::PageBoundary>) {
    let marker_format = super::super::ocr_page_marker_format(config);
    let mut text = String::new();
    let mut boundaries = Vec::with_capacity(page_count);
    for page_index in 0..page_count {
        text.push_str(&super::super::ocr_page_prefix(marker_format, page_index, 0));
        boundaries.push(crate::types::PageBoundary {
            byte_start: text.len(),
            byte_end: text.len(),
            page_number: page_index as u32 + 1,
        });
    }
    (text, boundaries)
}

/// Whether the render budget alone lowers the DPI the page is rendered at, by the same plan
/// the render uses. An unbounded page gains nothing from the single-backend route, so it is
/// decided before any OCR runs; a page the plan misses only loses the recovery. ~keep
#[cfg(all(
    paddle_ocr,
    feature = "ocr",
    feature = "pdf",
    feature = "tokio-runtime",
    not(target_arch = "wasm32")
))]
fn render_budget_bounds_first_page(document: &xberg_native_pdf::PdfDocument, config: &ExtractionConfig) -> bool {
    let default_limits = crate::extractors::security::SecurityLimits::default();
    let limits = config.security_limits.as_ref().unwrap_or(&default_limits);
    let ocr = super::resolve_ocr_config(config);
    let target = registered_preprocessing_render_dpi(config, &ocr);
    let (requested, planned) = crate::image::dpi::planned_pdf_ocr_render_dpi_with_preprocessing(
        document,
        0,
        config.images.as_ref(),
        limits,
        target,
    );
    planned < requested
}

/// Reads a one-page PDF on the single-backend route and keeps the result when the first stage
/// of the synthesized classical pipeline would have accepted it, so that route's source-image
/// recoveries and budgeted scaling apply. A page below the pipeline's quality threshold, or one
/// the single-backend route fails, returns `None` and takes the per-page route with its
/// automatic fallback stage. So does a read whose scale the budget did not bound: it gains
/// nothing from this route, and the per-page route carries the frame metadata and point
/// placement of detached rasters that this one does not.
#[cfg(all(
    paddle_ocr,
    feature = "ocr",
    feature = "pdf",
    feature = "tokio-runtime",
    not(target_arch = "wasm32")
))]
pub(crate) async fn accepted_single_page_first_pass(
    content: &[u8],
    config: &ExtractionConfig,
    path: Option<&std::path::Path>,
    page_ocr_hints: Option<PageOcrHints>,
    thresholds: &crate::core::config::OcrQualityThresholds,
) -> Option<(WholeDirectOcrOutput, bool)> {
    let Ok((document, 1, _)) = open_pdf_for_page_ocr(content) else {
        return None;
    };
    if !render_budget_bounds_first_page(&document, config) {
        return None;
    }
    let (output, discarded_visible_native_page, unpruned_text) = Box::pin(extract_with_ocr_with_unpruned_text(
        Some(content),
        None,
        #[cfg(feature = "layout-detection")]
        None,
        config,
        path,
        page_ocr_hints,
    ))
    .await
    .ok()?;
    let output = (output, discarded_visible_native_page);
    let budget_bounded = output
        .0
        .9
        .values()
        .any(|metadata| metadata.final_dpi < metadata.target_dpi);
    if !budget_bounded {
        return None;
    }
    first_pass_clears_quality_gate(&output.0.0, unpruned_text.as_deref(), output.0.1, thresholds).then_some(output)
}

/// Whether a first read clears the pipeline quality gate. The read is scored before symbol-noise
/// pruning, so pruning cannot lift a noisy page over the gate and change the route (#2312).
#[cfg(all(
    any(feature = "ocr", feature = "ocr-pipeline"),
    any(paddle_ocr, all(test, feature = "ocr"))
))]
pub(super) fn first_pass_clears_quality_gate(
    text: &str,
    unpruned_text: Option<&str>,
    mean_confidence: Option<f64>,
    thresholds: &crate::core::config::OcrQualityThresholds,
) -> bool {
    let score = pipeline_stage_score(
        compute_quality_score(unpruned_text.unwrap_or(text), thresholds),
        mean_confidence,
    );
    score >= thresholds.pipeline_min_quality
}

#[cfg(all(
    any(feature = "ocr", feature = "ocr-pipeline"),
    feature = "pdf",
    any(
        paddle_ocr,
        all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32"))
    )
))]
type PerPageOcrOutput = (
    String,
    Vec<crate::types::Table>,
    Vec<crate::types::OcrElement>,
    Option<crate::types::internal::InternalDocument>,
    Vec<crate::types::LlmUsage>,
    Vec<String>,
    Option<Vec<crate::types::ExtractedImage>>,
    Vec<crate::types::Formula>,
    ahash::AHashMap<u32, crate::types::ImagePreprocessingMetadata>,
    ahash::AHashMap<u32, crate::types::page::PageOcrConfidence>,
);

#[cfg(all(paddle_ocr, any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(crate) async fn extract_full_document_ocr_pipeline_per_page(
    content: &[u8],
    config: &ExtractionConfig,
    path: Option<&std::path::Path>,
    page_ocr_hints: Option<PageOcrHints>,
    #[cfg(feature = "layout-detection")] prepared_layout_inputs: Option<&PreparedLayoutInputs>,
) -> OcrRunResult<PerPageOcrOutput> {
    let run = extract_pages_with_per_page_pipeline(
        content,
        config,
        path,
        page_ocr_hints,
        #[cfg(feature = "layout-detection")]
        prepared_layout_inputs,
    )
    .await;
    #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
    if let Err(failed) = &run
        && let Some((
            text,
            _,
            tables,
            elements,
            document,
            usage,
            page_texts,
            rasters,
            formulas,
            preprocessing,
            confidence,
        )) = try_gray_jpeg_render_recovery(Some(content), config, failed).await?
    {
        return Ok((
            text,
            tables,
            elements,
            document,
            usage,
            page_texts,
            rasters,
            formulas,
            preprocessing,
            confidence,
        ));
    }
    run
}

#[cfg(all(
    any(feature = "ocr", feature = "ocr-pipeline"),
    feature = "pdf",
    any(
        paddle_ocr,
        all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32"))
    )
))]
async fn extract_pages_with_per_page_pipeline(
    content: &[u8],
    config: &ExtractionConfig,
    _path: Option<&std::path::Path>,
    page_ocr_hints: Option<PageOcrHints>,
    #[cfg(feature = "layout-detection")] prepared_layout_inputs: Option<&PreparedLayoutInputs>,
) -> OcrRunResult<PerPageOcrOutput> {
    if config.cancel_token.as_ref().is_some_and(|token| token.is_cancelled()) {
        return Err(crate::XbergError::Cancelled.into());
    }
    let (_, page_count, _) = open_pdf_for_page_ocr(content)?;
    let page_numbers = (1..=page_count as u32).collect::<Vec<_>>();
    let native_layers = page_ocr_hints
        .as_ref()
        .and_then(|hints| hints.native_page_layers.clone());
    let raster_dpis = page_ocr_hints.as_ref().and_then(|hints| hints.raster_dpis.clone());
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    let probe_reads = page_ocr_hints.as_ref().and_then(|hints| hints.probe_reads.clone());
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    let single_native_type0_page = page_ocr_hints
        .as_ref()
        .and_then(|hints| hints.single_native_type0_page.clone());
    let mut single_block_pages = page_ocr_hints
        .and_then(|hints| hints.single_block_pages)
        .map(|pages| pages.iter().copied().collect::<Vec<_>>())
        .unwrap_or_default();
    single_block_pages.sort_unstable();
    let (seed_text, boundaries) = full_document_page_seed(page_count, config);
    let MixedOcrRun {
        output:
            (
                text,
                accepted_pages,
                structured_pages,
                llm_usage,
                rasters,
                formulas,
                preprocessing,
                ocr_confidence,
                warnings,
                page_failures,
            ),
        ..
    } = Box::pin(extract_mixed_ocr_native_with_layout_inputs(
        &seed_text,
        &boundaries,
        MixedOcrPageSelection {
            #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
            probe_reads,
            ocr: &page_numbers,
            raster_dpis,
            single_block: &single_block_pages,
            preserve_native: &[],
            native_script: &[],
            native_layers,
            hidden_layer_backend: None,
            #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
            single_native_type0_page: single_native_type0_page.as_deref(),
        },
        content,
        config,
        AllPagesFailedPolicy::ReturnError,
        #[cfg(feature = "layout-detection")]
        MixedLayoutInputs::Prepared(prepared_layout_inputs),
    ))
    .await?;
    if config.cancel_token.as_ref().is_some_and(|token| token.is_cancelled()) {
        return Err(crate::XbergError::Cancelled.into());
    }

    let page_texts = page_numbers
        .iter()
        .map(|page| accepted_pages.get(page).cloned().unwrap_or_default())
        .collect();
    let mut document = crate::types::internal::InternalDocument::new("pdf");
    document.processing_warnings = warnings;
    document.ocr_page_failures = page_failures;
    merge_structured_ocr_pages_into_internal_document(&mut document, &accepted_pages, &structured_pages);
    per_page_flat_metadata::lift(&mut document, &structured_pages);
    let tables = document.tables.clone();
    let ocr_elements = document.prebuilt_ocr_elements.clone().unwrap_or_default();

    Ok((
        text,
        tables,
        ocr_elements,
        Some(document),
        llm_usage,
        page_texts,
        rasters,
        formulas,
        preprocessing,
        ocr_confidence,
    ))
}
/// Extract text from PDF using OCR on pre-rendered page images.
///
/// When `layout_detections` are provided (pixel-space, from the same images), uses
/// layout-aware markdown assembly for structured output. Otherwise, when
/// `config.output_format` is not [`OutputFormat::Plain`], structure (headings, list
/// items) is instead recovered document-wide by the same font-clustering heuristic
/// the native xberg_native_pdf path uses
/// (`pdf::structure::pipeline::extract_document_structure_from_segments`), fed from
/// segments harvested out of the per-page OCR paragraphs
/// (`pdf::structure::adapters::segments_from_ocr_pages`). Under `Plain`, or when that
/// heuristic yields nothing, falls back to plain OCR text concatenation.
///
/// # Arguments
///
/// * `images` - Pre-rendered page images (shared with layout detection)
/// * `layout_detections` - Optional pixel-space layout detections per page
/// * `config` - Extraction configuration including OCR settings
///
/// # Returns
///
/// Concatenated text from all pages, with markdown structure when layout is available
///
/// Thin, signature-preserving wrapper over [`extract_with_ocr_for_page`] with no page
/// rotation override (`0`), so every pre-existing caller keeps today's behavior: content-based
/// per-page auto-detection when `content` is available and index-aligned to `images`, or no
/// rotation correction at all otherwise.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
type WholeDirectOcrOutput = (
    String,
    Option<f64>,
    Vec<crate::types::Table>,
    Vec<crate::types::OcrElement>,
    Option<crate::types::internal::InternalDocument>,
    Vec<crate::types::LlmUsage>,
    Vec<String>,
    Option<Vec<crate::types::ExtractedImage>>,
    Vec<crate::types::Formula>,
    ahash::AHashMap<u32, crate::types::ImagePreprocessingMetadata>,
    ahash::AHashMap<u32, crate::types::page::PageOcrConfidence>,
);

#[cfg(all(test, any(feature = "ocr", feature = "ocr-pipeline")))]
// ~keep Test-only entry point; only the `feature = "ocr"` test modules call it, so it is dead
// on an `ocr-pipeline`-only test leg. See the GH#1951 rationale on `MixedLayoutInputs`.
#[allow(dead_code)]
pub(crate) async fn extract_with_ocr(
    content: Option<&[u8]>,
    images: Option<&[image::DynamicImage]>,
    #[cfg(feature = "layout-detection")] layout_detections: Option<&[crate::layout::DetectionResult]>,
    config: &ExtractionConfig,
    path: Option<&std::path::Path>,
) -> crate::Result<WholeDirectOcrOutput> {
    extract_with_ocr_with_page_hints(
        content,
        images,
        #[cfg(feature = "layout-detection")]
        layout_detections,
        config,
        path,
        None,
    )
    .await
    .map_err(crate::XbergError::from)
}

#[cfg(all(test, any(feature = "ocr", feature = "ocr-pipeline")))]
#[allow(dead_code)]
pub(crate) async fn extract_with_ocr_with_page_hints(
    content: Option<&[u8]>,
    images: Option<&[image::DynamicImage]>,
    #[cfg(feature = "layout-detection")] layout_detections: Option<&[crate::layout::DetectionResult]>,
    config: &ExtractionConfig,
    path: Option<&std::path::Path>,
    page_ocr_hints: Option<PageOcrHints>,
) -> OcrRunResult<WholeDirectOcrOutput> {
    extract_with_ocr_with_page_hints_and_evidence(
        content,
        images,
        #[cfg(feature = "layout-detection")]
        layout_detections,
        config,
        path,
        page_ocr_hints,
    )
    .await
    .map(|(output, _)| output)
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(crate) async fn extract_with_ocr_with_page_hints_and_evidence(
    content: Option<&[u8]>,
    images: Option<&[image::DynamicImage]>,
    #[cfg(feature = "layout-detection")] layout_detections: Option<&[crate::layout::DetectionResult]>,
    config: &ExtractionConfig,
    path: Option<&std::path::Path>,
    page_ocr_hints: Option<PageOcrHints>,
) -> OcrRunResult<(WholeDirectOcrOutput, bool)> {
    extract_with_ocr_with_unpruned_text(
        content,
        images,
        #[cfg(feature = "layout-detection")]
        layout_detections,
        config,
        path,
        page_ocr_hints,
    )
    .await
    .map(|(output, discarded_visible_native_page, _)| (output, discarded_visible_native_page))
}

/// One read of a raster the caller rendered, through the page route with `config`: the page text
/// and its confidence, when the backend reports one.
#[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
pub(in crate::extractors::pdf) async fn read_rendered_page(
    image: image::DynamicImage,
    config: &ExtractionConfig,
) -> crate::Result<(String, Option<crate::types::page::PageOcrConfidence>)> {
    let images = [image];
    let run = Box::pin(extract_with_ocr_for_page(
        None,
        Some(&images),
        #[cfg(feature = "layout-detection")]
        None,
        config,
        None,
        0,
        false,
        None,
        None,
        0,
        None,
    ))
    .await
    .map_err(|failed| failed.error)?;
    Ok((run.0, run.11.get(&1).cloned()))
}

/// [`extract_with_ocr_with_page_hints_and_evidence`] plus the page text as it was before
/// symbol-noise pruning, when a page was pruned, for callers that score the read (#2312).
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
async fn extract_with_ocr_with_unpruned_text(
    content: Option<&[u8]>,
    images: Option<&[image::DynamicImage]>,
    #[cfg(feature = "layout-detection")] layout_detections: Option<&[crate::layout::DetectionResult]>,
    config: &ExtractionConfig,
    path: Option<&std::path::Path>,
    page_ocr_hints: Option<PageOcrHints>,
) -> OcrRunResult<(WholeDirectOcrOutput, bool, Option<String>)> {
    let run_layers = page_ocr_hints
        .as_ref()
        .and_then(|hints| hints.native_page_layers.clone());
    let run = Box::pin(extract_with_ocr_for_page(
        content,
        images,
        #[cfg(feature = "layout-detection")]
        layout_detections,
        config,
        path,
        0,
        false,
        None,
        page_ocr_hints,
        0,
        None,
    ))
    .await;
    #[cfg(all(
        feature = "pdf",
        feature = "ocr",
        feature = "tokio-runtime",
        not(target_arch = "wasm32")
    ))]
    if images.is_none()
        && let Err(failed) = &run
        && let Some(output) = try_gray_jpeg_render_recovery(content, config, failed).await?
    {
        return Ok((output, false, None));
    }
    #[cfg(all(
        feature = "pdf",
        feature = "ocr",
        feature = "tokio-runtime",
        not(paddle_ocr),
        not(target_arch = "wasm32")
    ))]
    if images.is_none()
        && let Err(failed) = &run
        && let Some(output) = direct_rgb_budget_recovery::try_recovery(content, config, path, failed).await?
    {
        return Ok((output, false, None));
    }
    let (
        text,
        mean_conf,
        tables,
        elements,
        doc,
        usage,
        page_texts,
        rasters,
        formulas,
        raw_page_paragraphs,
        preprocessing,
        ocr_confidence,
        recognition_noise_verdicts,
        read_evidence,
    ) = run?;
    #[cfg(all(
        test,
        feature = "pdf",
        feature = "ocr",
        feature = "tokio-runtime",
        not(target_arch = "wasm32")
    ))]
    record_region_replay(&read_evidence);
    let (text, page_texts, doc) = with_run_layers_first(
        run_layers,
        RunLayerSources {
            verdicts: &recognition_noise_verdicts,
            page_index_offset: 0,
            content,
            raw_page_paragraphs: &raw_page_paragraphs,
        },
        text,
        page_texts,
        doc,
        config,
    );
    super::super::ensure_pdf_not_cancelled(config)?;
    Ok((
        (
            text,
            mean_conf,
            tables,
            elements,
            doc,
            usage,
            page_texts,
            rasters,
            formulas,
            preprocessing,
            ocr_confidence,
        ),
        read_evidence.discarded_visible_native_page,
        read_evidence.unpruned_text,
    ))
}

#[cfg(all(
    feature = "pdf",
    feature = "ocr",
    feature = "tokio-runtime",
    not(target_arch = "wasm32")
))]
fn budget_retry_backend(
    config: &ExtractionConfig,
    ocr: &crate::core::config::OcrConfig,
    failed: &FailedOcrRun,
) -> Option<std::sync::Arc<dyn crate::plugins::OcrBackend>> {
    // Native preservation excludes explicit layout: the refused render produced no detections
    // to validate before a source retry could replace the read. ~keep
    if !render_refused_by_source_image_budget(&failed.error)
        || config.ocr_strategy != crate::core::config::OcrStrategy::Auto
        || !super::document::native_preservation_is_automatic(config, 1)
        || config.pages.as_ref().is_some_and(|pages| pages.insert_page_markers)
        || !uses_default_ocr_pipeline(ocr)
        // A language change is a different recognition request, not a render recovery. ~keep
        || ocr.language != crate::core::config::OcrConfig::default().language
    {
        return None;
    }
    let registry = crate::plugins::registry::get_ocr_backend_registry();
    let registry = registry.read();
    let backend = registry
        .get(&ocr.backend)
        .ok()
        .filter(|backend| registry.is_builtin_tesseract(backend))?;
    super::tall_source::supports_for(config, ocr, backend.as_ref()).then_some(backend)
}

#[cfg(all(
    feature = "pdf",
    feature = "ocr",
    feature = "tokio-runtime",
    not(target_arch = "wasm32")
))]
async fn try_gray_jpeg_render_recovery(
    content: Option<&[u8]>,
    config: &ExtractionConfig,
    failed: &FailedOcrRun,
) -> crate::Result<Option<WholeDirectOcrOutput>> {
    let ocr = super::resolve_ocr_config(config);
    let Some(backend) = budget_retry_backend(config, &ocr, failed) else {
        return Ok(None);
    };
    let Some(content) = content else { return Ok(None) };
    let Ok((document, 1, _)) = open_pdf_for_full_ocr(content) else {
        return Ok(None);
    };
    let Some((result, psm)) =
        super::tall_source::recover_after_gray_jpeg_render_failure(&document, &backend, config, &ocr).await?
    else {
        return Ok(None);
    };
    Ok(Some(gray_jpeg_recovery_output(result, psm, backend.as_ref())))
}

#[cfg(all(
    test,
    feature = "pdf",
    feature = "ocr",
    feature = "tokio-runtime",
    feature = "bundle-tessdata-eng",
    not(paddle_ocr),
    not(target_arch = "wasm32")
))]
#[tokio::test]
async fn tall_source_rejects_an_unrelated_image_error_even_with_a_certifiable_jpeg() {
    crate::plugins::ensure_ocr_backends_initialized();
    let content = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/repros/tall-scan-strips/jpeg-tall.pdf"
    ));
    let config = ExtractionConfig {
        use_cache: false,
        ocr: Some(crate::core::config::OcrConfig::default()),
        ..Default::default()
    };
    let failed = crate::XbergError::Parsing {
        message: "an unrelated image decoder refused the source".into(),
        source: Some(Box::new(xberg_native_pdf::Error::Image("image decoder failure".into()))),
    }
    .into();
    assert!(
        try_gray_jpeg_render_recovery(Some(content), &config, &failed)
            .await
            .unwrap()
            .is_none()
    );
}

#[cfg(all(
    feature = "pdf",
    feature = "ocr",
    feature = "tokio-runtime",
    not(target_arch = "wasm32")
))]
fn gray_jpeg_recovery_output(
    mut result: crate::types::ExtractedDocument,
    psm: i32,
    backend: &dyn crate::plugins::OcrBackend,
) -> WholeDirectOcrOutput {
    let confidence = mean_text_conf_of(&result.metadata.additional);
    let summary = page_ocr_confidence(
        backend.confidence_semantics(),
        confidence,
        word_count_of(&result.metadata.additional).unwrap_or(0),
        backend.name(),
    );
    let text = result.content;
    let mut doc = attach_ocr_fallback_warnings(result.ocr_internal_document.take(), &text, result.processing_warnings);
    if let Some(doc) = &mut doc {
        attach_page_segmentation_modes(doc, [(1, psm)]);
    }
    (
        text.clone(),
        summary.as_ref().and_then(|value| value.score),
        Vec::new(),
        Vec::new(),
        doc,
        Vec::new(),
        vec![text],
        None,
        Vec::new(),
        Default::default(),
        summary.into_iter().map(|value| (1, value)).collect(),
    )
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[derive(Clone, Copy)]
pub(super) struct DetachedOcrPage<'a> {
    pub(super) document: &'a xberg_native_pdf::PdfDocument,
    #[cfg(all(paddle_ocr, feature = "ocr", not(target_arch = "wasm32")))]
    pub(super) stored_scan: Option<&'a stored_scan_retry::StoredScan>,
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    pub(super) orientation_retry: Option<&'a std::sync::Arc<super::orientation_retry::OrientationRetry>>,
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[derive(Clone, Default)]
pub(crate) struct PageOcrHints {
    #[cfg(all(paddle_ocr, feature = "ocr", not(target_arch = "wasm32")))]
    pub(crate) sparse_glyph_trial: Option<sparse_glyph_read::MarkedWords>,
    #[cfg(all(
        feature = "pdf",
        feature = "ocr",
        feature = "tokio-runtime",
        not(target_arch = "wasm32")
    ))]
    pub(crate) number_source: Option<super::number_recovery::DetachedSource>,
    #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
    pub(crate) probe_reads: Option<std::sync::Arc<super::read_reuse::ProbeReads>>,
    pub(crate) source_dpi: Option<f64>,
    #[cfg(feature = "pdf")]
    pub(crate) raster_dpis: Option<std::sync::Arc<crate::pdf::render::OcrRasterDpis>>,
    /// Page-aligned flags for real layout rasters that cover the MediaBox.
    #[cfg(feature = "pdf")]
    pub(crate) layout_rasters_in_page_frame: Option<std::sync::Arc<[bool]>>,
    /// `true` is authoritative; `false` means the detached caller has no positive scan signal,
    /// so a content-backed whole-document route still performs its normal per-page detection.
    pub(crate) known_full_page_scan: bool,
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    pub(crate) detached_recognition_recovery: Option<super::orientation_retry::RecognitionRecovery>,
    /// One-indexed document page numbers selected by automatic fabricated-map routing.
    pub(crate) single_block_pages: Option<std::sync::Arc<std::collections::HashSet<u32>>>,
    /// Readable text layers by one-indexed document page, for a whole-document run. After the run
    /// has chosen its output, a page whose OCR text is judged recognition noise gets its layer
    /// first.
    pub(crate) native_page_layers: Option<std::sync::Arc<ahash::AHashMap<u32, String>>>,
    /// The layers the mixed route's page decision compares each page's OCR text with, for the
    /// per-page calls that carry no `native_page_layers`.
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    pub(crate) decision_layers: Option<std::sync::Arc<ahash::AHashMap<u32, String>>>,
    /// Original marker-bearing text of the only physical page in a whole automatic fallback.
    #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
    pub(crate) single_native_type0_page: Option<std::sync::Arc<str>>,
    #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
    pub(crate) native_coverage_layers: Option<std::sync::Arc<ahash::AHashMap<u32, String>>>,
}

#[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
impl PageOcrHints {
    pub(super) fn decision_layer(&self, page: u32) -> Option<&str> {
        self.decision_layers
            .as_ref()
            .or(self.native_page_layers.as_ref())
            .and_then(|layers| layers.get(&page))
            .map(String::as_str)
    }
    fn native_layer(&self, page: u32) -> Option<&str> {
        self.native_coverage_layers
            .as_ref()
            .or(self.native_page_layers.as_ref())
            .and_then(|layers| layers.get(&page))
            .map(String::as_str)
    }
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(super) fn effective_tesseract_psm(metadata: &crate::types::Metadata, backend: &str) -> Option<i32> {
    if backend != "tesseract" {
        return None;
    }
    match metadata.format.as_ref()? {
        crate::types::FormatMetadata::Ocr(ocr) => Some(ocr.psm),
        _ => None,
    }
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn attach_page_segmentation_modes(
    doc: &mut crate::types::internal::InternalDocument,
    modes: impl IntoIterator<Item = (u32, i32)>,
) {
    let entries = modes
        .into_iter()
        .map(|(page_number, psm)| serde_json::json!({ "page_number": page_number, "psm": psm }))
        .collect::<Vec<_>>();
    if !entries.is_empty() {
        doc.metadata.additional.insert(
            std::borrow::Cow::Borrowed(crate::ocr_metadata_keys::OCR_PAGE_SEGMENTATION_MODES_METADATA_KEY),
            serde_json::Value::Array(entries),
        );
    }
}
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[derive(Default)]
pub(super) struct OcrReadEvidence {
    #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
    caption: Option<super::caption_conversion::CaptionRewrite>,
    no_admitted_images: bool,
    /// The page text joined as it was before symbol-noise pruning, when any page was pruned, so
    /// pipeline stage scoring judges the same text as without pruning (#2312).
    pub(super) unpruned_text: Option<String>,
    #[cfg(all(
        feature = "pdf",
        feature = "ocr",
        feature = "tokio-runtime",
        not(target_arch = "wasm32")
    ))]
    matched_layer_occurrences: Option<super::document::MatchedLayerOccurrences>,
    #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
    number_primary_text: Option<String>,
    // A fresh selected read, kept separate from aggregate or rerendered text. ~keep
    retained_hocr_read: Option<crate::types::formats::RetainedHocrRead>,
    #[cfg(all(
        test,
        feature = "pdf",
        feature = "ocr",
        feature = "tokio-runtime",
        not(target_arch = "wasm32")
    ))]
    region_replay_raster: Option<(u32, u32, usize)>,
    original_tesseract_raster: bool,
    discarded_visible_native_page: bool,
    #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
    complete_native_words: bool,
    #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
    native_ink_owned: bool,
    /// The read words shown at a native word's own place (see `read_words_at_native_place`).
    #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
    native_placed_words: std::collections::HashSet<String>,
}

#[cfg(all(
    test,
    feature = "pdf",
    feature = "ocr",
    feature = "tokio-runtime",
    not(target_arch = "wasm32")
))]
fn record_region_replay(evidence: &OcrReadEvidence) {
    tracing::trace!(receipt = evidence.retained_hocr_read.is_some(), raster = ?evidence.region_replay_raster,
        "final selected OCR replay receipt");
    if let (Some(read), Some(raster)) = (&evidence.retained_hocr_read, evidence.region_replay_raster) {
        super::tall_source::region_replay::record_selected(read, raster);
    }
}

#[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
fn lost_glyph_ocr_rejection(lost: usize) -> FailedOcrRun {
    FailedOcrRun {
        error: crate::XbergError::Ocr {
            message: format!(
                "Page 1 OCR read did not recover the {lost} glyphs without a Unicode value; it only re-read glyph ink that the native text paints"
            ),
            source: None,
        },
        page_failures: Vec::new(),
        unrecovered_glyphs_on_page_one: lost,
    }
}

#[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
fn type0_ocr_rejection() -> FailedOcrRun {
    crate::XbergError::Ocr {
        message: "Page 1 OCR read omitted native text after its render lost Type0 glyph ink".to_owned(),
        source: None,
    }
    .into()
}

#[cfg(all(
    any(feature = "ocr", feature = "ocr-pipeline"),
    feature = "tokio-runtime",
    not(target_arch = "wasm32")
))]
struct PageRead<OrderedProof, CaptionProof> {
    page_idx: usize,
    correction_degrees: u32,
    upright_width: u32,
    upright_height: u32,
    result: crate::Result<crate::types::ExtractedDocument>,
    #[cfg(all(
        feature = "ocr",
        feature = "layout-detection",
        feature = "tokio-runtime",
        not(target_arch = "wasm32")
    ))]
    column_order: Option<super::column_retry::LineOrder>,
    ordered_proof: Option<OrderedProof>,
    detached_frame_read: bool,
    captured_caption: Option<CaptionProof>,
    number_frame: bool,
    #[cfg(feature = "ocr")]
    paint_frame: bool,
    #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
    background: Option<super::symbol_noise_background::Evidence>,
}

#[cfg(all(
    any(feature = "ocr", feature = "ocr-pipeline"),
    feature = "tokio-runtime",
    not(target_arch = "wasm32")
))]
impl<OrderedProof, CaptionProof> PageRead<OrderedProof, CaptionProof> {
    fn failed(page_idx: usize, correction_degrees: u32, dimensions: (u32, u32), error: crate::XbergError) -> Self {
        Self {
            page_idx,
            correction_degrees,
            upright_width: dimensions.0,
            upright_height: dimensions.1,
            result: Err(error),
            #[cfg(all(
                feature = "ocr",
                feature = "layout-detection",
                feature = "tokio-runtime",
                not(target_arch = "wasm32")
            ))]
            column_order: None,
            ordered_proof: None,
            detached_frame_read: false,
            captured_caption: None,
            number_frame: false,
            #[cfg(feature = "ocr")]
            paint_frame: false,
            #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
            background: None,
        }
    }
}

#[cfg(all(feature = "ocr", feature = "pdf", not(target_arch = "wasm32")))]
fn forced_khmer_retry_allowed(config: &ExtractionConfig, ocr: &crate::core::config::OcrConfig) -> bool {
    #[cfg(feature = "layout-types")]
    if config.layout.is_some() {
        return false;
    }
    config.force_ocr
        && config
            .ocr
            .as_ref()
            .is_some_and(|ocr| ocr.enabled && uses_default_ocr_pipeline(ocr))
        && uses_default_ocr_pipeline(ocr)
        && ocr.effective_tesseract_language().as_slice() == ["eng"]
        && ocr.auto_rotate != Some(true)
        && ocr.element_config.is_none()
        && !config.images.as_ref().is_some_and(|images| images.include_page_rasters)
}

#[cfg(all(feature = "ocr", feature = "pdf", not(target_arch = "wasm32")))]
fn append_forced_script_paragraphs(
    document: &mut crate::types::internal::InternalDocument,
    supplements: &ahash::AHashMap<usize, String>,
    page_index_offset: usize,
) {
    use crate::types::internal::{ElementKind, InternalElement};

    let mut insertions = std::collections::BTreeMap::<usize, Vec<InternalElement>>::new();
    for (&index, text) in supplements {
        let page = (page_index_offset + index + 1) as u32;
        let slot = document
            .elements
            .iter()
            .position(|element| element.page.is_some_and(|next| next > page))
            .unwrap_or(document.elements.len());
        let slot = if slot > 0 && matches!(document.elements[slot - 1].kind, ElementKind::PageBreak) {
            slot - 1
        } else {
            slot
        };
        insertions
            .entry(slot)
            .or_default()
            .push(InternalElement::text(ElementKind::Paragraph, text.clone(), 0).with_page(page));
    }
    for elements in insertions.values_mut() {
        elements.sort_by_key(|element| element.page);
    }
    let original = std::mem::take(&mut document.elements);
    let mut remap = Vec::with_capacity(original.len());
    for (index, element) in original.into_iter().enumerate() {
        if let Some(paragraphs) = insertions.remove(&index) {
            document.elements.extend(paragraphs);
        }
        remap.push(Some(document.push_element(element)));
    }
    document.elements.extend(insertions.into_values().flatten());
    super::document::remap_relationships(&mut document.relationships, &remap, &document.elements);
}

/// Same as [`extract_with_ocr`], but `page_rotation_override` -- when non-zero -- is used as
/// the known `/Rotate` value for every image in `images` instead of this function's own
/// content-based per-page lookup (`lazy_pdf_render_state` / `external_image_page_rotations`).
///
/// Needed by a caller that hands in a single image detached from its original page index
/// (currently only [`run_ocr_pipeline_for_page`]'s per-stage runs), where an index-based
/// content lookup would silently resolve the wrong page's rotation (or none at all) for any
/// page but the document's first. `0` defers entirely to the existing per-page
/// auto-detection -- see [`extract_with_ocr`], the public entry point every other caller uses.
///
/// `skip_document_global_heuristic` -- when `true`, skips the internal
/// [`heuristically_restructured_ocr_pages`] call and always uses the plain,
/// unstructured `assemble_internal_document` fallback for the returned document instead.
/// Needed by [`extract_mixed_ocr_native`]'s per-page pipeline route (#665/#1423-followup):
/// that caller drives this function with exactly one image per call (one PDF page detached
/// from the rest of the document), so the heuristic -- which clusters font sizes and needs
/// several pages in hand to be useful -- would only ever see a single page here and either
/// do nothing or, worse, mark that page's paragraphs "already structured" and block the
/// caller's own *document-wide* pass over every OCR'd page from running at all. The extra
/// tuple element this function returns (bare, unclassified per-page paragraphs) is what lets
/// that caller run the heuristic itself, once, over the whole document.
///
/// `points_per_pixel_override` -- when `Some`, used in place of this function's own
/// [`ocr_points_per_pixel`] lookup for every image in `images`. That lookup resolves a page's
/// pixel -> point scale from `lazy_pdf_render_state`, derived from `content` and indexed by
/// this function's own 0-based loop position over `images` -- exactly the "single detached
/// image is not really page 0" problem `page_rotation_override` already exists to solve, so a
/// caller driving one detached image per call (currently only
/// `extract_mixed_ocr_native`'s per-page pipeline route) needs the same override here: without
/// it, `content: None` makes the lookup fall back to `1.0` (pixels treated as points), silently
/// defeating the document-global heading heuristic's absolute-point font-gap comparisons.
///
/// ~keep: `page_ocr_hints` carries source DPI and scan classification when the caller detached a
/// rendered page from the PDF that supplied those facts. `None` preserves this function's
/// content-based derivation.
///
/// `page_index_offset` maps this function's local image indices back to document page
/// indices when a caller supplies a detached page image. It affects externally visible page
/// identity only; internal vectors remain indexed from zero.
///
/// `detached_page` -- the open document a detached page image came from. Only the
/// embedded-image retry reads it, at `page_index_offset`, so a caller that hands in a
/// detached page without `content` still gets the retry of a blank or failed page (#1912).
///
/// The final return flag records when the primary's existing detached-page XObject recovery
/// admitted no alternate image bytes. It remains false when the recovery was not checked.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
// Each parameter is independently documented above and forwarded verbatim by every caller
// (see `run_ocr_pipeline_for_page`); bundling them into a params struct would only move the
// arity, not remove it, since callers still need to name each field individually. Matches the
// same suppression already used for other multi-parameter internal helpers in this crate (see
// `onnx::download_model_files`).
#[allow(clippy::too_many_arguments)]
pub(super) async fn extract_with_ocr_for_page(
    content: Option<&[u8]>,
    images: Option<&[image::DynamicImage]>,
    #[cfg(feature = "layout-detection")] layout_detections: Option<&[crate::layout::DetectionResult]>,
    config: &ExtractionConfig,
    path: Option<&std::path::Path>,
    page_rotation_override: u32,
    skip_document_global_heuristic: bool,
    points_per_pixel_override: Option<f32>,
    page_ocr_hints: Option<PageOcrHints>,
    page_index_offset: usize,
    detached_page: Option<DetachedOcrPage<'_>>,
) -> OcrRunResult<page_ocr::PageOcrOutput> {
    page_ocr::run(
        content,
        images,
        #[cfg(feature = "layout-detection")]
        layout_detections,
        config,
        path,
        page_rotation_override,
        skip_document_global_heuristic,
        points_per_pixel_override,
        page_ocr_hints,
        page_index_offset,
        detached_page,
    )
    .await
}
#[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
async fn certify_discarded_native_layer(
    render: Option<super::rendering::FullPdfRenderState>,
    hints: Option<&PageOcrHints>,
    config: &ExtractionConfig,
) -> crate::Result<bool> {
    let Some((doc, 1, _)) = render else { return Ok(false) };
    let Some(layers) = hints.and_then(|hints| hints.native_page_layers.clone()) else {
        return Ok(false);
    };
    let byte_limit = super::hidden_layer::preview_byte_limit(config);
    super::super::ensure_pdf_not_cancelled(config)?;
    let audit = super::super::spawn_pdf_blocking(move || {
        crate::pdf::native::guard_native_panic(
            || {
                Ok(layers
                    .get(&1)
                    .is_some_and(|layer| super::hidden_layer::has_visible_native_line(&doc, layer, byte_limit)))
            },
            |panic| crate::XbergError::parsing(format!("PDF native line audit panicked: {panic}")),
        )
    })
    .await;
    super::super::ensure_pdf_not_cancelled(config)?;
    Ok(match audit {
        Ok(certified) => certified,
        Err(error) => {
            tracing::debug!(%error, "could not certify native text after OCR discard");
            false
        }
    })
}

/// Build an [`crate::types::ExtractedImage`] for a full-page OCR raster.
///
/// `image_index` is set to 0; the caller must reindex after merging into
/// the document's image collection.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(crate) fn build_page_raster_image(
    page_idx: usize,
    png_bytes: bytes::Bytes,
    width: u32,
    height: u32,
) -> crate::types::ExtractedImage {
    crate::types::ExtractedImage {
        data: png_bytes,
        format: std::borrow::Cow::Borrowed("png"),
        image_index: 0,
        page_number: Some((page_idx + 1) as u32),
        width: Some(width),
        height: Some(height),
        colorspace: Some("RGB".to_string()),
        bits_per_component: Some(8),
        is_mask: false,
        description: None,
        ocr_result: None,
        bounding_box: None,
        source_path: None,
        image_kind: Some(crate::types::ImageKind::PageRaster),
        kind_confidence: None,
        cluster_id: None,
        caption: None,
        qr_codes: None,
        data_base64: None,
    }
}
/// Adapt batch size to available system memory.
///
/// Estimates per-page memory cost based on typical page dimensions at 300 DPI
/// and compares against available system memory. Returns a batch size that
/// should keep peak memory within safe bounds.
///
/// Conservative estimate: each page in a batch needs approximately:
/// - ~50MB for render + encode working set (RGB buffer briefly, then PNG)
/// - ~100MB for OCR working set per concurrent page
/// - Plus the document itself and base allocations
///
/// Both callers are `async fn`s on the OCR route, and reading free memory blocks: on Linux it
/// reads `/proc/meminfo` and the cgroup files, and on macOS it calls `sysctlbyname`. Doing that on a runtime worker stalls every other task sharing the thread,
/// so the read goes to the blocking pool and the result is awaited. A read that cannot be
/// joined reports `None`, the same "could not tell" answer a failed query gives, which leaves
/// the configured batch size alone.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(super) async fn adapt_batch_size_to_memory(
    configured: usize,
    document_size: usize,
    per_page_bytes: usize,
) -> usize {
    adapt_batch_size_to_memory_inner(
        configured,
        document_size,
        per_page_bytes,
        available_memory_off_executor().await,
    )
}

/// Free memory, read without blocking the async executor.
///
/// `None` means the read could not tell. `Some(0)` means it did read the host and found no
/// headroom, which is a different answer and sizes a different batch.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
async fn available_memory_off_executor() -> Option<usize> {
    #[cfg(all(test, any(feature = "ocr", feature = "ocr-pipeline")))]
    {
        let pinned = TEST_AVAILABLE_MEMORY.load(std::sync::atomic::Ordering::SeqCst);
        if pinned != 0 {
            return Some(pinned);
        }
    }

    #[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
    {
        tokio::task::spawn_blocking(get_available_memory).await.ok().flatten()
    }
    #[cfg(not(all(feature = "tokio-runtime", not(target_arch = "wasm32"))))]
    {
        get_available_memory()
    }
}

/// Free memory to report instead of reading the host, so a test that drives the whole route can
/// pin the figure the batch width is computed from rather than asserting against whatever the
/// machine running it happens to have free.
///
/// Zero means "not set". A test that wants to exercise a measured zero calls
/// [`adapt_batch_size_to_memory_inner`] directly. It is process-wide, so every test that sets it
/// is `#[serial]`.
#[cfg(all(test, any(feature = "ocr", feature = "ocr-pipeline")))]
pub(super) static TEST_AVAILABLE_MEMORY: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Pure core of [`adapt_batch_size_to_memory`], parameterized on free memory so a test can
/// exercise a constrained host whatever the machine running it reports.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(super) fn adapt_batch_size_to_memory_inner(
    configured: usize,
    document_size: usize,
    per_page_bytes: usize,
    available_bytes: Option<usize>,
) -> usize {
    // An unknown reading and a reading of zero are different answers. Unknown leaves the
    // configured width alone, because nothing was measured to narrow it with. Zero was
    // measured: the host has no headroom, so the batch carries one page. Spelling both as 0
    // handed the widest batch to exactly the host with the least memory. The reader does not
    // charge a cgroup's inactive page cache, so a warmed container is not what reads as zero. ~keep
    let Some(available_bytes) = available_bytes else {
        return configured;
    };
    if per_page_bytes == 0 {
        return configured;
    }

    let session_reserve =
        usize::try_from(crate::core::config::concurrency::TESSERACT_SESSION_MEMORY_BYTES).unwrap_or(usize::MAX);
    let reserved = document_size.saturating_add(session_reserve);
    let usable = available_bytes.saturating_sub(reserved);
    let memory_limited_batch = (usable / per_page_bytes).max(1);
    let result = configured.min(memory_limited_batch);

    tracing::debug!(
        available_mb = available_bytes / (1024 * 1024),
        usable_mb = usable / (1024 * 1024),
        document_mb = document_size / (1024 * 1024),
        per_page_mb = per_page_bytes / (1024 * 1024),
        memory_limited_batch,
        configured,
        result,
        "OCR batch size adaptation"
    );

    result
}

/// What one page in flight costs when the caller does not yet know how big a page is.
///
/// The whole-document OCR route decides its batch width before it opens the document, so it
/// has no page box to measure and falls back to this. Every caller that does know a page's
/// size passes [`ocr_page_working_set_bytes`] instead: batch width is the only thing between
/// a wide thread budget and the peak, so a figure that ignores page size and DPI
/// under-reserves exactly on the documents where the peak matters.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(super) const UNMEASURED_OCR_PAGE_WORKING_SET_BYTES: usize = 150 * 1024 * 1024;

/// One page's render-and-encode working set at the DPI this route renders it: the renderer's
/// RGBA bitmap, the RGB raster decoded from it, the RGB clone the PNG encoder needs, and the PNG
/// buffer, which is what a batch holds per page while it runs. Falls back to
/// [`UNMEASURED_OCR_PAGE_WORKING_SET_BYTES`] when the dimensions overflow the estimate.
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) fn ocr_page_working_set_bytes(
    page_width_pt: f64,
    page_height_pt: f64,
    images_config: Option<&crate::core::config::ImageExtractionConfig>,
) -> usize {
    const PDF_POINTS_PER_INCH: f64 = 72.0;
    // The encode estimate starts from the decoded RGB raster, but the renderer's RGBA bitmap is
    // resident beside it while a page renders. Without it the estimate read 87 MB for a 300 DPI
    // A4 scan whose measured render wave cost about 115 MB a page. ~keep
    const RENDER_SOURCE_BYTES_PER_PIXEL: u64 = 4;

    let render_dpi = crate::image::dpi::effective_pdf_render_dpi(images_config, page_width_pt, page_height_pt);
    let width = ((page_width_pt / PDF_POINTS_PER_INCH) * f64::from(render_dpi))
        .round()
        .max(1.0) as u32;
    let height = ((page_height_pt / PDF_POINTS_PER_INCH) * f64::from(render_dpi))
        .round()
        .max(1.0) as u32;

    let render_source =
        crate::extraction::image_decode::decoded_byte_count(width, height, RENDER_SOURCE_BYTES_PER_PIXEL).ok();
    let encode = estimate_png_encode_page_peak_bytes(width, height).ok();
    render_source
        .zip(encode)
        .and_then(|(render_source, encode)| render_source.checked_add(encode))
        .and_then(|bytes| usize::try_from(bytes).ok())
        .filter(|bytes| *bytes > 0)
        .unwrap_or(UNMEASURED_OCR_PAGE_WORKING_SET_BYTES)
}

/// One page's estimated render-and-encode byte cost, matching
/// [`validate_png_encode_batch_peak`]'s own per-page accounting: the source raster, the RGB
/// conversion buffer, and the PNG output buffer.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(crate) fn estimate_png_encode_page_peak_bytes(width: u32, height: u32) -> crate::Result<u64> {
    let source = crate::extraction::image_decode::decoded_byte_count(width, height, 3)?;
    let conversion = crate::extraction::image_decode::decoded_byte_count(width, height, 3)?;
    let output = crate::extraction::image_decode::decoded_byte_count(width, height, OCR_PNG_ENCODE_BYTES_PER_PIXEL)?
        .checked_add(OCR_PNG_ENCODE_FIXED_BYTES)
        .ok_or_else(|| crate::extraction::image_decode::image_dimension_error(width, height, u64::MAX, u64::MAX))?;
    Ok(source + conversion + output)
}
/// Query available system memory without external dependencies.
///
/// Delegates to the crate's single memory reader, which takes the lower of the
/// cgroup headroom and the host's free memory on Linux and half of `hw.memsize`
/// on macOS. This module used to carry its own copy of that logic; recognition
/// concurrency needs the same number, and two readers of the same three files
/// drift apart. `None` means the query could not tell, and the caller keeps its
/// configured batch size; `Some(0)` is a real reading of a host with no headroom,
/// and the caller narrows to one page. ~keep
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(super) fn get_available_memory() -> Option<usize> {
    crate::core::config::concurrency::available_memory_bytes().and_then(|bytes| usize::try_from(bytes).ok())
}
/// Minimum meaningful-word density (per 1,000 non-whitespace characters) a
/// [`OcrPipelineSelection::PreferLastNonEmpty`] candidate must clear -- when the incumbent it
/// would replace already clears it -- for the override to be accepted (F46).
///
/// Provenance: measured on a 25-document sample where the VLM stage of a `vlm_fallback`
/// pipeline was taken unconditionally whenever non-empty. Scored by absolute dictionary-valid
/// word count, unconditional acceptance gained +877 valid words over tesseract-only, but a
/// "keep whichever result is better" oracle would have gained +1,071 -- 3 of 25 documents were
/// actively damaged by the VLM stage. A *dictionary-based* valid-word-density floor of ~60
/// words per 1,000 characters recovered +1,027 of that +1,071.
///
/// This constant applies that floor to [`NativeTextStats::meaningful_words`] instead: a
/// dictionary lookup is unavailable to this pure, backend-free decision point, and
/// `meaningful_words` (alphanumeric tokens of at least
/// [`OcrQualityThresholds::min_meaningful_word_len`] characters) is the existing dictionary-free
/// stand-in the codebase already computes for quality scoring. It is a looser signal than a real
/// dictionary check -- a 4+ character garbled token still counts -- so this reuses F46's measured
/// value as a starting operating point rather than a reproduction of that measurement; it has not
/// been separately calibrated against the `meaningful_words` metric.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(super) const MIN_VLM_OVERRIDE_WORD_DENSITY_PER_1000_CHARS: f64 = 60.0;
/// Minimum whitespace-delimited token count before meaningful-word density is treated as a
/// judgement rather than noise.
///
/// Mirrors the sibling convention in [`NativeTextStats::compute`], where `fragmented_word_ratio`
/// and `consecutive_repeat_ratio` both abstain below a token floor: the question is "is there
/// enough on this page to judge at all". Density is a ratio, so it is blind to amount -- a bare
/// `"Page 12"` header scores ~167 per 1,000 and would otherwise read as a dense page worth
/// protecting, and a single extra or missing token swings a short page's density by tens of
/// points. ~keep
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(super) const MIN_TOKENS_FOR_DENSITY_JUDGEMENT: usize = 10;
/// Meaningful-word density of `text`, per 1,000 non-whitespace characters, or `None` when the
/// text is too short to judge (see [`MIN_TOKENS_FOR_DENSITY_JUDGEMENT`]).
///
/// Shared scale for comparing a `PreferLastNonEmpty` candidate against the incumbent it would
/// replace (see [`MIN_VLM_OVERRIDE_WORD_DENSITY_PER_1000_CHARS`]). Scores the same normalized
/// input as [`compute_quality_score`](super::scoring::compute_quality_score) via
/// [`scoring_input`](super::scoring::scoring_input), so Markdown scaffolding cannot inflate the
/// denominator. Empty (post-whitespace-strip) text yields `None` rather than a
/// division-by-zero `NaN`.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(super) fn meaningful_word_density_per_1000_chars(text: &str, thresholds: &OcrQualityThresholds) -> Option<f64> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    let input = super::scoring::scoring_input(trimmed);
    let stats = NativeTextStats::compute(&input, thresholds);
    if stats.non_whitespace == 0 || stats.word_count < MIN_TOKENS_FOR_DENSITY_JUDGEMENT {
        return None;
    }
    Some(stats.meaningful_words as f64 / stats.non_whitespace as f64 * 1000.0)
}
/// Minimum share of a text's non-whitespace characters that must fall in a CJK ideographic
/// or kana block before `split_whitespace` token counts are treated as unreliable for it (see
/// [`is_non_space_delimited_script`]).
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
const MIN_CJK_CHAR_RATIO_FOR_SCRIPT_ABSTAIN: f64 = 0.3;
/// True for Chinese Han ideographs and Japanese kana -- scripts that do not delimit words
/// with whitespace. Deliberately excludes Hangul: modern Korean orthography is
/// space-delimited, so a Hangul page does not share the failure mode this guards against. ~keep
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn is_cjk_ideographic_or_kana(ch: char) -> bool {
    matches!(ch as u32,
        0x3400..=0x4DBF   // CJK Unified Ideographs Extension A
        | 0x4E00..=0x9FFF // CJK Unified Ideographs
        | 0xF900..=0xFAFF // CJK Compatibility Ideographs
        | 0x3040..=0x309F // Hiragana
        | 0x30A0..=0x30FF // Katakana
    )
}
/// True when `text` is dense enough in CJK ideographs/kana that `meaningful_word_density_per_1000_chars`'s
/// `split_whitespace` tokenization is not a meaningful word-count proxy for it: a whole
/// correct, dense paragraph in these scripts carries no internal spaces, so an OCR line break
/// is the only token boundary -- one line of genuinely dense, correct text collapses to a
/// single token while `non_whitespace` still counts every character, cratering density
/// independent of quality. This is a false-negative direction distinct from the ratio's
/// intended use (rejecting genuinely sparse/garbled Latin-script OCR output) that a
/// space-delimited-word assumption cannot see (F46 CJK false-negative). ~keep
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn is_non_space_delimited_script(text: &str) -> bool {
    let mut non_whitespace = 0usize;
    let mut cjk = 0usize;
    for ch in text.chars() {
        if ch.is_whitespace() {
            continue;
        }
        non_whitespace += 1;
        if is_cjk_ideographic_or_kana(ch) {
            cjk += 1;
        }
    }
    non_whitespace > 0 && (cjk as f64 / non_whitespace as f64) >= MIN_CJK_CHAR_RATIO_FOR_SCRIPT_ABSTAIN
}
/// Whether a non-empty `PreferLastNonEmpty` candidate is a materially worse replacement for a
/// non-empty incumbent, per [`MIN_VLM_OVERRIDE_WORD_DENSITY_PER_1000_CHARS`] (F46).
///
/// One-sided by construction: this only ever *blocks* a replacement, and only when the
/// incumbent was itself dense enough to be worth protecting. An incumbent that was already
/// below the floor has nothing worth protecting, so the later stage's non-empty override still
/// wins by default -- preserving #1341's intent that a later stage's result is a deliberate
/// override, not noise, and must not be pinned to a correctness-blind score comparison.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(super) fn candidate_is_materially_degraded(
    candidate_text: &str,
    incumbent_text: &str,
    thresholds: &OcrQualityThresholds,
) -> bool {
    // Word-count density cannot judge either side of this comparison when either text is
    // predominantly CJK: abstain entirely rather than let the space-delimited-word
    // assumption misjudge a script it was never built for (see
    // `is_non_space_delimited_script`). ~keep
    if is_non_space_delimited_script(incumbent_text) || is_non_space_delimited_script(candidate_text) {
        return false;
    }
    // An incumbent too short to judge has nothing established to protect, so the later stage's
    // override still wins -- same outcome as an incumbent measurably below the floor. ~keep
    let Some(incumbent_density) = meaningful_word_density_per_1000_chars(incumbent_text, thresholds) else {
        return false;
    };
    if incumbent_density < MIN_VLM_OVERRIDE_WORD_DENSITY_PER_1000_CHARS {
        return false;
    }
    // The incumbent is established dense text. A candidate too short to judge is not evidence
    // of a better result, and density alone cannot see that it is a fragment: `"Page 12"` scores
    // far above the floor. Keep the incumbent rather than trade it for an unjudgeable stub. ~keep
    let Some(candidate_density) = meaningful_word_density_per_1000_chars(candidate_text, thresholds) else {
        return true;
    };
    candidate_density < MIN_VLM_OVERRIDE_WORD_DENSITY_PER_1000_CHARS
}
/// Decide whether a pipeline stage's result should replace the current best-effort
/// candidate, given the pipeline's [`OcrPipelineSelection`](crate::core::config::OcrPipelineSelection) policy.
///
/// Only called once no stage has cleared `quality_thresholds.pipeline_min_quality` (the
/// accept-threshold early return in [`run_ocr_pipeline`] handles that case directly).
/// Pure and backend-free so the policy can be unit-tested without a registered OCR
/// backend.
///
/// - [`OcrPipelineSelection::HighestScore`]: replace only if `candidate_score` strictly
///   exceeds the current best score (or there is no current best). This is the original,
///   correctness-blind quality-max behavior.
/// - [`OcrPipelineSelection::PreferLastNonEmpty`]: replace whenever `candidate_text` is
///   non-empty, regardless of score, since a later stage in a fallback pipeline only ran
///   because the earlier stage(s) were judged inadequate. An empty candidate never
///   replaces an existing best, so a destroyed page still keeps the earlier text. The one
///   exception (F46) is a one-sided quality guard: a non-empty candidate that is materially
///   worse than a non-empty, already-dense incumbent (see
///   [`candidate_is_materially_degraded`]) does not replace it either. The later stage still
///   wins whenever it is not a clear regression -- this does not resurrect `HighestScore`'s
///   symmetric best-score comparison.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(super) fn should_replace_best_effort_result(
    selection: crate::core::config::OcrPipelineSelection,
    best_score: Option<f64>,
    best_text: Option<&str>,
    candidate_text: &str,
    candidate_score: f64,
    thresholds: &OcrQualityThresholds,
) -> bool {
    use crate::core::config::OcrPipelineSelection;

    match selection {
        OcrPipelineSelection::HighestScore => match best_score {
            Some(best) => candidate_score > best,
            None => true,
        },
        OcrPipelineSelection::PreferLastNonEmpty => {
            if candidate_text.trim().is_empty() {
                return best_score.is_none();
            }
            match best_text.map(str::trim).filter(|text| !text.is_empty()) {
                Some(incumbent_text) => !candidate_is_materially_degraded(candidate_text, incumbent_text, thresholds),
                None => true,
            }
        }
    }
}
/// A document that holds `text` as paragraphs, for a pipeline result that has no document.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn ocr_pipeline_text_document(text: &str) -> crate::types::internal::InternalDocument {
    let mut doc = crate::types::internal::InternalDocument::new("pdf");
    // Backend text verbatim (see `flat_ocr_page_document`): normalize before splitting.
    let text = crate::extraction::transform::normalize_line_endings(text);
    for paragraph in text.split("\n\n").map(str::trim).filter(|text| !text.is_empty()) {
        doc.push_element(crate::types::internal::InternalElement::text(
            crate::types::internal::ElementKind::Paragraph,
            paragraph,
            0,
        ));
    }
    doc
}

/// Attach skipped and failed stage diagnostics to the result that survives the pipeline.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(super) fn attach_ocr_pipeline_stage_warnings(
    mut doc: Option<crate::types::internal::InternalDocument>,
    text: &str,
    unavailable_backends: &[String],
    stage_failures: &[(String, crate::XbergError)],
) -> Option<crate::types::internal::InternalDocument> {
    if unavailable_backends.is_empty() && stage_failures.is_empty() {
        return doc;
    }

    let retained_doc = doc.get_or_insert_with(|| ocr_pipeline_text_document(text));

    for backend in unavailable_backends {
        retained_doc.processing_warnings.push(crate::types::ProcessingWarning {
            source: std::borrow::Cow::Borrowed("ocr_pipeline"),
            message: std::borrow::Cow::Owned(format!(
                "Requested OCR pipeline backend '{backend}' is unavailable and was skipped."
            )),
        });
    }
    for (backend, error) in stage_failures {
        retained_doc.processing_warnings.push(crate::types::ProcessingWarning {
            source: std::borrow::Cow::Borrowed("ocr_pipeline"),
            message: std::borrow::Cow::Owned(format!(
                "OCR fallback backend '{backend}' failed and was skipped: {error}"
            )),
        });
    }

    doc
}
/// Attach force_ocr image-XObject fallback warnings (#1355) to the OCR-produced
/// document, mirroring [`attach_ocr_pipeline_stage_warnings`]'s `get_or_insert_with`
/// shape so the warning always survives even when no structured document was built.
//
// `ocr-pipeline` (not just `ocr`): the caller is inside `extract_with_ocr`
// (`any(ocr, ocr-pipeline)`), and the `binstall` CLI profile enables `ocr-pipeline`
// via `liter-llm` without `ocr`. ~keep
#[cfg(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
pub(super) fn attach_ocr_fallback_warnings(
    mut doc: Option<crate::types::internal::InternalDocument>,
    text: &str,
    warnings: Vec<crate::types::ProcessingWarning>,
) -> Option<crate::types::internal::InternalDocument> {
    if warnings.is_empty() {
        return doc;
    }

    let retained_doc = doc.get_or_insert_with(|| {
        let mut doc = crate::types::internal::InternalDocument::new("pdf");
        // Backend text verbatim (see `flat_ocr_page_document`): normalize before splitting.
        let text = crate::extraction::transform::normalize_line_endings(text);
        for paragraph in text.split("\n\n").map(str::trim).filter(|text| !text.is_empty()) {
            doc.push_element(crate::types::internal::InternalElement::text(
                crate::types::internal::ElementKind::Paragraph,
                paragraph,
                0,
            ));
        }
        doc
    });

    retained_doc.processing_warnings.extend(warnings);

    doc
}
/// The warning that makes an image-XObject recovery visible in the output.
///
/// `draw_failed` comes from the same [`XObjectFallbackTrigger`] that gated the retry. The retry
/// runs both when the rasterizer produced a blank page and when a perfectly-drawn page simply
/// yielded no OCR text, and only the first is a rasterizer failure -- reporting the second with
/// that wording sends a reader looking for a drawing bug that did not happen (GH#1826).
///
/// [`XObjectFallbackTrigger`]: super::rendering::XObjectFallbackTrigger
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) fn xobject_fallback_warning(
    page_idx: usize,
    attempted: usize,
    draw_failed: bool,
) -> crate::types::ProcessingWarning {
    let message = if draw_failed {
        format!(
            "Page {} rendered blank but contains {} image XObject(s) the PDF rasterizer \
             could not draw; OCR was retried on the embedded image bytes.",
            page_idx + 1,
            attempted
        )
    } else {
        format!(
            "Page {} contains {} image XObject(s) but OCR returned no usable text; \
             OCR was retried on the embedded image bytes.",
            page_idx + 1,
            attempted
        )
    };
    crate::types::ProcessingWarning {
        source: std::borrow::Cow::Borrowed("ocr"),
        message: std::borrow::Cow::Owned(message),
    }
}

/// Whether the image-XObject retry's text should replace the page's own OCR text.
///
/// The retry fires for a page carrying up to `MAX_INK_PROBE_TEXT_CHARS` of real text, so adopting
/// on non-emptiness alone let a single character of noise overwrite text the page OCR had already
/// read correctly (GH#1826). Compare what each attempt actually read instead.
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) fn should_adopt_xobject_retry_text(original: &str, retry: &str) -> bool {
    let non_whitespace = |text: &str| text.chars().filter(|c| !c.is_whitespace()).count();
    !retry.is_empty() && non_whitespace(retry) > non_whitespace(original)
}

/// Last-resort image-XObject recovery for the OCR *pipeline* route, used when every stage
/// failed outright (#1444).
///
/// The per-page fallback inside [`extract_with_ocr_for_page`] covers a stage that ran and
/// came back blank. It cannot cover a stage that never produced a result at all — a VLM that
/// errored on every page, an unreachable service — because [`run_ocr_pipeline_for_page`]
/// catches that `Err` and moves on, and once the last stage is exhausted the pipeline used to
/// error out having never tried the pages' embedded images.
///
/// Uses the highest-priority *available* stage's backend, which is the one the pipeline
/// itself would have preferred. Returns `None` when there is no PDF content, no usable
/// backend, or no page yielded any recoverable payload — the caller must then still report the failure.
#[cfg(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
pub(super) struct PipelineXObjectRecoveryOutcome {
    pub(super) text: String,
    pub(super) page_failures: Vec<crate::types::OcrPageFailure>,
    pub(super) page_texts: Vec<String>,
    pub(super) images: Vec<crate::types::ExtractedImage>,
    pub(super) warnings: Vec<crate::types::ProcessingWarning>,
    pub(super) llm_usage: Vec<crate::types::LlmUsage>,
    pub(super) tables: Vec<crate::types::Table>,
    pub(super) formulas: Vec<crate::types::Formula>,
    pub(super) preprocessing: ahash::AHashMap<u32, crate::types::ImagePreprocessingMetadata>,
}
#[cfg(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
impl PipelineXObjectRecoveryOutcome {
    fn new(page_count: usize) -> Self {
        Self {
            text: String::new(),
            page_failures: Vec::new(),
            page_texts: vec![String::new(); page_count],
            images: Vec::new(),
            warnings: Vec::new(),
            llm_usage: Vec::new(),
            tables: Vec::new(),
            formulas: Vec::new(),
            preprocessing: ahash::AHashMap::new(),
        }
    }

    fn has_content(&self) -> bool {
        self.page_texts.iter().any(|text| !text.trim().is_empty())
            || !self.tables.is_empty()
            || !self.formulas.is_empty()
    }
}

/// Whether the same config without its pipeline reads each page through the synthesized
/// Tesseract-to-Paddle pipeline. That route recovers a refused page without classifying its
/// paragraphs, so its Plain output keeps each recovered block a paragraph, list markers or not;
/// a single-backend route retries the page itself and lists them. ~keep
#[cfg(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
fn default_route_reads_pages_through_a_pipeline(config: &ExtractionConfig) -> bool {
    cfg!(paddle_ocr)
        && config.ocr.as_ref().is_some_and(|ocr| {
            ocr.backend == "tesseract" && ocr.vlm_fallback == crate::core::config::VlmFallbackPolicy::Disabled
        })
}

#[cfg(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
fn plain_budget_recovery_list_document(
    recovery: &PipelineXObjectRecoveryOutcome,
    config: &ExtractionConfig,
) -> Option<crate::types::internal::InternalDocument> {
    if config.output_format != crate::core::config::OutputFormat::Plain
        || config.pages.as_ref().is_some_and(|pages| pages.insert_page_markers)
    {
        return None;
    }
    let table_pages: ahash::AHashSet<_> = recovery.tables.iter().map(|table| table.page_number).collect();
    let flat: Vec<_> = recovery
        .page_texts
        .iter()
        .map(|text| crate::pdf::structure::adapters::ocr_text_to_paragraphs(text))
        .collect();
    let mut listed = flat.clone();
    for (index, paragraphs) in listed.iter_mut().enumerate() {
        if !table_pages.contains(&((index + 1) as u32)) {
            apply_ocr_text_list_fallback(paragraphs);
        }
    }
    if !listed.iter().flatten().any(|paragraph| paragraph.is_list_item) {
        return None;
    }
    let pages = if default_route_reads_pages_through_a_pipeline(config) {
        flat
    } else {
        listed
    };
    let mut document =
        crate::pdf::structure::assemble_internal_document(pages, &recovery.tables, None, &[], &Default::default());
    // The assembler skips tables without Markdown, but their sidecars still belong to the result. ~keep
    document.tables.extend(
        recovery
            .tables
            .iter()
            .filter(|table| table.markdown.trim().is_empty())
            .cloned(),
    );
    Some(document)
}

#[cfg(all(test, feature = "pdf", feature = "ocr"))]
mod plain_budget_recovery_list_document_tests {
    use super::*;

    const NOTES: &str = "GENERAL NOTES\n\n1. All lot lines are shown for\n\n1\n\nplanning purposes only.";

    fn plain(backend: &str) -> String {
        let mut recovery = PipelineXObjectRecoveryOutcome::new(1);
        recovery.page_texts[0] = NOTES.to_owned();
        let config = ExtractionConfig {
            ocr: Some(crate::core::config::OcrConfig {
                backend: backend.to_owned(),
                ..Default::default()
            }),
            ..Default::default()
        };
        let document = plain_budget_recovery_list_document(&recovery, &config).expect("the notes hold a list item");
        crate::extraction::derive::derive_extraction_result(document, false, crate::core::config::OutputFormat::Plain)
            .content
    }

    /// The built-in Tesseract's default route reads each page through the synthesized
    /// Tesseract-to-Paddle pipeline when Paddle is compiled in; that route keeps every recovered
    /// block its own paragraph, so a numbered note is not joined to the next block.
    #[test]
    fn recovered_notes_keep_the_paragraphs_of_the_default_route_of_their_backend() {
        let listed = "GENERAL NOTES\n\n1. All lot lines are shown for\n1\n\nplanning purposes only.";
        let tesseract = if cfg!(paddle_ocr) { NOTES } else { listed };
        assert_eq!(plain("tesseract"), tesseract);
        assert_eq!(plain("budget-recovery-custom-backend"), listed);
    }
}

#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) struct XObjectRecoveryContext<'a> {
    pub(super) budget: &'a mut crate::extractors::security::SecurityBudget,
    pub(super) blank_raster_warnings: Option<&'a mut Vec<crate::types::ProcessingWarning>>,
    pub(super) no_admitted_images: Option<&'a mut bool>,
    pub(super) stored_jpegs: Option<&'a mut crate::pdf::native::images::StoredJpegRecoveryBudget>,
    pub(super) cancel_token: Option<&'a crate::cancellation::CancellationToken>,
    /// Hand the backend the resolution a page-covering image's placement proves. Off, the
    /// backend estimates it as before.
    pub(super) use_proven_source_dpi: bool,
    /// How each image read made at a proven resolution is judged for noise. `None` leaves every
    /// read to the page policy.
    pub(super) proven_read: Option<ProvenReadPolicy<'a>>,
    pub(super) emit_progress: bool,
}

/// The recognition-noise thresholds every route judges a page read or an image retry read by.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn route_noise_thresholds(config: &ExtractionConfig) -> crate::core::config::OcrQualityThresholds {
    config
        .ocr
        .as_ref()
        .and_then(|ocr| ocr.quality_thresholds.clone())
        .unwrap_or_default()
}

/// The noise policy a route applies to each retry read made at a placement-proven resolution.
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
#[derive(Clone, Copy)]
pub(super) struct ProvenReadPolicy<'a> {
    pub(super) thresholds: &'a crate::core::config::OcrQualityThresholds,
    /// Whether this route warns about a read it keeps.
    pub(super) warn_when_kept: bool,
}

#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
impl<'a> XObjectRecoveryContext<'a> {
    pub(super) fn with_budget(
        budget: &'a mut crate::extractors::security::SecurityBudget,
        cancel_token: Option<&'a crate::cancellation::CancellationToken>,
    ) -> Self {
        Self {
            budget,
            blank_raster_warnings: None,
            no_admitted_images: None,
            stored_jpegs: None,
            cancel_token,
            use_proven_source_dpi: false,
            proven_read: None,
            emit_progress: true,
        }
    }

    /// The context of a route that reads an embedded image at its proven resolution and judges each
    /// read for noise by `thresholds`, warning about a read it keeps. The whole-document recovery
    /// never passes its text through the page-level noise check, so its reads warn here; the
    /// mixed route's retry of a page does pass through it and sets `warn_when_kept` to false.
    fn with_proven_reads(mut self, thresholds: &'a crate::core::config::OcrQualityThresholds) -> Self {
        self.use_proven_source_dpi = true;
        self.proven_read = Some(ProvenReadPolicy {
            thresholds,
            warn_when_kept: true,
        });
        self
    }

    pub(super) fn check_cancelled(&self) -> crate::Result<()> {
        crate::cancellation::check_cancelled(self.cancel_token)
    }
}
#[cfg(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
#[derive(Clone, Copy)]
pub(super) enum XObjectProjection {
    #[cfg(all(
        test,
        feature = "pdf",
        feature = "tokio-runtime",
        any(feature = "ocr", feature = "ocr-pipeline")
    ))]
    RawPayloads,
    CompletePage,
}
#[cfg(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
pub(super) async fn collect_pipeline_xobject_pages(
    backend: &std::sync::Arc<dyn crate::plugins::OcrBackend>,
    doc: &xberg_native_pdf::PdfDocument,
    page_count: usize,
    ocr_config: &crate::core::config::OcrConfig,
    context: XObjectRecoveryContext<'_>,
    projection: XObjectProjection,
) -> crate::Result<PipelineXObjectRecoveryOutcome> {
    let mut outcome = PipelineXObjectRecoveryOutcome::new(page_count);
    let limits = ocr_config.security_limits.clone().unwrap_or_default();
    let mut stored_jpegs = crate::pdf::native::images::StoredJpegRecoveryBudget::new(limits.max_content_size);
    for page_idx in 0..page_count {
        let mut page_context = XObjectRecoveryContext::with_budget(context.budget, context.cancel_token);
        page_context.stored_jpegs = Some(&mut stored_jpegs);
        page_context.use_proven_source_dpi = context.use_proven_source_dpi;
        page_context.emit_progress = context.emit_progress;
        page_context.proven_read = context.proven_read;
        if matches!(projection, XObjectProjection::CompletePage)
            && !super::rendering::can_project_stored_jpeg_page(doc, page_idx, &limits, &mut page_context)?
        {
            outcome.page_failures.push(incomplete_stored_projection(page_idx));
            continue;
        }
        let recovery =
            recover_page_text_from_image_xobjects(backend, doc, page_idx, page_count, ocr_config, page_context).await;
        context.check_cancelled()?;
        let Some(recovery) = recovery? else {
            continue;
        };
        append_pipeline_xobject_page(&mut outcome, recovery, page_idx, page_idx, false);
    }
    if stored_jpegs.skipped_payload() {
        outcome.warnings.push(crate::core::diagnostics::warning(
            "ocr",
            concat!(
                "Some embedded JPEG images were skipped during OCR recovery because ",
                "the retained encoded payload limit was reached."
            ),
        ));
    }
    Ok(outcome)
}
#[cfg(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
fn incomplete_stored_projection(page_idx: usize) -> crate::types::OcrPageFailure {
    crate::types::OcrPageFailure {
        page: (page_idx + 1) as u32,
        error: "Stored JPEG images cannot represent the complete visible page composition".into(),
        recovered: false,
    }
}
#[cfg(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
fn append_pipeline_xobject_page(
    outcome: &mut PipelineXObjectRecoveryOutcome,
    mut recovery: XObjectRecoveryOutcome,
    page_idx: usize,
    local_page_idx: usize,
    page_stages_failed: bool,
) {
    let recovered_payload = !recovery.text.is_empty() || !recovery.tables.is_empty() || !recovery.formulas.is_empty();
    // The returned document is built from this text beside the recovered tables (#2014).
    outcome.page_texts[local_page_idx] = std::mem::take(&mut recovery.text_outside_tables);
    if recovered_payload {
        let warning = if page_stages_failed {
            crate::types::ProcessingWarning {
                source: std::borrow::Cow::Borrowed("ocr"),
                message: std::borrow::Cow::Owned(format!(
                    "Page {} contains {} image XObject(s); OCR was retried on the embedded image bytes after all page OCR stages failed.",
                    page_idx + 1,
                    recovery.attempted
                )),
            }
        } else {
            xobject_fallback_warning(page_idx, recovery.attempted, true)
        };
        outcome.warnings.push(warning);
    } else if !recovery.failure_causes.is_empty() {
        outcome.page_failures.push(crate::types::OcrPageFailure {
            page: (page_idx + 1) as u32,
            recovered: false,
            error: format!(
                "Embedded-image retry recovered no text ({})",
                retry_cause_summary(&recovery.failure_causes)
            ),
        });
    }
    outcome.warnings.append(&mut recovery.warnings);
    outcome.llm_usage.append(&mut recovery.llm_usage);
    outcome.tables.append(&mut recovery.tables);
    outcome.formulas.append(&mut recovery.formulas);
    if let Some(metadata) = recovery.image_preprocessing {
        outcome.preprocessing.insert((page_idx + 1) as u32, metadata);
    }
    outcome.images.append(&mut recovery.images);
}
#[cfg(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
pub(super) fn join_pipeline_xobject_text(
    page_texts: &[String],
    config: &ExtractionConfig,
    budget: &mut crate::extractors::security::SecurityBudget,
) -> crate::Result<String> {
    join_pipeline_xobject_text_at_offset(page_texts, config, budget, 0)
}
#[cfg(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
fn join_pipeline_xobject_text_at_offset(
    page_texts: &[String],
    config: &ExtractionConfig,
    budget: &mut crate::extractors::security::SecurityBudget,
    page_offset: usize,
) -> crate::Result<String> {
    let marker_format = super::super::ocr_page_marker_format(config);
    let mut text = String::new();
    for (page_idx, page_text) in page_texts.iter().enumerate() {
        let prefix = super::super::ocr_page_prefix(marker_format, page_idx, page_offset);
        budget.account_text(prefix.len())?;
        text.push_str(&prefix);
        text.push_str(page_text);
    }
    Ok(text)
}
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn pipeline_xobject_content_limit(config: &ExtractionConfig, ocr_config: &crate::core::config::OcrConfig) -> usize {
    match (config.security_limits.as_ref(), ocr_config.security_limits.as_ref()) {
        (Some(outer), Some(ocr)) => outer.max_content_size.min(ocr.max_content_size),
        (Some(outer), None) => outer.max_content_size,
        (None, Some(ocr)) => ocr.max_content_size,
        (None, None) => crate::extractors::security::SecurityLimits::default().max_content_size,
    }
}
/// The OCR config both image-XObject retries hand the backend. The limits follow the OCR precedence (the OCR
/// config's own limits, then the extraction limits, then the defaults); the byte cap is the smaller of the two.
#[cfg(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
fn pipeline_xobject_recovery_ocr_config(
    config: &ExtractionConfig,
    ocr_config: &crate::core::config::OcrConfig,
) -> crate::core::config::OcrConfig {
    let mut limits = ocr_config
        .security_limits
        .clone()
        .or_else(|| config.security_limits.clone())
        .unwrap_or_default();
    limits.max_content_size = pipeline_xobject_content_limit(config, ocr_config);
    let mut recovery_ocr_config = ocr_config.clone();
    recovery_ocr_config.security_limits = Some(limits);
    recovery_ocr_config
}
#[cfg(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
pub(super) async fn recover_pipeline_document_from_image_xobjects(
    content: Option<&[u8]>,
    config: &ExtractionConfig,
    ocr_config: &crate::core::config::OcrConfig,
    backend_name: &str,
) -> crate::Result<Option<PipelineXObjectRecoveryOutcome>> {
    if config.cancel_token.as_ref().is_some_and(|token| token.is_cancelled()) {
        return Err(crate::XbergError::Cancelled);
    }
    let Some(content) = content else {
        return Ok(None);
    };
    let cancel_token = config.cancel_token.as_ref();
    if cancel_token.is_some_and(|token| token.is_cancelled()) {
        return Err(crate::XbergError::Cancelled);
    }
    let backend = {
        let registry = crate::plugins::registry::get_ocr_backend_registry();
        let registry = registry.read();
        let Ok(backend) = registry.get(backend_name) else {
            return Ok(None);
        };
        backend
    };
    let Ok((doc, page_count, _)) = open_pdf_for_full_ocr(content) else {
        return Ok(None);
    };

    let mut budget = crate::extractors::security::SecurityBudget::from_config(config);
    let recovery_ocr_config = pipeline_xobject_recovery_ocr_config(config, ocr_config);
    let thresholds = route_noise_thresholds(config);
    let context = XObjectRecoveryContext::with_budget(&mut budget, cancel_token).with_proven_reads(&thresholds);
    let mut outcome = collect_pipeline_xobject_pages(
        &backend,
        &doc,
        page_count,
        &recovery_ocr_config,
        context,
        XObjectProjection::CompletePage,
    )
    .await?;
    if !outcome.has_content() {
        return Ok(None);
    }
    outcome.text = join_pipeline_xobject_text(&outcome.page_texts, config, &mut budget)?;
    Ok(Some(outcome))
}
#[cfg(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
async fn recover_pipeline_page_from_image_xobjects(
    doc: &xberg_native_pdf::PdfDocument,
    page_idx: usize,
    config: &ExtractionConfig,
    ocr_config: &crate::core::config::OcrConfig,
    backend_name: &str,
) -> crate::Result<Option<PipelineXObjectRecoveryOutcome>> {
    if config.cancel_token.as_ref().is_some_and(|token| token.is_cancelled()) {
        return Err(crate::XbergError::Cancelled);
    }
    let backend = {
        let registry = crate::plugins::registry::get_ocr_backend_registry();
        let registry = registry.read();
        let Ok(backend) = registry.get(backend_name) else {
            return Ok(None);
        };
        backend
    };
    let Ok(page_count) = doc.page_count() else {
        return Ok(None);
    };
    if page_idx >= page_count {
        return Ok(None);
    }
    let mut budget = crate::extractors::security::SecurityBudget::from_config(config);
    let recovery_ocr_config = pipeline_xobject_recovery_ocr_config(config, ocr_config);
    let recovery = recover_page_text_from_bounded_image_xobjects(
        &backend,
        doc,
        page_idx,
        page_count,
        &recovery_ocr_config,
        XObjectRecoveryContext::with_budget(&mut budget, config.cancel_token.as_ref()),
    )
    .await?;
    if config.cancel_token.as_ref().is_some_and(|token| token.is_cancelled()) {
        return Err(crate::XbergError::Cancelled);
    }
    let Some(recovery) = recovery else {
        return Ok(None);
    };
    let mut outcome = PipelineXObjectRecoveryOutcome::new(1);
    append_pipeline_xobject_page(&mut outcome, recovery, page_idx, 0, true);
    if !outcome.has_content() {
        return Ok(None);
    }
    outcome.text = join_pipeline_xobject_text_at_offset(&outcome.page_texts, config, &mut budget, page_idx)?;
    Ok(Some(outcome))
}
/// Run a multi-backend OCR pipeline with quality-based fallback.
///
/// Images and layout detections are computed once and shared across all stages.
/// Each stage produces OCR output that is scored; if the score meets the
/// pipeline's quality threshold, the result is accepted. Otherwise, the next
/// backend is tried. If no stage clears the threshold, `pipeline.selection`
/// decides which stage's result is returned as the best effort: the
/// highest-scoring one ([`OcrPipelineSelection::HighestScore`], the default, used
/// for explicit and classical auto-fallback pipelines), or the last stage that
/// produced non-empty text ([`OcrPipelineSelection::PreferLastNonEmpty`], used by
/// `vlm_fallback`-synthesised pipelines -- see
/// [`should_replace_best_effort_result`]).
///
/// Thin, signature-preserving wrapper over [`run_ocr_pipeline_for_page`] with no page
/// rotation (`0`), so every pre-existing caller keeps today's behavior -- each stage's own
/// [`extract_with_ocr`] call falls back to its content-based per-page auto-detection.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(crate) async fn run_ocr_pipeline(
    content: Option<&[u8]>,
    images: Option<&[image::DynamicImage]>,
    #[cfg(feature = "layout-detection")] layout_detections: Option<&[crate::layout::DetectionResult]>,
    config: &ExtractionConfig,
    pipeline: &crate::core::config::OcrPipelineConfig,
    path: Option<&std::path::Path>,
    page_ocr_hints: Option<PageOcrHints>,
) -> OcrRunResult<(
    String,
    Vec<crate::types::Table>,
    Vec<crate::types::OcrElement>,
    Option<crate::types::internal::InternalDocument>,
    Vec<crate::types::LlmUsage>,
    Vec<String>,
    Option<Vec<crate::types::ExtractedImage>>,
    Vec<crate::types::Formula>,
    ahash::AHashMap<u32, crate::types::ImagePreprocessingMetadata>,
    ahash::AHashMap<u32, crate::types::page::PageOcrConfidence>,
)> {
    let (
        text,
        tables,
        elements,
        doc,
        usage,
        page_texts,
        rasters,
        formulas,
        _raw_page_paragraphs,
        preprocessing,
        ocr_confidence,
        _noise_verdicts,
        _read_evidence,
    ) = Box::pin(run_ocr_pipeline_for_page(
        content,
        images,
        #[cfg(feature = "layout-detection")]
        layout_detections,
        config,
        pipeline,
        path,
        0,
        false,
        None,
        page_ocr_hints,
        0,
        None,
    ))
    .await?;
    #[cfg(all(
        test,
        feature = "pdf",
        feature = "ocr",
        feature = "tokio-runtime",
        not(target_arch = "wasm32")
    ))]
    record_region_replay(&_read_evidence);
    Ok((
        text,
        tables,
        elements,
        doc,
        usage,
        page_texts,
        rasters,
        formulas,
        preprocessing,
        ocr_confidence,
    ))
}
/// Same as [`run_ocr_pipeline`], but `page_rotation_degrees` -- the page's known PDF
/// `/Rotate` value, `0` if unrotated or genuinely unknown to the caller -- is forwarded to
/// every stage's [`extract_with_ocr_for_page`] call as its `page_rotation_override`. Each
/// stage backend still resolves its own [`crate::plugins::PageOrientationHandling`], so a
/// `RequiresUpright` stage gets an upright raster via `upright_raster_for_backend` while a
/// `SelfCorrecting` or `RecognisesRotatedText` stage's raster is left untouched (or only gets
/// the `ocr_config_with_page_rotation_hint` block-order hint) -- see `extract_with_ocr_for_page`.
///
/// Needed by a caller that drives a single page's image through the pipeline detached from
/// the rest of its document (currently only `extract_mixed_ocr_native`'s per-page pipeline
/// route, see #651): that caller already knows the page's own `/Rotate` value, but the
/// stage's own content-based auto-detection cannot recover it from a lone, index-0 image.
///
/// `skip_document_global_heuristic` is forwarded verbatim to every stage's
/// [`extract_with_ocr_for_page`] call, and this function's own extra return element -- the
/// winning stage's bare, unclassified per-page paragraphs -- is that stage's own extra
/// element, so callers get back exactly the material a stage never got to structure itself.
/// `points_per_pixel_override` is likewise forwarded verbatim to every stage's
/// [`extract_with_ocr_for_page`] call. See that function's doc comments for why both exist.
/// `page_index_offset` carries the detached image's original zero-based document page index
/// through each stage so warnings, traces, and structured payloads retain their real page.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
// Same rationale as `extract_with_ocr_for_page`, whose full parameter set this function
// forwards to every pipeline stage verbatim (see doc comment above).
#[allow(clippy::too_many_arguments)]
pub(super) async fn run_ocr_pipeline_for_page(
    content: Option<&[u8]>,
    images: Option<&[image::DynamicImage]>,
    #[cfg(feature = "layout-detection")] layout_detections: Option<&[crate::layout::DetectionResult]>,
    config: &ExtractionConfig,
    pipeline: &crate::core::config::OcrPipelineConfig,
    path: Option<&std::path::Path>,
    page_rotation_degrees: u32,
    skip_document_global_heuristic: bool,
    points_per_pixel_override: Option<f32>,
    page_ocr_hints: Option<PageOcrHints>,
    page_index_offset: usize,
    detached_page: Option<DetachedOcrPage<'_>>,
) -> OcrRunResult<OcrPipelineOutput> {
    let mut stage_page_failures: Vec<crate::types::OcrPageFailure> = Vec::new();
    let result = Box::pin(run_ocr_pipeline_stages(
        PipelineStageInputs {
            content,
            images,
            #[cfg(feature = "layout-detection")]
            layout_detections,
            path,
            page_rotation_degrees,
            skip_document_global_heuristic,
            points_per_pixel_override,
            page_ocr_hints,
            page_index_offset,
            detached_page,
        },
        config,
        pipeline,
        &mut stage_page_failures,
    ))
    .await;
    match result {
        Ok(mut output) => {
            output.3 = attach_unread_page_failures(
                output.3.take(),
                &output.0,
                &output.5,
                page_index_offset,
                stage_page_failures,
            );
            Ok(output)
        }
        Err(mut failed_run) => {
            merge_ocr_page_failures(&mut failed_run.page_failures, stage_page_failures);
            Err(failed_run)
        }
    }
}

/// What a pipeline run returns: the text, the tables, the OCR elements, the document, the LLM
/// usage, the text of each page, the page rasters, the formulas, the bare paragraphs of each
/// page, the preprocessing metadata, the OCR confidence of each page and the recognition-noise
/// verdicts of the returned stage.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
type OcrPipelineOutput = (
    String,
    Vec<crate::types::Table>,
    Vec<crate::types::OcrElement>,
    Option<crate::types::internal::InternalDocument>,
    Vec<crate::types::LlmUsage>,
    Vec<String>,
    Option<Vec<crate::types::ExtractedImage>>,
    Vec<crate::types::Formula>,
    Vec<RunPageParagraphs>,
    ahash::AHashMap<u32, crate::types::ImagePreprocessingMetadata>,
    ahash::AHashMap<u32, crate::types::page::PageOcrConfidence>,
    Vec<OcrPageNoiseVerdict>,
    OcrReadEvidence,
);

/// Add to `doc` the records that the stages of a pipeline run made for the pages the run
/// returns without text.
///
/// A page that the returned stage read gets no record from another stage. A record that is
/// kept is not recovered: the output of the stage that made it is not in the result. A caller
/// that has native text for the page decides from that text.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(super) fn attach_unread_page_failures(
    mut doc: Option<crate::types::internal::InternalDocument>,
    text: &str,
    page_texts: &[String],
    page_index_offset: usize,
    stage_page_failures: Vec<crate::types::OcrPageFailure>,
) -> Option<crate::types::internal::InternalDocument> {
    let unread: Vec<crate::types::OcrPageFailure> = stage_page_failures
        .into_iter()
        .filter(|failure| {
            let page_text = (failure.page as usize)
                .checked_sub(page_index_offset + 1)
                .and_then(|local_index| page_texts.get(local_index));
            match page_text {
                Some(page_text) => page_text.trim().is_empty(),
                None => text.trim().is_empty(),
            }
        })
        .map(|failure| crate::types::OcrPageFailure {
            recovered: false,
            ..failure
        })
        .collect();
    if unread.is_empty() {
        return doc;
    }
    let retained_doc = doc.get_or_insert_with(|| ocr_pipeline_text_document(text));
    merge_ocr_page_failures(&mut retained_doc.ocr_page_failures, unread);
    retained_doc.ocr_page_failures.sort_by_key(|failure| failure.page);
    doc
}

/// Put the layer first on each page of the returned stage of a whole-document run whose OCR text
/// got the noise verdict or confirms the layer, after the stage was scored and chosen on its OCR
/// text alone. A run without layers is unchanged.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn with_run_layers_first(
    layers: Option<std::sync::Arc<ahash::AHashMap<u32, String>>>,
    run: RunLayerSources<'_>,
    mut text: String,
    mut page_texts: Vec<String>,
    mut doc: Option<crate::types::internal::InternalDocument>,
    config: &ExtractionConfig,
) -> (String, Vec<String>, Option<crate::types::internal::InternalDocument>) {
    if let Some(layers) = layers {
        let boxes = run_merge_geometry(&layers, &run, &page_texts, config);
        let remaining = |page: u32, layer: &str, flat: &str| {
            #[cfg(all(
                feature = "pdf",
                feature = "ocr",
                feature = "tokio-runtime",
                not(target_arch = "wasm32")
            ))]
            {
                (page as usize)
                    .checked_sub(run.page_index_offset + 1)
                    .and_then(|index| run.raw_page_paragraphs.get(index))
                    .and_then(|page| page.leading_khmer_rows.as_ref())
                    .map_or_else(Default::default, |rows| rows.prepend_policy(layer, flat))
            }
            #[cfg(not(all(
                feature = "pdf",
                feature = "ocr",
                feature = "tokio-runtime",
                not(target_arch = "wasm32")
            )))]
            {
                let _ = (page, layer, flat);
                super::document::NativePrepend::Original
            }
        };
        let lost = super::document::put_layers_first_in_run_with_recovered_rows(
            &mut text,
            &mut page_texts,
            doc.as_mut(),
            super::document::PageLayers {
                lines: &layers,
                boxes: &boxes,
            },
            run.verdicts,
            run.page_index_offset,
            config,
            remaining,
        );
        if let Some(warning) = super::super::unmapped_glyphs::lost_glyph_warning(lost, config) {
            let doc = doc.get_or_insert_with(|| super::document::flat_ocr_page_document(&text));
            crate::core::diagnostics::push_warning_deduped(&mut doc.processing_warnings, warning);
        }
    }
    (text, page_texts, doc)
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn run_merge_geometry(
    layers: &ahash::AHashMap<u32, String>,
    run: &RunLayerSources<'_>,
    page_texts: &[String],
    config: &ExtractionConfig,
) -> ahash::AHashMap<u32, super::document::PageLineBoxes> {
    let mut boxes = ahash::AHashMap::new();
    let mut geometry = run.content.map(|content| MergeGeometrySession::new(content, config));
    let thresholds = route_noise_thresholds(config);
    for (&page_number, layer) in layers {
        if config.cancel_token.as_ref().is_some_and(|token| token.is_cancelled()) {
            break;
        }
        if run
            .verdicts
            .iter()
            .any(|verdict| verdict.page_index as u32 + 1 == page_number)
        {
            continue;
        }
        let Some((page, read)) = (page_number as usize)
            .checked_sub(run.page_index_offset + 1)
            .and_then(|index| run.raw_page_paragraphs.get(index).zip(page_texts.get(index)))
        else {
            continue;
        };
        if !super::document::ocr_confirms_layer(layer, read, &thresholds) {
            continue;
        }
        // Per-page OCR frames are optional; unknown frames cannot authorize removal. ~keep
        if let Some(geometry) = geometry.as_mut()
            && let Some(page_boxes) = geometry.page_boxes(page_number, Some(&page.paragraphs), page.ocr_scale)
        {
            boxes.insert(page_number, page_boxes);
        }
    }
    boxes
}

/// What a whole-document run gives the layer merge beside its text: the noise verdicts, the
/// document bytes for the native segment pass, and the OCR paragraphs of each page by local
/// index. ~keep
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
struct RunLayerSources<'a> {
    verdicts: &'a [OcrPageNoiseVerdict],
    page_index_offset: usize,
    content: Option<&'a [u8]>,
    raw_page_paragraphs: &'a [RunPageParagraphs],
}

#[cfg(all(
    feature = "ocr",
    feature = "layout-detection",
    feature = "tokio-runtime",
    not(target_arch = "wasm32")
))]
pub(super) fn original_ocr_layout_blocks_reconstruction(
    paragraphs: &mut Vec<crate::pdf::structure::types::PdfParagraph>,
    config: &ExtractionConfig,
) -> bool {
    apply_ocr_layout_content_filter(paragraphs, config);
    crate::pdf::structure::adapters::promote_anchored_ordered_list_sequences(std::slice::from_mut(paragraphs));
    super::document::ocr_paragraphs_already_structured(paragraphs)
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(super) fn ocr_heuristic_input(
    paragraphs: &[crate::pdf::structure::types::PdfParagraph],
    preserve_ordered_layout: bool,
) -> &[crate::pdf::structure::types::PdfParagraph] {
    // Rebuilding from segments loses classified furniture and line order.
    if preserve_ordered_layout { &[] } else { paragraphs }
}

#[cfg(all(
    feature = "ocr",
    feature = "layout-detection",
    feature = "tokio-runtime",
    not(target_arch = "wasm32")
))]
fn apply_owned_column_order(
    doc: &mut crate::types::internal::InternalDocument,
    lanes: super::column_retry::ColumnLanes,
    page_size: (f32, f32),
    cancel: Option<&crate::cancellation::CancellationToken>,
) -> crate::Result<()> {
    if cancel.is_some_and(|token| token.is_cancelled()) {
        return Err(crate::XbergError::Cancelled);
    }
    let (w, h) = lanes.frame();
    if cancel.is_some_and(|token| token.time_limit_reached()) {
        super::column_retry::note_deadline(&mut doc.processing_warnings, cancel);
        return Ok(());
    }
    let order = lanes.plan(doc, (page_size.0 / w as f32, page_size.1 / h as f32));
    if cancel.is_some_and(|token| token.is_cancelled()) {
        return Err(crate::XbergError::Cancelled);
    }
    if cancel.is_some_and(|token| token.time_limit_reached()) {
        super::column_retry::note_deadline(&mut doc.processing_warnings, cancel);
    } else if let Some(order) = order {
        let mut slots: Vec<_> = std::mem::take(&mut doc.elements).into_iter().map(Some).collect();
        doc.elements = order
            .into_iter()
            .map(|index| slots[index].take().expect("column order is a permutation"))
            .collect();
        tracing::debug!("Reordered source-owned body paragraphs by column");
    }
    Ok(())
}

/// The OCR paragraphs of one page and their per-axis conversion to MediaBox points.
/// Certified layout pages already use points and have an identity conversion.
/// A missing conversion means the page cannot authorize removal during the layer merge. ~keep
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[derive(Debug, Default)]
pub(super) struct RunPageParagraphs {
    pub(super) paragraphs: Vec<crate::pdf::structure::types::PdfParagraph>,
    #[cfg(all(
        feature = "ocr",
        feature = "layout-detection",
        feature = "tokio-runtime",
        not(target_arch = "wasm32")
    ))]
    pub(super) column_lanes: Option<super::column_retry::ColumnLanes>,
    pub(super) preserve_ordered_layout: bool,
    pub(super) ocr_scale: Option<(f32, f32)>,
    pub(super) detached_ocr_scale: Option<(f32, f32)>,
    /// Raster dimensions used to assemble paragraphs, after undoing any backend rotation.
    #[cfg(feature = "pdf")]
    pub(super) raster_px: Option<(u32, u32)>,
    #[cfg(all(
        feature = "pdf",
        feature = "ocr",
        feature = "tokio-runtime",
        not(target_arch = "wasm32")
    ))]
    pub(super) leading_khmer_rows: Option<super::layer_token_recovery::LeadingKhmerRows>,
}

/// The factor per axis from the frame of the OCR paragraphs of a page of a whole-document run
/// (the raster, or the size preprocessing resampled it to) to PDF points, the same conversion
/// the mixed route applies to its pipeline pages. `None` when the
/// run rendered no PDF page or the raster is empty. ~keep
#[cfg(all(
    any(feature = "ocr", feature = "ocr-pipeline"),
    any(not(feature = "layout-detection"), feature = "ocr", feature = "ocr-wasm")
))]
fn run_page_ocr_scale(
    #[cfg(feature = "pdf")] lazy_pdf_render_state: Option<&super::rendering::FullPdfRenderState>,
    page_idx: usize,
    raster_px: (u32, u32),
) -> Option<(f32, f32)> {
    #[cfg(feature = "pdf")]
    {
        let (pdf, _, _) = lazy_pdf_render_state?;
        let (width_px, height_px) = raster_px;
        let (width_pt, height_pt) = page_dimensions_pt(pdf, page_idx);
        (width_px > 0 && height_px > 0).then(|| (width_pt / width_px as f32, height_pt / height_px as f32))
    }
    #[cfg(not(feature = "pdf"))]
    {
        let _ = (page_idx, raster_px);
        None
    }
}

/// One lazy authenticated native document per merge, including a cached failure. Geometry is
/// optional evidence: unavailable pages keep both payloads and never authorize OCR deletion. ~keep
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
struct MergeGeometrySession<'a> {
    content: &'a [u8],
    config: &'a ExtractionConfig,
    #[cfg(feature = "pdf")]
    document: Option<Option<crate::pdf::native::NativeDocument>>,
}

#[cfg(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
fn open_merge_geometry_document(
    content: &[u8],
    config: &ExtractionConfig,
) -> Option<crate::pdf::native::NativeDocument> {
    if config.cancel_token.as_ref().is_some_and(|token| token.is_cancelled()) {
        return None;
    }
    let passwords = config
        .pdf_options
        .as_ref()
        .and_then(|options| options.passwords.as_deref())
        .unwrap_or(&[]);
    #[cfg(test)]
    geometry_session_tests::record_open();
    let document = crate::pdf::native::NativeDocument::open_bytes_with_passwords(content, passwords).ok()?;
    document.doc.set_keep_white_text(super::super::keeps_white_text(config));
    Some(document)
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
impl<'a> MergeGeometrySession<'a> {
    fn new(content: &'a [u8], config: &'a ExtractionConfig) -> Self {
        Self {
            content,
            config,
            #[cfg(feature = "pdf")]
            document: None,
        }
    }

    fn cancelled(&self) -> bool {
        self.config
            .cancel_token
            .as_ref()
            .is_some_and(|token| token.is_cancelled())
    }

    fn page_boxes(
        &mut self,
        page_number: u32,
        paragraphs: Option<&Vec<crate::pdf::structure::types::PdfParagraph>>,
        scale: Option<(f32, f32)>,
    ) -> Option<super::document::PageLineBoxes> {
        if self.cancelled() {
            return None;
        }
        let scale = scale?;
        let paragraphs =
            paragraphs.filter(|paragraphs| paragraphs.iter().any(|paragraph| !paragraph.lines.is_empty()))?;
        let page_index = (page_number as usize).checked_sub(1)?;
        #[cfg(feature = "pdf")]
        {
            let config = self.config;
            let content = self.content;
            let document = self
                .document
                .get_or_insert_with(|| open_merge_geometry_document(content, config))
                .as_mut()?;
            if config.cancel_token.as_ref().is_some_and(|token| token.is_cancelled()) {
                return None;
            }
            let artifacts =
                crate::pdf::native::hierarchy::ArtifactRetention::from_content_filter(config.content_filter.as_ref());
            let segments =
                crate::pdf::native::hierarchy::extract_segments_from_page(document, page_index, artifacts).ok()?;
            super::document::page_line_boxes(&segments, paragraphs, scale)
        }
        #[cfg(not(feature = "pdf"))]
        {
            let _ = (self.content, paragraphs, scale, page_index);
            None
        }
    }
}

/// What a pipeline run gives to each of its stages without a change: the parameters of
/// [`run_ocr_pipeline_for_page`] that go to [`extract_with_ocr_for_page`].
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
struct PipelineStageInputs<'a> {
    content: Option<&'a [u8]>,
    images: Option<&'a [image::DynamicImage]>,
    #[cfg(feature = "layout-detection")]
    layout_detections: Option<&'a [crate::layout::DetectionResult]>,
    path: Option<&'a std::path::Path>,
    page_rotation_degrees: u32,
    skip_document_global_heuristic: bool,
    points_per_pixel_override: Option<f32>,
    page_ocr_hints: Option<PageOcrHints>,
    page_index_offset: usize,
    detached_page: Option<DetachedOcrPage<'a>>,
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn can_skip_blank_automatic_paddle_raster(images: Option<&[image::DynamicImage]>) -> bool {
    #[cfg(all(paddle_ocr, feature = "ocr", feature = "pdf", not(target_arch = "wasm32")))]
    {
        let Some([image]) = images else {
            return false;
        };
        if !crate::plugins::registry::get_ocr_backend_registry()
            .read()
            .has_builtin_paddle()
        {
            return false;
        }
        // Sampled ink can miss faint or sparse text. Check the retained raster exactly. ~keep
        let bytes = image.as_bytes();
        let channels = match image.color() {
            image::ColorType::Rgb8 => 3,
            image::ColorType::Rgba8 if bytes.get(3) == Some(&255) => 4,
            _ => return false,
        };
        let mut pixels = bytes.chunks_exact(channels);
        pixels.next().is_some_and(|first| pixels.all(|pixel| pixel == first))
    }
    #[cfg(not(all(paddle_ocr, feature = "ocr", feature = "pdf", not(target_arch = "wasm32"))))]
    {
        let _ = images;
        false
    }
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn pre_page_ocr_config(config: &ExtractionConfig) -> Cow<'_, crate::core::config::OcrConfig> {
    let mut resolved = super::resolve_ocr_config(config);
    if (resolved.acceleration.is_none() && config.acceleration.is_some())
        || (resolved.security_limits.is_none() && config.security_limits.is_some())
    {
        let ocr = resolved.to_mut();
        if ocr.acceleration.is_none() {
            ocr.acceleration = config.acceleration.clone();
        }
        // GH#1554: mirrors `acceleration` above so the scanned-page OCR route inherits the
        // caller's configured decode limits instead of always falling back to
        // `SecurityLimits::default()`. ~keep
        if ocr.security_limits.is_none() {
            ocr.security_limits = config.security_limits.clone();
        }
    }
    resolved
}
#[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
async fn diagnose_djot_budget_refusal(
    error: crate::XbergError,
    doc: &xberg_native_pdf::PdfDocument,
    config: &ExtractionConfig,
    backend: &std::sync::Arc<dyn crate::plugins::OcrBackend>,
    ocr: &crate::core::config::OcrConfig,
    budget: &mut crate::extractors::security::SecurityBudget,
) -> crate::Result<crate::XbergError> {
    use crate::core::config::{OcrStrategy, OutputFormat};
    if config.output_format != OutputFormat::Djot
        || config.ocr_strategy != OcrStrategy::Auto
        || doc.page_count().ok() != Some(1)
        || !render_refused_by_source_image_budget(&error)
    {
        return Ok(error);
    }
    if !crate::plugins::registry::get_ocr_backend_registry()
        .read()
        .is_builtin_tesseract(backend)
    {
        return Ok(error);
    }
    let thresholds = route_noise_thresholds(config);
    let mut context =
        XObjectRecoveryContext::with_budget(budget, config.cancel_token.as_ref()).with_proven_reads(&thresholds);
    context.emit_progress = false;
    let probe = recover_page_text_from_image_xobjects(backend, doc, 0, 1, ocr, context).await;
    super::super::ensure_pdf_not_cancelled(config)?;
    finish_djot_budget_probe(error, probe.map(|value| value.map(|read| read.failure_causes)))
}

#[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
fn finish_djot_budget_probe(
    error: crate::XbergError,
    probe: crate::Result<Option<Vec<String>>>,
) -> crate::Result<crate::XbergError> {
    match probe {
        Ok(Some(causes)) => Ok(with_retry_causes(error, &causes)),
        Err(crate::XbergError::Cancelled) => Err(crate::XbergError::Cancelled),
        Ok(None) | Err(_) => Ok(error),
    }
}

#[cfg(all(test, feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
#[test]
fn djot_budget_probe_preserves_refusal_except_on_cancellation() {
    let refusal = || crate::XbergError::Parsing {
        message: "render refused".to_owned(),
        source: Some(Box::new(std::io::Error::other("typed source"))),
    };
    for (probe, expected) in [
        (Ok(None), "render refused"),
        (Ok(Some(Vec::new())), "render refused"),
        (Err(crate::XbergError::validation("probe limit")), "render refused"),
        (
            Ok(Some(vec!["noisy image".to_owned()])),
            "render refused; embedded-image retry recovered no text (noisy image)",
        ),
    ] {
        let crate::XbergError::Parsing {
            message,
            source: Some(source),
        } = finish_djot_budget_probe(refusal(), probe).unwrap()
        else {
            panic!("expected typed render refusal");
        };
        assert_eq!(message, expected);
        assert!(source.is::<std::io::Error>());
    }
    assert!(matches!(
        finish_djot_budget_probe(refusal(), Err(crate::XbergError::Cancelled)),
        Err(crate::XbergError::Cancelled)
    ));
}

/// Why a retry read made at a placement-proven resolution was not adopted.
#[cfg(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
const PROVEN_READ_NOISE_CAUSE: &str = "the retry read was rejected as noise";

/// Name the noise cause once, however many images of the page were rejected.
#[cfg(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
fn push_proven_noise_cause(causes: &mut Vec<String>) {
    if !causes.iter().any(|cause| cause == PROVEN_READ_NOISE_CAUSE) {
        causes.push(PROVEN_READ_NOISE_CAUSE.to_owned());
    }
}

/// What a retry read made at a placement-proven resolution is judged against.
#[cfg(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
pub(super) struct ProvenRead {
    pub(super) used_proven_source_dpi: bool,
    pub(super) possibly_illegible: bool,
    pub(super) page_idx: usize,
    /// Whether this route warns about a read it keeps. The single-backend route assesses every
    /// adopted page afterwards and warns there.
    pub(super) warn_when_kept: bool,
}

/// Apply the page's recognition-noise policy to a retry read made at a proven resolution.
///
/// The proven resolution lets the backend read an image the budget refused. A dimension clamp or a
/// shrink below the legibility floor can leave the glyphs illegible, and only the output shows
/// it, so a noisy read of such an image (or one with no preprocessing record) is dropped. Any other
/// read, including a plain downscale, follows the caller's `discard_suspected_ocr_noise` like a
/// page read: kept with the warning, or dropped when the caller asked for that. The assessment
/// carries no confidence or dictionary signal, so only the fragmented-word test judges it.
#[cfg(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
pub(super) fn judge_proven_read(
    text: String,
    read: ProvenRead,
    thresholds: &crate::core::config::OcrQualityThresholds,
    warnings: &mut Vec<crate::types::ProcessingWarning>,
    retry_causes: &mut Vec<String>,
) -> String {
    if !read.used_proven_source_dpi {
        return text;
    }
    if read.possibly_illegible {
        if super::scoring::is_ocr_recognition_noise(&text, thresholds) {
            push_proven_noise_cause(retry_causes);
            return String::new();
        }
        return text;
    }
    let mut assessment = Vec::new();
    let acceptance = accept_or_reject_ocr_page(
        read.page_idx,
        text,
        thresholds,
        &mut assessment,
        None,
        crate::plugins::ConfidenceSemantics::None,
        None,
    );
    if acceptance.discarded {
        push_proven_noise_cause(retry_causes);
    }
    if read.warn_when_kept || acceptance.discarded {
        for warning in assessment {
            crate::core::diagnostics::push_warning_deduped(warnings, warning);
        }
    }
    acceptance.content
}

/// Most image causes a page error names before summarizing the rest.
#[cfg(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
const RETRY_CAUSES_SHOWN: usize = 3;

#[cfg(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
fn retry_cause_summary(causes: &[String]) -> String {
    let mut named = causes.iter().take(RETRY_CAUSES_SHOWN).cloned().collect::<Vec<_>>();
    if causes.len() > RETRY_CAUSES_SHOWN {
        named.push(format!("+{} more", causes.len() - RETRY_CAUSES_SHOWN));
    }
    named.join("; ")
}

/// The error of a page whose embedded-image retry also recovered nothing, naming why each image
/// failed. The typed source stays, so callers still match on the original cause.
#[cfg(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
fn with_retry_causes(error: crate::XbergError, causes: &[String]) -> crate::XbergError {
    match error {
        crate::XbergError::Parsing { message, source } if !causes.is_empty() => crate::XbergError::Parsing {
            message: format!(
                "{message}; embedded-image retry recovered no text ({})",
                retry_cause_summary(causes)
            ),
            source,
        },
        other => other,
    }
}
#[cfg(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
fn render_refused_by_source_image_budget(error: &crate::XbergError) -> bool {
    use std::error::Error as _;
    error
        .source()
        .is_some_and(|source| source.is::<super::rendering::SourceImageBudgetExceeded>())
}
/// The stages of [`run_ocr_pipeline_for_page`]. Each stage adds its page records to
/// `stage_page_failures`: a stage that failed, and a stage whose result is not returned.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
async fn run_ocr_pipeline_stages(
    inputs: PipelineStageInputs<'_>,
    config: &ExtractionConfig,
    pipeline: &crate::core::config::OcrPipelineConfig,
    stage_page_failures: &mut Vec<crate::types::OcrPageFailure>,
) -> OcrRunResult<OcrPipelineOutput> {
    use crate::plugins::registry::get_ocr_backend_registry;

    // Re-seed the built-in backends before deciding which pipeline stages are available. A
    // prior `clear_ocr_backends()` call in the same process (e.g. a binding e2e suite's
    // backend-management test running before this one) otherwise leaves the registry
    // empty, so every stage below reads as unavailable and the pipeline fails outright
    // instead of falling back to (or re-using) the built-in defaults. ~keep
    crate::plugins::ensure_ocr_backends_initialized();

    let resolved_ocr_config = super::resolve_ocr_config(config);
    let ocr_config = resolved_ocr_config.as_ref();

    // Best-effort selection policy is derived from the config that produced this pipeline
    // (a `vlm_fallback`-synthesised pipeline prefers its last non-empty stage; explicit and
    // classical pipelines stay score-based) rather than carried on `OcrPipelineConfig`, so
    // the binding-facing config stays unchanged (#1341).
    let selection = ocr_config.pipeline_selection();
    // The layers of a whole-document run; a page route hands none and puts them first itself.
    let run_layers = inputs
        .page_ocr_hints
        .as_ref()
        .and_then(|hints| hints.native_page_layers.clone());

    let mut stages = pipeline.stages.clone();
    stages.sort_by_key(|b| std::cmp::Reverse(b.priority));

    let requested_backends: Vec<String> = stages.iter().map(|s| s.backend.clone()).collect();
    let (available_stages, unavailable_backends): (Vec<_>, Vec<_>) = {
        let registry = get_ocr_backend_registry();
        let registry = registry.read();
        stages
            .into_iter()
            .partition(|stage| registry.get(&stage.backend).is_ok())
    };
    let unavailable_backends = unavailable_backends
        .into_iter()
        .map(|stage| stage.backend)
        .collect::<Vec<_>>();

    if available_stages.is_empty() {
        return Err(crate::XbergError::Parsing {
            message: format!(
                "No available OCR backends for pipeline (requested: {})",
                requested_backends.join(", ")
            ),
            source: None,
        }
        .into());
    }

    #[allow(clippy::type_complexity)]
    let mut best_result: Option<(
        String,
        f64,
        Vec<crate::types::Table>,
        Vec<crate::types::OcrElement>,
        Option<crate::types::internal::InternalDocument>,
        Vec<String>,
        Option<Vec<crate::types::ExtractedImage>>,
        Vec<crate::types::Formula>,
        Vec<RunPageParagraphs>,
        ahash::AHashMap<u32, crate::types::ImagePreprocessingMetadata>,
        ahash::AHashMap<u32, crate::types::page::PageOcrConfidence>,
        Vec<OcrPageNoiseVerdict>,
        OcrReadEvidence,
    )> = None;

    let mut accumulated_usage: Vec<crate::types::LlmUsage> = Vec::new();
    // Track stages that errored outright (e.g. a VLM fallback that failed
    // authentication) so the failure is surfaced to the caller instead of being
    // silently replaced by a lower-quality earlier result (issue #1339).
    let mut stage_failures = Vec::new();
    let mut unrecovered_glyphs_on_page_one = 0;
    #[cfg(feature = "pdf")]
    let mut preferred_recovery_config = None;
    #[cfg(feature = "pdf")]
    let mut preferred_recovery_budget_refused = false;

    let automatic_paddle = inputs.detached_page.is_some()
        && uses_default_ocr_pipeline(ocr_config)
        && ocr_config.backend == "tesseract"
        && matches!(available_stages.as_slice(), [first, second]
            if first.backend == "tesseract" && second.backend == "paddleocr");

    #[cfg(all(paddle_ocr, feature = "ocr", not(target_arch = "wasm32")))]
    let sparse_trial = inputs
        .page_ocr_hints
        .as_ref()
        .is_some_and(|hints| hints.sparse_glyph_trial.is_some());
    #[cfg(not(all(paddle_ocr, feature = "ocr", not(target_arch = "wasm32"))))]
    let sparse_trial = false;

    for stage in &available_stages {
        #[cfg(all(paddle_ocr, feature = "ocr", not(target_arch = "wasm32")))]
        if sparse_trial && (!automatic_paddle || stage.backend != "tesseract") {
            return Err(sparse_glyph_read::declined());
        }
        let mut stage_ocr = ocr_config.clone();
        super::stage_model_inputs::StageModelInputs::new(ocr_config, stage).apply_to(&mut stage_ocr);
        if stage.paddle_ocr_config.is_some() || stage.paddle_ocr_settings.is_some() {
            stage_ocr.paddle_ocr_config = stage.paddle_ocr_config.clone();
            stage_ocr.paddle_ocr_settings = stage.paddle_ocr_settings.clone();
        }
        stage_ocr.quality_thresholds = Some(pipeline.quality_thresholds.clone());
        stage_ocr.vlm_config = stage.vlm_config.clone();
        stage_ocr.backend_options = stage.backend_options.clone();
        #[cfg(all(paddle_ocr, feature = "ocr", not(target_arch = "wasm32")))]
        let stage_ocr = sparse_glyph_read::ocr_config(stage_ocr, sparse_trial);

        let stage_config = ExtractionConfig {
            ocr: Some(stage_ocr),
            ..config.clone()
        };
        #[cfg(feature = "pdf")]
        let is_preferred_recovery_stage = preferred_recovery_config.is_none();
        #[cfg(feature = "pdf")]
        if is_preferred_recovery_stage {
            preferred_recovery_config = Some(pre_page_ocr_config(&stage_config).into_owned());
        }

        tracing::debug!(
            backend = %stage.backend,
            priority = stage.priority,
            "Pipeline: trying OCR backend"
        );

        let page_ocr_hints = inputs.page_ocr_hints.clone();
        #[cfg(all(
            feature = "pdf",
            feature = "ocr",
            feature = "tokio-runtime",
            not(target_arch = "wasm32")
        ))]
        let page_ocr_hints = page_ocr_hints.map(|mut hints| {
            if !automatic_paddle || stage.backend != "tesseract" {
                hints.number_source = None;
            }
            hints
        });
        let result = Box::pin(extract_with_ocr_for_page(
            inputs.content,
            inputs.images,
            #[cfg(feature = "layout-detection")]
            inputs.layout_detections,
            &stage_config,
            inputs.path,
            inputs.page_rotation_degrees,
            inputs.skip_document_global_heuristic,
            inputs.points_per_pixel_override,
            page_ocr_hints,
            inputs.page_index_offset,
            inputs.detached_page,
        ))
        .await;

        match result {
            Ok((
                text,
                mean_conf,
                stage_tables,
                stage_ocr_elements,
                stage_doc,
                stage_llm_usage,
                stage_page_texts,
                stage_rasters,
                stage_formulas,
                stage_raw_paragraphs,
                stage_preprocessing,
                stage_ocr_confidence,
                stage_recognition_noise_verdicts,
                read_evidence,
            )) => {
                // Scored before pruning, so symbol-noise pruning never changes stage selection.
                let text_score = compute_quality_score(
                    read_evidence.unpruned_text.as_deref().unwrap_or(&text),
                    &pipeline.quality_thresholds,
                );
                let score = pipeline_stage_score(text_score, mean_conf);

                tracing::debug!(
                    backend = %stage.backend,
                    score,
                    text_score,
                    mean_text_conf = ?mean_conf,
                    threshold = pipeline.quality_thresholds.pipeline_min_quality,
                    "Pipeline: backend produced result"
                );

                // Plumbed to the accept decision for observability only (see
                // `OcrPageNoiseVerdict`) -- the accept/reject outcome below must not change
                // until a threshold is calibrated against real corpus data. ~keep
                for verdict in &stage_recognition_noise_verdicts {
                    tracing::debug!(
                        backend = %stage.backend,
                        page = verdict.page_index + 1,
                        fragmented_word_ratio = verdict.fragmented_word_ratio,
                        word_count = verdict.word_count,
                        mean_confidence = verdict.mean_confidence,
                        low_confidence = verdict.low_confidence,
                        fragmented_noise = verdict.fragmented_noise,
                        dictionary_noise = verdict.dictionary_noise,
                        dict_invalid_word_ratio = verdict.dict_invalid_word_ratio,
                        discarded = verdict.discarded,
                        "Pipeline: OCR recognition-noise verdict in scope at accept decision"
                    );
                }

                accumulated_usage.extend(stage_llm_usage);
                if let Some(stage_doc) = stage_doc.as_ref() {
                    merge_ocr_page_failures(stage_page_failures, stage_doc.ocr_page_failures.clone());
                }

                let skip_automatic_paddle = automatic_paddle
                    && read_evidence.no_admitted_images
                    && text.trim().is_empty()
                    && stage_page_texts.iter().all(|text| text.trim().is_empty())
                    && stage_tables.is_empty()
                    && stage_ocr_elements.is_empty()
                    && stage_formulas.is_empty()
                    && stage_doc.as_ref().is_none_or(|doc| doc.ocr_page_failures.is_empty())
                    && can_skip_blank_automatic_paddle_raster(inputs.images);
                if skip_automatic_paddle && config.cancel_token.as_ref().is_some_and(|token| token.is_cancelled()) {
                    return Err(crate::XbergError::Cancelled.into());
                }

                let selected_read_evidence = if automatic_paddle && read_evidence.original_tesseract_raster {
                    read_evidence
                } else {
                    OcrReadEvidence {
                        #[cfg(all(
                            feature = "pdf",
                            feature = "ocr",
                            feature = "tokio-runtime",
                            not(target_arch = "wasm32")
                        ))]
                        matched_layer_occurrences: read_evidence.matched_layer_occurrences,
                        #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
                        number_primary_text: read_evidence.number_primary_text,
                        retained_hocr_read: read_evidence.retained_hocr_read,
                        #[cfg(all(
                            test,
                            feature = "pdf",
                            feature = "ocr",
                            feature = "tokio-runtime",
                            not(target_arch = "wasm32")
                        ))]
                        region_replay_raster: read_evidence.region_replay_raster,
                        ..Default::default()
                    }
                };
                #[cfg(all(paddle_ocr, feature = "ocr", not(target_arch = "wasm32")))]
                if sparse_trial && (score < pipeline.quality_thresholds.pipeline_min_quality || text.trim().is_empty())
                {
                    return Err(sparse_glyph_read::declined());
                }
                if score >= pipeline.quality_thresholds.pipeline_min_quality || (skip_automatic_paddle && !sparse_trial)
                {
                    let (text, stage_page_texts, stage_doc) = with_run_layers_first(
                        run_layers,
                        RunLayerSources {
                            verdicts: &stage_recognition_noise_verdicts,
                            page_index_offset: inputs.page_index_offset,
                            content: inputs.content,
                            raw_page_paragraphs: &stage_raw_paragraphs,
                        },
                        text,
                        stage_page_texts,
                        stage_doc,
                        config,
                    );
                    super::super::ensure_pdf_not_cancelled(config)?;
                    if let Some(read) = selected_read_evidence.retained_hocr_read.as_ref() {
                        tracing::trace!(
                            occurrences = read.occurrences.len(),
                            page_matches = matches!(stage_page_texts.as_slice(), [page] if page == &read.transcript),
                            document_matches = text == read.transcript,
                            "selected hOCR region proof carried to rendering"
                        );
                    }
                    // ~keep Attach prior-stage diagnostics before this accepted-stage early
                    // return; otherwise successful fallback silently erases why it ran.
                    let stage_doc =
                        attach_ocr_pipeline_stage_warnings(stage_doc, &text, &unavailable_backends, &stage_failures);
                    return Ok((
                        text,
                        stage_tables,
                        stage_ocr_elements,
                        stage_doc,
                        accumulated_usage,
                        stage_page_texts,
                        stage_rasters,
                        stage_formulas,
                        stage_raw_paragraphs,
                        stage_preprocessing,
                        stage_ocr_confidence,
                        stage_recognition_noise_verdicts,
                        selected_read_evidence,
                    ));
                }

                // Selection policy decides which stage's result to keep once no stage has
                // cleared the accept threshold (see `should_replace_best_effort_result`).
                // `HighestScore` (explicit / classical auto-fallback pipelines) keeps the
                // original strict quality-max behavior. `PreferLastNonEmpty`
                // (`vlm_fallback`-synthesised pipelines) prefers the deepest non-empty
                // fallback instead: stages run in priority order (primary first), so a
                // later non-empty result was invoked precisely because the higher-priority
                // stages were inadequate, and a correctness-blind score-max heuristic can
                // otherwise pin selection to an inadequate primary (e.g. merged-word
                // tesseract text scoring above a correct VLM transcription), discarding the
                // very fallback the pipeline ran (#1341). An empty fallback never
                // overwrites, so the earlier text is still kept in that case -- nor does a
                // non-empty one that is a materially worse replacement for an already-dense
                // incumbent (F46).
                let best_score = best_result.as_ref().map(|(_, best_score, ..)| *best_score);
                let best_text = best_result.as_ref().map(|(text, ..)| text.as_str());
                if should_replace_best_effort_result(
                    selection,
                    best_score,
                    best_text,
                    &text,
                    score,
                    &pipeline.quality_thresholds,
                ) {
                    best_result = Some((
                        text,
                        score,
                        stage_tables,
                        stage_ocr_elements,
                        stage_doc,
                        stage_page_texts,
                        stage_rasters,
                        stage_formulas,
                        stage_raw_paragraphs,
                        stage_preprocessing,
                        stage_ocr_confidence,
                        stage_recognition_noise_verdicts,
                        selected_read_evidence,
                    ));
                }
            }
            Err(failed_stage) => {
                if sparse_trial
                    || matches!(failed_stage.error, crate::XbergError::Cancelled)
                    || crate::extraction::derive::is_invalid_public_ocr_document(&failed_stage.error)
                {
                    return Err(failed_stage);
                }
                #[cfg(feature = "pdf")]
                if is_preferred_recovery_stage {
                    preferred_recovery_budget_refused = render_refused_by_source_image_budget(&failed_stage.error);
                }
                tracing::warn!(
                    backend = %stage.backend,
                    error = %failed_stage.error.pdf_ocr_trace_category(),
                    "Pipeline: backend failed, trying next"
                );
                stage_failures.push((stage.backend.clone(), failed_stage.error));
                unrecovered_glyphs_on_page_one =
                    unrecovered_glyphs_on_page_one.max(failed_stage.unrecovered_glyphs_on_page_one);
                merge_ocr_page_failures(stage_page_failures, failed_stage.page_failures);
            }
        }
    }

    match best_result {
        Some((
            text,
            score,
            tables,
            elements,
            doc,
            page_texts,
            rasters,
            formulas,
            raw_page_paragraphs,
            preprocessing,
            ocr_confidence,
            noise_verdicts,
            read_evidence,
        )) => {
            let (text, page_texts, doc) = with_run_layers_first(
                run_layers,
                RunLayerSources {
                    verdicts: &noise_verdicts,
                    page_index_offset: inputs.page_index_offset,
                    content: inputs.content,
                    raw_page_paragraphs: &raw_page_paragraphs,
                },
                text,
                page_texts,
                doc,
                config,
            );
            super::super::ensure_pdf_not_cancelled(config)?;
            let threshold = pipeline.quality_thresholds.pipeline_min_quality;
            tracing::warn!(
                score,
                threshold,
                selection = ?selection,
                "All OCR pipeline backends produced suboptimal quality, using best-effort result \
                 selected per the pipeline's selection policy"
            );
            let mut doc = doc.unwrap_or_else(|| {
                let mut d = crate::types::internal::InternalDocument::new("pdf");
                // Backend text verbatim (see `flat_ocr_page_document`). This best-effort arm
                // is where `PreferLastNonEmpty` lands VLM output, the most likely CR source.
                let text = crate::extraction::transform::normalize_line_endings(&text);
                for paragraph in text.split("\n\n") {
                    let trimmed = paragraph.trim();
                    if !trimmed.is_empty() {
                        d.push_element(crate::types::internal::InternalElement::text(
                            crate::types::internal::ElementKind::Paragraph,
                            trimmed,
                            0,
                        ));
                    }
                }
                d
            });
            doc.processing_warnings.push(crate::types::ProcessingWarning {
                source: std::borrow::Cow::Borrowed("ocr_pipeline"),
                message: std::borrow::Cow::Owned(format!(
                    "All OCR pipeline backends scored below the configured quality threshold \
                     (best score {score:.3} < {threshold:.3}); returning the best-effort result \
                     chosen by the pipeline's {:?} selection policy, which may be inaccurate or \
                     incomplete.",
                    selection
                )),
            });
            let doc = attach_ocr_pipeline_stage_warnings(Some(doc), &text, &unavailable_backends, &stage_failures);
            Ok((
                text,
                tables,
                elements,
                doc,
                accumulated_usage,
                page_texts,
                rasters,
                formulas,
                raw_page_paragraphs,
                preprocessing,
                ocr_confidence,
                noise_verdicts,
                read_evidence,
            ))
        }
        None => {
            // #1444: every stage errored, so no stage ever reached its own per-page
            // image-XObject fallback. Try the pages' embedded images once here before
            // giving up -- for a scanned PDF whose rasterizer output is blank, those
            // images are the only place the text ever was.
            #[cfg(feature = "pdf")]
            let xobject_recovery = if let Some(recovery_ocr_config) = preferred_recovery_config.as_ref() {
                if inputs.content.is_some() {
                    Box::pin(recover_pipeline_document_from_image_xobjects(
                        inputs.content,
                        config,
                        recovery_ocr_config,
                        &recovery_ocr_config.backend,
                    ))
                    .await?
                } else if let Some(source) = inputs.detached_page {
                    Box::pin(recover_pipeline_page_from_image_xobjects(
                        source.document,
                        inputs.page_index_offset,
                        config,
                        recovery_ocr_config,
                        &recovery_ocr_config.backend,
                    ))
                    .await?
                } else {
                    None
                }
            } else {
                None
            };
            #[cfg(feature = "pdf")]
            if let Some(mut recovery) = xobject_recovery {
                accumulated_usage.append(&mut recovery.llm_usage);
                let doc = if preferred_recovery_budget_refused && inputs.content.is_some() {
                    plain_budget_recovery_list_document(&recovery, config)
                } else {
                    None
                };
                let doc =
                    attach_ocr_pipeline_stage_warnings(doc, &recovery.text, &unavailable_backends, &stage_failures);
                let mut doc = attach_ocr_fallback_warnings(doc, &recovery.text, recovery.warnings);
                if let Some(doc) = doc.as_mut() {
                    merge_ocr_page_failures(&mut doc.ocr_page_failures, std::mem::take(&mut recovery.page_failures));
                }
                let capture_rasters = config.images.as_ref().is_some_and(|c| c.include_page_rasters);
                return Ok((
                    recovery.text,
                    recovery.tables,
                    Vec::new(),
                    doc,
                    accumulated_usage,
                    recovery.page_texts,
                    if capture_rasters { Some(recovery.images) } else { None },
                    recovery.formulas,
                    Vec::new(),
                    recovery.preprocessing,
                    ahash::AHashMap::new(),
                    Vec::new(),
                    OcrReadEvidence::default(),
                ));
            }

            let detail = if stage_failures.is_empty() {
                String::new()
            } else {
                let causes = stage_failures
                    .iter()
                    .map(|(backend, error)| format!("{backend}: {error}"))
                    .collect::<Vec<_>>()
                    .join("; ");
                format!(" ({causes})")
            };
            const HEADLINE: &str = "All OCR pipeline backends failed";
            Err(FailedOcrRun {
                error: crate::XbergError::ocr_pipeline_failure(
                    format!("{HEADLINE}{detail}"),
                    HEADLINE,
                    stage_failures.into_iter().next().map(|(_, error)| error),
                ),
                page_failures: Vec::new(),
                unrecovered_glyphs_on_page_one,
            })
        }
    }
}
#[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
fn recover_automatic_latin_rows(
    pdf: &xberg_native_pdf::PdfDocument,
    page: (usize, u32),
    read: &mut crate::types::ExtractedDocument,
    decision_layer: Option<&str>,
    config: &ExtractionConfig,
    thresholds: &crate::core::config::OcrQualityThresholds,
) {
    let (page_index, page_number) = page;
    if config.ocr_strategy != crate::core::config::OcrStrategy::Auto
        || !super::document::automatic_tesseract_page(config, page_number)
        || super::document::resolved_ocr_correction_degrees(&read.metadata).is_some_and(|angle| angle != 0)
        || config
            .pdf_options
            .as_ref()
            .is_some_and(|pdf| pdf.has_nonzero_raw_margins())
    {
        return;
    }
    super::layer_token_recovery::recover_latin_rows(
        pdf,
        page_index,
        read,
        decision_layer,
        &super::native_word_coverage::InkProofLimits {
            thresholds,
            max_working_bytes: super::hidden_layer::preview_byte_limit(config),
            cancelled: &|| config.cancel_token.as_ref().is_some_and(|token| token.is_cancelled()),
        },
    );
}

/// Clone an OCR config with word-level elements forced on for structure consumers.
///
/// Table recognition requires word geometry while semantic paragraph assembly
/// consumes the backend's line-only internal document even without ML layout.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(super) fn ensure_elements_enabled(
    config: &crate::core::config::ocr::OcrConfig,
) -> crate::core::config::ocr::OcrConfig {
    let mut config = config.clone();
    match config.element_config.as_mut() {
        Some(ec) => {
            ec.include_elements = true;
            ec.min_level = crate::types::OcrElementLevel::Word;
        }
        None => {
            config.element_config = Some(crate::types::OcrElementConfig {
                include_elements: true,
                min_level: crate::types::OcrElementLevel::Word,
                ..Default::default()
            });
        }
    }
    config
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn ocr_config_for_page_extraction(config: &crate::core::config::ocr::OcrConfig) -> crate::core::config::ocr::OcrConfig {
    #[cfg(target_arch = "wasm32")]
    return config.clone();

    #[cfg(not(target_arch = "wasm32"))]
    if crate::extractors::image::is_mixed_ltr_recovery_eligible_config(config) {
        ensure_elements_enabled(config)
    } else {
        config.clone()
    }
}

#[cfg(all(test, not(target_arch = "wasm32"), any(feature = "ocr", feature = "ocr-pipeline")))]
mod mixed_ltr_recovery_config_tests {
    use super::ocr_config_for_page_extraction;

    #[test]
    fn should_capture_pdf_word_elements_only_for_recovery_eligible_mixed_rtl_config() {
        let ordinary = crate::core::config::ocr::OcrConfig::default();
        let mixed_rtl = crate::core::config::ocr::OcrConfig {
            language: vec!["ara".to_string(), "eng".to_string()],
            ..Default::default()
        };
        let mut explicit_psm = mixed_rtl.clone();
        explicit_psm.tesseract_config.get_or_insert_default().psm = Some(6);

        assert!(ocr_config_for_page_extraction(&ordinary).element_config.is_none());
        assert!(
            ocr_config_for_page_extraction(&mixed_rtl)
                .element_config
                .is_some_and(|elements| elements.include_elements)
        );
        assert!(ocr_config_for_page_extraction(&explicit_psm).element_config.is_none());
    }
}
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(super) fn filter_public_ocr_elements(
    elements: &[crate::types::OcrElement],
    config: &crate::core::config::ocr::OcrConfig,
) -> Vec<crate::types::OcrElement> {
    let Some(element_config) = config.element_config.as_ref() else {
        return Vec::new();
    };
    element_config.select_elements(elements)
}
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(super) fn discard_ocr_elements_from_rejected_pages(
    elements: &mut Vec<crate::types::OcrElement>,
    rejected_pages: &[bool],
    page_index_offset: usize,
) {
    elements.retain(|element| !ocr_page_is_rejected(element.page_number, rejected_pages, page_index_offset));
}
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(super) fn ocr_page_is_rejected(page_number: u32, rejected_pages: &[bool], page_index_offset: usize) -> bool {
    page_number
        .checked_sub(1)
        .and_then(|index| usize::try_from(index).ok())
        .and_then(|index| index.checked_sub(page_index_offset))
        .and_then(|local_index| rejected_pages.get(local_index))
        .copied()
        .unwrap_or(false)
}
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(super) fn discard_rejected_ocr_page_payloads(
    tables: &mut Vec<crate::types::Table>,
    formulas: &mut Vec<crate::types::Formula>,
    rejected_pages: &[bool],
    page_index_offset: usize,
) {
    tables.retain(|table| !ocr_page_is_rejected(table.page_number, rejected_pages, page_index_offset));
    formulas.retain(|formula| {
        formula
            .page
            .is_none_or(|page_number| !ocr_page_is_rejected(page_number, rejected_pages, page_index_offset))
    });
}
/// Default font size for a synthetic fallback paragraph: matches
/// `pdf::structure::adapters::DEFAULT_OCR_FONT_SIZE_PT`, the value the rest of the OCR paragraph
/// pipeline uses when no real font-size evidence is available.
#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
const FALLBACK_TABLE_TEXT_FONT_SIZE_PT: f32 = 12.0;

/// Append reading-order text from `Table` regions TATR could not structurally reconstruct (an
/// invalid or degenerate cell grid, see [`crate::ocr::layout_assembly::TableRegionOutcome`]) as
/// ordinary paragraphs, so OCR text is never silently discarded when table recognition fails
/// (xberg-io/xberg#1622).
///
/// Skips any fallback text already present verbatim in one of the page's existing paragraphs:
/// `assemble_ocr_page_paragraphs`'s own `Table`-tagged paragraph is built from a separate OCR
/// representation (the hOCR-derived `InternalDocument`) and, in the common case, already carries
/// the same text as the `OcrElement` list this fallback is drawn from. The check guards only the
/// case where the two representations diverge, avoiding a duplicate paragraph in the ordinary
/// case. ~keep
#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
pub(super) fn append_unrecognized_table_fallback_paragraphs(
    paragraphs: &mut Vec<crate::pdf::structure::types::PdfParagraph>,
    fallback_texts: &[String],
) {
    if fallback_texts.is_empty() {
        return;
    }
    let existing_text = paragraphs
        .iter()
        .map(|paragraph| paragraph.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    for text in fallback_texts {
        let trimmed = text.trim();
        if trimmed.is_empty() || existing_text.contains(trimmed) {
            continue;
        }
        paragraphs.push(crate::pdf::structure::types::PdfParagraph {
            text: trimmed.to_string(),
            lines: Vec::new(),
            dominant_font_size: FALLBACK_TABLE_TEXT_FONT_SIZE_PT,
            heading_level: None,
            is_bold: false,
            is_list_item: false,
            is_code_block: false,
            is_formula: false,
            is_page_furniture: false,
            layout_class: None,
            layout_region_path: None,
            ocr_source_spans: Vec::new(),
            caption_for: None,
            block_bbox: None,
            word_count: crate::pdf::structure::types::PdfParagraph::compute_word_count(trimmed, &[]),
        });
    }
}
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(super) fn retain_ocr_formulas_for_accepted_pages(
    formulas: &mut Vec<crate::types::Formula>,
    accepted_pages: &ahash::AHashMap<u32, String>,
) {
    formulas.retain(|formula| {
        formula
            .page
            .is_none_or(|page_number| accepted_pages.contains_key(&page_number))
    });
}
/// Inject a page's PDF `/Rotate` value into `OcrConfig.backend_options` so a
/// backend that can use it (currently PaddleOCR, see
/// `paddle_ocr::backend::PaddleOcrBackend::reorder_blocks_for_page_rotation`) can
/// correct its detector's raster-space block order into true reading order
/// (#640). `normalize_rendered_page_for_ocr` deliberately hands every backend a
/// raster in the page's raw MediaBox orientation rather than its display
/// orientation (see the #530 regression test in `crate::pdf::render`), which
/// Tesseract's own layout analysis already reads correctly; PaddleOCR's detector
/// does not, so this hint is the minimal, backend-local fix rather than changing
/// the shared raster every backend (including Tesseract) receives.
///
/// Also injects `source_dpi`: the true resolution of the raster accompanying this call, which
/// only this route can know because it rendered the page. Without it the OCR preprocessor
/// assumes 72 DPI (`image::preprocessing::normalize_image_dpi_owned`), which is wrong for every
/// page here — they are rendered at 150, or lower still when `choose_safe_dpi` reduced an
/// oversized page — and that mistake both inflates the resize past the `max_image_dimension`
/// clamp and hands Tesseract a `scan_res` unrelated to the raster it is looking at.
///
/// Both hints are per page, not per document: the config is cloned for each page, so mixed page
/// sizes and per-page DPI reductions each report their own value.
///
/// When `whole_page_raster` is set (the page is one full-page image, a scan) and the caller
/// chose no Tesseract `psm`, the page gets the segmentation mode standalone image OCR uses
/// (`extractors::image::apply_default_whole_image_tesseract_psm`). The engine default,
/// automatic layout, loses table values on a scanned table page that the sparse-text mode
/// reads, and a scanned page OCR'd as an image already got that mode (#1786). A page that is
/// not a scan keeps the engine default, so forced OCR of a vector page is unchanged.
///
/// `whole_page_raster` is also stamped into `backend_options` as
/// [`KNOWN_FULL_PAGE_SCAN_BACKEND_OPTION`] when the backend is Tesseract, so
/// `config_to_tesseract` can make the page take Tesseract's default preprocessing
/// unconditionally rather than deferring to a pixel-brightness heuristic tuned for callers with
/// no scan-detection signal of their own (GH#1894). Gated the same way `apply_whole_image_psm`
/// is: only Tesseract's `config_to_tesseract` reads it, so another backend's config is not
/// cloned just to carry a hint nothing on that route consumes.
///
/// Layout assembly does not change page segmentation for a known scan. Layout regions are
/// applied after recognition, so changing the scan's PSM here can alter its words before those
/// regions are assembled. The scan and source-DPI hints remain intact. ~keep
///
/// A no-op when there is nothing to say (`page_rotation_degrees == 0`, no known DPI, and either
/// the page is not a whole-page scan or the backend is not Tesseract) so such pages never pay a
/// config clone. Backends that don't recognise a key ignore it, per
/// `OcrConfig.backend_options`'s documented contract.
///
/// [`KNOWN_FULL_PAGE_SCAN_BACKEND_OPTION`]: crate::core::config::ocr::KNOWN_FULL_PAGE_SCAN_BACKEND_OPTION
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) fn ocr_config_with_page_rotation_hint(
    config: &crate::core::config::ocr::OcrConfig,
    page_rotation_degrees: u32,
    source_dpi: Option<f64>,
    whole_page_raster: bool,
    prefer_single_block: bool,
    _layout_assembly: bool,
) -> Cow<'_, crate::core::config::ocr::OcrConfig> {
    let source_dpi = source_dpi.and_then(serde_json::Number::from_f64);
    let is_tesseract = config.backend == "tesseract";
    let is_tesseract_scan = whole_page_raster && is_tesseract;
    let apply_single_block_psm =
        prefer_single_block && is_tesseract && config.tesseract_config.as_ref().and_then(|c| c.psm).is_none();
    let apply_whole_image_psm =
        is_tesseract_scan && config.tesseract_config.as_ref().and_then(|config| config.psm).is_none();
    if page_rotation_degrees == 0 && source_dpi.is_none() && !is_tesseract_scan && !apply_single_block_psm {
        return Cow::Borrowed(config);
    }
    let mut config = config.clone();
    if apply_single_block_psm {
        let tesseract = config.tesseract_config.get_or_insert_with(Default::default);
        tesseract.psm = Some(6);
    } else if apply_whole_image_psm {
        crate::extractors::image::apply_default_whole_image_tesseract_psm(&mut config);
    }
    let mut opts = config.backend_options.take().unwrap_or_else(|| serde_json::json!({}));
    if !opts.is_object() {
        opts = serde_json::json!({});
    }
    if let Some(obj) = opts.as_object_mut() {
        if page_rotation_degrees != 0 {
            obj.insert(
                crate::core::config::ocr::PAGE_ROTATION_DEGREES_BACKEND_OPTION.to_string(),
                serde_json::Value::Number(page_rotation_degrees.into()),
            );
        }
        if let Some(source_dpi) = source_dpi {
            obj.insert(
                crate::core::config::ocr::SOURCE_DPI_BACKEND_OPTION.to_string(),
                serde_json::Value::Number(source_dpi),
            );
        }
        if is_tesseract_scan {
            obj.insert(
                crate::core::config::ocr::KNOWN_FULL_PAGE_SCAN_BACKEND_OPTION.to_string(),
                serde_json::Value::Bool(true),
            );
        }
    }
    config.backend_options = Some(opts);
    Cow::Owned(config)
}
/// Derive the [`ocr_config_with_page_rotation_hint`] `source_dpi` value for one rendered page.
///
/// `rendered_width_px` must be the width of the MediaBox-oriented raster as
/// `normalize_rendered_page_for_ocr` produced it, before any `upright_raster_for_backend`
/// rotation — see `crate::pdf::render::rendered_page_dpi`.
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) fn rendered_page_source_dpi(
    doc: &xberg_native_pdf::PdfDocument,
    page_index: usize,
    rendered_width_px: u32,
) -> Option<f64> {
    let (page_width_pt, _) = page_dimensions_pt(doc, page_index);
    crate::pdf::render::rendered_page_dpi(rendered_width_px, page_width_pt)
}
/// Rotate a page raster upright before handing it to a backend that cannot cope with a
/// sideways page (`PageOrientationHandling::RequiresUpright`), returning the bytes/dimensions
/// actually sent to the backend and the rotation (in `image`-crate terms: one of `0`, `90`,
/// `180`, `270`) that must later be undone on any pixel-space geometry the backend returns.
///
/// `normalize_rendered_page_for_ocr` deliberately hands every backend a raster in the page's
/// raw MediaBox orientation rather than its display orientation (#530), which PaddleOCR
/// (`RecognisesRotatedText`) recognises correctly
/// (only its block order needs fixing, see `ocr_config_with_page_rotation_hint`). A backend
/// that declares `RequiresUpright` (sceptre, and Tesseract since #2172) produces character garbage on that same
/// sideways raster instead — measured this session on `/Rotate 270` scanned pages, and
/// confirmed by feeding the same page through an upright render. This is the backend-local
/// fix: re-apply the page's original `/Rotate` value on top of the already-corrected raster,
/// undoing exactly what `normalize_rendered_page_for_ocr` undid, so only that backend's input
/// changes (#643).
///
/// A no-op — returns `data` unchanged and a correction of `0` — when `page_rotation_degrees ==
/// 0` or `orientation_handling` is not `RequiresUpright`. In particular, `SelfCorrecting` and
/// `RecognisesRotatedText` backends never pay a re-encode and never see a different raster than
/// they do today.
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) fn upright_raster_for_backend(
    data: &std::sync::Arc<Vec<u8>>,
    width: u32,
    height: u32,
    page_rotation_degrees: u32,
    orientation_handling: crate::plugins::PageOrientationHandling,
    security_limits: Option<&crate::extractors::security::SecurityLimits>,
) -> crate::Result<(std::sync::Arc<Vec<u8>>, u32, u32, u32)> {
    if page_rotation_degrees == 0 || orientation_handling != crate::plugins::PageOrientationHandling::RequiresUpright {
        return Ok((std::sync::Arc::clone(data), width, height, 0));
    }
    let correction_degrees = page_rotation_degrees % 360;
    let default_security_limits = crate::extractors::security::SecurityLimits::default();
    let security_limits = security_limits.unwrap_or(&default_security_limits);
    let (rotated, new_width, new_height) = crate::pdf::render::rotate_png_page_if_needed_with_security_limits(
        (**data).clone(),
        width,
        height,
        correction_degrees,
        security_limits,
    )?;
    Ok((std::sync::Arc::new(rotated), new_width, new_height, correction_degrees))
}
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), not(target_arch = "wasm32")))]
fn retained_ocr_source_bytes(
    original: &std::sync::Arc<Vec<u8>>,
    upright: &std::sync::Arc<Vec<u8>>,
    retained_pixels: usize,
) -> usize {
    // The backend already charges the upright PNG. Only a distinct original allocation is extra. ~keep
    if std::sync::Arc::ptr_eq(original, upright) {
        retained_pixels
    } else {
        retained_pixels.saturating_add(original.capacity())
    }
}

/// Undo [`upright_raster_for_backend`]'s rotation on a backend result's pixel-space geometry,
/// mapping bboxes from the upright raster the backend actually ran OCR on back to the raw
/// MediaBox raster every downstream consumer (`build_mixed_ocr_page_document`,
/// `rescale_ocr_bboxes_to_page_points`, the #530 regression test) expects.
///
/// The backend's own rotation is inverted first, in its processed frame. The upright inverse then
/// uses that restored frame; neither operation changes the original preprocessing/orientation record.
/// With no rotation at either stage this is a no-op. Reuses [`undo_auto_rotate_point`] — the same inverse
/// [`undo_auto_rotate_document_bboxes`] uses for a backend's own internal auto-rotation — since
/// the geometry problem is identical: a point in a rotated raster mapped back to the unrotated
/// one it will be rescaled against.
///
/// `ocr_internal_document` element bboxes, `tables` bboxes, `formulas` bboxes, and
/// `ocr_elements` word/line/block geometry are corrected: those are the pixel-space
/// geometry sources read from a backend result before rescaling into page points.
/// `formulas` matters for GLM paired mode, which pushes the SAME `region_bbox` into
/// both `formulas[].bbox` and `table_bboxes` (`candle_ocr/glm_ocr_backend.rs`); correcting only
/// the table half left `formula_bbox_to_page_points` rescaling an upright-raster box against
/// MediaBox-raster page dimensions on any `/Rotate != 0` page. `ocr_elements` matters because
/// `attach_page_ocr_payload` copies it straight onto the assembled document's
/// `prebuilt_ocr_elements` with no further pixel-space transform of its own (unlike
/// `ocr_internal_document`, which `rescale_ocr_bboxes_to_page_points` still rescales downstream)
/// — leaving it uncorrected here means every word/line box a `RequiresUpright` backend reports
/// stays in the upright raster's frame forever on any `/Rotate != 0` page.
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) fn undo_upright_raster_correction(
    result: &mut crate::types::ExtractedDocument,
    correction_degrees: u32,
    upright_width: u32,
    upright_height: u32,
) {
    undo_backend_raster_correction(result, upright_width, upright_height);
    if correction_degrees == 0 {
        return;
    }
    let dimensions =
        super::document::processed_ocr_layout_dimensions(&result.metadata).unwrap_or((upright_width, upright_height));
    undo_result_rotation(result, correction_degrees as u16, dimensions);
}

#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) fn undo_backend_raster_correction(
    result: &mut crate::types::ExtractedDocument,
    input_width: u32,
    input_height: u32,
) {
    let Some(correction) = super::document::resolved_ocr_correction_degrees(&result.metadata) else {
        return;
    };
    if correction == 0 {
        return;
    }
    let rotated_input = if matches!(correction, 90 | 270) {
        (input_height, input_width)
    } else {
        (input_width, input_height)
    };
    let dimensions = super::document::processed_ocr_layout_dimensions(&result.metadata).unwrap_or(rotated_input);
    undo_result_rotation(result, correction, dimensions);
    result.metadata.additional.insert(
        crate::ocr_metadata_keys::OCR_BACKEND_ROTATION_UNDONE_METADATA_KEY.into(),
        true.into(),
    );
}

#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
fn undo_result_rotation(result: &mut crate::types::ExtractedDocument, correction: u16, dimensions: (u32, u32)) {
    rotate_processed_ocr_frame(result, correction, dimensions);
    let undo_point = |x, y| undo_auto_rotate_point(x, y, correction, f64::from(dimensions.0), f64::from(dimensions.1));
    let undo_bbox = |bbox: &mut crate::types::extraction::BoundingBox| {
        let (x0, y0) = undo_point(bbox.x0, bbox.y0);
        let (x1, y1) = undo_point(bbox.x1, bbox.y1);
        bbox.x0 = x0.min(x1);
        bbox.x1 = x0.max(x1);
        bbox.y0 = y0.min(y1);
        bbox.y1 = y0.max(y1);
    };
    if let Some(doc) = result.ocr_internal_document.as_mut() {
        let raw_resize = result.raw_ocr_source_frame.is_some();
        for element in &mut doc.elements {
            if let Some(bbox) = element.bbox.as_mut() {
                undo_bbox(bbox);
            }
            if raw_resize && let Some(geometry) = element.ocr_geometry.as_mut() {
                undo_ocr_element_geometry(geometry, undo_point);
            }
        }
        if raw_resize {
            crate::types::internal::table_grid::transform_retained_grid_bboxes(doc, &result.tables, undo_bbox);
            for bbox in doc.tables.iter_mut().filter_map(|table| table.bounding_box.as_mut()) {
                undo_bbox(bbox);
            }
        }
    }
    for table in &mut result.tables {
        if let Some(bbox) = table.bounding_box.as_mut() {
            undo_bbox(bbox);
        }
    }
    for formula in &mut result.formulas {
        if let Some(bbox) = formula.bbox.as_mut() {
            undo_bbox(bbox);
        }
    }
    if let Some(elements) = result.ocr_elements.as_mut() {
        for element in elements {
            undo_ocr_element_geometry(&mut element.geometry, undo_point);
        }
    }
}
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
fn rotate_processed_ocr_frame(result: &mut crate::types::ExtractedDocument, correction: u16, dimensions: (u32, u32)) {
    if matches!(correction, 90 | 270) && super::document::processed_ocr_layout_dimensions(&result.metadata).is_some() {
        result.metadata.additional.insert(
            crate::ocr_metadata_keys::OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY.into(),
            dimensions.1.into(),
        );
        result.metadata.additional.insert(
            crate::ocr_metadata_keys::OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY.into(),
            dimensions.0.into(),
        );
    }
}

/// Undo an upright-raster (or auto-rotate) correction on one `OcrElement`'s geometry, the
/// word/line/block-level boxes reported by [`crate::types::OcrElement`]. Missed entirely by
/// [`undo_upright_raster_correction`] before this fix (#657 follow-up): that function only
/// walked `ocr_internal_document`, `tables`, and `formulas`, leaving every `ocr_elements` entry
/// in the upright raster's pixel frame.
///
/// `Rectangle` is corrected corner-to-corner like a [`crate::types::extraction::BoundingBox`]
/// (top-left and bottom-right corners, then re-derived into `left`/`top`/`width`/`height` from
/// the transformed corners' min/max, since a quarter turn can swap which corner is which).
/// `Quadrilateral` is corrected point-by-point — a quarter turn does not preserve the
/// "clockwise from top-left" point order the type promises, but no downstream reader currently
/// depends on point order, only on the region the four points enclose.
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) fn undo_ocr_element_geometry(
    geometry: &mut crate::types::ocr_elements::OcrBoundingGeometry,
    undo_point: impl Fn(f64, f64) -> (f64, f64),
) {
    use crate::types::ocr_elements::OcrBoundingGeometry;
    match geometry {
        OcrBoundingGeometry::Rectangle {
            left,
            top,
            width,
            height,
        } => {
            let (x0, y0) = (f64::from(*left), f64::from(*top));
            let (x1, y1) = (x0 + f64::from(*width), y0 + f64::from(*height));
            let (nx0, ny0) = undo_point(x0, y0);
            let (nx1, ny1) = undo_point(x1, y1);
            let min_x = nx0.min(nx1);
            let min_y = ny0.min(ny1);
            let max_x = nx0.max(nx1);
            let max_y = ny0.max(ny1);
            *left = min_x.round().max(0.0) as u32;
            *top = min_y.round().max(0.0) as u32;
            *width = (max_x - min_x).round().max(0.0) as u32;
            *height = (max_y - min_y).round().max(0.0) as u32;
        }
        OcrBoundingGeometry::Quadrilateral { points } => {
            for point in points.iter_mut() {
                let (x, y) = undo_point(f64::from(point.x), f64::from(point.y));
                point.x = x.round().max(0.0) as u32;
                point.y = y.round().max(0.0) as u32;
            }
        }
    }
}
/// Inject layout-detection settings into OcrConfig backend options for paired-mode backends.
///
/// When layout detection is active and provides detections, certain backends (e.g., GLM-OCR)
/// may need configuration injected from the layout-detection config. This function ensures
/// that the `enable_chart_understanding` flag from `ExtractionConfig.layout` is propagated
/// to the OCR backend via `backend_options` so per-region task dispatch can honor it.
#[cfg(all(feature = "ocr", feature = "layout-detection"))]
pub(super) fn inject_layout_config_to_backend(
    config: &crate::core::config::ocr::OcrConfig,
    extraction_config: &ExtractionConfig,
) -> crate::core::config::ocr::OcrConfig {
    let mut config = config.clone();
    if let Some(layout_cfg) = &extraction_config.layout {
        let mut opts = config.backend_options.take().unwrap_or_else(|| serde_json::json!({}));

        if !opts.is_object() {
            if !opts.is_null() {
                tracing::warn!(
                    backend_options = %opts,
                    "backend_options was not a JSON object; replacing with new object to inject enable_chart_understanding"
                );
            }
            opts = serde_json::json!({});
        }

        if let Some(obj) = opts.as_object_mut() {
            obj.insert(
                "enable_chart_understanding".to_string(),
                serde_json::Value::Bool(layout_cfg.enable_chart_understanding),
            );
        }

        config.backend_options = Some(opts);
    }
    config
}

#[cfg(all(
    test,
    feature = "pdf",
    feature = "ocr",
    feature = "tokio-runtime",
    not(target_arch = "wasm32")
))]
#[path = "tracing_tests.rs"]
mod tracing_tests;

#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
#[derive(Clone, Copy)]
enum RecognizedTableFrame {
    ImagePixels,
    DetachedPdfRaster { height: u32 },
    PdfPage { raster: (u32, u32), points: (f32, f32) },
}

#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
impl RecognizedTableFrame {
    fn place(self, table: &mut crate::types::Table, grid: Option<&mut crate::types::TableGrid>) {
        let (scale, height, place_table) = match self {
            Self::ImagePixels => return,
            Self::DetachedPdfRaster { height } => ((1.0, 1.0), f64::from(height), false),
            Self::PdfPage { raster, points } if raster.0 != 0 && raster.1 != 0 => (
                (
                    f64::from(points.0) / f64::from(raster.0),
                    f64::from(points.1) / f64::from(raster.1),
                ),
                f64::from(points.1),
                true,
            ),
            Self::PdfPage { .. } => return,
        };
        let place = |bbox: &mut crate::types::BoundingBox| {
            let top = bbox.y0;
            bbox.x0 *= scale.0;
            bbox.x1 *= scale.0;
            bbox.y0 = height - bbox.y1 * scale.1;
            bbox.y1 = height - top * scale.1;
        };
        if place_table && let Some(bbox) = table.bounding_box.as_mut() {
            place(bbox);
        }
        // Detached page assembly expects table rectangles in top-left raster pixels, but its
        // retained cell boxes already share the bottom-left raster frame of document elements.
        // The page owner later scales those cells and separately scale-flips tables exactly once. ~keep
        if let Some(grid) = grid {
            for bbox in grid.cells.iter_mut().filter_map(|cell| cell.bbox.as_mut()) {
                place(bbox);
            }
        }
    }
}

#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
fn append_recognized_tables(
    outcome: &crate::ocr::layout_assembly::RecognizedTablesOutcome,
    page_number: u32,
    frame: RecognizedTableFrame,
    tables: &mut Vec<crate::types::Table>,
    grids: &mut Vec<crate::types::internal::table_grid::TableGridSidecar>,
) {
    for (index, recognized) in outcome.tables.iter().enumerate() {
        if recognized.markdown.is_empty() {
            continue;
        }
        let mut sidecar = outcome.sidecar_at(index, page_number);
        let mut table = sidecar
            .as_ref()
            .map(|sidecar| sidecar.table.clone())
            .unwrap_or_else(|| recognized_table_to_public_table(recognized, page_number, tables.len()));
        table.table_id = Some(format!("table-{}", tables.len() + 1));
        frame.place(&mut table, sidecar.as_mut().map(|sidecar| &mut sidecar.grid));
        if let Some(mut sidecar) = sidecar {
            sidecar.table = table.clone();
            grids.push(sidecar);
        }
        tables.push(table);
    }
}

#[cfg(all(test, feature = "layout-detection", feature = "ocr"))]
#[path = "pipeline/recognized_table_grid_tests.rs"]
mod recognized_table_grid_tests;

#[cfg(all(
    test,
    feature = "pdf",
    feature = "tokio-runtime",
    any(feature = "ocr", feature = "ocr-pipeline")
))]
#[path = "memory_recovery_tests.rs"]
mod memory_recovery_tests;

#[cfg(all(test, feature = "pdf", feature = "tokio-runtime", feature = "ocr"))]
#[path = "xobject_budget_parity_tests.rs"]
mod xobject_budget_parity_tests;

#[cfg(all(test, feature = "pdf", feature = "tokio-runtime", feature = "ocr"))]
#[path = "english_recovery_tests.rs"]
mod english_recovery_tests;

#[cfg(all(test, feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
#[path = "pipeline/retained_raster_budget_tests.rs"]
mod retained_raster_budget_tests;

#[cfg(all(test, feature = "pdf", feature = "layout-detection", feature = "ocr"))]
#[path = "pipeline/upright_processed_geometry_tests.rs"]
mod upright_processed_geometry_tests;

#[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
#[path = "pipeline/detached_table_pair.rs"]
mod detached_table_pair;

#[cfg(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
#[path = "pipeline/detached_mixed_frame.rs"]
mod detached_mixed_frame;

#[cfg(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
#[path = "pipeline/raw_workspace_geometry.rs"]
mod raw_workspace_geometry;

#[cfg(all(test, feature = "pdf", feature = "layout-detection", feature = "ocr"))]
#[path = "pipeline/raster_budget_tests.rs"]
mod raster_budget_tests;

#[cfg(all(
    test,
    paddle_ocr,
    feature = "pdf",
    feature = "ocr",
    feature = "tokio-runtime",
    feature = "bundle-tessdata-eng",
    not(target_arch = "wasm32")
))]
#[path = "pipeline/first_pass_route_tests.rs"]
mod first_pass_route_tests;

#[cfg(all(test, feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
#[path = "pipeline/native_script_kept_tests.rs"]
mod native_script_kept_tests;

#[cfg(all(test, feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
#[path = "pipeline/second_site_clean_admission_tests.rs"]
mod second_site_clean_admission_tests;

#[cfg(all(test, feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
#[path = "pipeline/budget_refusal_tests.rs"]
mod budget_refusal_tests;

#[cfg(all(test, feature = "pdf", feature = "layout-detection", feature = "ocr"))]
#[path = "pipeline/layer_lost_glyph_tests.rs"]
mod layer_lost_glyph_tests;

#[cfg(all(
    test,
    feature = "pdf",
    feature = "tokio-runtime",
    any(feature = "ocr", feature = "ocr-pipeline")
))]
#[path = "xobject_raw_collection_test_support.rs"]
mod raw_collection_test_support;

#[cfg(all(test, feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
#[path = "pipeline/geometry_session_tests.rs"]
mod geometry_session_tests;

#[cfg(all(paddle_ocr, feature = "ocr", feature = "pdf", not(target_arch = "wasm32")))]
#[path = "pipeline/stored_scan_paint.rs"]
mod stored_scan_paint;

#[cfg(all(paddle_ocr, feature = "ocr", feature = "pdf", not(target_arch = "wasm32")))]
#[path = "pipeline/stored_scan_retry.rs"]
mod stored_scan_retry;

/// A configured preprocessing block requests its target density, including the default 300.
/// Only native renders consume this request; supplied rasters retain their original source. ~keep
#[cfg(all(feature = "ocr", feature = "pdf", not(target_arch = "wasm32")))]
fn configured_preprocessing_render_dpi(
    config: &ExtractionConfig,
    ocr: &crate::core::config::OcrConfig,
    builtin_tesseract: bool,
) -> Option<i32> {
    if !config.force_ocr || config.images.is_some() || !builtin_tesseract || !uses_default_ocr_pipeline(ocr) {
        return None;
    }
    ocr.tesseract_config
        .as_ref()?
        .preprocessing
        .as_ref()
        .map(|preprocessing| preprocessing.target_dpi)
        .filter(|target| *target > crate::image::dpi::DEFAULT_PDF_RENDER_DPI)
}

/// Rendering may now hold more pixels before normalization. The page security cap bounds
/// that raster/encode peak, so reserve at least that cap rather than the old fallback alone. ~keep
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn preprocessing_render_reservation(target: Option<i32>, page_cap: usize) -> usize {
    if target.is_some() {
        UNMEASURED_OCR_PAGE_WORKING_SET_BYTES.max(page_cap)
    } else {
        UNMEASURED_OCR_PAGE_WORKING_SET_BYTES
    }
}

/// The mixed route promotes native page content to the configured density only when nothing
/// keeps the source render: active layout detection and a hidden-text-layer probe each run
/// their own render policy. ~keep
#[cfg(all(feature = "ocr", feature = "pdf", not(target_arch = "wasm32")))]
fn mixed_route_promotes_preprocessing_dpi(mixed_route_layout_active: bool, probing_hidden_layer: bool) -> bool {
    !mixed_route_layout_active && !probing_hidden_layer
}

/// Every condition that decides whether a marker-bearing ink-proof layer may be offered at all,
/// stated once where the layers are built (#2900). Each consumer re-gates the routes it reaches,
/// so removing one conjunct alone changes no output; the per-condition unit tests below pin the
/// predicate itself, turning red when a conjunct leaves this function. ~keep
#[cfg(all(feature = "ocr", feature = "pdf", not(target_arch = "wasm32")))]
fn marker_layers_allowed(
    all_pages_failed_policy: AllPagesFailedPolicy,
    probing_hidden_layer: bool,
    mixed_route_layout_active: bool,
    ocr_strategy: &crate::core::config::OcrStrategy,
    ocr: Option<&crate::core::config::OcrConfig>,
) -> bool {
    all_pages_failed_policy == AllPagesFailedPolicy::PreserveNative
        && !probing_hidden_layer
        && !mixed_route_layout_active
        && *ocr_strategy == crate::core::config::OcrStrategy::Auto
        && ocr.is_some_and(uses_default_ocr_pipeline)
}

/// A promoted mixed-route batch sizes itself from the page security cap instead of the bare
/// fallback, so the wider batch cannot outrun the memory the promotion assumes. ~keep
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
fn mixed_route_batch_working_set(
    measured_page_bytes: usize,
    preprocessing_render_dpi: Option<i32>,
    page_cap: usize,
) -> usize {
    measured_page_bytes
        .max(preprocessing_render_dpi.map_or(0, |target| preprocessing_render_reservation(Some(target), page_cap)))
}

#[cfg(all(test, feature = "ocr", feature = "pdf", not(target_arch = "wasm32")))]
#[path = "pipeline/preprocessing_density_tests.rs"]
mod preprocessing_density_tests;

#[cfg(all(test, feature = "ocr", feature = "pdf", not(target_arch = "wasm32")))]
#[path = "pipeline/marker_layers_allowed_tests.rs"]
mod marker_layers_allowed_tests;

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(in crate::extractors::pdf) fn uses_default_ocr_pipeline(ocr: &crate::core::config::OcrConfig) -> bool {
    ocr.pipeline.is_none() && ocr.vlm_fallback == crate::core::config::VlmFallbackPolicy::Disabled
}

#[cfg(all(feature = "ocr", feature = "pdf", not(target_arch = "wasm32")))]
fn registered_preprocessing_render_dpi(config: &ExtractionConfig, ocr: &crate::core::config::OcrConfig) -> Option<i32> {
    let target = configured_preprocessing_render_dpi(config, ocr, true)?;
    // ~keep Planning precedes stage dispatch, which normally re-seeds a cleared registry.
    crate::plugins::ensure_ocr_backends_initialized();
    let registry = crate::plugins::registry::get_ocr_backend_registry();
    let registry = registry.read();
    let backend = registry.get(&ocr.backend).ok()?;
    registry.is_builtin_tesseract(&backend).then_some(target)
}

#[cfg(all(test, any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
#[path = "pipeline/time_limit_run_error_tests.rs"]
mod time_limit_run_error_tests;

#[cfg(all(
    test,
    any(feature = "ocr", feature = "ocr-pipeline"),
    feature = "pdf",
    feature = "tokio-runtime",
    not(target_arch = "wasm32")
))]
mod in_flight_page_tests {
    use super::*;
    use crate::cancellation::CancellationToken;
    use std::time::Duration;
    use tokio::task::JoinSet;

    #[tokio::test]
    async fn queued_ocr_error_keeps_pre_deadline_classification() {
        let token = CancellationToken::new();
        let worker_token = token.clone();
        let (completed, wait) = tokio::sync::oneshot::channel();
        let mut join_set = JoinSet::new();
        join_set.spawn(async move {
            let completion = capture_ocr_completion(
                Err::<(), _>(crate::XbergError::Ocr {
                    message: "original OCR error".to_string(),
                    source: None,
                }),
                Some(&worker_token),
            );
            completed.send(()).unwrap();
            completion
        });
        wait.await.unwrap();
        token.set_deadline(std::time::Instant::now());
        assert!(token.time_limit_reached());

        let (result, expired) = join_set.join_next().await.unwrap().unwrap();
        assert!(matches!(result, Err(crate::XbergError::Ocr { message, .. }) if message == "original OCR error"));
        assert!(expired.is_none());
    }

    /// A page task that takes `secs` of (paused, virtual) time.
    fn slow_pages(secs: &[u64]) -> JoinSet<usize> {
        let mut join_set = JoinSet::new();
        for (page, &secs) in secs.iter().enumerate() {
            join_set.spawn(async move {
                tokio::time::sleep(Duration::from_secs(secs)).await;
                page
            });
        }
        join_set
    }

    #[tokio::test(start_paused = true)]
    async fn cancellation_interrupts_the_wait_for_a_stuck_page_during_soft_stop() {
        let token = CancellationToken::new();
        token.request_stop();
        let mut join_set = JoinSet::new();
        join_set.spawn(std::future::pending::<()>());
        let started = tokio::time::Instant::now();
        let mut abandon_at = None;
        let caller = token.clone();
        tokio::spawn(async move {
            tokio::time::sleep(STOP_POLL_INTERVAL * 2).await;
            caller.cancel();
        });

        let outcome = next_page_task(&mut join_set, Some(&token), &mut abandon_at).await;

        assert!(token.is_cancelled());
        assert!(
            started.elapsed() <= STOP_POLL_INTERVAL * 3,
            "cancellation did not interrupt the page wait"
        );
        assert!(matches!(outcome, NextPageTask::Cancelled));
        assert!(join_set.join_next().await.unwrap().unwrap_err().is_cancelled());
    }

    #[tokio::test(start_paused = true)]
    async fn a_batch_without_a_stop_is_awaited_to_its_end() {
        let token = CancellationToken::new();
        let mut join_set = slow_pages(&[1, 60]);
        let mut abandon_at = None;

        let mut finished = Vec::new();
        loop {
            match next_page_task(&mut join_set, Some(&token), &mut abandon_at).await {
                NextPageTask::Finished(page) => finished.push(page.unwrap()),
                NextPageTask::Drained => break,
                NextPageTask::Cancelled => panic!("no cancellation was requested"),
                NextPageTask::Abandoned => panic!("no stop was requested"),
            }
        }
        assert_eq!(finished, [0, 1]);
    }

    #[tokio::test(start_paused = true)]
    async fn a_stop_abandons_the_pages_still_in_flight_after_the_grace_and_keeps_the_finished_ones() {
        let token = CancellationToken::new();
        let mut join_set = slow_pages(&[1, 600]);
        let mut abandon_at = None;
        let started = tokio::time::Instant::now();

        let first = next_page_task(&mut join_set, Some(&token), &mut abandon_at).await;
        assert!(
            matches!(first, NextPageTask::Finished(Ok(0))),
            "the quick page finishes first"
        );

        token.request_stop();
        let second = next_page_task(&mut join_set, Some(&token), &mut abandon_at).await;

        assert!(
            matches!(second, NextPageTask::Abandoned),
            "the slow page must be given up"
        );
        let waited = started.elapsed();
        assert!(
            waited < Duration::from_secs(1) + IN_FLIGHT_PAGE_GRACE + STOP_POLL_INTERVAL * 2,
            "the batch was awaited for {waited:?}, past the in-flight grace"
        );
        assert!(
            join_set.join_next().await.is_none() || join_set.is_empty(),
            "the slow task was aborted"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn a_page_that_finishes_inside_the_grace_after_a_stop_is_kept() {
        let token = CancellationToken::new();
        token.request_stop();
        let mut join_set = slow_pages(&[2]);
        let mut abandon_at = None;

        let outcome = next_page_task(&mut join_set, Some(&token), &mut abandon_at).await;

        assert!(matches!(outcome, NextPageTask::Finished(Ok(0))));
    }

    #[test]
    fn unread_pages_are_recorded_as_failures_with_the_time_limit_error() {
        let token = CancellationToken::new();
        let mut failures = Vec::new();
        let mut errors = ahash::AHashMap::new();

        mark_pages_unread(
            Some(&token),
            [4usize, 2].into_iter(),
            &mut failures,
            &mut errors,
            |page| page == 2,
        );

        assert_eq!(token.first_unread_page(), Some(3));
        assert_eq!(failures.len(), 2);
        assert!(
            failures
                .iter()
                .all(|failure| failure.error == TIME_LIMIT_UNREAD_PAGE_ERROR)
        );
        assert!(matches!(errors.get(&4), Some(crate::XbergError::Timeout { .. })));
        let recovered: Vec<_> = failures
            .iter()
            .map(|failure| (failure.page, failure.recovered))
            .collect();
        assert!(recovered.contains(&(3, true)) && recovered.contains(&(5, false)));
    }
}
#[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
fn orientation_retry_page_eligible(
    whole_page_raster: bool,
    has_layout_detections: bool,
    forced_layout_frames: Option<&[bool]>,
    page_index: usize,
) -> bool {
    whole_page_raster
        && (!has_layout_detections || forced_layout_frames.and_then(|frames| frames.get(page_index)) == Some(&true))
}

#[cfg(all(test, feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
#[test]
fn layout_orientation_retry_uses_each_siblings_frame() {
    let frames = Some([false, true, false].as_slice());
    let eligible: Vec<_> = (0..4)
        .filter(|&page| orientation_retry_page_eligible(true, true, frames, page))
        .collect();
    assert_eq!(eligible, [1]);
    assert!(!orientation_retry_page_eligible(false, true, frames, 1));
    assert!(!orientation_retry_page_eligible(true, true, None, 1));
    assert!(orientation_retry_page_eligible(true, false, None, 1));
}

#[cfg(all(test, feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
mod mixed_fallback_furniture_tests {
    use super::*;
    use crate::core::config::ContentFilterConfig;
    use crate::pdf::hierarchy::SegmentData;
    use crate::pdf::structure::types::{PdfLine, PdfParagraph};

    fn paragraph(text: &str, (left, bottom, right, top): (f32, f32, f32, f32)) -> PdfParagraph {
        let segment = SegmentData {
            text: text.to_string(),
            x: left,
            y: bottom,
            width: right - left,
            height: top - bottom,
            font_size: 12.0,
            is_bold: false,
            is_italic: false,
            is_monospace: false,
            baseline_y: bottom,
            rotation_degrees: 0.0,
            assigned_role: None,
            ocr_heading_spans: None,
        };
        PdfParagraph {
            text: String::new(),
            lines: vec![PdfLine {
                native_scripts: Vec::new(),
                segments: vec![segment],
                baseline_y: Some(bottom),
                dominant_font_size: 12.0,
                is_bold: false,
                is_monospace: false,
            }],
            dominant_font_size: 12.0,
            heading_level: None,
            is_bold: false,
            is_list_item: false,
            is_code_block: false,
            is_formula: false,
            is_page_furniture: false,
            layout_class: None,
            layout_region_path: None,
            ocr_source_spans: Vec::new(),
            caption_for: None,
            block_bbox: Some((left, bottom, right, top)),
            word_count: text.split_whitespace().count(),
        }
    }

    fn body_text(index: usize) -> &'static str {
        match index {
            0 => "The harbour committee recorded exactly one hundred arrivals for the southern market this evening.",
            1 => "The orchard committee recorded exactly one hundred shipments for the northern market this morning.",
            _ => "The railway committee recorded exactly one hundred departures for the western market this afternoon.",
        }
    }

    fn text(paragraph: &PdfParagraph) -> String {
        paragraph
            .lines
            .iter()
            .flat_map(|line| &line.segments)
            .map(|segment| segment.text.as_str())
            .collect()
    }

    /// A page recognition read from a raster stores its paragraphs in raster pixels (here
    /// 2x the 612x792pt page). The furniture pass must still see the header and the footer
    /// in the band it knows: the header survives because it is included, the footer goes.
    #[test]
    fn raster_scaled_fallback_keeps_the_header_and_drops_the_footer() {
        let page_numbers = [1u32, 2, 3];
        let raster: ahash::AHashMap<u32, (u32, u32)> = page_numbers.iter().map(|&page| (page, (1224, 1584))).collect();
        let mut pages: Vec<Vec<PdfParagraph>> = page_numbers
            .iter()
            .map(|&page_number| {
                vec![
                    paragraph("ACME ANNUAL REPORT 2026", (100.0, 1500.0, 800.0, 1520.0)),
                    paragraph(body_text(page_number as usize - 1), (100.0, 700.0, 1100.0, 800.0)),
                    paragraph(
                        &format!("CONFIDENTIAL - PAGE {page_number}"),
                        (100.0, 40.0, 500.0, 60.0),
                    ),
                ]
            })
            .collect();
        let header_bbox = pages[0][0].block_bbox;

        strip_fallback_furniture_in_page_points(
            &mut pages,
            &page_numbers,
            &raster,
            |_| false,
            |_| (612.0, 792.0),
            &[792.0; 3],
            &ContentFilterConfig {
                include_headers: true,
                include_footers: false,
                ..Default::default()
            },
        );

        for page in &pages {
            assert!(
                page.iter().any(|paragraph| text(paragraph).contains("ACME")),
                "include_headers keeps the running header: {:?}",
                page.iter().map(text).collect::<Vec<_>>()
            );
            assert!(
                !page.iter().any(|paragraph| text(paragraph).contains("CONFIDENTIAL")),
                "the excluded footer must be stripped: {:?}",
                page.iter().map(text).collect::<Vec<_>>()
            );
            assert!(
                page.iter().any(|paragraph| text(paragraph).contains("committee")),
                "body text must survive"
            );
        }
        assert_eq!(
            pages[0][0].block_bbox, header_bbox,
            "the surviving header geometry must be back in the raster frame"
        );
    }
}

#[cfg(all(test, feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
mod matched_layer_evidence_tests {
    use super::captures_matched_layer_occurrences;

    #[test]
    fn matched_layer_occurrences_stay_empty_without_scale_only_geometry() {
        assert!(!captures_matched_layer_occurrences(false, false));
        assert!(!captures_matched_layer_occurrences(false, true));
        assert!(captures_matched_layer_occurrences(true, false));
        assert!(!captures_matched_layer_occurrences(true, true));
    }
}

#[cfg(all(test, feature = "ocr", not(target_arch = "wasm32")))]
mod geometry_evidence_tests {
    use super::GeometryEvidence;

    #[test]
    fn only_the_concurrent_route_runs_the_geometry_and_language_passes() {
        assert!(GeometryEvidence::concurrent().runs());
        assert!(!GeometryEvidence::sequential().runs());
    }
}
