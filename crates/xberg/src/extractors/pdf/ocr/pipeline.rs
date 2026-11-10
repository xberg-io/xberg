#[cfg(all(
    feature = "pdf",
    feature = "ocr",
    feature = "tokio-runtime",
    not(paddle_ocr),
    not(target_arch = "wasm32")
))]
#[path = "pipeline/direct_rgb_budget_recovery.rs"]
mod direct_rgb_budget_recovery;

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
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
use super::document::{
    accepted_ocr_page_replacements, apply_ocr_page_replacements, apply_ocr_text_list_fallback,
    fill_unstructured_ocr_pages, heuristically_restructured_ocr_pages,
};
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
    render_full_pdf_ocr_batch_with_dpi, render_selected_pages_with_dpi, share_rendered_page_images,
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

/// ~keep The warning for a force-OCR extraction whose table model could not be checked out although
/// layout detection found table regions and table recognition is on. `None` when a model is held,
/// recognition is off, or no page has a table region.
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "layout-detection"))]
pub(super) fn tatr_unavailable_warning(
    config: &ExtractionConfig,
    layout_detections: Option<&[crate::layout::DetectionResult]>,
    model_held: bool,
) -> Option<crate::types::ProcessingWarning> {
    let has_table_region = layout_detections.is_some_and(|pages| {
        pages
            .iter()
            .flat_map(|page| &page.detections)
            .any(|detection| detection.class_name == crate::layout::LayoutClass::Table)
    });
    (!model_held && has_table_region && pdf_ocr_table_recognition_enabled(config))
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
    document: Option<&mut crate::types::internal::InternalDocument>,
    tables: &mut [crate::types::Table],
) {
    if let Cow::Owned(repaired) = repair_ocr_numeric_tokens(text) {
        *text = repaired;
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
/// distinguishable while source messages, paths and secrets never reach a warning, a failure
/// record, the run error or a trace. ~keep
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
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(crate) type OcrRunResult<T> = Result<T, FailedOcrRun>;

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
impl From<crate::XbergError> for FailedOcrRun {
    fn from(error: crate::XbergError) -> Self {
        Self {
            error,
            page_failures: Vec::new(),
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

/// Add to `failures` the records of a pipeline run that the route for selected pages got back.
///
/// The pipeline does not see the native text, so its record says only what its stages read.
/// A page that has native text keeps it when the run returns the page blank, so its record is
/// recovered: the rule for a page whose run failed.
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
fn merge_returned_page_failures(
    failures: &mut Vec<crate::types::OcrPageFailure>,
    returned: Vec<crate::types::OcrPageFailure>,
    native_text: &str,
    boundaries: &[crate::types::PageBoundary],
) {
    let returned = returned
        .into_iter()
        .map(|failure| crate::types::OcrPageFailure {
            recovered: failure.recovered
                || mixed_native_page_has_content(native_text, boundaries, failure.page.saturating_sub(1) as usize),
            ..failure
        })
        .collect();
    merge_ocr_page_failures(failures, returned);
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
            tracing::debug!(error = %failure.error, "Hidden PDF text layer probe preserved native text");
            Ok(None)
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
        return Ok(MixedOcrRun::native(native_text));
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
        return Ok(MixedOcrRun::native(native_text));
    }

    // Pages whose OCR text got the recognition-noise verdict and was not discarded. The caller
    // decides each page on its OCR text alone and puts the layer first only on a replaced page.
    let mut noise_verdict_pages: std::collections::HashSet<u32> = std::collections::HashSet::new();
    // Pages whose calibrated mean confidence fell below `min_ocr_mean_confidence`.
    let mut low_confidence_pages: std::collections::HashSet<u32> = std::collections::HashSet::new();
    // Pages whose dictionary-invalid word ratio crossed its threshold.
    let mut dictionary_noise_pages: std::collections::HashSet<u32> = std::collections::HashSet::new();
    // The readable text layers that a replaced page keeps first when its OCR text is judged
    // recognition noise or confirms the layer. A forced page is replaced by definition and keeps
    // none. ~keep
    let ocr_output_thresholds = config
        .ocr
        .as_ref()
        .and_then(|ocr| ocr.quality_thresholds.clone())
        .unwrap_or_default();
    let page_layers = match pages.native_layers {
        Some(layers) => Some(layers),
        None if !probing_hidden_layer && all_pages_failed_policy == AllPagesFailedPolicy::PreserveNative => {
            let layers = super::document::clean_native_page_layers(
                native_text,
                boundaries,
                Some(&ocr_set),
                &ocr_output_thresholds,
            );
            (!layers.is_empty()).then(|| std::sync::Arc::new(layers))
        }
        None => None,
    };

    // Layout detection for this mixed OCR route (#665). The full-document OCR routes
    // (`force_ocr`, the OCR-gate fallback) already run layout via `run_ocr_with_layout` ->
    // `layout_runner::run_layout_for_ocr`, keyed on `config.resolved_layout_config()` (i.e.
    // `config.layout` being set, which `--layout` alone does). Reuse the resolved layout outcome
    // supplied by a whole-document caller, including `None` when its gate skipped every page or
    // the pass soft-failed; otherwise run the pass here for a selected-page call. This prevents
    // duplicate layout work while keeping selected-page layout classification. `page_idx` is the
    // document-wide 0-based index the layout pass uses. ~keep
    #[cfg(feature = "layout-detection")]
    let mut layout_raster_dpis = crate::pdf::render::OcrRasterDpis::new();
    #[cfg(feature = "layout-detection")]
    let (resolved_layout_inputs, layout_pass_warning, layout_pass_glyph_drop_warnings): MixedLayoutOutcome =
        if let MixedLayoutInputs::Prepared(_) = layout_inputs {
            (None, None, Vec::new())
        } else if let Some(layout_config) = config.resolved_layout_config() {
            let layout_thread_budget =
                crate::core::config::concurrency::resolve_thread_budget(config.concurrency.as_ref());
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
    #[cfg(feature = "layout-detection")]
    let layout_inputs_for_mixed = match layout_inputs {
        MixedLayoutInputs::Prepared(inputs) => inputs,
        MixedLayoutInputs::Resolve(_) => resolved_layout_inputs.as_ref(),
    };
    #[cfg(feature = "layout-detection")]
    let layout_detections_for_mixed = layout_inputs_for_mixed.map(|(_, detections)| detections.as_slice());
    #[cfg(feature = "layout-detection")]
    let layout_images_for_mixed = layout_inputs_for_mixed.map(|(images, _)| images.as_slice());
    #[cfg(feature = "layout-detection")]
    let mixed_route_layout_active = layout_inputs_for_mixed.is_some();
    #[cfg(not(feature = "layout-detection"))]
    let mixed_route_layout_active = false;

    use image::ImageEncoder;
    use image::codecs::png::PngEncoder;
    // rayon's work-stealing pool needs OS threads; wasm32 has none, so the parallel encode
    // paths below fall back to sequential `.iter()` there. Gate the import to match. ~keep
    #[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
    use rayon::prelude::*;
    use std::io::Cursor;
    use std::sync::Arc;

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
    let preprocessing_render_dpi = (!mixed_route_layout_active && !probing_hidden_layer)
        .then(|| registered_preprocessing_render_dpi(config, &ocr_config_resolved))
        .flatten();
    #[cfg(not(all(feature = "ocr", feature = "pdf", not(target_arch = "wasm32"))))]
    let preprocessing_render_dpi: Option<i32> = None;
    let render_page_cap = config
        .security_limits
        .as_ref()
        .cloned()
        .unwrap_or_default()
        .max_content_size;

    let configured_batch_size = crate::core::config::concurrency::resolve_thread_budget(config.concurrency.as_ref());
    // How many pages this route renders, encodes and recognises at once. `max_content_size`
    // limits the text one extraction returns; it says nothing about the rasters a batch holds
    // while it works. Sizing the batch from it pinned the width at four pages on an ordinary
    // 150 DPI document, so no stage of this route could ever use more than four threads and
    // the thread budget changed nothing (#1666, #1716).
    //
    // What a batch really costs is this document's own page cost times the batch width, so
    // that product is what the width is bounded by, against the memory the host has free.
    // Measuring the page rather than assuming a fixed per-page figure is what keeps the bound
    // honest at a high render DPI, where one page costs an order of magnitude more than the
    // assumption does. Each page's own peak is still checked against `max_content_size`, one
    // page at a time, before that page is encoded. ~keep
    let batch_size = match page_indices.first() {
        Some(&first_page_idx) => {
            let (page_width_pt, page_height_pt) = page_dimensions_pt(&render_doc, first_page_idx);
            adapt_batch_size_to_memory(
                configured_batch_size,
                content.len(),
                ocr_page_working_set_bytes(
                    f64::from(page_width_pt),
                    f64::from(page_height_pt),
                    config.images.as_ref(),
                )
                .max(preprocessing_render_dpi.map_or(0, |target| {
                    preprocessing_render_reservation(Some(target), render_page_cap)
                })),
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

    let capture_rasters = config.images.as_ref().is_some_and(|c| c.include_page_rasters);
    let ocr_config_owned = ensure_elements_enabled(&ocr_config_resolved);
    // When any effective pipeline is configured or synthesized, each page must run through
    // the shared pipeline runner so its fallback decision stays page-local. This includes the
    // implicit Tesseract-to-Paddle pipeline as well as `vlm_fallback` and an explicit pipeline;
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
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    let orientation_retry = effective_pipeline
        .as_ref()
        .filter(|_| super::orientation_retry::OrientationRetry::allows(config) && !mixed_route_layout_active)
        .map(|pipeline| {
            Arc::new(super::orientation_retry::OrientationRetry::new(
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
    let backend = if let Some(backend) = pages.hidden_layer_backend {
        Some(backend)
    } else if effective_pipeline.is_none() {
        let registry = crate::plugins::registry::get_ocr_backend_registry();
        let registry = registry.read();
        Some(registry.get(&ocr_config_owned.backend)?)
    } else {
        None
    };

    let total = page_indices.len();
    let mut raster_dpis = pages.raster_dpis.as_deref().cloned().unwrap_or_default();
    #[cfg(feature = "layout-detection")]
    raster_dpis.extend(layout_raster_dpis);
    let mut ocr_results: ahash::AHashMap<u32, String> = ahash::AHashMap::with_capacity(total);
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    let mut original_tesseract_raster_pages = ahash::AHashSet::new();
    #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
    let mut caption_rewrites = ahash::AHashMap::new();
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    let mut native_words_covered_pages = ahash::AHashSet::new();
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    let mut native_ink_owned_pages = ahash::AHashSet::new();
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    let ink_coverage_layers = {
        let markers = if !probing_hidden_layer && all_pages_failed_policy == AllPagesFailedPolicy::PreserveNative {
            super::document::marker_native_ink_layers(
                native_text,
                boundaries,
                &ocr_set,
                page_count,
                &ocr_output_thresholds,
            )
        } else {
            ahash::AHashMap::new()
        };
        if markers.is_empty() {
            page_layers.clone()
        } else {
            let mut layers = page_layers.as_deref().cloned().unwrap_or_default();
            layers.extend(markers);
            Some(std::sync::Arc::new(layers))
        }
    };
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    let native_coverage_layers = ink_coverage_layers.as_ref().filter(|_| {
        all_pages_failed_policy == AllPagesFailedPolicy::PreserveNative
            && !probing_hidden_layer
            && !mixed_route_layout_active
            && config.ocr_strategy == crate::core::config::OcrStrategy::Auto
            && config.ocr.as_ref().is_some_and(uses_default_ocr_pipeline)
    });
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    let mut alternative_raster_pages = ahash::AHashSet::new();
    // Tesseract's own mean confidence for each page it read, 0-100. Captured here because
    // the per-page metadata is gone by the time `ocr_results` is judged.
    let mut page_mean_confidence: ahash::AHashMap<u32, f64> = ahash::AHashMap::new();
    let mut page_dictionary_invalid_word_ratio: ahash::AHashMap<u32, f64> = ahash::AHashMap::new();
    // Collected next to `page_mean_confidence` for the same reason: the per-page metadata is
    // gone by the time the summary attached to `PageContent.ocr_confidence` is built (#1568). ~keep
    let mut page_word_count: ahash::AHashMap<u32, u32> = ahash::AHashMap::new();
    let mut ocr_confidence_by_page: ahash::AHashMap<u32, crate::types::page::PageOcrConfidence> =
        ahash::AHashMap::new();
    let mut structured_ocr_pages: ahash::AHashMap<u32, crate::types::internal::InternalDocument> =
        ahash::AHashMap::with_capacity(total);
    // Table-only embedded-image retries have no flat text replacement, but their structured
    // page must survive the acceptance pass and count as recovered content. ~keep
    let mut table_only_xobject_recovery_pages: ahash::AHashSet<u32> = ahash::AHashSet::new();
    // Bare, unclassified per-page paragraphs for every OCR'd page, real 1-indexed page
    // number -> that page's paragraphs. Collected across both the pipeline and
    // single-backend routes below so the document-global heading/list heuristic
    // (`heuristically_restructured_ocr_pages`) can run exactly once, after this
    // function's per-page loop ends, over every OCR'd page at once -- the same
    // font-clustering pass `extract_with_ocr_for_page` runs for the OCR-only route,
    // previously unreachable here (#665-adjacent gap; see that function's own doc
    // comment on `skip_document_global_heuristic`).
    let mut ocr_page_paragraphs: ahash::AHashMap<u32, Vec<crate::pdf::structure::types::PdfParagraph>> =
        ahash::AHashMap::with_capacity(total);
    // Pipeline paragraphs without an explicit point mapping retain the legacy raster scale
    // for layer matching. Single-backend and already placed detached paragraphs use points. ~keep
    let mut ocr_raster_by_page: ahash::AHashMap<u32, (u32, u32)> = ahash::AHashMap::with_capacity(total);
    let mut accumulated_llm_usage: Vec<crate::types::LlmUsage> = Vec::new();
    let mut accumulated_formulas: Vec<crate::types::Formula> = Vec::new();
    let mut accumulated_warnings: Vec<crate::types::ProcessingWarning> = Vec::new();
    let mut page_failures: Vec<crate::types::OcrPageFailure> = Vec::new();
    let mut failed_page_errors: ahash::AHashMap<usize, crate::XbergError> = ahash::AHashMap::new();
    #[cfg(feature = "layout-detection")]
    {
        if let Some(warning) = layout_pass_warning {
            crate::core::diagnostics::push_warning_deduped(&mut accumulated_warnings, warning);
        }
        for warning in layout_pass_glyph_drop_warnings {
            crate::core::diagnostics::push_warning_deduped(&mut accumulated_warnings, warning);
        }
    }
    let mut captured_rasters: Vec<crate::types::ExtractedImage> = Vec::new();
    let mut preprocessing_by_page: ahash::AHashMap<u32, crate::types::ImagePreprocessingMetadata> =
        ahash::AHashMap::new();
    let mut xobject_recovery_budget = crate::extractors::security::SecurityBudget::from_config(config);
    for batch_start in (0..total).step_by(batch_size) {
        let batch_end = (batch_start + batch_size).min(total);
        let default_security_limits = crate::extractors::security::SecurityLimits::default();
        let security_limits = config.security_limits.as_ref().unwrap_or(&default_security_limits);
        #[cfg(feature = "layout-detection")]
        let (page_images, render_failures, batch_raster_dpis) = if let Some(images) = layout_images_for_mixed {
            let mut page_images = Vec::with_capacity(batch_end - batch_start);
            let mut render_failures = Vec::new();
            for &page_idx in &page_indices[batch_start..batch_end] {
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
            render_selected_pages_with_dpi(
                &render_doc,
                &page_rotations,
                &page_indices[batch_start..batch_end],
                security_limits,
                config.images.as_ref(),
                preprocessing_render_dpi,
            )
        };
        #[cfg(not(feature = "layout-detection"))]
        let (page_images, render_failures, batch_raster_dpis) = render_selected_pages_with_dpi(
            &render_doc,
            &page_rotations,
            &page_indices[batch_start..batch_end],
            security_limits,
            config.images.as_ref(),
            preprocessing_render_dpi,
        );
        raster_dpis.extend(batch_raster_dpis);
        let mut budget_refused: Vec<(usize, crate::XbergError)> = Vec::new();
        for (page_idx, error) in render_failures {
            if render_refused_by_source_image_budget(&error) {
                budget_refused.push((page_idx, error));
                continue;
            }
            let recovered = mixed_native_page_has_content(native_text, boundaries, page_idx);
            push_mixed_ocr_page_failure_warning(
                &mut accumulated_warnings,
                &mut page_failures,
                page_idx,
                &error,
                recovered,
            );
            failed_page_errors.insert(page_idx, error);
        }

        // Multi-stage pipeline route (#1341): drive each page through `run_ocr_pipeline`
        // so `vlm_fallback` / explicit-pipeline stages apply here, mirroring the image
        // extractor's per-image pipeline path. Bounded to this batch's page count (at
        // most `batch_size`, the resolved worker budget) via a `JoinSet`, mirroring the
        // concurrency shape of the single-backend path below.
        if let Some(ref pipeline) = effective_pipeline {
            for (page_idx, error) in std::mem::take(&mut budget_refused) {
                let backend = crate::plugins::registry::get_ocr_backend_registry()
                    .read()
                    .get(&ocr_config_owned.backend)
                    .ok();
                let recovery = match backend {
                    Some(backend) => {
                        recover_page_text_from_image_xobjects(
                            &backend,
                            &render_doc,
                            page_idx,
                            page_rotations.len(),
                            &ocr_config_owned,
                            XObjectRecoveryContext {
                                budget: &mut xobject_recovery_budget,
                                blank_raster_warnings: Some(&mut accumulated_warnings),
                                no_admitted_images: None,
                                stored_jpegs: None,
                                cancel_token: config.cancel_token.as_ref(),
                                use_proven_source_dpi: true,
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
                    used_proven_source_dpi,
                    possibly_illegible,
                }) = recovery
                {
                    retry_causes = failure_causes;
                    let text = judge_proven_read(
                        text,
                        ProvenRead {
                            used_proven_source_dpi,
                            possibly_illegible,
                            page_idx,
                            warn_when_kept: true,
                        },
                        &ocr_output_thresholds,
                        &mut accumulated_warnings,
                        &mut retry_causes,
                    );
                    recovered = should_adopt_xobject_retry_text("", &text) || !tables.is_empty();
                    if should_adopt_xobject_retry_text("", &text) {
                        structured_ocr_pages.insert(
                            page_number,
                            super::document::flat_ocr_page_document(&text_outside_tables),
                        );
                        ocr_results.insert(page_number, text);
                    }
                    if !tables.is_empty() {
                        let current_text = ocr_results.get(&page_number).map_or("", String::as_str);
                        let page_doc = structured_ocr_pages
                            .entry(page_number)
                            .or_insert_with(|| super::document::flat_ocr_page_document(current_text));
                        super::document::attach_page_ocr_payload(page_doc, tables, Vec::new(), page_number);
                        if !ocr_results.contains_key(&page_number) {
                            table_only_xobject_recovery_pages.insert(page_number);
                        }
                    }
                    if let Some(psm) = tesseract_psm
                        && let Some(page_doc) = structured_ocr_pages.get_mut(&page_number)
                    {
                        attach_page_segmentation_modes(page_doc, [(page_number, psm)]);
                    }
                    accumulated_llm_usage.append(&mut llm_usage);
                    accumulated_formulas.append(&mut formulas);
                    if let Some(metadata) = image_preprocessing {
                        preprocessing_by_page.insert(page_number, metadata);
                    }
                    if capture_rasters {
                        captured_rasters.extend(images);
                    }
                    accumulated_warnings.push(xobject_fallback_warning(page_idx, attempted, true));
                }
                if recovered {
                    let recovered_kind = if ocr_results
                        .get(&page_number)
                        .is_some_and(|text| !text.trim().is_empty())
                    {
                        "text"
                    } else {
                        "content"
                    };
                    accumulated_warnings.push(crate::types::ProcessingWarning {
                        source: std::borrow::Cow::Borrowed("ocr"),
                        message: std::borrow::Cow::Owned(format!(
                            "OCR of page {page_number} failed ({error}); its {recovered_kind} was recovered from the \
                             page's embedded image XObjects instead."
                        )),
                    });
                    merge_ocr_page_failures(
                        &mut page_failures,
                        vec![crate::types::OcrPageFailure {
                            page: page_number,
                            error: error.to_string(),
                            recovered: true,
                        }],
                    );
                } else {
                    let error = with_retry_causes(error, &retry_causes);
                    let native_recovered = mixed_native_page_has_content(native_text, boundaries, page_idx);
                    push_mixed_ocr_page_failure_warning(
                        &mut accumulated_warnings,
                        &mut page_failures,
                        page_idx,
                        &error,
                        native_recovered,
                    );
                    failed_page_errors.insert(page_idx, error);
                }
            }
            let page_images = share_rendered_page_images(page_images);
            // on wasm32 (no OS threads, and extractor/backend futures are `!Send` there —
            // see the matching gate on the single-backend path below). Falls back to the
            // sequential loop there even though `tokio-runtime` may be active.
            #[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
            let page_images_for_capture = capture_rasters.then(|| page_images.clone());
            #[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
            {
                let mut join_set = tokio::task::JoinSet::new();
                let dimensions: ahash::AHashMap<_, _> = page_images
                    .iter()
                    .map(|(index, image)| (*index, (image.width(), image.height())))
                    .collect();
                for (page_idx, image_arc) in page_images {
                    if config.cancel_token.as_ref().is_some_and(|t| t.is_cancelled()) {
                        break;
                    }
                    let render_doc_clone = Arc::clone(&render_doc);
                    let pipeline_clone = pipeline.clone();
                    let config_clone = pipeline_stage_config.clone();
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
                    let page_ocr_hints = PageOcrHints {
                        source_dpi: rendered_page_source_dpi(&render_doc, page_idx, image_arc.width()),
                        known_full_page_scan: crate::pdf::scan_detect::full_page_raster_density(&render_doc, page_idx)
                            .is_some(),
                        single_block_pages: Some(std::sync::Arc::clone(&single_block_pages)),
                        native_page_layers: None,
                        #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
                        native_coverage_layers: native_coverage_layers
                            .filter(|_| {
                                super::document::native_preservation_is_automatic(config, (page_idx + 1) as u32)
                            })
                            .cloned(),
                        #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
                        single_native_type0_page: pages
                            .single_native_type0_page
                            .filter(|_| page_count == 1 && page_idx == 0)
                            .map(std::sync::Arc::from),
                        ..Default::default()
                    };
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
                    // This page's own pixel-space detection from the whole-document layout
                    // pass above (#665), cloned out here because the spawned task below must
                    // own everything it captures. `extract_with_ocr_for_page` (reached through
                    // `run_ocr_pipeline_for_page`) rescales it to match this exact raster via
                    // `scale_detection_to_dimensions`/`scale_detection_to_ocr_coordinates`, so
                    // a DPI mismatch between the layout pass's own render and this page's OCR
                    // render is handled there, not here.
                    #[cfg(feature = "layout-detection")]
                    let page_detection: Option<crate::layout::DetectionResult> =
                        detection_for_mixed_route_page(layout_detections_for_mixed, page_idx).cloned();
                    #[cfg(all(paddle_ocr, feature = "ocr"))]
                    let allow_stored_retry = !capture_rasters
                        && !mixed_route_layout_active
                        && page_ocr_hints.known_full_page_scan
                        && stored_scan_retry::eligible(config, (idx + 1) as u32);
                    let task = crate::engine::seams::inherit_progress(async move {
                        if config_clone.cancel_token.as_ref().is_some_and(|t| t.is_cancelled()) {
                            return (idx, Err(crate::XbergError::Cancelled.into()));
                        }
                        #[cfg(feature = "layout-detection")]
                        let page_detection_slice = page_detection.as_ref().map(std::slice::from_ref);
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
                        (idx, result)
                    });
                    #[cfg(test)]
                    let task = crate::plugins::registry::inherit_test_ocr_backend(task);
                    join_set.spawn(crate::core::extractor::cacheability::inherit(task));
                }
                while let Some(join_result) = join_set.join_next().await {
                    let (page_idx, result) = join_result.map_err(|e| crate::XbergError::Plugin {
                        message: format!("OCR pipeline task panicked: {}", e),
                        plugin_name: "ocr".to_string(),
                    })?;
                    let result = match result.map_err(crate::XbergError::from) {
                        Ok(result) => result,
                        Err(error @ crate::XbergError::Cancelled) => return Err(error.into()),
                        Err(error) => {
                            let recovered = mixed_native_page_has_content(native_text, boundaries, page_idx);
                            push_mixed_ocr_page_failure_warning(
                                &mut accumulated_warnings,
                                &mut page_failures,
                                page_idx,
                                &error,
                                recovered,
                            );
                            failed_page_errors.insert(page_idx, error);
                            continue;
                        }
                    };
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
                    accumulated_llm_usage.extend(usage);
                    ocr_confidence_by_page.extend(page_ocr_confidences);
                    let page_number = (page_idx + 1) as u32;
                    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
                    if read_evidence.original_tesseract_raster {
                        original_tesseract_raster_pages.insert(page_number);
                        if read_evidence.complete_native_words {
                            native_words_covered_pages.insert(page_number);
                        }
                        if read_evidence.native_ink_owned {
                            native_ink_owned_pages.insert(page_number);
                        }
                    }
                    #[cfg(not(all(feature = "ocr", not(target_arch = "wasm32"))))]
                    let _ = read_evidence;
                    if noise_verdicts.iter().any(|verdict| !verdict.discarded) {
                        noise_verdict_pages.insert(page_number);
                    }
                    let ocr_frame_px = preprocessing.values().next().and_then(processed_frame_px);
                    if let Some(metadata) = preprocessing.into_values().next() {
                        preprocessing_by_page.insert(page_number, metadata);
                    }
                    // `run_ocr_pipeline_for_page` drove exactly one detached image
                    // (`std::slice::from_ref` above), so its own per-page paragraph vec has
                    // at most one entry -- this page's own, still bare/unclassified because
                    // `skip_document_global_heuristic = true` was passed above.
                    let mut paragraph_page = page_raw_paragraphs.pop().unwrap_or_default();
                    let paragraphs_in_points = paragraph_page.place_detached_in_points();
                    if !paragraph_page.paragraphs.is_empty() {
                        ocr_page_paragraphs.insert(page_number, paragraph_page.paragraphs);
                    }
                    let page_dims = dimensions.get(&page_idx).copied();
                    for mut formula in formulas {
                        formula.page = Some(page_number);
                        if let Some((w, h)) = page_dims {
                            formula_bbox_to_page_points(&mut formula, &render_doc, page_idx, None, w, h);
                        }
                        accumulated_formulas.push(formula);
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
                    if !paragraphs_in_points {
                        ocr_raster_by_page.insert(page_number, ocr_frame_px.unwrap_or(raster_size_px));
                    }
                    let page_size_pt = page_dimensions_pt(&render_doc, page_idx);
                    #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
                    if let Some(rewrite) = read_evidence
                        .caption
                        .and_then(|rewrite| rewrite.bind_page(raster_size_px, page_size_pt))
                    {
                        caption_rewrites.insert(page_number, rewrite);
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
                        crate::core::diagnostics::dedup_extend_warnings(
                            &mut accumulated_warnings,
                            std::mem::take(&mut d.processing_warnings),
                        );
                        merge_returned_page_failures(
                            &mut page_failures,
                            std::mem::take(&mut d.ocr_page_failures),
                            native_text,
                            boundaries,
                        );
                        structured_ocr_pages.insert(page_number, d);
                    }
                    ocr_results.insert(page_number, page_text);
                }
            }
            #[cfg(any(not(feature = "tokio-runtime"), target_arch = "wasm32"))]
            {
                for (page_idx, image) in &page_images {
                    if config.cancel_token.as_ref().is_some_and(|t| t.is_cancelled()) {
                        break;
                    }
                    // See the matching comments on the sibling `JoinSet` branch above (#651,
                    // and `points_per_pixel_override` re. `extract_with_ocr_for_page`'s doc
                    // comment).
                    let page_rotation_degrees = page_rotations.get(*page_idx).copied().unwrap_or(0);
                    let page_ocr_hints = PageOcrHints {
                        source_dpi: rendered_page_source_dpi(&render_doc, *page_idx, image.width()),
                        known_full_page_scan: crate::pdf::scan_detect::full_page_raster_density(&render_doc, *page_idx)
                            .is_some(),
                        single_block_pages: Some(std::sync::Arc::clone(&single_block_pages)),
                        native_page_layers: None,
                        #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
                        native_coverage_layers: native_coverage_layers
                            .filter(|_| {
                                super::document::native_preservation_is_automatic(config, (*page_idx + 1) as u32)
                            })
                            .cloned(),
                        #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
                        single_native_type0_page: pages
                            .single_native_type0_page
                            .filter(|_| page_count == 1 && *page_idx == 0)
                            .map(std::sync::Arc::from),
                        ..Default::default()
                    };
                    let points_per_pixel_override = {
                        let (_, page_height_pt) = page_dimensions_pt(&render_doc, *page_idx);
                        let image_height_px = image.height();
                        (image_height_px > 0).then(|| page_height_pt / image_height_px as f32)
                    };
                    // See the matching comment on the sibling `JoinSet` branch above (#665).
                    #[cfg(feature = "layout-detection")]
                    let page_detection: Option<&crate::layout::DetectionResult> =
                        detection_for_mixed_route_page(layout_detections_for_mixed, *page_idx);
                    let source = DetachedOcrPage {
                        document: render_doc.as_ref(),
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
                        &pipeline_stage_config,
                        pipeline,
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
                            let recovered = mixed_native_page_has_content(native_text, boundaries, *page_idx);
                            push_mixed_ocr_page_failure_warning(
                                &mut accumulated_warnings,
                                &mut page_failures,
                                *page_idx,
                                &error,
                                recovered,
                            );
                            failed_page_errors.insert(*page_idx, error);
                            continue;
                        }
                    };
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
                    accumulated_llm_usage.extend(usage);
                    ocr_confidence_by_page.extend(page_ocr_confidences);
                    let page_number = (*page_idx + 1) as u32;
                    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
                    if read_evidence.original_tesseract_raster {
                        original_tesseract_raster_pages.insert(page_number);
                        if read_evidence.complete_native_words {
                            native_words_covered_pages.insert(page_number);
                        }
                        if read_evidence.native_ink_owned {
                            native_ink_owned_pages.insert(page_number);
                        }
                    }
                    #[cfg(not(all(feature = "ocr", not(target_arch = "wasm32"))))]
                    let _ = read_evidence;
                    if noise_verdicts.iter().any(|verdict| !verdict.discarded) {
                        noise_verdict_pages.insert(page_number);
                    }
                    let ocr_frame_px = preprocessing.values().next().and_then(processed_frame_px);
                    if let Some(metadata) = preprocessing.into_values().next() {
                        preprocessing_by_page.insert(page_number, metadata);
                    }
                    let mut paragraph_page = page_raw_paragraphs.pop().unwrap_or_default();
                    let paragraphs_in_points = paragraph_page.place_detached_in_points();
                    if !paragraph_page.paragraphs.is_empty() {
                        ocr_page_paragraphs.insert(page_number, paragraph_page.paragraphs);
                    }
                    for mut formula in formulas {
                        formula.page = Some(page_number);
                        formula_bbox_to_page_points(
                            &mut formula,
                            &render_doc,
                            *page_idx,
                            None,
                            image.width(),
                            image.height(),
                        );
                        accumulated_formulas.push(formula);
                    }
                    let page_text = page_texts.into_iter().next().unwrap_or(text);
                    if !paragraphs_in_points {
                        ocr_raster_by_page.insert(page_number, ocr_frame_px.unwrap_or((image.width(), image.height())));
                    }
                    if let Some(mut d) = build_pipeline_ocr_page_document(
                        doc,
                        tables,
                        elements,
                        &page_text,
                        page_number,
                        (image.width(), image.height()),
                        page_dimensions_pt(&render_doc, *page_idx),
                    ) {
                        crate::core::diagnostics::dedup_extend_warnings(
                            &mut accumulated_warnings,
                            std::mem::take(&mut d.processing_warnings),
                        );
                        merge_returned_page_failures(
                            &mut page_failures,
                            std::mem::take(&mut d.ocr_page_failures),
                            native_text,
                            boundaries,
                        );
                        structured_ocr_pages.insert(page_number, d);
                    }
                    ocr_results.insert(page_number, page_text);
                }
            }
            if capture_rasters {
                #[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
                let page_images = page_images_for_capture.expect("raster capture retains its inputs");
                let default_security_limits = crate::extractors::security::SecurityLimits::default();
                let security_limits = config.security_limits.as_ref().unwrap_or(&default_security_limits);
                // One page at a time, as on the single-backend path below. `max_content_size`
                // bounds what a single page costs to render and encode; summing a whole batch
                // against it makes that ceiling a function of the thread budget, so a Letter
                // page at the 300 DPI an `images` config implies rejects the whole batch, and
                // every page in it, from a width of two upwards. The batch's own footprint is
                // bounded by `batch_size` above, which free memory decides. ~keep
                validate_png_encode_pages_individually(
                    page_images.iter().map(|(_, image)| image.as_ref()),
                    security_limits,
                )?;
                for (page_idx, image) in &page_images {
                    let rgb = clone_rgb_for_png_encode(image, security_limits)?;
                    let (w, h) = rgb.dimensions();
                    let mut buf = Cursor::new(Vec::new());
                    PngEncoder::new(&mut buf)
                        .write_image(&rgb, w, h, image::ColorType::Rgb8.into())
                        .map_err(|e| crate::XbergError::Parsing {
                            message: format!("Failed to encode page {} raster: {}", page_idx + 1, e),
                            source: None,
                        })?;
                    captured_rasters.push(build_page_raster_image(
                        *page_idx,
                        bytes::Bytes::from(buf.into_inner()),
                        w,
                        h,
                    ));
                }
            }
            continue;
        }

        // Reached only when `effective_pipeline` is `None`, so `backend` was resolved above.
        let backend = backend.as_ref().ok_or_else(|| crate::XbergError::Ocr {
            message: "OCR backend was not resolved for the direct page route".to_string(),
            source: None,
        })?;
        let orientation_handling = backend.page_orientation_handling();
        #[cfg(all(
            feature = "ocr",
            feature = "pdf",
            feature = "tokio-runtime",
            not(target_arch = "wasm32")
        ))]
        let tall_source_candidate = !mixed_route_layout_active
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
            .then(|| super::tall_source::unsupported_reason(config, &ocr_config_resolved))
            .flatten();
        #[cfg(all(
            feature = "ocr",
            feature = "pdf",
            feature = "tokio-runtime",
            not(target_arch = "wasm32")
        ))]
        let try_tall_source = tall_source_candidate && tall_source_unsupported.is_none();
        let batch_slice = &page_images;
        let default_security_limits = crate::extractors::security::SecurityLimits::default();
        let security_limits = config.security_limits.as_ref().unwrap_or(&default_security_limits);
        // One page at a time. `max_content_size` bounds what a single page may cost to render
        // and encode; summing a whole batch against it made that ceiling a function of the
        // thread budget, so a wide batch rejected every page in it (#1665) and the narrow
        // batch that hid the defect in #1666 was the workaround. The batch's own footprint is
        // bounded by `batch_size` above, which free memory decides. ~keep
        validate_png_encode_pages_individually(batch_slice.iter().map(|(_, image)| image), security_limits)?;

        #[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
        let encoded: crate::Result<Vec<EncodedPage>> = batch_slice
            .par_iter()
            .map(|(page_idx, image)| {
                let default_security_limits = crate::extractors::security::SecurityLimits::default();
                let security_limits = config.security_limits.as_ref().unwrap_or(&default_security_limits);
                let rgb = clone_rgb_for_png_encode(image, security_limits)?;
                let (w, h) = rgb.dimensions();
                let mut buf = Cursor::new(Vec::new());
                PngEncoder::new(&mut buf)
                    .write_image(&rgb, w, h, image::ColorType::Rgb8.into())
                    .map_err(|e| crate::XbergError::Parsing {
                        message: format!("Failed to encode page {} for OCR: {}", page_idx + 1, e),
                        source: None,
                    })?;
                Ok((*page_idx, Arc::new(buf.into_inner()), w, h))
            })
            .collect();
        #[cfg(any(not(feature = "tokio-runtime"), target_arch = "wasm32"))]
        let encoded: crate::Result<Vec<EncodedPage>> = batch_slice
            .iter()
            .map(|(page_idx, image)| {
                let default_security_limits = crate::extractors::security::SecurityLimits::default();
                let security_limits = config.security_limits.as_ref().unwrap_or(&default_security_limits);
                let rgb = clone_rgb_for_png_encode(image, security_limits)?;
                let (w, h) = rgb.dimensions();
                let mut buf = Cursor::new(Vec::new());
                PngEncoder::new(&mut buf)
                    .write_image(&rgb, w, h, image::ColorType::Rgb8.into())
                    .map_err(|e| crate::XbergError::Parsing {
                        message: format!("Failed to encode page {} for OCR: {}", page_idx + 1, e),
                        source: None,
                    })?;
                Ok((*page_idx, Arc::new(buf.into_inner()), w, h))
            })
            .collect();
        let encoded = encoded?;
        drop(page_images);
        // A page whose backend call failed waits here for the embedded-image retry below.
        let mut failed_pages: ahash::AHashMap<usize, crate::XbergError> = ahash::AHashMap::new();
        let budget_refused_pages: Vec<usize> = budget_refused.iter().map(|(page_idx, _)| *page_idx).collect();
        failed_pages.extend(budget_refused);

        // `tokio::task::JoinSet::spawn` requires `Send` futures, but extractor/backend futures
        // are `!Send` on wasm32 (async_trait(?Send), see plugins/extractor/trait.rs) — and
        // wasm32 has no OS threads to run them on regardless. Fall back to the sequential path
        // there even though `tokio-runtime` is active (it's pulled in by
        // `chunking-tokenizers`/`static-embeddings`, not concurrency support). ~keep
        #[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
        {
            let mut join_set = tokio::task::JoinSet::new();
            for (page_idx, data, width, height) in &encoded {
                if config.cancel_token.as_ref().is_some_and(|t| t.is_cancelled()) {
                    break;
                }
                let backend_clone = Arc::clone(backend);
                let page_rotation_degrees = page_rotations.get(*page_idx).copied().unwrap_or(0);
                // Derived from the MediaBox-oriented raster, before `upright_raster_for_backend`
                // may swap its axes.
                let source_dpi = rendered_page_source_dpi(&render_doc, *page_idx, *width);
                let whole_page_raster =
                    crate::pdf::scan_detect::full_page_raster_density(&render_doc, *page_idx).is_some();
                #[cfg(feature = "layout-detection")]
                let layout_assembly = detection_for_mixed_route_page(layout_detections_for_mixed, *page_idx).is_some();
                #[cfg(not(feature = "layout-detection"))]
                let layout_assembly = false;
                let config_clone = ocr_config_with_page_rotation_hint(
                    &ocr_config_owned,
                    page_rotation_degrees,
                    source_dpi,
                    whole_page_raster,
                    single_block_pages.contains(&(*page_idx as u32 + 1)),
                    layout_assembly,
                )
                .into_owned();
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
                let original_config = ocr_config_resolved.clone();
                let retained_bytes = retained_ocr_source_bytes(data, &upright_data, 0);
                let task = crate::engine::seams::inherit_progress(async move {
                    if cancel_token.as_ref().is_some_and(|t| t.is_cancelled()) {
                        return (
                            idx,
                            correction_degrees,
                            upright_width,
                            upright_height,
                            Err(crate::XbergError::Cancelled),
                        );
                    }
                    let (original_config, config_clone) = match crate::extractors::image::reserve_retained_ocr_raster(
                        &backend_clone,
                        (original_config, config_clone),
                        retained_bytes,
                    ) {
                        Ok(configs) => configs,
                        Err(error) => return (idx, correction_degrees, upright_width, upright_height, Err(error)),
                    };
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
                    (idx, correction_degrees, upright_width, upright_height, result)
                });
                join_set.spawn(crate::core::extractor::cacheability::inherit(task));
            }
            while let Some(join_result) = join_set.join_next().await {
                let (page_idx, correction_degrees, upright_width, upright_height, result) =
                    join_result.map_err(|e| crate::XbergError::Plugin {
                        message: format!("OCR task panicked: {}", e),
                        plugin_name: "ocr".to_string(),
                    })?;
                #[cfg(all(
                    feature = "ocr",
                    feature = "pdf",
                    feature = "tokio-runtime",
                    not(target_arch = "wasm32")
                ))]
                let result = {
                    let mut result = result;
                    if try_tall_source
                        && correction_degrees == 0
                        && let Ok(extraction_result) = &mut result
                        && let Some(raster) = encoded.iter().find(|(index, ..)| *index == page_idx)
                        && super::tall_source::try_replace(
                            extraction_result,
                            backend,
                            config,
                            &ocr_config_resolved,
                            (&render_doc, page_idx),
                            raster,
                        )
                        .await?
                    {
                        alternative_raster_pages.insert((page_idx + 1) as u32);
                    } else if let Some(reason) = tall_source_unsupported
                        && correction_degrees == 0
                        && let Ok(extraction_result) = &mut result
                        && let Some(raster) = encoded.iter().find(|(index, ..)| *index == page_idx)
                    {
                        super::tall_source::note_unrecovered(
                            extraction_result,
                            &ocr_config_resolved,
                            (&render_doc, page_idx),
                            raster.3,
                            super::tall_source::Unrecovered::Config(reason),
                        );
                    }
                    result
                };
                crate::engine::seams::emit_ocr_page(page_idx + 1, page_rotations.len(), backend.name());
                let mut extraction_result = match result {
                    Ok(extraction_result) => extraction_result,
                    Err(error @ crate::XbergError::Cancelled) => return Err(error.into()),
                    Err(error) => {
                        failed_pages.insert(page_idx, error);
                        continue;
                    }
                };
                #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
                if correction_degrees == 0
                    && !alternative_raster_pages.contains(&((page_idx + 1) as u32))
                    && let Some(layer) = native_coverage_layers
                        .filter(|_| super::document::native_preservation_is_automatic(config, (page_idx + 1) as u32))
                        .and_then(|layers| layers.get(&((page_idx + 1) as u32)))
                    && crate::plugins::registry::get_ocr_backend_registry()
                        .read()
                        .is_builtin_tesseract(backend)
                {
                    let page_number = (page_idx + 1) as u32;
                    if super::native_word_coverage::covers_complete_native_page(
                        render_doc.as_ref(),
                        page_idx,
                        layer,
                        &extraction_result,
                    ) {
                        native_words_covered_pages.insert(page_number);
                    } else if super::native_word_coverage::native_text_paints_all_read_ink(
                        render_doc.as_ref(),
                        page_idx,
                        layer,
                        &extraction_result,
                        super::native_word_coverage::InkProofLimits {
                            thresholds: &ocr_output_thresholds,
                            max_working_bytes: super::hidden_layer::preview_byte_limit(config),
                            cancelled: &|| config.cancel_token.as_ref().is_some_and(|token| token.is_cancelled()),
                        },
                    ) {
                        native_ink_owned_pages.insert(page_number);
                    }
                }
                crate::extraction::derive::adopt_public_ocr_document(
                    &mut extraction_result,
                    config.security_limits.as_ref().unwrap_or(&Default::default()),
                )?;
                if let Some(metadata) = extraction_result.metadata.image_preprocessing.clone() {
                    preprocessing_by_page.insert((page_idx + 1) as u32, metadata);
                }
                undo_upright_raster_correction(
                    &mut extraction_result,
                    correction_degrees,
                    upright_width,
                    upright_height,
                );
                if let Some(usage) = extraction_result.llm_usage.take() {
                    accumulated_llm_usage.extend(usage);
                }
                let page_dims = encoded
                    .iter()
                    .find(|(encoded_page, ..)| *encoded_page == page_idx)
                    .map(|(_, _, w, h)| (*w, *h));
                for mut formula in std::mem::take(&mut extraction_result.formulas) {
                    formula.page = Some((page_idx + 1) as u32);
                    if let Some((w, h)) = page_dims {
                        formula_bbox_to_page_points(
                            &mut formula,
                            &render_doc,
                            page_idx,
                            Some(&extraction_result.metadata),
                            w,
                            h,
                        );
                    }
                    accumulated_formulas.push(formula);
                }
                // The backend's own warnings used to be dropped on this route (#60).
                crate::core::diagnostics::dedup_extend_warnings(
                    &mut accumulated_warnings,
                    std::mem::take(&mut extraction_result.processing_warnings),
                );
                let (width, height) = encoded
                    .iter()
                    .find(|(encoded_page, ..)| *encoded_page == page_idx)
                    .map_or((0, 0), |(_, _, w, h)| (*w, *h));
                let (page_width_pt, page_height_pt) = page_dimensions_pt(&render_doc, page_idx);
                let page_psm = effective_tesseract_psm(&extraction_result.metadata, backend.name());
                if let Some((mut page_doc, paragraphs)) = build_mixed_ocr_page_document(
                    &mut extraction_result,
                    &ocr_config_resolved,
                    (page_idx + 1) as u32,
                    width,
                    height,
                    page_width_pt,
                    page_height_pt,
                    crate::pdf::native::text::PageMarginFractions::from_extraction_config(Some(config)),
                ) {
                    crate::core::diagnostics::dedup_extend_warnings(
                        &mut accumulated_warnings,
                        std::mem::take(&mut page_doc.processing_warnings),
                    );
                    if !paragraphs.is_empty() {
                        ocr_page_paragraphs.insert((page_idx + 1) as u32, paragraphs);
                    }
                    if let Some(psm) = page_psm {
                        attach_page_segmentation_modes(&mut page_doc, [((page_idx + 1) as u32, psm)]);
                    }
                    structured_ocr_pages.insert((page_idx + 1) as u32, page_doc);
                }
                if let Some(conf) = mean_text_conf_of(&extraction_result.metadata.additional) {
                    page_mean_confidence.insert((page_idx + 1) as u32, conf);
                }
                if let Some(words) = word_count_of(&extraction_result.metadata.additional) {
                    page_word_count.insert((page_idx + 1) as u32, words);
                }
                if let Some(ratio) = extraction_result
                    .metadata
                    .additional
                    .get(crate::ocr_metadata_keys::OCR_TESSERACT_DICT_INVALID_WORD_RATIO_METADATA_KEY)
                    .and_then(serde_json::Value::as_f64)
                {
                    page_dictionary_invalid_word_ratio.insert((page_idx + 1) as u32, ratio);
                }
                ocr_results.insert((page_idx + 1) as u32, extraction_result.content);
            }
        }
        #[cfg(any(not(feature = "tokio-runtime"), target_arch = "wasm32"))]
        {
            for (page_idx, data, width, height) in &encoded {
                if config.cancel_token.as_ref().is_some_and(|t| t.is_cancelled()) {
                    break;
                }
                let page_rotation_degrees = page_rotations.get(*page_idx).copied().unwrap_or(0);
                let source_dpi = rendered_page_source_dpi(&render_doc, *page_idx, *width);
                let whole_page_raster =
                    crate::pdf::scan_detect::full_page_raster_density(&render_doc, *page_idx).is_some();
                #[cfg(feature = "layout-detection")]
                let layout_assembly = detection_for_mixed_route_page(layout_detections_for_mixed, *page_idx).is_some();
                #[cfg(not(feature = "layout-detection"))]
                let layout_assembly = false;
                let config_for_page = ocr_config_with_page_rotation_hint(
                    &ocr_config_owned,
                    page_rotation_degrees,
                    source_dpi,
                    whole_page_raster,
                    single_block_pages.contains(&(*page_idx as u32 + 1)),
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
                let result = async {
                    let retained_bytes = retained_ocr_source_bytes(data, &upright_data, 0);
                    let (original, effective) = crate::extractors::image::reserve_retained_ocr_raster(
                        backend,
                        (ocr_config_resolved.clone(), config_for_page.into_owned()),
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
                crate::engine::seams::emit_ocr_page(*page_idx + 1, page_rotations.len(), backend.name());
                let mut extraction_result = match result {
                    Ok(extraction_result) => extraction_result,
                    Err(error @ crate::XbergError::Cancelled) => return Err(error.into()),
                    Err(error) => {
                        failed_pages.insert(*page_idx, error);
                        continue;
                    }
                };
                crate::extraction::derive::adopt_public_ocr_document(
                    &mut extraction_result,
                    config.security_limits.as_ref().unwrap_or(&Default::default()),
                )?;
                if let Some(metadata) = extraction_result.metadata.image_preprocessing.clone() {
                    preprocessing_by_page.insert((*page_idx + 1) as u32, metadata);
                }
                undo_upright_raster_correction(
                    &mut extraction_result,
                    correction_degrees,
                    upright_width,
                    upright_height,
                );
                if let Some(usage) = extraction_result.llm_usage.take() {
                    accumulated_llm_usage.extend(usage);
                }
                for mut formula in std::mem::take(&mut extraction_result.formulas) {
                    formula.page = Some((*page_idx + 1) as u32);
                    formula_bbox_to_page_points(
                        &mut formula,
                        &render_doc,
                        *page_idx,
                        Some(&extraction_result.metadata),
                        *width,
                        *height,
                    );
                    accumulated_formulas.push(formula);
                }
                crate::core::diagnostics::dedup_extend_warnings(
                    &mut accumulated_warnings,
                    std::mem::take(&mut extraction_result.processing_warnings),
                );
                let (page_width_pt, page_height_pt) = page_dimensions_pt(&render_doc, *page_idx);
                let page_psm = effective_tesseract_psm(&extraction_result.metadata, backend.name());
                if let Some((mut page_doc, paragraphs)) = build_mixed_ocr_page_document(
                    &mut extraction_result,
                    &ocr_config_resolved,
                    (*page_idx + 1) as u32,
                    *width,
                    *height,
                    page_width_pt,
                    page_height_pt,
                    crate::pdf::native::text::PageMarginFractions::from_extraction_config(Some(config)),
                ) {
                    crate::core::diagnostics::dedup_extend_warnings(
                        &mut accumulated_warnings,
                        std::mem::take(&mut page_doc.processing_warnings),
                    );
                    if !paragraphs.is_empty() {
                        ocr_page_paragraphs.insert((*page_idx + 1) as u32, paragraphs);
                    }
                    if let Some(psm) = page_psm {
                        attach_page_segmentation_modes(&mut page_doc, [((*page_idx + 1) as u32, psm)]);
                    }
                    structured_ocr_pages.insert((*page_idx + 1) as u32, page_doc);
                }
                if let Some(conf) = mean_text_conf_of(&extraction_result.metadata.additional) {
                    page_mean_confidence.insert((*page_idx + 1) as u32, conf);
                }
                if let Some(words) = word_count_of(&extraction_result.metadata.additional) {
                    page_word_count.insert((*page_idx + 1) as u32, words);
                }
                if let Some(ratio) = extraction_result
                    .metadata
                    .additional
                    .get(crate::ocr_metadata_keys::OCR_TESSERACT_DICT_INVALID_WORD_RATIO_METADATA_KEY)
                    .and_then(serde_json::Value::as_f64)
                {
                    page_dictionary_invalid_word_ratio.insert((*page_idx + 1) as u32, ratio);
                }
                ocr_results.insert((*page_idx + 1) as u32, extraction_result.content);
            }
        }

        // A page whose OCR came back blank or failed is retried on its embedded image XObjects,
        // as the whole-document route does (#1355, #1444). The pipeline branch above gets the
        // same retry inside each stage, from the document it hands the runner (#1912).
        // A page whose source image the render budget refused has no raster to inspect: its embedded
        // images are the only place the text can come from. ~keep
        let retry_pages = encoded
            .iter()
            .map(|(page_idx, png, _, _)| (page_idx, Some(png.as_slice())))
            .chain(budget_refused_pages.iter().map(|page_idx| (page_idx, None)))
            .filter(|_| !probing_hidden_layer);
        for (page_idx, png) in retry_pages {
            let page_number = (page_idx + 1) as u32;
            let failure = failed_pages.remove(page_idx);
            let page_text = ocr_results.get(&page_number).map_or("", String::as_str);
            let trigger = match png {
                Some(png) => page_needs_xobject_fallback(page_text, png, security_limits),
                None => super::rendering::XObjectFallbackTrigger {
                    needs_fallback: true,
                    draw_failed: true,
                },
            };
            let recovery = if trigger.needs_fallback {
                recover_page_text_from_image_xobjects(
                    backend,
                    &render_doc,
                    *page_idx,
                    page_rotations.len(),
                    &ocr_config_owned,
                    XObjectRecoveryContext {
                        budget: &mut xobject_recovery_budget,
                        blank_raster_warnings: trigger.draw_failed.then_some(&mut accumulated_warnings),
                        no_admitted_images: None,
                        stored_jpegs: None,
                        cancel_token: config.cancel_token.as_ref(),
                        use_proven_source_dpi: true,
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
                used_proven_source_dpi,
                possibly_illegible,
            }) = recovery
            {
                retry_causes = failure_causes;
                let text = judge_proven_read(
                    text,
                    ProvenRead {
                        used_proven_source_dpi,
                        possibly_illegible,
                        page_idx: *page_idx,
                        warn_when_kept: false,
                    },
                    &ocr_output_thresholds,
                    &mut accumulated_warnings,
                    &mut retry_causes,
                );
                let recovered_tables = !tables.is_empty();
                let recovered_text_is_empty = text.trim().is_empty();
                if should_adopt_xobject_retry_text(page_text, &text) {
                    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
                    alternative_raster_pages.insert(page_number);
                    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
                    native_words_covered_pages.remove(&page_number);
                    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
                    native_ink_owned_pages.remove(&page_number);
                    // The render's paragraphs, confidence and word count describe the blank
                    // render, not the recovered text.
                    let recovered_paragraphs =
                        crate::pdf::structure::adapters::ocr_text_to_paragraphs(&text_outside_tables);
                    if recovered_paragraphs.is_empty() {
                        ocr_page_paragraphs.remove(&page_number);
                    } else {
                        ocr_page_paragraphs.insert(page_number, recovered_paragraphs);
                    }
                    page_mean_confidence.remove(&page_number);
                    page_word_count.remove(&page_number);
                    page_dictionary_invalid_word_ratio.remove(&page_number);
                    // The recovered tables print themselves beside these paragraphs (#2014).
                    structured_ocr_pages.insert(
                        page_number,
                        super::document::flat_ocr_page_document(&text_outside_tables),
                    );
                    ocr_results.insert(page_number, text);
                    recovered = true;
                }
                if !tables.is_empty() {
                    let current_text = ocr_results.get(&page_number).map_or("", String::as_str);
                    let page_doc = structured_ocr_pages
                        .entry(page_number)
                        .or_insert_with(|| super::document::flat_ocr_page_document(current_text));
                    super::document::attach_page_ocr_payload(page_doc, tables, Vec::new(), page_number);
                    if recovered_text_is_empty {
                        table_only_xobject_recovery_pages.insert(page_number);
                    }
                }
                recovered |= recovered_tables;
                if recovered
                    && let Some(psm) = tesseract_psm
                    && let Some(page_doc) = structured_ocr_pages.get_mut(&page_number)
                {
                    attach_page_segmentation_modes(page_doc, [(page_number, psm)]);
                }
                accumulated_llm_usage.append(&mut llm_usage);
                accumulated_formulas.append(&mut formulas);
                if let Some(metadata) = image_preprocessing {
                    preprocessing_by_page.insert(page_number, metadata);
                }
                if capture_rasters {
                    captured_rasters.extend(images);
                }
                accumulated_warnings.push(xobject_fallback_warning(*page_idx, attempted, trigger.draw_failed));
            }
            if let Some(error) = failure {
                if !recovered {
                    let error = with_retry_causes(error, &retry_causes);
                    let native_recovered = mixed_native_page_has_content(native_text, boundaries, *page_idx);
                    push_mixed_ocr_page_failure_warning(
                        &mut accumulated_warnings,
                        &mut page_failures,
                        *page_idx,
                        &error,
                        native_recovered,
                    );
                    failed_page_errors.insert(*page_idx, error);
                    continue;
                }
                let recovered_kind = if ocr_results
                    .get(&page_number)
                    .is_some_and(|text| !text.trim().is_empty())
                {
                    "text"
                } else {
                    "content"
                };
                accumulated_warnings.push(crate::types::ProcessingWarning {
                    source: std::borrow::Cow::Borrowed("ocr"),
                    message: std::borrow::Cow::Owned(format!(
                        "OCR of page {page_number} failed ({error}); its {recovered_kind} was recovered from the page's \
                         embedded image XObjects instead."
                    )),
                });
                merge_ocr_page_failures(
                    &mut page_failures,
                    vec![crate::types::OcrPageFailure {
                        page: page_number,
                        error: error.to_string(),
                        recovered: true,
                    }],
                );
            }
        }

        if capture_rasters {
            for (page_idx, png_arc, w, h) in &encoded {
                let png_bytes = bytes::Bytes::copy_from_slice(png_arc.as_ref());
                captured_rasters.push(build_page_raster_image(*page_idx, png_bytes, *w, *h));
            }
        }
    }

    if all_pages_failed_policy == AllPagesFailedPolicy::ReturnError
        && !page_indices.is_empty()
        && page_indices
            .iter()
            .all(|page_idx| failed_page_errors.contains_key(page_idx))
    {
        let error = page_indices
            .iter()
            .find_map(|page_idx| failed_page_errors.remove(page_idx))
            .unwrap_or_else(|| crate::XbergError::Ocr {
                message: "OCR failed for every requested page".to_string(),
                source: None,
            });
        return Err(FailedOcrRun { error, page_failures });
    }

    // Pipeline stages already assess their output in `extract_with_ocr_for_page` using the
    // actual producing backend. Assess only the direct single-backend route here to avoid
    // duplicate warnings and guessed pipeline confidence semantics. ~keep
    if let Some(producing_backend) = backend.as_ref() {
        let confidence_semantics = producing_backend.confidence_semantics();
        let producing_backend_name = producing_backend.name().to_string();
        for (page_number, text) in &mut ocr_results {
            let confidence = page_mean_confidence.get(page_number).copied();
            let dictionary_ratio = page_dictionary_invalid_word_ratio.get(page_number).copied();
            tracing::debug!(page = *page_number, ?confidence, "OCR page mean confidence");
            // Recorded before the accept/reject call below, and never retracted afterwards: a
            // page the noise gate discards was still OCR'd, and its confidence is exactly the
            // evidence a caller needs to understand why it was discarded (#1568). ~keep
            if let Some(summary) = page_ocr_confidence(
                confidence_semantics,
                confidence,
                page_word_count.get(page_number).copied().unwrap_or(0),
                &producing_backend_name,
            ) {
                ocr_confidence_by_page.insert(*page_number, summary);
            }
            let acceptance = accept_or_reject_ocr_page(
                (*page_number as usize).saturating_sub(1),
                std::mem::take(text),
                &ocr_output_thresholds,
                &mut accumulated_warnings,
                dictionary_ratio,
                confidence_semantics,
                confidence,
            );
            // The verdict decides whether the page keeps its text layer (see below). A page
            // the noise gate discarded has blank text and keeps the layer already. ~keep
            if acceptance.verdict.is_some() && !acceptance.discarded {
                noise_verdict_pages.insert(*page_number);
            }
            if acceptance.verdict.is_some_and(|verdict| verdict.low_confidence) {
                low_confidence_pages.insert(*page_number);
            }
            if acceptance.verdict.is_some_and(|verdict| verdict.dictionary_noise) {
                dictionary_noise_pages.insert(*page_number);
            }
            *text = acceptance.content;
        }
    }

    for text in ocr_results.values_mut() {
        if let std::borrow::Cow::Owned(repaired) = repair_ocr_list_markers(text) {
            *text = repaired;
        }
    }

    // Opt-in (GH#1789): unlike the list-marker repair above, this can misjudge a genuine
    // European-locale decimal, so it only runs when the caller has asserted the source
    // documents use the US/UK number convention. See `OcrConfig::numeric_repair`'s doc comment.
    if numeric_repair_enabled(config) {
        for text in ocr_results.values_mut() {
            if let std::borrow::Cow::Owned(repaired) = repair_ocr_numeric_tokens(text) {
                *text = repaired;
            }
        }
    }

    let mut accepted_replacements =
        accepted_ocr_page_replacements(native_text, boundaries, &ocr_results, &ocr_output_thresholds);
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    for boundary in boundaries {
        let page = boundary.page_number;
        if pages.native_script.contains(&page)
            && let Some(text) = accepted_replacements.get(&page)
            && let Some(native) = native_text.get(boundary.byte_start..boundary.byte_end)
            && super::super::unreadable_script::ocr_dropped_native_script(native, text)
        {
            accepted_replacements.remove(&page);
            structured_ocr_pages.remove(&page);
            ocr_confidence_by_page.remove(&page);
            preprocessing_by_page.remove(&page);
            crate::core::diagnostics::push_warning_deduped(
                &mut accumulated_warnings,
                super::super::unreadable_script::ocr_dropped_script_warning(page, native),
            );
        }
    }
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    if probing_hidden_layer || !pages.preserve_native.is_empty() {
        for boundary in boundaries {
            let page = boundary.page_number;
            if (probing_hidden_layer || pages.preserve_native.contains(&page))
                && let Some(text) = accepted_replacements.get(&page)
                && (noise_verdict_pages.contains(&page)
                    || !native_text
                        .get(boundary.byte_start..boundary.byte_end)
                        .is_some_and(|native| {
                            super::hidden_layer::adds_text_preserving_native(native, text)
                                && (probing_hidden_layer
                                    || super::hidden_layer::keeps_native_lines_in_order(native, text))
                        }))
                // Every probed page passed the hidden-text audit: all of its native text is render
                // mode 3. A confident read has a calibrated score that clears the default gate and
                // no low-confidence or dictionary-noise verdict. The fragmentation signal does not
                // veto it: a table of numbers is mostly short tokens. ~keep
                && !(probing_hidden_layer
                    && native_text
                        .get(boundary.byte_start..boundary.byte_end)
                        .is_some_and(|native| {
                            super::hidden_layer::confident_read_replaces_layer(
                                low_confidence_pages.contains(&page),
                                dictionary_noise_pages.contains(&page),
                                ocr_confidence_by_page.get(&page).and_then(|confidence| confidence.score),
                                native,
                                text,
                            )
                        }))
            {
                accepted_replacements.remove(&page);
            }
        }
    }

    // The replacement is decided on the OCR text alone. A page that it replaces, whose OCR text
    // got the noise verdict or confirms the layer, now gets the layer lines that its OCR text
    // lacks first. A page it keeps native stays as it was. ~keep
    let mut page_layers_first: ahash::AHashMap<u32, super::document::LayerFirst> = ahash::AHashMap::new();
    let mut complete_native_pages = std::collections::HashSet::new();
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    let mut clean_native_ink_pages = std::collections::HashSet::new();
    let native_preservation_enabled = effective_pipeline.is_none() && !probing_hidden_layer;
    #[cfg(feature = "layout-detection")]
    let native_preservation_enabled = native_preservation_enabled && resolved_layout_inputs.is_none();
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32"), feature = "layout-detection"))]
    let ink_proof_layout_free = resolved_layout_inputs.is_none();
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32"), not(feature = "layout-detection")))]
    let ink_proof_layout_free = true;
    if let Some(layers) = page_layers.as_ref() {
        let mut geometry = MergeGeometrySession::new(content, config);
        for (page_number, text) in &mut accepted_replacements {
            if geometry.cancelled() {
                break;
            }
            let ocr_scale = match ocr_raster_by_page.get(page_number) {
                Some(&(width_px, height_px)) => {
                    let (width_pt, height_pt) = page_dimensions_pt(&render_doc, page_number.saturating_sub(1) as usize);
                    (width_px > 0 && height_px > 0).then(|| (width_pt / width_px as f32, height_pt / height_px as f32))
                }
                None => Some((1.0, 1.0)),
            };
            let boxes = layers.get(page_number).and_then(|layer| {
                if noise_verdict_pages.contains(page_number)
                    || !super::document::ocr_confirms_layer(layer, text, &ocr_output_thresholds)
                {
                    return None;
                }
                geometry.page_boxes(*page_number, ocr_page_paragraphs.get(page_number), ocr_scale)
            });
            #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
            let clean_ink_layer = native_coverage_layers
                .and_then(|layers| layers.get(page_number))
                .map(String::as_str)
                .filter(|layer| !layer.contains(super::super::unmapped_glyphs::LOST_GLYPH_MARKER));
            #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
            if native_ink_owned_pages.contains(page_number)
                && ink_proof_layout_free
                && super::document::native_preservation_is_automatic(config, *page_number)
                && !noise_verdict_pages.contains(page_number)
                && structured_ocr_pages.get(page_number).is_some_and(|doc| {
                    super::native_word_coverage::has_only_certified_tesseract_text(doc)
                        || super::native_word_coverage::ink_owned_table_structure_can_yield_to_native(
                            doc,
                            text,
                            clean_ink_layer,
                        )
                })
                && layers.get(page_number).is_some_and(|layer| {
                    let mut stripped = layer.clone();
                    super::super::unmapped_glyphs::strip_native_fragment(&mut stripped);
                    super::document::ocr_confirms_layer(&stripped, text, &ocr_output_thresholds)
                })
            {
                complete_native_pages.insert(*page_number);
                // Marker-bearing candidates can own ink too; only this clean-layer admission permits omissions. ~keep
                if clean_ink_layer.is_some() {
                    clean_native_ink_pages.insert(*page_number);
                }
            }
            if native_preservation_enabled
                && super::document::native_preservation_is_automatic(config, *page_number)
                && !noise_verdict_pages.contains(page_number)
                && structured_ocr_pages
                    .get(page_number)
                    .is_some_and(super::document::ocr_page_has_only_plain_payload)
                && layers.get(page_number).is_some_and(|layer| {
                    super::document::native_page_covers_ocr(layer, text, &ocr_output_thresholds, boxes.as_ref())
                })
            {
                complete_native_pages.insert(*page_number);
            }
            if let Some(layer) = layers.get(page_number)
                && let Some(mut first) = super::document::layer_lines_to_put_first(
                    layer,
                    text,
                    noise_verdict_pages.contains(page_number),
                    &ocr_output_thresholds,
                    boxes.as_ref(),
                )
            {
                let lost = super::super::unmapped_glyphs::strip_native_fragment(&mut first.lines);
                if let Some(warning) = super::super::unmapped_glyphs::lost_glyph_warning(lost, config) {
                    crate::core::diagnostics::push_warning_deduped(&mut accumulated_warnings, warning);
                }
                if first.lines.is_empty() {
                    continue;
                }
                #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
                if native_words_covered_pages.contains(page_number)
                    && !noise_verdict_pages.contains(page_number)
                    && structured_ocr_pages
                        .get(page_number)
                        .is_some_and(super::native_word_coverage::has_only_certified_tesseract_text)
                    && super::document::unique_literal_embedding(layer, text)
                {
                    complete_native_pages.insert(*page_number);
                }
                *text = super::document::text_with_layer_first(text, &first);
                page_layers_first.insert(*page_number, first);
            }
        }
    }
    super::super::ensure_pdf_not_cancelled(config)?;
    structured_ocr_pages
        .retain(|page, _| accepted_replacements.contains_key(page) || table_only_xobject_recovery_pages.contains(page));
    retain_ocr_formulas_for_accepted_pages(&mut accumulated_formulas, &accepted_replacements);

    // Document-global heading/list heuristic (same font-clustering pass the OCR-only
    // route runs via `extract_with_ocr_for_page`, previously unreachable from this mixed
    // route -- see `ocr_page_paragraphs`'s own comment above). Runs once, over every
    // accepted OCR'd page's paragraphs at once: the heuristic clusters font sizes
    // document-wide and needs several pages in hand, which is exactly what the per-page
    // callers above could not give it on their own.
    if !structured_ocr_pages.is_empty() {
        let mut pages_for_heuristic: Vec<Vec<crate::pdf::structure::types::PdfParagraph>> =
            vec![Vec::new(); page_count];
        for (&page_number, paragraphs) in &ocr_page_paragraphs {
            // `structured_ocr_pages` (not `ocr_page_paragraphs`) is the source of truth for
            // which pages are still in play: the destructive noise-filter opt-in above may
            // have dropped a page after its paragraphs were already collected.
            if structured_ocr_pages.contains_key(&page_number)
                && let Some(slot) = pages_for_heuristic.get_mut((page_number - 1) as usize)
            {
                *slot = paragraphs.clone();
            }
        }
        let tables_for_heuristic: Vec<crate::types::Table> = structured_ocr_pages
            .values()
            .flat_map(|doc| doc.tables.iter().cloned())
            .collect();
        let mut ocr_page_numbers: Vec<u32> = structured_ocr_pages.keys().copied().collect();
        ocr_page_numbers.sort_unstable();
        let page_heights_for_heuristic = (0..page_count)
            .map(|page_index| page_dimensions_pt(&render_doc, page_index).1)
            .collect::<Vec<_>>();
        let mut split_pages = heuristically_restructured_ocr_pages(
            &pages_for_heuristic,
            &page_heights_for_heuristic,
            &tables_for_heuristic,
            config,
        )
        .map(|combined_doc| split_document_global_ocr_structure_by_page(combined_doc, &ocr_page_numbers))
        .unwrap_or_default();
        for page_number in &ocr_page_numbers {
            let Some(existing) = structured_ocr_pages.get(page_number) else {
                continue;
            };
            if !ocr_page_paragraphs.contains_key(page_number) {
                continue;
            }
            let new_page_doc = match split_pages.remove(page_number) {
                // The heuristic's combined document has no notion of the backend's raw
                // per-word OCR elements, this page's earlier warnings, or its OCR coordinate
                // frame -- all three come from the fallback per-page document already built
                // above (`carry_page_ocr_payload_forward`); only the *structural* elements
                // (headings/paragraphs/list items/tables) come from the document-global pass.
                Some(mut new_page_doc) => {
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
                    let mut paragraphs = pages_for_heuristic
                        .get((*page_number - 1) as usize)
                        .cloned()
                        .unwrap_or_default();
                    apply_ocr_text_list_fallback(&mut paragraphs);
                    // #729: recover a bare marker no ML hint ever classified, e.g. a
                    // marker paragraph whose body landed several paragraphs away --
                    // see `adapters::reattach_detached_ocr_list_markers`'s doc comment.
                    // #760: threaded with this page's known PDF `/Rotate` so the
                    // reattachment's baseline/indent comparison runs in the rotation-
                    // corrected frame instead of the raw raster one.
                    let page_rotation_degrees = page_rotations.get((*page_number - 1) as usize).copied().unwrap_or(0);
                    crate::pdf::structure::adapters::reattach_detached_ocr_list_markers(
                        &mut paragraphs,
                        page_rotation_degrees,
                    );
                    let mut new_page_doc = crate::pdf::structure::assemble_internal_document(
                        vec![paragraphs],
                        &existing.tables,
                        Some(&existing.images),
                        &[],
                        &Default::default(),
                    );
                    carry_page_ocr_payload_forward(existing, &mut new_page_doc);
                    new_page_doc
                }
            };
            structured_ocr_pages.insert(*page_number, new_page_doc);
        }
    }

    // GH#1840: the flat-string repair above only reaches the rendered output for Plain format.
    // Every other renderer rebuilds each OCR'd page from `structured_ocr_pages`
    // (`select_pdf_document` -> `merge_structured_ocr_pages_into_internal_document`), and the block
    // above has just rebuilt every page in that map from `ocr_page_paragraphs`, so this has to run
    // here rather than beside the repair at the top. ~keep
    // GH#1840: the flat-string repair above only reaches the rendered output for Plain format.
    // Every other renderer rebuilds each OCR'd page from `structured_ocr_pages`
    // (`select_pdf_document` -> `merge_structured_ocr_pages_into_internal_document`), and the block
    // above has just rebuilt every page in that map from `ocr_page_paragraphs`, so this has to run
    // here rather than beside the repair at the top. ~keep
    if numeric_repair_enabled(config) {
        apply_numeric_repair_to_structured_ocr_pages(&mut structured_ocr_pages);
    }
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    let mut type0_loss_pages = ahash::AHashSet::new();
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    let mut lost_glyph_ink_pages = ahash::AHashMap::new();
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    if !probing_hidden_layer
        && !mixed_route_layout_active
        && config.ocr_strategy == crate::core::config::OcrStrategy::Auto
        && config.ocr.as_ref().is_some_and(uses_default_ocr_pipeline)
    {
        let direct_tesseract = native_preservation_enabled
            && backend.as_ref().is_some_and(|backend| {
                crate::plugins::registry::get_ocr_backend_registry()
                    .read()
                    .is_builtin_tesseract(backend)
            });
        for boundary in boundaries {
            let page = boundary.page_number;
            let native = if page_count == 1 && page == 1 {
                pages.single_native_type0_page
            } else {
                None
            }
            .or_else(|| native_text.get(boundary.byte_start..boundary.byte_end));
            if (direct_tesseract || original_tesseract_raster_pages.contains(&page))
                && !alternative_raster_pages.contains(&page)
                && raster_dpis
                    .get(&(page.saturating_sub(1) as usize))
                    .is_some_and(|raster| raster.type0_ink_loss)
                && accepted_replacements.contains_key(&page)
                && let (Some(native), Some(read)) = (native, ocr_results.get(&page))
                && super::document::type0_native_is_corroborated(
                    native,
                    read,
                    &ocr_output_thresholds,
                    native_ink_owned_pages.contains(&page),
                    native_words_covered_pages.contains(&page),
                )
            {
                complete_native_pages.insert(page);
                type0_loss_pages.insert(page);
            }
            if native_ink_owned_pages.contains(&page)
                && ink_proof_layout_free
                && native.is_some_and(|native| native.contains(super::super::unmapped_glyphs::LOST_GLYPH_MARKER))
                && accepted_replacements.contains_key(&page)
                && !noise_verdict_pages.contains(&page)
            {
                complete_native_pages.insert(page);
                let lost = native.map_or(0, |native| {
                    native.matches(super::super::unmapped_glyphs::LOST_GLYPH_MARKER).count()
                });
                lost_glyph_ink_pages.insert(page, lost);
            }
        }
    }
    // Reject only certified automatic replacements, preserving the original native page objects.
    // Recheck the final OCR structure because the document-global pass can add rich payload. ~keep
    for page in complete_native_pages {
        let plain = structured_ocr_pages
            .get(&page)
            .zip(ocr_results.get(&page))
            .is_some_and(|(doc, text)| {
                if super::document::plain_ocr_page_can_yield_to_native(doc, text) {
                    return true;
                }
                #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
                if native_words_covered_pages.contains(&page) {
                    return super::native_word_coverage::has_only_certified_tesseract_text(doc)
                        && super::document::ocr_elements_follow_text(doc, text);
                }
                #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
                if native_ink_owned_pages.contains(&page) {
                    let clean_ink_layer = native_coverage_layers
                        .filter(|_| clean_native_ink_pages.contains(&page))
                        .and_then(|layers| layers.get(&page))
                        .map(String::as_str);
                    return super::native_word_coverage::only_segments_certified_tesseract_text(doc, text)
                        || super::native_word_coverage::ink_owned_table_structure_can_yield_to_native(
                            doc,
                            text,
                            clean_ink_layer,
                        );
                }
                false
            });
        let has_formula = accumulated_formulas
            .iter()
            .any(|formula| formula.page.is_none_or(|owner| owner == page));
        if plain && !has_formula && super::document::native_preservation_is_automatic(config, page) {
            accepted_replacements.remove(&page);
            structured_ocr_pages.remove(&page);
            ocr_confidence_by_page.remove(&page);
            preprocessing_by_page.remove(&page);
            #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
            if let Some(lost) = lost_glyph_ink_pages.get(&page) {
                if all_pages_failed_policy == AllPagesFailedPolicy::ReturnError
                    && pages.single_native_type0_page.is_some()
                {
                    return Err(lost_glyph_ocr_rejection(*lost));
                }
                crate::core::diagnostics::push_warning_deduped(
                    &mut accumulated_warnings,
                    crate::core::diagnostics::warning(
                        "ocr",
                        format!(
                            "Page {page} retained native text; {lost} glyphs have no Unicode value and its OCR read did not recover them, only re-read glyph ink the native text paints."
                        ),
                    ),
                );
            }
            #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
            if type0_loss_pages.contains(&page) {
                if all_pages_failed_policy == AllPagesFailedPolicy::ReturnError
                    && page_count == 1
                    && pages.single_native_type0_page.is_some()
                {
                    return Err(type0_ocr_rejection());
                }
                crate::core::diagnostics::push_warning_deduped(
                    &mut accumulated_warnings,
                    crate::core::diagnostics::warning(
                        "ocr",
                        format!("Page {page} retained native text after its OCR render lost Type0 glyph ink."),
                    ),
                );
            }
        }
    }
    // The structured page keeps the layer first as well, after the passes above rebuilt it. ~keep
    for (page_number, first) in &page_layers_first {
        if accepted_replacements.contains_key(page_number)
            && let Some(page_doc) = structured_ocr_pages.get_mut(page_number)
        {
            super::document::put_layer_first_in_document(page_doc, None, first);
        }
    }

    if !config.cancel_token.as_ref().is_some_and(|token| token.is_cancelled()) {
        let retained_formula_pages: ahash::AHashSet<_> =
            accumulated_formulas.iter().filter_map(|formula| formula.page).collect();
        for &page_idx in &page_indices {
            let page = (page_idx + 1) as u32;
            if (ocr_results.get(&page).is_none_or(|text| text.trim().is_empty()) || noise_verdict_pages.contains(&page))
                && !table_only_xobject_recovery_pages.contains(&page)
                && !retained_formula_pages.contains(&page)
                && structured_ocr_pages.get(&page).is_none_or(|doc| doc.tables.is_empty())
                && let Some(dpi) = raster_dpis.get(&page_idx)
                && let Some(warning) = dpi.empty_text_warning(page_idx)
            {
                crate::core::diagnostics::push_warning_deduped(&mut accumulated_warnings, warning);
            }
        }
    }

    #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
    for (page, rewrite) in caption_rewrites {
        if let Some(text) = accepted_replacements.get_mut(&page)
            && let Some(document) = structured_ocr_pages.get_mut(&page)
        {
            rewrite.apply(document, text, page);
        }
    }

    let result = apply_ocr_page_replacements(native_text, boundaries, &accepted_replacements);
    page_failures.sort_by_key(|failure| failure.page);

    #[cfg(feature = "formula-recognition")]
    let formula_layout_inputs = formula_layout_inputs_for_replaced_pages(
        resolved_layout_inputs,
        &accepted_replacements,
        config.layout.as_ref(),
    );
    Ok(MixedOcrRun {
        output: (
            result,
            accepted_replacements,
            structured_ocr_pages,
            accumulated_llm_usage,
            if capture_rasters { Some(captured_rasters) } else { None },
            accumulated_formulas,
            preprocessing_by_page,
            ocr_confidence_by_page,
            accumulated_warnings,
            page_failures,
        ),
        #[cfg(feature = "formula-recognition")]
        formula_layout_inputs,
    })
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
    let marker_config = config.pages.as_ref().filter(|pages| pages.insert_page_markers);
    let mut text = String::new();
    let mut boundaries = Vec::with_capacity(page_count);
    for page_index in 0..page_count {
        if let Some(pages) = marker_config {
            text.push_str(&pages.marker_format.replace("{page_num}", &(page_index + 1).to_string()));
        } else if page_index > 0 {
            text.push_str("\n\n");
        }
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
async fn try_gray_jpeg_render_recovery(
    content: Option<&[u8]>,
    config: &ExtractionConfig,
    failed: &FailedOcrRun,
) -> crate::Result<Option<WholeDirectOcrOutput>> {
    use std::error::Error as _;

    if !failed
        .error
        .source()
        .is_some_and(|source| source.is::<super::rendering::SourceImageBudgetExceeded>())
        || config.ocr_strategy != crate::core::config::OcrStrategy::Auto
        || !super::document::native_preservation_is_automatic(config, 1)
        || config.pages.as_ref().is_some_and(|pages| pages.insert_page_markers)
    {
        return Ok(None);
    }
    let ocr = super::resolve_ocr_config(config);
    // No layout pass ran for the refused page, so its detections cannot be checked; a layout
    // request alone keeps the strips from replacing a read the caller wanted laid out. ~keep
    #[cfg(feature = "layout-detection")]
    let layout_requested = config.layout.is_some();
    #[cfg(not(feature = "layout-detection"))]
    let layout_requested = false;
    if layout_requested || !super::tall_source::supports(config, &ocr) || !uses_default_ocr_pipeline(&ocr) {
        return Ok(None);
    }
    let Some(content) = content else { return Ok(None) };
    let Ok((document, 1, _)) = open_pdf_for_full_ocr(content) else {
        return Ok(None);
    };
    let backend = {
        let registry = crate::plugins::registry::get_ocr_backend_registry();
        let registry = registry.read();
        let Ok(backend) = registry.get(&ocr.backend) else {
            return Ok(None);
        };
        if !registry.is_builtin_tesseract(&backend) {
            return Ok(None);
        }
        backend
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
    pub(crate) source_dpi: Option<f64>,
    #[cfg(feature = "pdf")]
    pub(crate) raster_dpis: Option<std::sync::Arc<crate::pdf::render::OcrRasterDpis>>,
    /// `true` is authoritative; `false` means the detached caller has no positive scan signal,
    /// so a content-backed whole-document route still performs its normal per-page detection.
    pub(crate) known_full_page_scan: bool,
    /// One-indexed document page numbers selected by automatic fabricated-map routing.
    pub(crate) single_block_pages: Option<std::sync::Arc<std::collections::HashSet<u32>>>,
    /// Readable text layers by one-indexed document page, for a whole-document run. After the run
    /// has chosen its output, a page whose OCR text is judged recognition noise gets its layer
    /// first.
    pub(crate) native_page_layers: Option<std::sync::Arc<ahash::AHashMap<u32, String>>>,
    /// Original marker-bearing text of the only physical page in a whole automatic fallback.
    #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
    pub(crate) single_native_type0_page: Option<std::sync::Arc<str>>,
    #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
    pub(crate) native_coverage_layers: Option<std::sync::Arc<ahash::AHashMap<u32, String>>>,
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
    original_tesseract_raster: bool,
    discarded_visible_native_page: bool,
    #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
    complete_native_words: bool,
    #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
    native_ink_owned: bool,
}

#[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
fn lost_glyph_ocr_rejection(lost: usize) -> FailedOcrRun {
    crate::XbergError::Ocr {
        message: format!("Page 1 OCR read did not recover the {lost} glyphs without a Unicode value; it only re-read glyph ink that the native text paints"),
        source: None,
    }
    .into()
}

#[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
fn type0_ocr_rejection() -> FailedOcrRun {
    crate::XbergError::Ocr {
        message: "Page 1 OCR read omitted native text after its render lost Type0 glyph ink".to_owned(),
        source: None,
    }
    .into()
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
) -> OcrRunResult<(
    String,
    Option<f64>,
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
)> {
    use crate::plugins::registry::get_ocr_backend_registry;
    use image::ImageEncoder;
    use image::codecs::png::PngEncoder;
    use std::io::Cursor;

    let xobject_document = detached_page.map(|source| source.document);
    #[cfg(all(paddle_ocr, feature = "ocr", feature = "pdf", not(target_arch = "wasm32")))]
    let stored_scan = detached_page.and_then(|source| source.stored_scan);
    #[cfg(all(paddle_ocr, feature = "ocr", feature = "pdf", not(target_arch = "wasm32")))]
    let has_stored_scan = stored_scan.is_some();
    #[cfg(not(all(paddle_ocr, feature = "ocr", feature = "pdf", not(target_arch = "wasm32"))))]
    let has_stored_scan = false;
    #[cfg(feature = "layout-detection")]
    let has_layout_detections = layout_detections.is_some();
    #[cfg(not(feature = "layout-detection"))]
    let has_layout_detections = false;

    // Re-seed the built-in backends if a prior `clear_ocr_backends()` call (a real user API,
    // also exercised by binding e2e suites sharing this process) left the registry without a
    // usable default. Cheap when the default is already present -- see
    // `ensure_ocr_backends_initialized`'s own doc comment. ~keep
    crate::plugins::ensure_ocr_backends_initialized();

    // Same slice as `ocr_points_per_pixel`'s suppression above: the only two readers of
    // `points_per_pixel_override` are compiled out, but every caller still passes it. ~keep
    #[cfg(all(feature = "layout-detection", not(feature = "ocr"), not(feature = "ocr-wasm")))]
    let _ = points_per_pixel_override;

    let resolved_ocr_config = pre_page_ocr_config(config);
    let base_ocr_config = resolved_ocr_config.as_ref();

    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    let can_retry_orientation;
    #[cfg(feature = "pdf")]
    let mut no_admitted_images = false;
    #[cfg(not(feature = "pdf"))]
    let no_admitted_images = false;
    #[cfg(all(feature = "pdf", not(all(paddle_ocr, feature = "ocr", not(target_arch = "wasm32")))))]
    let can_prove_no_admitted_images = false;
    #[cfg(all(paddle_ocr, feature = "ocr", feature = "pdf", not(target_arch = "wasm32")))]
    let can_prove_no_admitted_images;
    let backend = {
        let registry = get_ocr_backend_registry();
        let registry = registry.read();
        let backend = registry.get(&base_ocr_config.backend)?;
        #[cfg(all(paddle_ocr, feature = "ocr", feature = "pdf", not(target_arch = "wasm32")))]
        if has_stored_scan && !registry.is_builtin_tesseract(&backend) {
            return Err(
                crate::XbergError::validation("Stored JPEG OCR requires the built-in Tesseract backend").into(),
            );
        }
        #[cfg(all(paddle_ocr, feature = "ocr", feature = "pdf", not(target_arch = "wasm32")))]
        {
            can_prove_no_admitted_images = detached_page.is_some()
                && images.is_some_and(|images| images.len() == 1)
                && registry.is_builtin_tesseract(&backend);
        }
        #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
        {
            can_retry_orientation = !has_layout_detections && registry.is_builtin_tesseract(&backend);
        }
        backend
    };
    // Only `ConfidenceSemantics::Legibility` backends report a `mean_text_conf` whose scale
    // means legibility; anything else (`Uncalibrated`, `None`) must never be turned into a
    // normalized confidence, or a page-rejection gate downstream would compare it against an
    // absolute floor it was never calibrated against (see `resolve_confidence_semantics`).
    #[cfg(all(
        feature = "ocr",
        feature = "pdf",
        feature = "tokio-runtime",
        not(target_arch = "wasm32")
    ))]
    let tall_source_candidate =
        !has_layout_detections && !has_stored_scan && get_ocr_backend_registry().read().is_builtin_tesseract(&backend);
    #[cfg(all(
        feature = "ocr",
        feature = "pdf",
        feature = "tokio-runtime",
        not(target_arch = "wasm32")
    ))]
    let tall_source_unsupported = tall_source_candidate
        .then(|| super::tall_source::unsupported_reason(config, base_ocr_config))
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
    let mut tall_ordered_primary = None;
    let backend_confidence_semantics = backend.confidence_semantics();
    // Owned up front: the per-page summaries below outlive the borrow-heavy OCR loop, and
    // the backend that actually produced a page is part of what `ocr_confidence` reports. ~keep
    let backend_name = backend.name().to_string();
    let backend_confidence_scale = match backend_confidence_semantics {
        crate::plugins::ConfidenceSemantics::Legibility { scale_max } if scale_max > 0.0 => Some(scale_max),
        _ => None,
    };
    // Only meaningful with the `pdf` feature: without it there is no `/Rotate` to correct for
    // (`page_rotation_degrees` is always `0` below), so nothing reads this in that build.
    #[cfg(feature = "pdf")]
    let orientation_handling = backend.page_orientation_handling();

    let structured_ocr_config;
    let ocr_config = {
        let cfg = ensure_elements_enabled(base_ocr_config);
        #[cfg(all(feature = "ocr", feature = "layout-detection"))]
        let cfg = if layout_detections.is_some() || backend.emits_structured_markdown() {
            inject_layout_config_to_backend(&cfg, config)
        } else {
            cfg
        };
        structured_ocr_config = cfg;
        &structured_ocr_config
    };

    #[cfg(not(feature = "layout-detection"))]
    let supports_doc = backend.supports_document_processing();
    #[cfg(feature = "layout-detection")]
    let supports_doc = backend.supports_document_processing() && layout_detections.is_none();

    #[cfg(feature = "pdf")]
    let page_margins = crate::pdf::native::text::PageMarginFractions::from_extraction_config(Some(config));
    #[cfg(not(feature = "pdf"))]
    let use_document_processing = supports_doc && path.is_some();
    #[cfg(feature = "pdf")]
    let use_document_processing = should_use_document_processing(supports_doc, path.is_some(), page_margins);

    if let Some(doc_path) = path
        && use_document_processing
    {
        tracing::debug!(backend = %ocr_config.backend, "Using document-level OCR processing");
        let result = backend.process_document(doc_path, ocr_config).await?;
        let preprocessing = result
            .metadata
            .image_preprocessing
            .clone()
            .map(|metadata| ahash::AHashMap::from([(1, metadata)]))
            .unwrap_or_default();
        let mean_conf = backend_confidence_scale.and_then(|scale_max| {
            result
                .metadata
                .additional
                .get("mean_text_conf")
                .and_then(|v| v.as_f64())
                .map(|v| v / scale_max)
        });
        let backend_elements = result.ocr_elements.unwrap_or_default();
        let ocr_elements = filter_public_ocr_elements(&backend_elements, base_ocr_config);
        let llm_usage = result.llm_usage.unwrap_or_default();
        let formulas = result.formulas;
        let page_texts = if let Some(pages) = result.pages {
            pages.into_iter().map(|p| p.content).collect()
        } else {
            vec![result.content.clone()]
        };
        // #1575: this route returns `None` for the structured document, so a backend
        // warning (e.g. a document-processing capability notice) had nowhere to land and
        // was silently dropped. Only builds a document when there is a warning to carry --
        // `None` otherwise, matching this branch's pre-#1575 output byte-for-byte. Restricted
        // to the single-page case: `select_pdf_document`'s `ExtractionMethod::Ocr` arm uses
        // `ocr_doc` verbatim as *the* document when it is `Some`, and for `page_texts.len() >
        // 1` a document built from `result.content` alone would not reflect this route's own
        // per-page split (`page_texts`, above) -- risking a content regression worse than the
        // warning loss this fixes. `None` still falls through to the caller's own
        // `flat_pdf_document(text, ..)`, built from the correctly page-joined `text`, so the
        // warning is the only thing still missing for a multi-page document-level result.
        #[cfg(feature = "pdf")]
        let ocr_doc = if result.processing_warnings.is_empty() || page_texts.len() > 1 {
            None
        } else {
            let mut doc = super::document::flat_ocr_page_document(&result.content);
            doc.processing_warnings = result.processing_warnings.clone();
            Some(doc)
        };
        #[cfg(not(feature = "pdf"))]
        let ocr_doc = None;
        return Ok((
            result.content,
            mean_conf,
            Vec::new(),
            ocr_elements,
            ocr_doc,
            llm_usage,
            page_texts,
            None,
            formulas,
            // The document-level backend route returns whole-document content, not per-page
            // paragraph geometry, so there is nothing for the document-global heuristic to
            // cluster over here.
            Vec::new(),
            preprocessing,
            // The backend judged the document as a whole, so there is no per-page confidence
            // to report. An empty map keeps every page's `ocr_confidence` absent rather than
            // pinning a document-wide number onto page 1 and leaving the rest bare (#1568). ~keep
            ahash::AHashMap::new(),
            // Same reasoning: this route never calls `accept_or_reject_ocr_page` per page.
            Vec::new(),
            OcrReadEvidence::default(),
        ));
    }
    #[cfg(all(feature = "ocr", feature = "pdf", not(target_arch = "wasm32")))]
    let preprocessing_render_dpi = configured_preprocessing_render_dpi(
        config,
        base_ocr_config,
        get_ocr_backend_registry().read().is_builtin_tesseract(&backend),
    );
    #[cfg(not(all(feature = "ocr", feature = "pdf", not(target_arch = "wasm32"))))]
    let preprocessing_render_dpi: Option<i32> = None;
    let capture_rasters = config.images.as_ref().is_some_and(|c| c.include_page_rasters);
    let mut captured_rasters: Vec<crate::types::ExtractedImage> = Vec::new();
    let mut preprocessing_by_page: ahash::AHashMap<u32, crate::types::ImagePreprocessingMetadata> =
        ahash::AHashMap::new();

    #[cfg(feature = "pdf")]
    let lazy_pdf_render_state = if !use_document_processing && images.is_none() {
        content.map(open_pdf_for_full_ocr).transpose()?
    } else {
        None
    };
    #[cfg(feature = "pdf")]
    let lazy_pdf_page_count = lazy_pdf_render_state
        .as_ref()
        .map_or(0, |(_, page_count, _)| *page_count);
    #[cfg(not(feature = "pdf"))]
    let lazy_pdf_page_count = 0;

    // rayon's work-stealing pool needs OS threads; wasm32 has none, so the parallel encode
    // paths below fall back to sequential `.iter()` there. Gate the import to match. ~keep
    #[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
    use rayon::prelude::*;
    use std::sync::Arc;
    #[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
    use tokio::task::JoinSet;

    let configured_batch_size = crate::core::config::concurrency::resolve_thread_budget(config.concurrency.as_ref());

    let batch_size = if has_stored_scan {
        1
    } else if images.is_none() {
        adapt_batch_size_to_memory(
            configured_batch_size,
            content.map(|b| b.len()).unwrap_or(0),
            preprocessing_render_reservation(
                preprocessing_render_dpi,
                config
                    .security_limits
                    .as_ref()
                    .unwrap_or(&Default::default())
                    .max_content_size,
            ),
        )
        .await
    } else {
        configured_batch_size
    };

    if batch_size < configured_batch_size {
        tracing::info!(
            configured = configured_batch_size,
            adapted = batch_size,
            "Reduced OCR batch size to fit available memory"
        );
    }

    let mut ocr_config_owned = ocr_config_for_page_extraction(ocr_config);
    ocr_config_owned.acceleration = config.acceleration.clone();
    // Preserve OCR-specific settings while satisfying both configured byte caps (GH#1651). ~keep
    let mut limits = ocr_config_owned
        .security_limits
        .clone()
        .or_else(|| config.security_limits.clone())
        .unwrap_or_default();
    limits.max_content_size = pipeline_xobject_content_limit(config, &ocr_config_owned);
    ocr_config_owned.security_limits = Some(limits);
    let total_pages = if has_stored_scan {
        1
    } else if let Some(imgs) = images {
        imgs.len()
    } else {
        lazy_pdf_page_count
    };
    #[cfg(feature = "pdf")]
    let progress_total_pages = xobject_document
        .and_then(|document| document.page_count().ok())
        .unwrap_or(page_index_offset + total_pages);
    #[cfg(not(feature = "pdf"))]
    let progress_total_pages = page_index_offset + total_pages;

    // The layout-detection route hands in pre-rendered `images`, so `lazy_pdf_render_state`
    // above is never populated (it's gated on `images.is_none()`) and the page-rotation
    // lookups below would always fall back to their `unwrap_or(0)` default. When the
    // original PDF bytes are still available via `content`, read the same lightweight
    // `/Rotate` lookup the full-page and per-page routes use so this route's calls to
    // `ocr_config_with_page_rotation_hint` get a real hint too (see #530 / 972d2269f7).
    // `content` is `None` on some image-only callers (e.g. the image extractor's own OCR
    // path with no source PDF); there is genuinely no rotation to read in that case.
    // The same open also yields each page's MediaBox dimensions, which are what turns a
    // pre-rendered raster's pixel width into the `source_dpi` hint this route used to derive
    // only when it rendered the pages itself (#1753).
    #[cfg(feature = "pdf")]
    let external_image_page_geometry: Option<PreRenderedPageGeometry> = if images.is_some() {
        content.map(|c| pre_rendered_page_geometry(c, total_pages))
    } else {
        None
    };
    #[cfg(feature = "pdf")]
    let external_image_page_rotations: Option<&[u32]> = external_image_page_geometry
        .as_ref()
        .map(|geometry| geometry.rotations.as_slice());
    #[cfg(feature = "pdf")]
    let external_image_page_dimensions: Option<&[(f32, f32)]> = external_image_page_geometry
        .as_ref()
        .map(|geometry| geometry.dimensions_pt.as_slice());

    let mut page_texts = vec![String::new(); total_pages];
    // Which pages the quality gate rejected. Kept separately from `page_texts` because an
    // empty string is ambiguous -- a blank page produces one too -- and the structured
    // paragraphs of a rejected page must be dropped along with its text.
    let mut rejected_pages = vec![false; total_pages];
    let mut all_page_paragraphs: Vec<Option<Vec<crate::pdf::structure::types::PdfParagraph>>> = vec![None; total_pages];
    // The factor from each page's OCR raster to points, recorded beside its paragraphs for the
    // layer merge. ~keep
    #[cfg_attr(
        all(feature = "layout-detection", not(feature = "ocr"), not(feature = "ocr-wasm")),
        allow(unused_mut)
    )]
    let mut page_ocr_scales: Vec<Option<(f32, f32)>> = vec![None; total_pages];
    #[cfg_attr(
        all(feature = "layout-detection", not(feature = "ocr"), not(feature = "ocr-wasm")),
        allow(unused_mut)
    )]
    let mut detached_page_scales = vec![None; total_pages];
    let mut public_ocr_pages = ahash::AHashMap::new();
    // Written only by the two OCR-paragraph assembly blocks below (`layout-detection`
    // *with* `ocr`/`ocr-wasm`, or `not(layout-detection)`); still read unconditionally by
    // the document-global heuristic. `layout-detection` without either OCR frontend (the
    // `formula-recognition,pdf` CI leg) leaves it write-free. ~keep
    #[cfg_attr(
        all(feature = "layout-detection", not(feature = "ocr"), not(feature = "ocr-wasm")),
        allow(unused_mut)
    )]
    let mut ocr_page_heights = vec![0.0_f32; total_pages];
    // Written by the same two assembly blocks as `ocr_page_heights`, from each page backend
    // report that detected tables claimed all of its text (#1571).
    #[cfg_attr(
        all(feature = "layout-detection", not(feature = "ocr"), not(feature = "ocr-wasm")),
        allow(unused_mut)
    )]
    let mut pages_text_claimed_by_tables = vec![false; total_pages];
    // The embedded-image retry text of each page that adopted it, without the lines the retry's
    // tables carry (#2014). The page text keeps every recovered line, as a backend's own page
    // text does, so the quality checks and the pipeline score see the tables' text; the page's
    // paragraphs are built from this instead, beside the tables printed on their own.
    #[cfg_attr(
        all(feature = "layout-detection", not(feature = "ocr"), not(feature = "ocr-wasm")),
        allow(unused_mut)
    )]
    let mut retry_texts_outside_tables: Vec<Option<String>> = vec![None; total_pages];
    #[allow(unused_mut)]
    let mut collected_tables: Vec<crate::types::Table> = Vec::new();
    #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
    let mut caption = None;
    #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
    let capture_caption =
        detached_page.is_some() && !has_layout_detections && super::caption_conversion::eligible(config);
    #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
    let mut detached_table_pair = false;
    #[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
    let mut collected_table_grids = Vec::new();
    let mut all_ocr_elements: Vec<crate::types::OcrElement> = Vec::new();
    let mut accumulated_llm_usage: Vec<crate::types::LlmUsage> = Vec::new();
    let mut accumulated_formulas: Vec<crate::types::Formula> = Vec::new();
    let mut conf_sum: f64 = 0.0;
    let mut conf_count: usize = 0;
    // Per-page OCR confidence summaries, keyed by 1-based document page number (#1568).
    // Only pages this route actually OCR'd get an entry, so an absent key downstream means
    // "not OCR'd" rather than "OCR'd with nothing to report". ~keep
    let mut ocr_confidence_by_page: ahash::AHashMap<u32, crate::types::page::PageOcrConfidence> =
        ahash::AHashMap::new();
    // Warnings from the force_ocr image-XObject fallback (#1355): a page rendered
    // blank by xberg_native_pdf but carrying image XObjects the renderer couldn't paint.
    #[cfg(feature = "pdf")]
    let mut image_fallback_warnings: Vec<crate::types::ProcessingWarning> = Vec::new();
    #[cfg(feature = "pdf")]
    let mut xobject_recovery_budget = crate::extractors::security::SecurityBudget::from_config(config);

    // #1444: a backend failure on one page used to propagate with `?`, aborting the whole
    // extraction and — crucially — never reaching the image-XObject fallback below, which is
    // exactly the recovery such a page needs. Failures are captured per page instead: the
    // page degrades to empty text (so the blank-page fallback fires for it), the failure is
    // surfaced as a warning, and only a document where *every* page failed *and* nothing was
    // recovered still returns an error.
    let mut page_backend_errors: Vec<(usize, String)> = Vec::new();
    let mut page_failure_warnings: Vec<crate::types::ProcessingWarning> = Vec::new();
    let mut page_failures: Vec<crate::types::OcrPageFailure> = Vec::new();
    // #1673: page numbers (1-based) whose embedded-image retry ran (attempted at least one
    // image) but the backend returned successfully with empty content on every one of them --
    // the shape every candle VLM backend takes when it silently produces nothing
    // (`tracing::warn!("... output is empty")`, `Ok` rather than `Err`). Distinguishes, on the
    // failure paths below, "the retry never ran" from "the retry ran and recovered nothing".
    let mut pages_with_empty_xobject_retry: Vec<u32> = Vec::new();
    // A retry can recover only structured payload (most importantly a table), leaving the flat
    // page text empty. That still prevents a wholesale failure and keeps the structured page. ~keep
    let mut pages_with_recovered_xobject_payload = vec![false; total_pages];
    #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
    let mut complete_native_words = false;
    #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
    let mut native_ink_owned = false;
    #[cfg(all(feature = "ocr", feature = "pdf", not(target_arch = "wasm32")))]
    let tall_source_used = false;
    #[cfg(all(
        feature = "ocr",
        feature = "pdf",
        feature = "tokio-runtime",
        not(target_arch = "wasm32")
    ))]
    let mut tall_source_used = tall_source_used;

    #[cfg(feature = "pdf")]
    let mut margin_filter_warnings: Vec<crate::types::ProcessingWarning> = Vec::new();

    // #1575: the per-page backend result's own warnings (e.g. Tesseract's dictionary-filter
    // removal notice) and its `additional` metadata (psm, language, dict-invalid ratio) were
    // read for local bookkeeping but never forwarded to the caller -- unlike
    // `extract_mixed_ocr_native`, which already merges both via `dedup_extend_warnings`
    // (pipeline.rs:719-723). Collected here and folded into the returned document below,
    // mirroring that sibling route. Metadata is captured once, from the first page whose
    // backend result carries any -- psm/language are constant for the whole OCR run, and a
    // single representative page is a strict improvement over reporting none at all. ~keep
    #[cfg(feature = "pdf")]
    let mut render_warnings = Vec::new();
    #[cfg(feature = "pdf")]
    let mut raster_dpis = if images.is_some() {
        page_ocr_hints
            .as_ref()
            .and_then(|hints| hints.raster_dpis.as_deref())
            .cloned()
            .unwrap_or_default()
    } else {
        super::rendering::PageRasterDpis::new()
    };
    let mut backend_page_warnings: Vec<crate::types::ProcessingWarning> = Vec::new();
    let mut backend_additional_metadata: Option<ahash::AHashMap<std::borrow::Cow<'static, str>, serde_json::Value>> =
        None;
    let mut page_segmentation_modes = std::collections::BTreeMap::<u32, i32>::new();

    // Opened on first blank page only; see `fallback_render_document`.
    #[cfg(feature = "pdf")]
    let mut fallback_pdf_state: Option<Option<xberg_native_pdf::PdfDocument>> = None;

    // Judged per page just before the OCR text is accepted, so a drawing page contributes
    // nothing instead of contributing invented words. See `is_ocr_recognition_noise`.
    let ocr_output_thresholds = base_ocr_config.quality_thresholds.clone().unwrap_or_default();
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    let orientation_retry = if can_retry_orientation {
        detached_page
            .and_then(|source| source.orientation_retry.cloned())
            .or_else(|| {
                (super::orientation_retry::OrientationRetry::allows(config) && content.is_some() && images.is_none())
                    .then(|| {
                        Arc::new(super::orientation_retry::OrientationRetry::new(
                            config,
                            ocr_output_thresholds.clone(),
                        ))
                    })
            })
    } else {
        None
    };
    let mut recognition_noise_warnings: Vec<crate::types::ProcessingWarning> = Vec::new();
    // Numeric evidence behind each fired `recognition_noise_warnings` entry, so a caller
    // above the accept/reject decision can observe the signal without recomputing stats
    // (see `OcrPageNoiseVerdict`). Populated exactly when a warning above is pushed. ~keep
    let mut recognition_noise_verdicts: Vec<OcrPageNoiseVerdict> = Vec::new();

    // #1812: `take_or_create_tatr` can block on a `std::sync::Condvar` (pool exhaustion, or a
    // competing default-acceleration model load) and, on a cache miss, performs synchronous
    // model-file I/O and ONNX Runtime session construction. None of that may run on a tokio
    // worker thread -- two concurrent layout-enabled extractions on a small worker pool could
    // starve every worker with nothing left to poll the task that would return a lease. Route
    // through `spawn_blocking`, mirroring the boundary already used for the RT-DETR layout
    // engine in `layout_runner::run_layout_for_pdf_pages_async`. ~keep
    #[cfg(feature = "layout-detection")]
    let mut tatr_model = if layout_detections.is_some() && pdf_ocr_table_recognition_enabled(config) {
        let tatr_acceleration = config.resolved_layout_acceleration().cloned();
        let tatr_thread_budget = crate::core::config::concurrency::resolve_thread_budget(config.concurrency.as_ref());
        let span = tracing::Span::current();
        tokio::task::spawn_blocking(move || {
            let _entered = span.entered();
            crate::layout::take_or_create_tatr(tatr_acceleration.as_ref(), tatr_thread_budget)
        })
        .await
        .map_err(|e| crate::XbergError::Plugin {
            message: format!("TATR model checkout panicked: {}", e),
            plugin_name: "layout".to_string(),
        })?
    } else {
        None
    };

    #[cfg(feature = "layout-detection")]
    if let Some(warning) = tatr_unavailable_warning(config, layout_detections, tatr_model.is_some()) {
        crate::core::diagnostics::push_warning_deduped(&mut backend_page_warnings, warning);
    }

    for batch_start in (0..total_pages).step_by(batch_size) {
        let batch_end = (batch_start + batch_size).min(total_pages);

        #[allow(unused_variables)]
        let (batch_slice, encoded_batch) = if has_stored_scan {
            (None, Vec::new())
        } else if let Some(imgs) = images {
            let slice: Cow<'_, [image::DynamicImage]> = Cow::Borrowed(&imgs[batch_start..batch_end]);
            let default_security_limits = crate::extractors::security::SecurityLimits::default();
            let security_limits = config.security_limits.as_ref().unwrap_or(&default_security_limits);
            // One page at a time. `max_content_size` bounds what a single page may cost to
            // render and encode; summing a whole batch against it made that per-image ceiling a
            // function of the thread budget. On the parallel encode path below, five US Letter
            // pages at the default 150 dpi already cross the default limit, so an eight-page
            // batch was refused whole and every page the caller had already rendered was
            // dropped (#1731). Matches the per-page check the mixed native-and-OCR route runs
            // at its own encode step. The batch's own footprint is bounded by `batch_size`,
            // which the thread budget decides; `max_content_size` does not bound it. ~keep
            validate_png_encode_pages_individually(slice.iter(), security_limits)?;
            #[allow(clippy::type_complexity)]
            #[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
            let encoded: crate::Result<Vec<(usize, Arc<Vec<u8>>, u32, u32)>> = slice
                .par_iter()
                .enumerate()
                .map(|(offset, image)| {
                    let page_idx = batch_start + offset;
                    let default_security_limits = crate::extractors::security::SecurityLimits::default();
                    let security_limits = config.security_limits.as_ref().unwrap_or(&default_security_limits);
                    let rgb_image = clone_rgb_for_png_encode(image, security_limits)?;
                    let (width, height) = rgb_image.dimensions();
                    let mut image_bytes = Cursor::new(Vec::new());
                    let encoder = PngEncoder::new(&mut image_bytes);
                    encoder
                        .write_image(&rgb_image, width, height, image::ColorType::Rgb8.into())
                        .map_err(|e| crate::XbergError::Parsing {
                            message: format!("Failed to encode image: {}", e),
                            source: None,
                        })?;
                    Ok((page_idx, Arc::new(image_bytes.into_inner()), width, height))
                })
                .collect();
            #[allow(clippy::type_complexity)]
            #[cfg(any(not(feature = "tokio-runtime"), target_arch = "wasm32"))]
            let encoded: crate::Result<Vec<(usize, Arc<Vec<u8>>, u32, u32)>> = slice
                .iter()
                .enumerate()
                .map(|(offset, image)| {
                    let page_idx = batch_start + offset;
                    let default_security_limits = crate::extractors::security::SecurityLimits::default();
                    let security_limits = config.security_limits.as_ref().unwrap_or(&default_security_limits);
                    let rgb_image = clone_rgb_for_png_encode(image, security_limits)?;
                    let (width, height) = rgb_image.dimensions();
                    let mut image_bytes = Cursor::new(Vec::new());
                    let encoder = PngEncoder::new(&mut image_bytes);
                    encoder
                        .write_image(&rgb_image, width, height, image::ColorType::Rgb8.into())
                        .map_err(|e| crate::XbergError::Parsing {
                            message: format!("Failed to encode image: {}", e),
                            source: None,
                        })?;
                    Ok((page_idx, Arc::new(image_bytes.into_inner()), width, height))
                })
                .collect();
            (Some(slice), encoded?)
        } else {
            #[cfg(feature = "pdf")]
            let encoded = {
                let (doc, _, page_rotations) =
                    lazy_pdf_render_state
                        .as_ref()
                        .ok_or_else(|| crate::XbergError::Parsing {
                            message: "PDF content is required for OCR rendering but was not provided".to_string(),
                            source: None,
                        })?;
                let default_security_limits = crate::extractors::security::SecurityLimits::default();
                let security_limits = config.security_limits.as_ref().unwrap_or(&default_security_limits);
                let ((pages, batch_raster_dpis), warnings) = crate::pdf::render::capture_render_warnings(|| {
                    render_full_pdf_ocr_batch_with_dpi(
                        doc,
                        page_rotations,
                        batch_start..batch_end,
                        security_limits,
                        config.images.as_ref(),
                        preprocessing_render_dpi,
                    )
                })?;
                crate::core::diagnostics::dedup_extend_warnings(&mut render_warnings, warnings);
                raster_dpis.extend(batch_raster_dpis);
                pages
            };
            #[cfg(not(feature = "pdf"))]
            let encoded: Vec<(usize, Arc<Vec<u8>>, u32, u32)> = Vec::new();
            (None::<Cow<'_, [image::DynamicImage]>>, encoded)
        };

        #[cfg(all(paddle_ocr, feature = "ocr", feature = "pdf", not(target_arch = "wasm32")))]
        let encoded_batch = if let Some(scan) = stored_scan {
            vec![(0, Arc::clone(&scan.data), scan.render.0, scan.render.1)]
        } else {
            encoded_batch
        };
        let batch_count = encoded_batch.len();
        let mut batch_ocr_results: Vec<Option<crate::types::ExtractedDocument>> = vec![None; batch_count];
        // Backend error text for a page whose OCR call failed, if any (#1444). The page's
        // entry in `batch_ocr_results` is an empty document in that case, so the blank-page
        // image-XObject fallback below gets its chance before the failure is reported.
        let mut batch_page_errors: Vec<Option<String>> = vec![None; batch_count];
        // (correction_degrees, upright_width, upright_height) applied by
        // `upright_raster_for_backend` before this page's backend call, so the matching
        // `undo_upright_raster_correction` below maps the result back correctly (#643).
        let mut batch_upright_correction: Vec<(u32, u32, u32)> = vec![(0, 0, 0); batch_count];
        #[cfg(all(
            feature = "pdf",
            feature = "ocr",
            feature = "tokio-runtime",
            not(target_arch = "wasm32")
        ))]
        let mut detached_frame_reads = vec![false; batch_count];

        // See the sibling JoinSet block above: `Send` futures aren't available on wasm32. ~keep
        #[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
        {
            let mut join_set = JoinSet::new();
            for (page_idx, image_data, width, height) in &encoded_batch {
                let idx = *page_idx;
                let cancel_token = config.cancel_token.clone();
                // Every iteration must spawn exactly one task: the drain loop below and the
                // `for offset in 0..batch_count` pass after it both index `batch_ocr_results`
                // by position and `.expect()` a `Some` at every offset (#1620-style
                // invariant), so a cancelled page still needs an entry -- it just skips the
                // real OCR work to get it. ~keep
                if cancel_token.as_ref().is_some_and(|t| t.is_cancelled()) {
                    let task = crate::engine::seams::inherit_progress(async move {
                        (idx, 0, 0, 0, Err(crate::XbergError::Cancelled), None, false, None)
                    });
                    join_set.spawn(crate::core::extractor::cacheability::inherit(task));
                    continue;
                }
                let backend_clone = std::sync::Arc::clone(&backend);
                #[cfg(feature = "pdf")]
                let page_rotation_degrees = if page_rotation_override != 0 {
                    page_rotation_override
                } else {
                    lazy_pdf_render_state
                        .as_ref()
                        .and_then(|(_, _, rotations)| rotations.get(*page_idx))
                        .or_else(|| external_image_page_rotations.as_ref().and_then(|r| r.get(*page_idx)))
                        .copied()
                        .unwrap_or(0)
                };
                #[cfg(not(feature = "pdf"))]
                let page_rotation_degrees: u32 = 0;
                // `lazy_pdf_render_state` marks the branch that rendered the pages itself (it is
                // only opened when `images.is_none()`, see its `let` above). Pre-rendered images
                // reach the same answer from the source document's MediaBox instead, and stay
                // hint-free when they are not a whole-page render of it (#1753).
                #[cfg(feature = "pdf")]
                let source_dpi = page_ocr_hints.as_ref().and_then(|hints| hints.source_dpi).or_else(|| {
                    lazy_pdf_render_state
                        .as_ref()
                        .and_then(|(doc, _, _)| rendered_page_source_dpi(doc, *page_idx, *width))
                        .or_else(|| {
                            external_image_page_dimensions
                                .and_then(|dimensions| dimensions.get(*page_idx))
                                .and_then(|dimensions| pre_rendered_page_source_dpi(*dimensions, *width, *height))
                        })
                });
                #[cfg(not(feature = "pdf"))]
                let source_dpi: Option<f64> = None;
                #[cfg(feature = "pdf")]
                let whole_page_raster = page_ocr_hints
                    .as_ref()
                    .filter(|hints| hints.known_full_page_scan)
                    .map_or_else(
                        || {
                            whole_page_raster_for_ocr_page(
                                lazy_pdf_render_state.as_ref(),
                                &mut fallback_pdf_state,
                                content,
                                *page_idx,
                            )
                        },
                        |_| true,
                    );
                #[cfg(not(feature = "pdf"))]
                let whole_page_raster = false;
                #[cfg(feature = "pdf")]
                let prefer_single_block = single_block_for_ocr_page(
                    page_ocr_hints
                        .as_ref()
                        .and_then(|hints| hints.single_block_pages.as_deref()),
                    page_index_offset + *page_idx + 1,
                    lazy_pdf_render_state.as_ref(),
                    &mut fallback_pdf_state,
                    content,
                    *page_idx,
                );
                #[cfg(not(feature = "pdf"))]
                let prefer_single_block = false;
                let config_clone = ocr_config_with_page_rotation_hint(
                    &ocr_config_owned,
                    page_rotation_degrees,
                    source_dpi,
                    whole_page_raster,
                    prefer_single_block,
                    has_layout_detections,
                )
                .into_owned();
                let original_config = ocr_config_owned.clone();
                // No PDF `/Rotate` is ever known without the `pdf` feature (`page_rotation_degrees`
                // is always `0` above in that build), so there is nothing to correct upright.
                #[cfg(feature = "pdf")]
                let (upright_data, upright_width, upright_height, correction_degrees) = upright_raster_for_backend(
                    image_data,
                    *width,
                    *height,
                    page_rotation_degrees,
                    orientation_handling,
                    config.security_limits.as_ref(),
                )?;
                #[cfg(not(feature = "pdf"))]
                let (upright_data, upright_width, upright_height, correction_degrees) =
                    (Arc::clone(image_data), *width, *height, 0u32);
                #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
                let retry = orientation_retry
                    .as_ref()
                    .filter(|_| whole_page_raster)
                    .map(|retry| (Arc::clone(retry), Arc::clone(&upright_data)));
                #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
                let allow_prose = detached_page.is_none() && images.is_none();
                #[cfg(feature = "ocr")]
                let caption_raster =
                    (capture_caption && retry.is_some() && page_rotation_degrees == 0 && correction_degrees == 0).then(
                        || {
                            (
                                Arc::clone(&upright_data),
                                super::caption_conversion::proof_limits(config, base_ocr_config),
                            )
                        },
                    );
                let retained_pixels = batch_slice
                    .as_ref()
                    .and_then(|images| images.get(idx - batch_start))
                    .map_or(0, |image| image.as_bytes().len());
                let retained_source_bytes = retained_ocr_source_bytes(image_data, &upright_data, retained_pixels);
                #[cfg(all(feature = "ocr", feature = "pdf", not(target_arch = "wasm32")))]
                let ordered_raster = (try_tall_source
                    && super::tall_source::augmentation::supports_run(base_ocr_config, *height)
                    && total_pages == 1
                    && page_index_offset == 0
                    && idx == 0
                    && correction_degrees == 0
                    && detached_page.is_none()
                    && lazy_pdf_render_state.is_some()
                    && config.pages.as_ref().is_none_or(|pages| !pages.insert_page_markers))
                .then_some((*width, *height, image_data.capacity()));
                #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
                let observe_detached_frame = detached_page
                    .is_some_and(|source| detached_mixed_frame::supports(config, source.document.page_count().ok()))
                    && can_retry_orientation
                    && !has_stored_scan
                    && whole_page_raster
                    && page_rotation_degrees == 0
                    && correction_degrees == 0
                    && base_ocr_config.auto_rotate.is_none()
                    && base_ocr_config.tesseract_config.is_none()
                    && base_ocr_config.effective_tesseract_language().as_slice() == ["eng"];
                let task = crate::engine::seams::inherit_progress(async move {
                    if cancel_token.as_ref().is_some_and(|t| t.is_cancelled()) {
                        return (
                            idx,
                            correction_degrees,
                            upright_width,
                            upright_height,
                            Err(crate::XbergError::Cancelled),
                            None,
                            false,
                            None,
                        );
                    }
                    let (original_config, config_clone) = match crate::extractors::image::reserve_retained_ocr_raster(
                        &backend_clone,
                        (original_config, config_clone),
                        retained_source_bytes,
                    ) {
                        Ok(configs) => configs,
                        Err(error) => {
                            return (
                                idx,
                                correction_degrees,
                                upright_width,
                                upright_height,
                                Err(error),
                                None,
                                false,
                                None,
                            );
                        }
                    };
                    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
                    let primary_config = retry
                        .as_ref()
                        .and_then(|(retry, _)| retry.primary_config(&config_clone));
                    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
                    let read_config = primary_config.as_ref().unwrap_or(&config_clone);
                    #[cfg(not(all(feature = "ocr", not(target_arch = "wasm32"))))]
                    let read_config = &config_clone;
                    let read = crate::extractors::image::process_whole_image_ocr_with_sparse_retry(
                        &backend_clone,
                        upright_data,
                        &original_config,
                        read_config,
                        crate::extractors::image::SparseRetryPolicy {
                            whole_page_raster,
                            allow_standard_retry: false,
                        },
                        cancel_token.as_ref(),
                    );
                    #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
                    let primary = async {
                        if let Some((retry, _)) = &retry {
                            retry.read_primary(&config_clone, read).await
                        } else {
                            read.await
                        }
                    };
                    #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
                    let (result, scale_only, inventory) = if caption_raster.is_some() {
                        crate::ocr::preprocessing::geometry_evidence::observe_with_word_inventory(primary).await
                    } else if ordered_raster.is_some() || observe_detached_frame {
                        let (result, scale_only) = crate::ocr::preprocessing::geometry_evidence::observe(primary).await;
                        (result, scale_only, None)
                    } else {
                        (primary.await, false, None)
                    };
                    #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
                    let detached_frame_read = observe_detached_frame
                        && scale_only
                        && result.as_ref().is_ok_and(detached_mixed_frame::stock_mixed_read);
                    #[cfg(not(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32"))))]
                    let detached_frame_read = false;
                    // Stock orientation preserves table-bearing primaries but later restores
                    // the caller's public element policy. Capture the complete internal read here.
                    #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
                    let ordered_proof = ordered_raster
                        .filter(|_| scale_only)
                        .zip(result.as_ref().ok())
                        .and_then(|(raster, document)| {
                            super::tall_source::augmentation::PrimaryProof::capture(
                                document,
                                document.ocr_elements.as_deref()?,
                                raster,
                            )
                        });
                    #[cfg(not(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32"))))]
                    let (result, ordered_proof) = (read.await, None::<()>);
                    // ~keep A table makes the recognition retry keep the original attempt.
                    #[cfg(feature = "ocr")]
                    let captured_caption = if let Some((raster, mut limits)) = caption_raster
                        && primary_config.is_some()
                        && scale_only
                    {
                        let candidate = inventory.zip(result.as_ref().ok()).and_then(|(inventory, result)| {
                            super::caption_conversion::capture(
                                result,
                                (upright_width, upright_height),
                                &mut limits,
                                inventory,
                            )
                        });
                        super::caption_conversion::verify_paint(
                            candidate,
                            raster,
                            retained_source_bytes,
                            limits,
                            cancel_token.clone(),
                        )
                        .await
                    } else {
                        None
                    };
                    #[cfg(not(feature = "ocr"))]
                    let captured_caption = None::<()>;
                    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
                    let result = if let Some((retry, raster)) = retry {
                        retry
                            .after_ocr(
                                &backend_clone,
                                raster,
                                &config_clone,
                                result,
                                cancel_token.as_ref(),
                                allow_prose,
                            )
                            .await
                    } else {
                        result
                    };
                    #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
                    let ordered_proof =
                        ordered_proof.filter(|proof| result.as_ref().is_ok_and(|selected| proof.matches(selected)));
                    #[cfg(feature = "ocr")]
                    let captured_caption = captured_caption
                        .filter(|proof| result.as_ref().is_ok_and(|selected| proof.matches_selected(selected)));
                    (
                        idx,
                        correction_degrees,
                        upright_width,
                        upright_height,
                        result,
                        ordered_proof,
                        detached_frame_read,
                        captured_caption,
                    )
                });
                join_set.spawn(crate::core::extractor::cacheability::inherit(task));
            }
            while let Some(join_result) = join_set.join_next().await {
                let (
                    page_idx,
                    correction_degrees,
                    upright_width,
                    upright_height,
                    ocr_result,
                    ordered_proof,
                    detached_frame_read,
                    captured_caption,
                ) = join_result.map_err(|e| crate::XbergError::Plugin {
                    message: format!("OCR task panicked: {}", e),
                    plugin_name: "ocr".to_string(),
                })?;
                #[cfg(not(all(feature = "ocr", feature = "pdf", not(target_arch = "wasm32"))))]
                let _ = (ordered_proof, detached_frame_read);
                #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
                {
                    detached_frame_reads[page_idx - batch_start] = detached_frame_read;
                }
                #[cfg(all(
                    feature = "ocr",
                    feature = "pdf",
                    feature = "tokio-runtime",
                    not(target_arch = "wasm32")
                ))]
                let ocr_result = {
                    let mut ocr_result = ocr_result;
                    if let Some(proof) = ordered_proof {
                        tall_ordered_primary = Some(proof);
                    }
                    if try_tall_source
                        && correction_degrees == 0
                        && let Ok(document) = &mut ocr_result
                        && let Some(source) = lazy_pdf_render_state
                            .as_ref()
                            .map(|(pdf, _, _)| (pdf, page_idx))
                            .or_else(|| xobject_document.map(|pdf| (pdf, page_index_offset + page_idx)))
                    {
                        tall_source_used |= super::tall_source::try_replace(
                            document,
                            &backend,
                            config,
                            base_ocr_config,
                            source,
                            &encoded_batch[page_idx - batch_start],
                        )
                        .await?;
                    } else if let Some(reason) = tall_source_unsupported
                        && correction_degrees == 0
                        && let Ok(document) = &mut ocr_result
                        && let Some(source) = lazy_pdf_render_state
                            .as_ref()
                            .map(|(pdf, _, _)| (pdf, page_idx))
                            .or_else(|| xobject_document.map(|pdf| (pdf, page_index_offset + page_idx)))
                    {
                        super::tall_source::note_unrecovered(
                            document,
                            base_ocr_config,
                            source,
                            encoded_batch[page_idx - batch_start].3,
                            super::tall_source::Unrecovered::Config(reason),
                        );
                    }
                    ocr_result
                };
                crate::engine::seams::emit_ocr_page(
                    page_index_offset + page_idx + 1,
                    progress_total_pages,
                    backend_name.as_str(),
                );
                batch_upright_correction[page_idx - batch_start] = (correction_degrees, upright_width, upright_height);
                #[cfg(feature = "ocr")]
                if ocr_result.is_ok() && captured_caption.is_some() {
                    caption = captured_caption;
                }
                #[cfg(not(feature = "ocr"))]
                let _ = captured_caption;
                match ocr_result {
                    Ok(document) => batch_ocr_results[page_idx - batch_start] = Some(document),
                    Err(error) => {
                        let described = describe_page_failure(&error);
                        tracing::warn!(
                            page = page_index_offset + page_idx + 1,
                            error = %described,
                            "OCR backend failed for page"
                        );
                        batch_page_errors[page_idx - batch_start] = Some(described);
                        batch_ocr_results[page_idx - batch_start] = Some(crate::types::ExtractedDocument::default());
                    }
                }
            }
        }
        #[cfg(any(not(feature = "tokio-runtime"), target_arch = "wasm32"))]
        {
            for (page_idx, image_data, width, height) in &encoded_batch {
                #[cfg(feature = "pdf")]
                let page_rotation_degrees = if page_rotation_override != 0 {
                    page_rotation_override
                } else {
                    lazy_pdf_render_state
                        .as_ref()
                        .and_then(|(_, _, rotations)| rotations.get(*page_idx))
                        .or_else(|| external_image_page_rotations.as_ref().and_then(|r| r.get(*page_idx)))
                        .copied()
                        .unwrap_or(0)
                };
                #[cfg(not(feature = "pdf"))]
                let page_rotation_degrees: u32 = 0;
                // See the JoinSet branch above for both derivations.
                #[cfg(feature = "pdf")]
                let source_dpi = page_ocr_hints.as_ref().and_then(|hints| hints.source_dpi).or_else(|| {
                    lazy_pdf_render_state
                        .as_ref()
                        .and_then(|(doc, _, _)| rendered_page_source_dpi(doc, *page_idx, *width))
                        .or_else(|| {
                            external_image_page_dimensions
                                .and_then(|dimensions| dimensions.get(*page_idx))
                                .and_then(|dimensions| pre_rendered_page_source_dpi(*dimensions, *width, *height))
                        })
                });
                #[cfg(not(feature = "pdf"))]
                let source_dpi: Option<f64> = None;
                #[cfg(feature = "pdf")]
                let whole_page_raster = page_ocr_hints
                    .as_ref()
                    .filter(|hints| hints.known_full_page_scan)
                    .map_or_else(
                        || {
                            whole_page_raster_for_ocr_page(
                                lazy_pdf_render_state.as_ref(),
                                &mut fallback_pdf_state,
                                content,
                                *page_idx,
                            )
                        },
                        |_| true,
                    );
                #[cfg(not(feature = "pdf"))]
                let whole_page_raster = false;
                #[cfg(feature = "pdf")]
                let prefer_single_block = single_block_for_ocr_page(
                    page_ocr_hints
                        .as_ref()
                        .and_then(|hints| hints.single_block_pages.as_deref()),
                    page_index_offset + *page_idx + 1,
                    lazy_pdf_render_state.as_ref(),
                    &mut fallback_pdf_state,
                    content,
                    *page_idx,
                );
                #[cfg(not(feature = "pdf"))]
                let prefer_single_block = false;
                let config_for_page = ocr_config_with_page_rotation_hint(
                    &ocr_config_owned,
                    page_rotation_degrees,
                    source_dpi,
                    whole_page_raster,
                    prefer_single_block,
                    has_layout_detections,
                );
                #[cfg(feature = "pdf")]
                let (upright_data, upright_width, upright_height, correction_degrees) = upright_raster_for_backend(
                    image_data,
                    *width,
                    *height,
                    page_rotation_degrees,
                    orientation_handling,
                    config.security_limits.as_ref(),
                )?;
                #[cfg(not(feature = "pdf"))]
                let (upright_data, upright_width, upright_height, correction_degrees) =
                    (Arc::clone(image_data), *width, *height, 0u32);
                // A cancelled page still needs a `batch_ocr_results` entry -- see the ~keep
                // comment on the sibling `JoinSet` branch above -- so this skips the backend
                // call rather than breaking the loop.
                let ocr_result = if config.cancel_token.as_ref().is_some_and(|t| t.is_cancelled()) {
                    Err(crate::XbergError::Cancelled)
                } else {
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        async {
                            let retained_pixels = batch_slice
                                .as_ref()
                                .and_then(|images| images.get(*page_idx - batch_start))
                                .map_or(0, |image| image.as_bytes().len());
                            let retained = retained_ocr_source_bytes(image_data, &upright_data, retained_pixels);
                            let (original, effective) = crate::extractors::image::reserve_retained_ocr_raster(
                                &backend,
                                (ocr_config_owned.clone(), config_for_page.as_ref().clone()),
                                retained,
                            )?;
                            crate::extractors::image::process_whole_image_ocr_with_sparse_retry(
                                &backend,
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
                        .await
                    }
                    #[cfg(target_arch = "wasm32")]
                    {
                        backend
                            .process_image(upright_data.as_slice(), config_for_page.as_ref())
                            .await
                    }
                };
                crate::engine::seams::emit_ocr_page(
                    page_index_offset + *page_idx + 1,
                    progress_total_pages,
                    backend_name.as_str(),
                );
                batch_upright_correction[page_idx - batch_start] = (correction_degrees, upright_width, upright_height);
                match ocr_result {
                    Ok(document) => batch_ocr_results[page_idx - batch_start] = Some(document),
                    Err(error) => {
                        let described = describe_page_failure(&error);
                        tracing::warn!(
                            page = page_index_offset + page_idx + 1,
                            error = %described,
                            "OCR backend failed for page"
                        );
                        batch_page_errors[page_idx - batch_start] = Some(described);
                        batch_ocr_results[page_idx - batch_start] = Some(crate::types::ExtractedDocument::default());
                    }
                }
            }
        }

        for offset in 0..batch_count {
            let page_idx = batch_start + offset;
            let document_page_idx = page_index_offset + page_idx;
            let document_page_number = (document_page_idx + 1) as u32;
            let mut ocr_result = batch_ocr_results[offset].take().expect("OCR result missing for page");
            #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
            if can_retry_orientation && batch_upright_correction[offset].0 == 0 {
                let coverage_layer = page_ocr_hints
                    .as_ref()
                    .and_then(|hints| hints.native_coverage_layers.as_ref())
                    .and_then(|layers| layers.get(&document_page_number))
                    .map(String::as_str);
                let lost_glyph_layer = page_ocr_hints
                    .as_ref()
                    .and_then(|hints| hints.single_native_type0_page.as_deref())
                    .filter(|_| document_page_idx == 0 && super::document::native_preservation_is_automatic(config, 1));
                let pdf = detached_page
                    .map(|source| source.document)
                    .or_else(|| lazy_pdf_render_state.as_ref().map(|(pdf, _, _)| pdf));
                if let (Some(pdf), Some(layer)) = (pdf, coverage_layer.filter(|_| detached_page.is_some())) {
                    complete_native_words = super::native_word_coverage::covers_complete_native_page(
                        pdf,
                        document_page_idx,
                        layer,
                        &ocr_result,
                    );
                }
                if let (Some(pdf), Some(layer)) = (
                    pdf,
                    coverage_layer.filter(|_| detached_page.is_some()).or(lost_glyph_layer),
                ) {
                    native_ink_owned = !complete_native_words
                        && super::document::native_preservation_is_automatic(config, document_page_number)
                        && super::native_word_coverage::native_text_paints_all_read_ink(
                            pdf,
                            document_page_idx,
                            layer,
                            &ocr_result,
                            super::native_word_coverage::InkProofLimits {
                                thresholds: &ocr_output_thresholds,
                                max_working_bytes: super::hidden_layer::preview_byte_limit(config),
                                cancelled: &|| config.cancel_token.as_ref().is_some_and(|token| token.is_cancelled()),
                            },
                        );
                }
            }
            crate::extraction::derive::adopt_public_ocr_document(
                &mut ocr_result,
                config.security_limits.as_ref().unwrap_or(&Default::default()),
            )?;
            if let Some(psm) = effective_tesseract_psm(&ocr_result.metadata, &backend_name) {
                page_segmentation_modes.insert(document_page_number, psm);
            }
            if let Some(metadata) = ocr_result.metadata.image_preprocessing.clone() {
                preprocessing_by_page.insert(document_page_number, metadata);
            }
            crate::core::diagnostics::dedup_extend_warnings(
                &mut backend_page_warnings,
                std::mem::take(&mut ocr_result.processing_warnings),
            );
            #[cfg(feature = "pdf")]
            {
                let (correction_degrees, upright_width, upright_height) = batch_upright_correction[offset];
                #[cfg(all(auto_rotate, feature = "ocr", not(target_arch = "wasm32")))]
                let normalize_recovered_table = xobject_document.is_some()
                    && (orientation_retry.is_some() || base_ocr_config.auto_rotate == Some(true))
                    && !ocr_result.tables.is_empty()
                    && super::document::resolved_ocr_correction_degrees(&ocr_result.metadata)
                        .is_some_and(|angle| angle != 0);
                undo_upright_raster_correction(&mut ocr_result, correction_degrees, upright_width, upright_height);
                #[cfg(all(paddle_ocr, feature = "ocr", not(target_arch = "wasm32")))]
                if let Some(scan) = stored_scan {
                    stored_scan_retry::place_in_render_frame(&mut ocr_result, scan)?;
                }
                let render = (encoded_batch[offset].2, encoded_batch[offset].3);
                let normalized_raw = (xobject_document.is_some() || lazy_pdf_render_state.is_some())
                    && raw_workspace_geometry::normalize(
                        &mut ocr_result,
                        render,
                        matches!(correction_degrees, 90 | 270),
                    );
                #[cfg(all(auto_rotate, feature = "ocr", not(target_arch = "wasm32")))]
                if normalize_recovered_table && !normalized_raw {
                    // The detached stage's owner will place raw tables using the render raster. ~keep
                    super::orientation_retry::scale_recovered_table_to_raster(&mut ocr_result, render);
                }
                #[cfg(not(all(auto_rotate, feature = "ocr", not(target_arch = "wasm32"))))]
                let _ = normalized_raw;
                #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
                if detached_page.is_some()
                    && can_retry_orientation
                    && !has_stored_scan
                    && total_pages == 1
                    && ocr_result.document.is_none()
                    && ocr_result.formulas.is_empty()
                    && ocr_result.ocr_internal_document.as_ref().is_some_and(|doc| {
                        doc.ocr_text_claimed_by_tables && doc.elements.is_empty() && doc.tables.is_empty()
                    })
                    && let Some(processed) = super::document::processed_ocr_layout_dimensions(&ocr_result.metadata)
                {
                    // The raw path rewrites the processed dimensions to render size. The certified
                    // orientation path already rescales its tables but leaves the text frame alone. ~keep
                    #[cfg(auto_rotate)]
                    let processed = if normalize_recovered_table { render } else { processed };
                    detached_table_pair = detached_table_pair::normalize(&mut ocr_result.tables, processed, render);
                }
            }
            // #1575: capture once, from the first page that has any -- see this function's
            // `backend_additional_metadata` declaration for why a single representative page
            // is the right granularity here.
            if backend_additional_metadata.is_none() && !ocr_result.metadata.additional.is_empty() {
                backend_additional_metadata = Some(ocr_result.metadata.additional.clone());
            }
            #[cfg(feature = "layout-detection")]
            let _height = encoded_batch[offset].3;

            if let Some(conf_val) = ocr_result
                .metadata
                .additional
                .get("mean_text_conf")
                .and_then(|v| v.as_i64())
            {
                conf_sum += conf_val as f64;
                conf_count += 1;
            }

            if let Some(usage) = ocr_result.llm_usage.take() {
                accumulated_llm_usage.extend(usage);
            }

            if let Some(document) = ocr_result
                .ocr_internal_document
                .as_mut()
                .filter(|_| ocr_result.document.is_some())
            {
                let (width, height) = resolved_ocr_layout_dimensions(
                    &ocr_result.metadata,
                    encoded_batch[offset].2,
                    encoded_batch[offset].3,
                );
                let outcome =
                    super::document::filter_structured_ocr_page_by_page_margins(document, height as f32, page_margins);
                if outcome.removed {
                    ocr_result.content = crate::rendering::render_plain(document);
                }
                if outcome.missing_geometry {
                    crate::core::diagnostics::push_warning_deduped(
                        &mut margin_filter_warnings,
                        ocr_margin_filter_capability_warning(),
                    );
                }
                let mut document = document.clone();
                use super::document::PublicOcrOutputFrame;
                let frame = if let Some((pdf, _, _)) = lazy_pdf_render_state.as_ref() {
                    PublicOcrOutputFrame::PdfPage {
                        raster: (width, height),
                        points: page_dimensions_pt(pdf, page_idx),
                    }
                } else if xobject_document.is_some() {
                    // ~keep A detached stage has no PDF bytes/lazy render state, but its
                    // explicit PDF owner still requires the canonical bottom-left
                    // raster frame. build_pipeline_ocr_page_document scales it once.
                    PublicOcrOutputFrame::PdfRaster {
                        processed: (width, height),
                        raster: (encoded_batch[offset].2, encoded_batch[offset].3),
                    }
                } else {
                    PublicOcrOutputFrame::ImagePixels
                };
                super::document::place_public_ocr_bboxes(&mut document, frame);
                super::document::normalize_mixed_ocr_document_page(&mut document, document_page_number);
                public_ocr_pages.insert(document_page_number, document);
            }
            // An upright, scale-only stock table is kept by implicit orientation selection.
            // Preserve its source dimensions for paragraphs while putting the table in the detached owner's render frame.
            #[cfg(all(
                feature = "pdf",
                feature = "ocr",
                feature = "tokio-runtime",
                not(target_arch = "wasm32")
            ))]
            let detached_paragraph_scale = if detached_frame_reads[offset]
                && !tall_source_used
                && detached_mixed_frame::stock_mixed_read(&ocr_result)
                && let Some(source) = detached_page
            {
                detached_mixed_frame::place_table_in_render(
                    &mut ocr_result,
                    (encoded_batch[offset].2, encoded_batch[offset].3),
                    page_dimensions_pt(source.document, document_page_idx),
                )
            } else {
                None
            };
            #[cfg(all(
                any(not(feature = "layout-detection"), feature = "ocr", feature = "ocr-wasm"),
                not(all(
                    feature = "pdf",
                    feature = "ocr",
                    feature = "tokio-runtime",
                    not(target_arch = "wasm32")
                ))
            ))]
            let detached_paragraph_scale: Option<(f32, f32)> = None;
            let mut backend_tables = std::mem::take(&mut ocr_result.tables);
            for table in &mut backend_tables {
                table.page_number = document_page_number;
            }
            #[cfg(feature = "pdf")]
            if let Some((doc, _, _)) = lazy_pdf_render_state.as_ref() {
                let (page_width_pt, page_height_pt) = page_dimensions_pt(doc, page_idx);
                // ~keep A backend table's rect is in the pixel space the backend OCR'd, which
                // preprocessing may have resized away from the render raster -- Tesseract
                // normalizes to `target_dpi` by default. Passing the render dims here scaled
                // every TSV table's page-space bbox by the processed/render ratio, while the
                // formula bbox eleven lines below already resolved the same dims from metadata
                // (xberg-io/xberg#1813).
                let (table_px_width, table_px_height) = resolved_ocr_layout_dimensions(
                    &ocr_result.metadata,
                    encoded_batch[offset].2,
                    encoded_batch[offset].3,
                );
                rescale_ocr_bboxes_to_page_points(
                    None,
                    &mut backend_tables,
                    table_px_width,
                    table_px_height,
                    page_width_pt,
                    page_height_pt,
                );
            }
            collected_tables.append(&mut backend_tables);

            if let Some(ref mut elems) = ocr_result.ocr_elements {
                #[cfg(feature = "pdf")]
                let public_elements = {
                    let (_, layout_height) = resolved_ocr_layout_dimensions(
                        &ocr_result.metadata,
                        encoded_batch[offset].2,
                        encoded_batch[offset].3,
                    );
                    let (public_elements, outcome) = public_ocr_elements_for_pdf_page(
                        elems,
                        base_ocr_config,
                        document_page_number,
                        layout_height,
                        page_margins,
                    );
                    if outcome.missing_geometry && !public_elements.is_empty() {
                        crate::core::diagnostics::push_warning_deduped(
                            &mut margin_filter_warnings,
                            ocr_margin_filter_capability_warning(),
                        );
                    }
                    public_elements
                };
                #[cfg(not(feature = "pdf"))]
                let public_elements = {
                    for elem in elems.iter_mut() {
                        elem.page_number = document_page_number;
                    }
                    filter_public_ocr_elements(elems, base_ocr_config)
                };
                all_ocr_elements.extend(public_elements);
            }

            for mut formula in ocr_result.formulas {
                formula.page = Some(document_page_number);
                #[cfg(feature = "pdf")]
                if let Some((doc, _, _)) = lazy_pdf_render_state.as_ref() {
                    let (w, h) = (encoded_batch[offset].2, encoded_batch[offset].3);
                    formula_bbox_to_page_points(&mut formula, doc, page_idx, Some(&ocr_result.metadata), w, h);
                }
                accumulated_formulas.push(formula);
            }

            // force_ocr image-XObject fallback (#1355): xberg_native_pdf can catch an
            // image-decode error internally and substitute a blank white bitmap for
            // the whole-page render, so the page comes back from OCR as blank with no
            // indication anything was wrong. When that happens and the page actually
            // carries image XObjects, retry OCR directly on the embedded image bytes
            // (decoded pixels re-encoded to PNG, or the raw JPEG/JP2 stream) and always
            // surface a warning so the silent drop becomes visible.
            //
            // #1444 widened when this runs: a page whose backend call *failed* arrives here
            // as an empty document (see `batch_page_errors`), and a page the backend merely
            // *described* as blank is caught by the ink probe in
            // `page_needs_xobject_fallback` -- both used to sail past this block.
            #[cfg(feature = "pdf")]
            let default_security_limits = crate::extractors::security::SecurityLimits::default();
            let security_limits = config.security_limits.as_ref().unwrap_or(&default_security_limits);
            // Set when the embedded-image retry ran (attempted at least one image) but the
            // backend returned successfully with empty content on every one of them (#1673).
            let mut xobject_retry_ran_empty = false;
            let xobject_trigger = if has_stored_scan {
                super::rendering::XObjectFallbackTrigger {
                    needs_fallback: false,
                    draw_failed: false,
                }
            } else {
                page_needs_xobject_fallback(&ocr_result.content, encoded_batch[offset].1.as_slice(), security_limits)
            };
            if xobject_trigger.needs_fallback {
                // The layout-detection route hands in pre-rendered `images`, which leaves
                // `lazy_pdf_render_state` unopened; that used to disable this fallback
                // entirely for exactly the scanned documents most likely to need it. The
                // fallback needs nothing from the render state but the page's XObject
                // table, so it opens its own handle rather than perturbing the rotation and
                // points-per-pixel lookups that state is indexed for (#1444).
                let render_doc = match lazy_pdf_render_state.as_ref() {
                    Some((doc, _, _)) => Some(doc),
                    None => xobject_document.or_else(|| fallback_render_document(&mut fallback_pdf_state, content)),
                };
                if let Some(render_doc) = render_doc
                    && let Some(recovery) = recover_page_text_from_image_xobjects(
                        &backend,
                        render_doc,
                        document_page_idx,
                        progress_total_pages,
                        &ocr_config_owned,
                        XObjectRecoveryContext {
                            budget: &mut xobject_recovery_budget,
                            blank_raster_warnings: xobject_trigger.draw_failed.then_some(&mut image_fallback_warnings),
                            no_admitted_images: can_prove_no_admitted_images.then_some(&mut no_admitted_images),
                            stored_jpegs: None,
                            cancel_token: config.cancel_token.as_ref(),
                            use_proven_source_dpi: false,
                        },
                    )
                    .await?
                {
                    let XObjectRecoveryOutcome {
                        text,
                        text_outside_tables,
                        attempted,
                        images,
                        mut llm_usage,
                        mut tables,
                        mut formulas,
                        image_preprocessing,
                        tesseract_psm: _,
                        failure_causes: _,
                        used_proven_source_dpi: _,
                        possibly_illegible: _,
                    } = recovery;
                    let recovered_tables = !tables.is_empty();
                    let recovered_payload = !text.trim().is_empty() || recovered_tables || !formulas.is_empty();
                    if should_adopt_xobject_retry_text(&ocr_result.content, &text) {
                        ocr_result.content = text;
                        retry_texts_outside_tables[page_idx] = Some(text_outside_tables);
                        // The page text is now the retry text, not the lines the tables of this
                        // pass claimed.
                        if let Some(document) = ocr_result.ocr_internal_document.as_mut() {
                            document.ocr_text_claimed_by_tables = false;
                        }
                    } else if recovered_tables {
                        retry_texts_outside_tables[page_idx] = Some(text_outside_tables);
                    } else if attempted > 0 && !recovered_payload {
                        xobject_retry_ran_empty = true;
                    }
                    pages_with_recovered_xobject_payload[page_idx] = recovered_payload;
                    accumulated_llm_usage.append(&mut llm_usage);
                    collected_tables.append(&mut tables);
                    accumulated_formulas.append(&mut formulas);
                    if let Some(metadata) = image_preprocessing {
                        preprocessing_by_page.insert(document_page_number, metadata);
                    }
                    if capture_rasters {
                        captured_rasters.extend(images);
                    }
                    image_fallback_warnings.push(xobject_fallback_warning(
                        document_page_idx,
                        attempted,
                        xobject_trigger.draw_failed,
                    ));
                }
            }

            // Report a page whose backend call failed, now that the fallback above has had
            // its chance to recover it (#1444). Either way the failure is visible: a page
            // that vanishes silently is the defect this replaces.
            if let Some(error) = batch_page_errors[offset].take() {
                let recovered = !ocr_result.content.trim().is_empty() || pages_with_recovered_xobject_payload[page_idx];
                if !recovered && xobject_retry_ran_empty {
                    pages_with_empty_xobject_retry.push(document_page_number);
                }
                page_failure_warnings.push(crate::types::ProcessingWarning {
                    source: std::borrow::Cow::Borrowed("ocr"),
                    message: std::borrow::Cow::Owned(if recovered {
                        let recovered_kind = if ocr_result.content.trim().is_empty() {
                            "content"
                        } else {
                            "text"
                        };
                        format!(
                            "OCR of page {} failed ({error}); its {recovered_kind} was recovered from the page's \
                             embedded image XObjects instead.",
                            document_page_number
                        )
                    } else if xobject_retry_ran_empty {
                        format!(
                            "OCR of page {} failed ({error}); the retry on the page's embedded image \
                             XObjects also returned no text.",
                            document_page_number
                        )
                    } else {
                        format!(
                            "OCR of page {} failed and could not be recovered: {error}",
                            document_page_number
                        )
                    }),
                });
                page_failures.push(crate::types::OcrPageFailure {
                    page: document_page_number,
                    error: error.clone(),
                    recovered,
                });
                page_backend_errors.push((page_idx, error));
            }

            // Both are read by the `pdf` margin-warning/page-content code below in every
            // feature set, but only written inside the two assembly blocks -- so with
            // `layout-detection` and neither OCR frontend they stay at their initializers.
            // ~keep
            #[cfg(feature = "pdf")]
            #[cfg_attr(
                all(feature = "layout-detection", not(feature = "ocr"), not(feature = "ocr-wasm")),
                allow(unused_mut)
            )]
            let mut margin_filtered_content: Option<String> = None;
            #[cfg(feature = "pdf")]
            #[cfg_attr(
                all(feature = "layout-detection", not(feature = "ocr"), not(feature = "ocr-wasm")),
                allow(unused_mut)
            )]
            let mut margin_filter_complete = false;

            #[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
            if ocr_result.ocr_internal_document.is_some()
                || ocr_result
                    .ocr_elements
                    .as_ref()
                    .is_some_and(|elements| !elements.is_empty())
            {
                let elements = ocr_result.ocr_elements.as_deref().unwrap_or_default();
                let detection = layout_detections.and_then(|detections| detections.get(page_idx));

                let ocr_render_width = encoded_batch[offset].2;
                let ocr_render_height = encoded_batch[offset].3;
                let render_scaled_detection =
                    detection.map(|det| scale_detection_to_dimensions(det, ocr_render_width, ocr_render_height));
                let (_, ocr_layout_height) =
                    resolved_ocr_layout_dimensions(&ocr_result.metadata, ocr_render_width, ocr_render_height);
                ocr_page_heights[page_idx] = ocr_layout_height as f32;
                let points_per_pixel = detached_paragraph_scale
                    .map(|(_, sy)| sy)
                    .or(points_per_pixel_override)
                    .unwrap_or_else(|| {
                        ocr_points_per_pixel(
                            #[cfg(feature = "pdf")]
                            lazy_pdf_render_state.as_ref(),
                            page_idx,
                            ocr_layout_height,
                        )
                    });
                let ocr_scaled_detection = detection.map(|det| {
                    scale_detection_to_ocr_coordinates(det, &ocr_result.metadata, ocr_render_width, ocr_render_height)
                });
                let render_ocr_elements = transform_ocr_elements_to_render_space(
                    elements,
                    &ocr_result.metadata,
                    ocr_render_width,
                    ocr_render_height,
                );
                // ~keep A `None` transform means the word boxes cannot be placed in the raster
                // TATR crops from. Running TATR anyway loses every word to the cell-overlap
                // filter and publishes a table holding none of the page's values, so skip the
                // table and let the words stay in the page's paragraph stream
                // (xberg-io/xberg#1813).
                if render_ocr_elements.is_none() && !elements.is_empty() {
                    crate::core::diagnostics::push_warning_deduped(
                        &mut backend_page_warnings,
                        crate::types::ProcessingWarning {
                            source: std::borrow::Cow::Borrowed("ocr"),
                            message: std::borrow::Cow::Borrowed(
                                "OCR word geometry could not be mapped onto the rendered page, so layout-based table \
                                 recognition was skipped for a page; its text is reported as paragraphs instead.",
                            ),
                        },
                    );
                }

                let recognized_tables = if let (Some(scaled_det), Some(render_ocr_elements), true) =
                    (render_scaled_detection, render_ocr_elements, tatr_model.is_some())
                {
                    let rgb = if let Some(ref slice) = batch_slice {
                        let default_security_limits = crate::extractors::security::SecurityLimits::default();
                        let security_limits = config.security_limits.as_ref().unwrap_or(&default_security_limits);
                        crate::extraction::image_decode::clone_dynamic_image_to_rgb8_with_security_limits(
                            &slice[offset],
                            security_limits,
                        )?
                    } else {
                        let png_data = &encoded_batch[offset].1;
                        let default_security_limits = crate::extractors::security::SecurityLimits::default();
                        let security_limits = config.security_limits.as_ref().unwrap_or(&default_security_limits);
                        crate::extraction::image_decode::decode_standard_rgb8_with_security_limits(
                            png_data,
                            security_limits,
                        )
                        .map_err(|e| crate::XbergError::Parsing {
                            message: format!("Failed to decode PNG for TATR: {}", e),
                            source: None,
                        })?
                    };
                    let mut model = tatr_model.take().expect("checked tatr_model.is_some() above");
                    // #1812: TATR inference is synchronous ONNX Runtime CPU work with no await
                    // points; running it inline would occupy a tokio worker thread for the
                    // call's full duration on every table-bearing page. Route through
                    // `spawn_blocking` and hand the lease back through the join so the next
                    // page reuses it instead of it sitting checked out on a parked task. ~keep
                    let span = tracing::Span::current();
                    let (model, outcome) = tokio::task::spawn_blocking(move || {
                        let _entered = span.entered();
                        let outcome = crate::ocr::layout_assembly::recognize_page_tables_with_fallback(
                            &rgb,
                            &scaled_det,
                            &render_ocr_elements,
                            &mut model,
                        );
                        (model, outcome)
                    })
                    .await
                    .map_err(|e| crate::XbergError::Plugin {
                        message: format!("TATR table recognition panicked: {}", e),
                        plugin_name: "layout".to_string(),
                    })?;
                    tatr_model = Some(model);
                    outcome
                } else {
                    crate::ocr::layout_assembly::RecognizedTablesOutcome::default()
                };

                let table_frame = if let Some((pdf, _, _)) = lazy_pdf_render_state.as_ref() {
                    RecognizedTableFrame::PdfPage {
                        raster: (ocr_render_width, ocr_render_height),
                        points: page_dimensions_pt(pdf, page_idx),
                    }
                } else if xobject_document.is_some() {
                    RecognizedTableFrame::DetachedPdfRaster {
                        height: ocr_render_height,
                    }
                } else {
                    RecognizedTableFrame::ImagePixels
                };
                append_recognized_tables(
                    &recognized_tables,
                    document_page_number,
                    table_frame,
                    &mut collected_tables,
                    &mut collected_table_grids,
                );

                if let Some(ref ocr_doc) = ocr_result.ocr_internal_document {
                    // Same lookup as the per-page `page_rotation_degrees` computed above for
                    // the backend call (#760) -- that one is scoped to the join_set/backend
                    // loop and does not survive to here, so it is recomputed identically:
                    // an explicit override wins, otherwise the PDF's own known `/Rotate` for
                    // this page (falling back to the image-derived rotation when the render
                    // state was never opened), defaulting to `0` when nothing is known.
                    #[cfg(feature = "pdf")]
                    let page_rotation_degrees = if page_rotation_override != 0 {
                        page_rotation_override
                    } else {
                        lazy_pdf_render_state
                            .as_ref()
                            .and_then(|(_, _, rotations)| rotations.get(page_idx))
                            .or_else(|| external_image_page_rotations.as_ref().and_then(|r| r.get(page_idx)))
                            .copied()
                            .unwrap_or(0)
                    };
                    #[cfg(not(feature = "pdf"))]
                    let page_rotation_degrees: u32 = 0;
                    let mut paragraphs = assemble_ocr_page_paragraphs(
                        ocr_doc,
                        ocr_layout_height,
                        ocr_scaled_detection.as_ref(),
                        points_per_pixel,
                        page_rotation_degrees,
                    );
                    append_unrecognized_table_fallback_paragraphs(
                        &mut paragraphs,
                        &recognized_tables.unrecognized_table_text,
                    );
                    apply_ocr_layout_content_filter(&mut paragraphs, config);
                    #[cfg(feature = "pdf")]
                    {
                        let outcome = filter_ocr_paragraphs_by_page_margins(
                            &mut paragraphs,
                            ocr_layout_height as f32,
                            page_margins,
                        );
                        margin_filter_complete = !outcome.missing_geometry;
                        // #1574: `page_margins` is now non-zero only when the caller set
                        // `top_margin_fraction`/`bottom_margin_fraction` explicitly (defaults
                        // resolve to 0.0, so this branch cannot fire on a default config). A
                        // successful removal here is therefore the caller's own documented
                        // filter working as configured -- the native-PDF span filter
                        // (`extraction.rs`'s use of the same `PageMarginFractions`) emits no
                        // warning for the identical case, and `diagnostics.rs`'s house rule is
                        // to stay silent for a deliberate, documented filter unless the caller
                        // could plausibly believe the content was extracted. It was not: the
                        // caller asked for it to be removed. `outcome.missing_geometry` (the
                        // capability gap where the filter could NOT run) is the one case that
                        // still warns, below. ~keep
                        if outcome.removed {
                            margin_filtered_content = Some(ocr_paragraphs_plain_text(&paragraphs));
                        }
                    }

                    tracing::debug!(
                        page = document_page_number,
                        paragraphs = paragraphs.len(),
                        raw_content_len = ocr_result.content.len(),
                        "OCR page layout classification complete"
                    );

                    all_page_paragraphs[page_idx] = Some(paragraphs);
                    detached_page_scales[page_idx] = detached_paragraph_scale;
                    page_ocr_scales[page_idx] = run_page_ocr_scale(
                        #[cfg(feature = "pdf")]
                        lazy_pdf_render_state.as_ref(),
                        page_idx,
                        (ocr_render_width, ocr_render_height),
                    );
                    pages_text_claimed_by_tables[page_idx] = ocr_doc.ocr_text_claimed_by_tables;
                }

                if capture_rasters {
                    let (_, png_arc, w, h) = &encoded_batch[offset];
                    let png_bytes = bytes::Bytes::copy_from_slice(png_arc.as_ref());
                    captured_rasters.push(build_page_raster_image(document_page_idx, png_bytes, *w, *h));
                }
                let dict_invalid_word_ratio = ocr_result
                    .metadata
                    .additional
                    .get(crate::ocr_metadata_keys::OCR_TESSERACT_DICT_INVALID_WORD_RATIO_METADATA_KEY)
                    .and_then(|v| v.as_f64());
                let confidence = mean_text_conf_of(&ocr_result.metadata.additional);
                if let Some(summary) = page_ocr_confidence(
                    backend_confidence_semantics,
                    confidence,
                    word_count_of(&ocr_result.metadata.additional).unwrap_or(0),
                    &backend_name,
                ) {
                    ocr_confidence_by_page.insert(document_page_number, summary);
                }
                #[cfg(feature = "pdf")]
                if (page_margins.top != 0.0 || page_margins.bottom != 0.0)
                    && !margin_filter_complete
                    && !ocr_result.content.trim().is_empty()
                {
                    crate::core::diagnostics::push_warning_deduped(
                        &mut margin_filter_warnings,
                        ocr_margin_filter_capability_warning(),
                    );
                }
                #[cfg(feature = "pdf")]
                let page_content = margin_filtered_content.unwrap_or(ocr_result.content);
                #[cfg(not(feature = "pdf"))]
                let page_content = ocr_result.content;
                let acceptance = accept_or_reject_ocr_page(
                    document_page_idx,
                    page_content,
                    &ocr_output_thresholds,
                    &mut recognition_noise_warnings,
                    dict_invalid_word_ratio,
                    backend_confidence_semantics,
                    confidence,
                );
                page_texts[page_idx] = acceptance.content;
                rejected_pages[page_idx] = acceptance.discarded;
                if let Some(verdict) = acceptance.verdict {
                    recognition_noise_verdicts.push(verdict);
                }
                continue;
            }

            #[cfg(not(feature = "layout-detection"))]
            if let Some(ref ocr_doc) = ocr_result.ocr_internal_document {
                let ocr_render_width = encoded_batch[offset].2;
                let ocr_render_height = encoded_batch[offset].3;
                let (_, ocr_layout_height) =
                    resolved_ocr_layout_dimensions(&ocr_result.metadata, ocr_render_width, ocr_render_height);
                ocr_page_heights[page_idx] = ocr_layout_height as f32;
                let points_per_pixel = detached_paragraph_scale
                    .map(|(_, sy)| sy)
                    .or(points_per_pixel_override)
                    .unwrap_or_else(|| {
                        ocr_points_per_pixel(
                            #[cfg(feature = "pdf")]
                            lazy_pdf_render_state.as_ref(),
                            page_idx,
                            ocr_layout_height,
                        )
                    });
                // `ocr_doc`'s bbox AND ocr_geometry are both still raw OCR raster pixels here
                // (same pure-OCR-route reasoning as `assemble_ocr_page_paragraphs` above).
                let font_size_scale = crate::pdf::structure::adapters::OcrFontSizeScale::uniform(points_per_pixel);
                let mut paragraphs =
                    crate::pdf::structure::adapters::ocr_doc_to_paragraphs(ocr_doc, ocr_layout_height, font_size_scale);
                apply_ocr_layout_content_filter(&mut paragraphs, config);
                #[cfg(feature = "pdf")]
                {
                    let outcome =
                        filter_ocr_paragraphs_by_page_margins(&mut paragraphs, ocr_layout_height as f32, page_margins);
                    margin_filter_complete = !outcome.missing_geometry;
                    if outcome.removed {
                        margin_filtered_content = Some(ocr_paragraphs_plain_text(&paragraphs));
                    }
                }
                all_page_paragraphs[page_idx] = Some(paragraphs);
                detached_page_scales[page_idx] = detached_paragraph_scale;
                page_ocr_scales[page_idx] = run_page_ocr_scale(
                    #[cfg(feature = "pdf")]
                    lazy_pdf_render_state.as_ref(),
                    page_idx,
                    (ocr_render_width, ocr_render_height),
                );
                pages_text_claimed_by_tables[page_idx] = ocr_doc.ocr_text_claimed_by_tables;
            }

            let _ = page_idx;
            if capture_rasters {
                let (_, png_arc, w, h) = &encoded_batch[offset];
                let png_bytes = bytes::Bytes::copy_from_slice(png_arc.as_ref());
                captured_rasters.push(build_page_raster_image(document_page_idx, png_bytes, *w, *h));
            }
            let dict_invalid_word_ratio = ocr_result
                .metadata
                .additional
                .get(crate::ocr_metadata_keys::OCR_TESSERACT_DICT_INVALID_WORD_RATIO_METADATA_KEY)
                .and_then(|v| v.as_f64());
            let confidence = mean_text_conf_of(&ocr_result.metadata.additional);
            if let Some(summary) = page_ocr_confidence(
                backend_confidence_semantics,
                confidence,
                word_count_of(&ocr_result.metadata.additional).unwrap_or(0),
                &backend_name,
            ) {
                ocr_confidence_by_page.insert(document_page_number, summary);
            }
            #[cfg(feature = "pdf")]
            if (page_margins.top != 0.0 || page_margins.bottom != 0.0)
                && !margin_filter_complete
                && !ocr_result.content.trim().is_empty()
            {
                crate::core::diagnostics::push_warning_deduped(
                    &mut margin_filter_warnings,
                    ocr_margin_filter_capability_warning(),
                );
            }
            #[cfg(feature = "pdf")]
            let page_content = margin_filtered_content.unwrap_or(ocr_result.content);
            #[cfg(not(feature = "pdf"))]
            let page_content = ocr_result.content;
            let acceptance = accept_or_reject_ocr_page(
                document_page_idx,
                page_content,
                &ocr_output_thresholds,
                &mut recognition_noise_warnings,
                dict_invalid_word_ratio,
                backend_confidence_semantics,
                confidence,
            );
            page_texts[page_idx] = acceptance.content;
            rejected_pages[page_idx] = acceptance.discarded;
            if let Some(verdict) = acceptance.verdict {
                recognition_noise_verdicts.push(verdict);
            }
        }
    }

    #[cfg(feature = "layout-detection")]
    if let Some(model) = tatr_model.take() {
        crate::layout::return_tatr(model);
    }

    // A cancelled run fails every page by design, which the #1444 guard below would report as
    // a wholesale backend failure. Surface the cancellation itself so a caller can tell a
    // cancelled extraction apart from a genuinely broken OCR backend. ~keep
    if config.cancel_token.as_ref().is_some_and(|t| t.is_cancelled()) {
        return Err(crate::XbergError::Cancelled.into());
    }

    // Degrading a per-page failure to a warning must not turn a wholesale OCR failure into a
    // silently empty document: when every page failed *and* nothing was recovered from any
    // page's embedded images, there is no partial result to preserve, so report it (#1444).
    if !page_backend_errors.is_empty()
        && page_backend_errors.len() == total_pages
        && page_texts.iter().all(|text| text.trim().is_empty())
        && pages_with_recovered_xobject_payload.iter().all(|recovered| !recovered)
    {
        let (_, first_error) = &page_backend_errors[0];
        let message = if pages_with_empty_xobject_retry.is_empty() {
            format!(
                "OCR failed on all {total_pages} page(s) and no text could be recovered from the pages' \
                 embedded images; first failure: {first_error}"
            )
        } else {
            format!(
                "OCR failed on all {total_pages} page(s); first failure: {first_error}. The embedded-image \
                 retry ran on {} of these page(s) and also returned no text.",
                pages_with_empty_xobject_retry.len()
            )
        };
        return Err(FailedOcrRun {
            error: crate::XbergError::Plugin {
                message,
                plugin_name: "ocr".to_string(),
            },
            page_failures,
        });
    }

    let mean_text_conf = match (conf_count > 0, backend_confidence_scale) {
        (true, Some(scale_max)) => Some((conf_sum / conf_count as f64) / scale_max),
        _ => None,
    };

    // Pages the verdict flagged but kept lose their symbol-noise lines only now, so the
    // verdict, backend choice and table acceptance above judged the unpruned text (#2312).
    // The pruned text is what this function returns; pipeline stage scoring reads
    // `OcrReadEvidence::unpruned_text` instead, so pruning never changes stage selection.
    let mut unpruned_page_texts: Vec<(usize, String)> = Vec::new();
    for verdict in recognition_noise_verdicts.iter().filter(|verdict| !verdict.discarded) {
        let Some(page_idx) = verdict.page_index.checked_sub(page_index_offset) else {
            continue;
        };
        let page_number = (verdict.page_index + 1) as u32;
        if page_idx >= page_texts.len()
            || rejected_pages[page_idx]
            || collected_tables.iter().any(|table| table.page_number == page_number)
        {
            continue;
        }
        if ocr_output_thresholds.prune_symbol_noise_lines
            && super::symbol_noise_lines::page_is_dominated_by_noise_lines(page_texts[page_idx].lines())
        {
            let (pruned, removed) = super::symbol_noise_lines::prune_text(&page_texts[page_idx]);
            unpruned_page_texts.push((page_idx, std::mem::replace(&mut page_texts[page_idx], pruned)));
            if let Some(paragraphs) = all_page_paragraphs[page_idx].as_mut() {
                super::symbol_noise_lines::prune_paragraphs(paragraphs);
            }
            recognition_noise_warnings.push(super::symbol_noise_lines::pruned_lines_warning(
                verdict.page_index + 1,
                removed,
            ));
        }
    }

    let page_marker_cfg = config.pages.as_ref().filter(|p| p.insert_page_markers);
    let join_pages = |texts: &[String]| {
        let mut joined = String::new();
        for (i, text) in texts.iter().enumerate() {
            if let Some(cfg) = page_marker_cfg {
                let marker = cfg
                    .marker_format
                    .replace("{page_num}", &(page_index_offset + i + 1).to_string());
                joined.push_str(&marker);
            } else if i > 0 {
                joined.push_str("\n\n");
            }
            joined.push_str(text);
        }
        joined
    };
    #[cfg_attr(
        not(all(
            feature = "ocr",
            feature = "pdf",
            feature = "tokio-runtime",
            not(target_arch = "wasm32")
        )),
        allow(unused_mut)
    )]
    let mut result = join_pages(&page_texts);
    #[cfg_attr(
        not(all(
            feature = "ocr",
            feature = "pdf",
            feature = "tokio-runtime",
            not(target_arch = "wasm32")
        )),
        allow(unused_mut)
    )]
    let mut unpruned_text = (!unpruned_page_texts.is_empty()).then(|| {
        let mut texts = page_texts.clone();
        for (page_idx, original) in &unpruned_page_texts {
            texts[*page_idx] = original.clone();
        }
        join_pages(&texts)
    });

    // A page the quality gate rejected must not keep its structured paragraphs either.
    //
    // The paragraphs and the page text are two renderings of the same OCR output, built by
    // different assemblies: `all_page_paragraphs` above, `page_texts` from
    // `accept_or_reject_ocr_page`. Only the text was ever judged, and the rendered document
    // is built exclusively from the paragraphs -- so a scanned survey plat lost its text,
    // logged its warning, and rendered its garbage anyway. The mixed route already couples
    // the two (`ocr_results.retain` feeding `merge_structured_ocr_pages_into_internal_document`);
    // this is that same coupling for the force-ocr route, which never had it.
    //
    // Clearing before `fill_unstructured_ocr_pages` is deliberate and safe: that function
    // only rebuilds paragraphs from `page_texts[page_index]`, which is empty for exactly
    // these pages, so it cannot resurrect them.
    for (page_idx, page_rejected) in rejected_pages.iter().enumerate() {
        if *page_rejected {
            all_page_paragraphs[page_idx] = None;
        }
    }
    discard_ocr_elements_from_rejected_pages(&mut all_ocr_elements, &rejected_pages, page_index_offset);
    discard_rejected_ocr_page_payloads(
        &mut collected_tables,
        &mut accumulated_formulas,
        &rejected_pages,
        page_index_offset,
    );

    #[cfg(feature = "pdf")]
    {
        let retained_payload_pages: ahash::AHashSet<u32> = collected_tables
            .iter()
            .map(|table| table.page_number)
            .chain(accumulated_formulas.iter().filter_map(|formula| formula.page))
            .collect();
        let retained_noise_pages: ahash::AHashSet<_> = recognition_noise_verdicts
            .iter()
            .filter(|verdict| !verdict.discarded)
            .map(|verdict| verdict.page_index)
            .collect();
        for (page_idx, text) in page_texts.iter().enumerate() {
            let document_page_index = page_index_offset + page_idx;
            if (text.trim().is_empty() || retained_noise_pages.contains(&document_page_index))
                && !retained_payload_pages.contains(&((document_page_index + 1) as u32))
                && let Some(dpi) = raster_dpis.get(&page_idx)
                && let Some(warning) = dpi.empty_text_warning(document_page_index)
            {
                crate::core::diagnostics::push_warning_deduped(&mut backend_page_warnings, warning);
            }
        }
    }

    // A page that adopted the retry text is refilled from the retry lines no recovered table
    // carries, and is claimed by its tables when they carry every line (#2014).
    let mut refill_texts = std::borrow::Cow::Borrowed(page_texts.as_slice());
    for (page_idx, text_outside_tables) in retry_texts_outside_tables.into_iter().enumerate() {
        if let Some(text_outside_tables) = text_outside_tables
            && !rejected_pages[page_idx]
        {
            pages_text_claimed_by_tables[page_idx] = text_outside_tables.trim().is_empty();
            refill_texts.to_mut()[page_idx] = text_outside_tables;
        }
    }
    // ~keep Detached PDF pages retain their PDF document; only standalone images reach
    // this fallback without either PDF source. Remove legacy table transcripts before
    // paragraph synthesis, preserving the raw OCR text and authoritative public nodes.
    if content.is_none() && xobject_document.is_none() {
        for (page_idx, paragraphs) in all_page_paragraphs.iter().enumerate() {
            let page = (page_index_offset + page_idx + 1) as u32;
            if paragraphs.as_ref().is_some_and(|paragraphs| !paragraphs.is_empty())
                || public_ocr_pages.contains_key(&page)
                || rejected_pages[page_idx]
            {
                continue;
            }
            let text = &refill_texts[page_idx];
            let prose = super::strip_embedded_gfm_tables(
                text,
                collected_tables.iter().filter(|table| table.page_number == page),
            );
            if prose != *text {
                pages_text_claimed_by_tables[page_idx] = prose.trim().is_empty();
                refill_texts.to_mut()[page_idx] = prose;
            }
        }
    }
    // ~keep A standalone image has no later native-PDF table handoff. Complete cell-only
    // sidecars before assembly so every accepted table receives its canonical IR node.
    // This follows page rejection and transcript stripping, and precedes normalization
    // and authoritative public-node replacement (#2226).
    let standalone_image = content.is_none() && xobject_document.is_none();
    if standalone_image {
        for table in &mut collected_tables {
            if table.markdown.trim().is_empty() {
                table.markdown = crate::rendering::common::render_table_markdown(&table.cells);
            }
        }
    }
    fill_unstructured_ocr_pages(&mut all_page_paragraphs, &refill_texts, &pages_text_claimed_by_tables);

    let (ocr_doc, mut raw_page_paragraphs) = {
        let has_structured =
            all_page_paragraphs
                .iter()
                .zip(&pages_text_claimed_by_tables)
                .any(|(paragraphs, claimed_by_tables)| {
                    paragraphs.as_ref().is_some_and(|paragraphs| !paragraphs.is_empty()) || *claimed_by_tables
                })
                || (standalone_image && collected_tables.iter().any(|table| !table.markdown.trim().is_empty()));
        if has_structured {
            let pages: Vec<Vec<crate::pdf::structure::types::PdfParagraph>> = all_page_paragraphs
                .into_iter()
                .map(|opt| opt.unwrap_or_default())
                .collect();
            #[cfg(feature = "layout-detection")]
            let pages = {
                let mut pages = pages;
                crate::pdf::structure::adapters::promote_anchored_ordered_list_sequences(&mut pages);
                pages
            };
            // Document-global heading/list heuristic: `ocr_doc_to_paragraphs` /
            // `ocr_doc_to_layout_paragraphs` (above) build `pages` one OCR page at a
            // time and never call it, which is why every non-layout OCR route has
            // emitted zero headings and zero list items regardless of output format.
            // `None` (Plain output, or pages ML layout already classified) falls
            // through to the pre-existing assembly unchanged. Skipped entirely when
            // `skip_document_global_heuristic` is set -- see this function's own doc
            // comment: the caller wants the *bare* `pages` back to run this pass
            // itself, document-wide, and a heuristic run here first would either be
            // wasted (too few pages to cluster) or would mark `pages` "already
            // structured" and block that wider pass from doing anything.
            #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
            let detached_table_document = (detached_table_pair
                && pages.as_slice().first().is_some_and(Vec::is_empty)
                && public_ocr_pages.is_empty()
                && pages_with_recovered_xobject_payload == [false])
            .then(|| detached_table_pair::assemble(&collected_tables))
            .flatten();
            #[cfg(not(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32"))))]
            let detached_table_document: Option<crate::types::internal::InternalDocument> = None;
            let doc = if detached_table_document.is_some() {
                detached_table_document
            } else if skip_document_global_heuristic {
                Some(crate::pdf::structure::assemble_internal_document(
                    pages.clone(),
                    &collected_tables,
                    None,
                    &[],
                    &Default::default(),
                ))
            } else {
                match heuristically_restructured_ocr_pages(&pages, &ocr_page_heights, &collected_tables, config) {
                    Some(doc) => Some(doc),
                    None => {
                        // The document-global heading/list heuristic declined to run (Plain
                        // output, or a paragraph was already ML-layout-classified) or found
                        // nothing. Apply a text-marker-only list pass to the bare pages before
                        // falling back to plain assembly, so a non-layout OCR document that
                        // never reaches the heuristic still gets *some* list-item recovery
                        // (#713) -- this never touches `heading_level`, so it cannot regress
                        // heading detection the way pre-classifying before the heuristic call
                        // would (see `apply_ocr_text_list_fallback`'s own doc comment on the
                        // "already structured" gate this must not trip).
                        let mut fallback_pages = pages.clone();
                        for (page_idx, page) in fallback_pages.iter_mut().enumerate() {
                            apply_ocr_text_list_fallback(page);
                            // #729: recover a bare marker no ML hint ever classified --
                            // see `adapters::reattach_detached_ocr_list_markers`'s doc
                            // comment.
                            // #760: same rotation lookup as `assemble_ocr_page_paragraphs`'s
                            // call site above -- an explicit override wins, otherwise this
                            // page's own known PDF `/Rotate`, defaulting to `0`.
                            #[cfg(feature = "pdf")]
                            let page_rotation_degrees = if page_rotation_override != 0 {
                                page_rotation_override
                            } else {
                                lazy_pdf_render_state
                                    .as_ref()
                                    .and_then(|(_, _, rotations)| rotations.get(page_idx))
                                    .or_else(|| external_image_page_rotations.as_ref().and_then(|r| r.get(page_idx)))
                                    .copied()
                                    .unwrap_or(0)
                            };
                            #[cfg(not(feature = "pdf"))]
                            let page_rotation_degrees: u32 = 0;
                            crate::pdf::structure::adapters::reattach_detached_ocr_list_markers(
                                page,
                                page_rotation_degrees,
                            );
                        }
                        Some(crate::pdf::structure::assemble_internal_document(
                            fallback_pages,
                            &collected_tables,
                            None,
                            &[],
                            &Default::default(),
                        ))
                    }
                }
            };
            let pages = pages
                .into_iter()
                .zip(page_ocr_scales)
                .zip(detached_page_scales)
                .map(|((paragraphs, ocr_scale), detached_ocr_scale)| RunPageParagraphs {
                    paragraphs,
                    ocr_scale,
                    detached_ocr_scale,
                })
                .collect();
            (doc, pages)
        } else {
            (None, Vec::new())
        }
    };

    // Public nodes are already semantic structure. Do not offer flattened copies
    // to the mixed route's document-global paragraph reconstruction pass.
    for page in public_ocr_pages.keys() {
        if let Some(index) = (*page as usize).checked_sub(page_index_offset + 1)
            && let Some(page) = raw_page_paragraphs.get_mut(index)
        {
            page.paragraphs.clear();
        }
    }
    let ocr_doc = if public_ocr_pages.is_empty() {
        ocr_doc
    } else {
        let accepted: ahash::AHashMap<u32, String> = page_texts
            .iter()
            .enumerate()
            .filter(|(index, _)| !rejected_pages[*index])
            .filter_map(|(index, text)| {
                let page = (page_index_offset + index + 1) as u32;
                public_ocr_pages.contains_key(&page).then(|| (page, text.clone()))
            })
            .collect();
        let mut document = ocr_doc.unwrap_or_else(|| crate::types::internal::InternalDocument::new("pdf"));
        super::document::merge_structured_ocr_pages_into_internal_document(&mut document, &accepted, &public_ocr_pages);
        Some(document)
    };

    #[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
    let ocr_doc = match ocr_doc {
        Some(mut document) => {
            crate::types::internal::table_grid::attach_recognized_table_grids(&mut document, &collected_table_grids)
                .map_err(|error| crate::XbergError::Parsing {
                    message: "Failed to retain recognized PDF OCR table geometry".to_owned(),
                    source: Some(Box::new(error)),
                })?;
            Some(document)
        }
        None => None,
    };

    #[cfg(all(
        feature = "ocr",
        feature = "pdf",
        feature = "tokio-runtime",
        not(target_arch = "wasm32")
    ))]
    let ocr_doc = {
        let mut ocr_doc = ocr_doc;
        if let (Some(proof), Some(document), [page], Some((pdf, _, _))) = (
            tall_ordered_primary,
            ocr_doc.as_mut(),
            page_texts.as_mut_slice(),
            lazy_pdf_render_state.as_ref(),
        ) && *page == result
            && super::tall_source::augmentation::augment(
                proof,
                (document, page),
                &backend,
                config,
                base_ocr_config,
                pdf,
            )
            .await?
        {
            result.clone_from(page);
            unpruned_text = None;
        }
        ocr_doc
    };

    #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
    if let Some(native) = page_ocr_hints
        .as_ref()
        .and_then(|hints| hints.single_native_type0_page.as_deref())
        && let [read] = page_texts.as_slice()
        && page_index_offset == 0
        && images.is_none()
        && !has_layout_detections
        && raster_dpis.get(&0).is_some_and(|raster| raster.type0_ink_loss)
        && super::document::native_preservation_is_automatic(config, 1)
        && get_ocr_backend_registry().read().is_builtin_tesseract(&backend)
        && !tall_source_used
        && pages_with_recovered_xobject_payload == [false]
        && public_ocr_pages.is_empty()
        && collected_tables.is_empty()
        && accumulated_formulas.is_empty()
        && accumulated_llm_usage.is_empty()
        && ocr_doc
            .as_ref()
            .is_none_or(|doc| super::document::plain_ocr_page_can_yield_to_native(doc, read))
        && super::document::type0_native_is_corroborated(
            native,
            read,
            &ocr_output_thresholds,
            native_ink_owned,
            complete_native_words,
        )
    {
        for warning in render_warnings {
            crate::pdf::render::record_render_warning(warning);
        }
        return Err(type0_ocr_rejection());
    }

    #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
    if native_ink_owned
        && detached_page.is_none()
        && images.is_none()
        && page_index_offset == 0
        && !has_layout_detections
        && !tall_source_used
        && let [read] = page_texts.as_slice()
        && !read.trim().is_empty()
        && pages_with_recovered_xobject_payload == [false]
        && public_ocr_pages.is_empty()
        && collected_tables.is_empty()
        && accumulated_formulas.is_empty()
        && accumulated_llm_usage.is_empty()
        && ocr_doc
            .as_ref()
            .is_none_or(|doc| super::native_word_coverage::only_segments_certified_tesseract_text(doc, read))
    {
        for warning in render_warnings {
            crate::pdf::render::record_render_warning(warning);
        }
        let lost = page_ocr_hints
            .as_ref()
            .and_then(|hints| hints.single_native_type0_page.as_deref())
            .map_or(0, |native| {
                native.matches(super::super::unmapped_glyphs::LOST_GLYPH_MARKER).count()
            });
        return Err(lost_glyph_ocr_rejection(lost));
    }

    #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
    let discarded_visible_native_page = matches!(recognition_noise_verdicts.as_slice(), [verdict] if verdict.page_index == 0 && verdict.discarded)
        && page_index_offset == 0
        && matches!(page_texts.as_slice(), [text] if text.trim().is_empty())
        && result.trim().is_empty()
        && ocr_doc.is_none()
        && !has_layout_detections
        && !capture_rasters
        && collected_tables.is_empty()
        && all_ocr_elements.is_empty()
        && accumulated_formulas.is_empty()
        && accumulated_llm_usage.is_empty()
        && page_failures.is_empty()
        && config.ocr_strategy == crate::core::config::OcrStrategy::Auto
        && super::document::native_preservation_is_automatic(config, 1)
        && get_ocr_backend_registry().read().is_builtin_tesseract(&backend)
        && certify_discarded_native_layer(lazy_pdf_render_state, page_ocr_hints.as_ref(), config).await?;
    #[cfg(not(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32"))))]
    let discarded_visible_native_page = false;

    #[cfg(feature = "pdf")]
    let ocr_doc = {
        let mut warnings = image_fallback_warnings;
        warnings.extend(recognition_noise_warnings);
        warnings.extend(page_failure_warnings);
        warnings.extend(margin_filter_warnings);
        warnings.extend(backend_page_warnings);
        let mut ocr_doc = attach_ocr_fallback_warnings(ocr_doc, &result, warnings);
        if let Some(doc) = ocr_doc.as_mut() {
            doc.ocr_page_failures = page_failures;
        }
        // #1575: only merged when a document already exists (or the warnings step above just
        // built one from `text`) -- forcing an empty document into existence here for the sole
        // purpose of carrying metadata would replace `select_pdf_document`'s `flat_pdf_document`
        // fallback with an empty one, silently deleting the page text it currently preserves.
        if let (Some(doc), Some(additional)) = (ocr_doc.as_mut(), backend_additional_metadata) {
            doc.metadata.additional.extend(additional);
        }
        if let Some(doc) = ocr_doc.as_mut() {
            let accepted_modes = page_segmentation_modes.into_iter().filter(|(page_number, _)| {
                let local_index = (*page_number as usize).saturating_sub(page_index_offset + 1);
                !rejected_pages.get(local_index).copied().unwrap_or(true)
                    && page_texts.get(local_index).is_some_and(|text| !text.trim().is_empty())
            });
            attach_page_segmentation_modes(doc, accepted_modes);
        }
        ocr_doc
    };
    // Without `pdf` there is no page renderer, so no page-level OCR runs and the vector is
    // always empty; bind it so the non-pdf build does not warn about an unused value.
    #[cfg(not(feature = "pdf"))]
    let _ = (
        recognition_noise_warnings,
        page_failure_warnings,
        backend_page_warnings,
        backend_additional_metadata,
        page_segmentation_modes,
    );

    #[cfg(feature = "pdf")]
    for warning in render_warnings {
        crate::pdf::render::record_render_warning(warning);
    }

    Ok((
        result,
        mean_text_conf,
        collected_tables,
        all_ocr_elements,
        ocr_doc,
        accumulated_llm_usage,
        page_texts,
        if capture_rasters { Some(captured_rasters) } else { None },
        accumulated_formulas,
        raw_page_paragraphs,
        preprocessing_by_page,
        ocr_confidence_by_page,
        recognition_noise_verdicts,
        OcrReadEvidence {
            #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
            caption,
            no_admitted_images,
            unpruned_text,
            discarded_visible_native_page,
            #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
            complete_native_words,
            #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
            native_ink_owned,
            original_tesseract_raster: {
                #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
                {
                    detached_page.is_some()
                        && images.is_some_and(|images| images.len() == 1)
                        && can_retry_orientation
                        && !tall_source_used
                        && pages_with_recovered_xobject_payload == [false]
                }
                #[cfg(not(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32"))))]
                {
                    false
                }
            },
        },
    ))
}
#[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
async fn certify_discarded_native_layer(
    render: Option<(xberg_native_pdf::PdfDocument, usize, Vec<u32>)>,
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
    // handed the widest batch to exactly the host with the least memory, and `memory.current`
    // counts page cache, so a container reporting no headroom is routine. ~keep
    let Some(available_bytes) = available_bytes else {
        return configured;
    };
    if per_page_bytes == 0 {
        return configured;
    }

    let reserved = document_size + 512 * 1024 * 1024;
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

/// One page's render-and-encode working set at the DPI this route renders it: the raster,
/// the RGB clone the PNG encoder needs, and the PNG buffer, which is what a batch holds per
/// page while it runs. Falls back to [`UNMEASURED_OCR_PAGE_WORKING_SET_BYTES`] when the
/// dimensions overflow the estimate.
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) fn ocr_page_working_set_bytes(
    page_width_pt: f64,
    page_height_pt: f64,
    images_config: Option<&crate::core::config::ImageExtractionConfig>,
) -> usize {
    const PDF_POINTS_PER_INCH: f64 = 72.0;

    let render_dpi = crate::image::dpi::effective_pdf_render_dpi(images_config, page_width_pt, page_height_pt);
    let width = ((page_width_pt / PDF_POINTS_PER_INCH) * f64::from(render_dpi))
        .round()
        .max(1.0) as u32;
    let height = ((page_height_pt / PDF_POINTS_PER_INCH) * f64::from(render_dpi))
        .round()
        .max(1.0) as u32;

    estimate_png_encode_page_peak_bytes(width, height)
        .ok()
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
    stage_failures: &[(String, String)],
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
        }
    }

    pub(super) fn check_cancelled(&self) -> crate::Result<()> {
        if self.cancel_token.is_some_and(|token| token.is_cancelled()) {
            return Err(crate::XbergError::Cancelled);
        }
        Ok(())
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
    }
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
    let page_marker_cfg = config.pages.as_ref().filter(|pages| pages.insert_page_markers);
    let mut text = String::new();
    for (page_idx, page_text) in page_texts.iter().enumerate() {
        if let Some(cfg) = page_marker_cfg {
            let marker = cfg
                .marker_format
                .replace("{page_num}", &(page_offset + page_idx + 1).to_string());
            budget.account_text(marker.len())?;
            text.push_str(&marker);
        } else if page_idx > 0 {
            budget.account_text(2)?;
            text.push_str("\n\n");
        }
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
    let mut outcome = collect_pipeline_xobject_pages(
        &backend,
        &doc,
        page_count,
        &recovery_ocr_config,
        XObjectRecoveryContext::with_budget(&mut budget, cancel_token),
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
        let lost = super::document::put_layers_first_in_run(
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
    let thresholds = config
        .ocr
        .as_ref()
        .and_then(|ocr| ocr.quality_thresholds.clone())
        .unwrap_or_default();
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

/// The OCR paragraphs of one page of a whole-document run, in the pixels of the raster the page
/// was read from, and the factor per axis that takes that raster to PDF points (the page's
/// MediaBox over the raster). The factor is `None` when the run rendered no PDF page (an image
/// run): the layer merge of that page then removes nothing (rule 3). ~keep
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[derive(Debug, Default)]
pub(super) struct RunPageParagraphs {
    pub(super) paragraphs: Vec<crate::pdf::structure::types::PdfParagraph>,
    pub(super) ocr_scale: Option<(f32, f32)>,
    pub(super) detached_ocr_scale: Option<(f32, f32)>,
}

/// The pixel frame the OCR paragraphs of a page are in when preprocessing resampled the render
/// before recognition: the resampled dimensions, not the render's. ~keep
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
fn processed_frame_px(metadata: &crate::types::ImagePreprocessingMetadata) -> Option<(u32, u32)> {
    let resized = metadata.new_dimensions?;
    Some((u32::try_from(resized.width).ok()?, u32::try_from(resized.height).ok()?))
}

/// The factor per axis from the raster a page of a whole-document run was read from to PDF
/// points, the same conversion the mixed route applies to its pipeline pages. `None` when the
/// run rendered no PDF page or the raster is empty. ~keep
#[cfg(all(
    any(feature = "ocr", feature = "ocr-pipeline"),
    any(not(feature = "layout-detection"), feature = "ocr", feature = "ocr-wasm")
))]
fn run_page_ocr_scale(
    #[cfg(feature = "pdf")] lazy_pdf_render_state: Option<&(xberg_native_pdf::PdfDocument, usize, Vec<u32>)>,
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
    crate::pdf::native::NativeDocument::open_bytes_with_passwords(content, passwords).ok()
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
/// Why a retry read made at a placement-proven resolution was not adopted.
#[cfg(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
const PROVEN_READ_NOISE_CAUSE: &str = "the retry read was rejected as noise";

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
            retry_causes.push(PROVEN_READ_NOISE_CAUSE.to_owned());
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
        retry_causes.push(PROVEN_READ_NOISE_CAUSE.to_owned());
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

/// The error of a page whose embedded-image retry also recovered nothing, naming why each image
/// failed. The typed source stays, so callers still match on the original cause.
#[cfg(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
fn with_retry_causes(error: crate::XbergError, causes: &[String]) -> crate::XbergError {
    match error {
        crate::XbergError::Parsing { message, source } if !causes.is_empty() => {
            let mut named = causes.iter().take(RETRY_CAUSES_SHOWN).cloned().collect::<Vec<_>>();
            if causes.len() > RETRY_CAUSES_SHOWN {
                named.push(format!("+{} more", causes.len() - RETRY_CAUSES_SHOWN));
            }
            crate::XbergError::Parsing {
                message: format!(
                    "{message}; embedded-image retry recovered no text ({})",
                    named.join("; ")
                ),
                source,
            }
        }
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
    let mut stage_failures: Vec<(String, String)> = Vec::new();
    #[cfg(feature = "pdf")]
    let mut preferred_recovery_config = None;

    let automatic_paddle = inputs.detached_page.is_some()
        && uses_default_ocr_pipeline(ocr_config)
        && ocr_config.backend == "tesseract"
        && matches!(available_stages.as_slice(), [first, second]
            if first.backend == "tesseract" && second.backend == "paddleocr");

    for stage in &available_stages {
        let mut stage_ocr = ocr_config.clone();
        super::stage_model_inputs::StageModelInputs::new(ocr_config, stage).apply_to(&mut stage_ocr);
        if stage.paddle_ocr_config.is_some() || stage.paddle_ocr_settings.is_some() {
            stage_ocr.paddle_ocr_config = stage.paddle_ocr_config.clone();
            stage_ocr.paddle_ocr_settings = stage.paddle_ocr_settings.clone();
        }
        stage_ocr.quality_thresholds = Some(pipeline.quality_thresholds.clone());
        stage_ocr.vlm_config = stage.vlm_config.clone();
        stage_ocr.backend_options = stage.backend_options.clone();

        let stage_config = ExtractionConfig {
            ocr: Some(stage_ocr),
            ..config.clone()
        };
        #[cfg(feature = "pdf")]
        if preferred_recovery_config.is_none() {
            preferred_recovery_config = Some(pre_page_ocr_config(&stage_config).into_owned());
        }

        tracing::debug!(
            backend = %stage.backend,
            priority = stage.priority,
            "Pipeline: trying OCR backend"
        );

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
            inputs.page_ocr_hints.clone(),
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
                    OcrReadEvidence::default()
                };
                if score >= pipeline.quality_thresholds.pipeline_min_quality || skip_automatic_paddle {
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
                if crate::extraction::derive::is_invalid_public_ocr_document(&failed_stage.error) {
                    return Err(failed_stage);
                }
                tracing::warn!(
                    backend = %stage.backend,
                    error = %failed_stage.error,
                    "Pipeline: backend failed, trying next"
                );
                stage_failures.push((stage.backend.clone(), failed_stage.error.to_string()));
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
                let doc =
                    attach_ocr_pipeline_stage_warnings(None, &recovery.text, &unavailable_backends, &stage_failures);
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
            Err(crate::XbergError::Parsing {
                message: format!("All OCR pipeline backends failed{detail}"),
                source: None,
            }
            .into())
        }
    }
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

#[cfg(all(test, feature = "pdf", feature = "layout-detection", feature = "ocr"))]
#[path = "pipeline/budget_refusal_tests.rs"]
mod budget_refusal_tests;

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

#[cfg(all(test, feature = "ocr", feature = "pdf", not(target_arch = "wasm32")))]
#[path = "pipeline/preprocessing_density_tests.rs"]
mod preprocessing_density_tests;

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
