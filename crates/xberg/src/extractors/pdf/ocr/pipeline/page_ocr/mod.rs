//! Phase methods for [`super::extract_with_ocr_for_page`].
//!
//! The per-page OCR route resolves its backend and render plan once, then renders, reads and
//! assembles each page batch. Each phase keeps its code in its own module and shares the
//! borrow-heavy state below, so a phase can read and write it without re-deriving it.

mod batches;
mod finish;
mod pages;
mod raster;
mod reads;
mod setup;

// The phases resolve their helpers through this module: the code they hold used to live
// beside [`super::extract_with_ocr_for_page`] and named them as `super::<module>`.
#[cfg(all(
    feature = "ocr",
    feature = "layout-detection",
    feature = "tokio-runtime",
    not(target_arch = "wasm32")
))]
use super::super::column_retry;
use super::super::{
    caption_conversion, document, hidden_layer, layer_token_recovery, native_word_coverage, number_recovery,
    orientation_retry, read_reuse, rendering, strip_embedded_gfm_tables, symbol_noise_background, symbol_noise_lines,
    tall_source,
};
#[cfg(all(paddle_ocr, feature = "ocr", feature = "pdf", not(target_arch = "wasm32")))]
use super::stored_scan_retry;
use super::{DetachedOcrPage, OcrPageNoiseVerdict, OcrReadEvidence, OcrRunResult, PageOcrHints, RunPageParagraphs};

use crate::core::config::ExtractionConfig;

/// Everything one [`super::extract_with_ocr_for_page`] call returns.
pub(super) type PageOcrOutput = (
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
);

/// State one per-page OCR run carries between its phases.
pub(super) struct PageOcrContext<'a> {
    // Caller inputs.
    content: Option<&'a [u8]>,
    images: Option<&'a [image::DynamicImage]>,
    #[cfg(feature = "layout-detection")]
    layout_detections: Option<&'a [crate::layout::DetectionResult]>,
    config: &'a ExtractionConfig,
    page_rotation_override: u32,
    skip_document_global_heuristic: bool,
    points_per_pixel_override: Option<f32>,
    page_ocr_hints: Option<PageOcrHints>,
    page_index_offset: usize,
    detached_page: Option<DetachedOcrPage<'a>>,
    xobject_document: Option<&'a xberg_native_pdf::PdfDocument>,
    #[cfg(all(paddle_ocr, feature = "ocr", feature = "pdf", not(target_arch = "wasm32")))]
    stored_scan: Option<&'a stored_scan_retry::StoredScan>,
    has_stored_scan: bool,
    has_layout_detections: bool,

    // Resolved routing, backend and render plan.
    resolved_ocr_config: std::borrow::Cow<'a, crate::core::config::ocr::OcrConfig>,
    ocr_config_owned: crate::core::config::ocr::OcrConfig,
    ocr_output_thresholds: crate::core::config::OcrQualityThresholds,
    backend: std::sync::Arc<dyn crate::plugins::OcrBackend>,
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    builtin_tesseract: bool,
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    can_retry_orientation: bool,
    can_prove_no_admitted_images: bool,
    no_admitted_images: bool,
    backend_confidence_semantics: crate::plugins::ConfidenceSemantics,
    backend_name: String,
    backend_confidence_scale: Option<f64>,
    #[cfg(feature = "pdf")]
    orientation_handling: crate::plugins::PageOrientationHandling,
    #[cfg(feature = "pdf")]
    page_margins: crate::pdf::native::text::PageMarginFractions,
    #[cfg(all(
        feature = "ocr",
        feature = "pdf",
        feature = "tokio-runtime",
        not(target_arch = "wasm32")
    ))]
    tall_source_unsupported: Option<&'static str>,
    #[cfg(all(
        feature = "ocr",
        feature = "pdf",
        feature = "tokio-runtime",
        not(target_arch = "wasm32")
    ))]
    try_tall_source: bool,
    #[cfg(all(
        feature = "ocr",
        feature = "pdf",
        feature = "tokio-runtime",
        not(target_arch = "wasm32")
    ))]
    tall_ordered_primary: Option<tall_source::augmentation::PrimaryProof>,
    #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
    force_khmer_retry: bool,
    #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
    forced_khmer_supplements: ahash::AHashMap<usize, String>,
    preprocessing_render_dpi: Option<i32>,
    capture_rasters: bool,
    #[cfg(feature = "pdf")]
    lazy_pdf_render_state: Option<rendering::FullPdfRenderState>,
    lazy_pdf_page_count: usize,
    batch_size: usize,
    total_pages: usize,
    progress_total_pages: usize,
    #[cfg(feature = "pdf")]
    external_image_page_geometry: Option<rendering::PreRenderedPageGeometry>,
    #[cfg(feature = "pdf")]
    fallback_pdf_state: Option<Option<xberg_native_pdf::PdfDocument>>,

    // Per-page OCR output and the evidence that decides whether to keep it.
    #[cfg(all(
        feature = "pdf",
        feature = "ocr",
        feature = "tokio-runtime",
        not(target_arch = "wasm32")
    ))]
    leading_khmer_rows: ahash::AHashMap<usize, layer_token_recovery::LeadingKhmerRows>,
    page_texts: Vec<String>,
    #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
    paint_retention: ahash::AHashMap<usize, symbol_noise_lines::paint::Retention>,
    #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
    background_noise: ahash::AHashMap<usize, symbol_noise_background::Evidence>,
    #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
    certified_background: bool,
    #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
    paint_work: hidden_layer::PreviewWorkBudget,
    rejected_pages: Vec<bool>,
    #[cfg(all(
        feature = "pdf",
        feature = "ocr",
        feature = "tokio-runtime",
        not(target_arch = "wasm32")
    ))]
    owned_scan_rows: Option<String>,
    all_page_paragraphs: Vec<Option<Vec<crate::pdf::structure::types::PdfParagraph>>>,
    #[cfg(all(
        feature = "ocr",
        feature = "layout-detection",
        feature = "tokio-runtime",
        not(target_arch = "wasm32")
    ))]
    column_lanes_by_page: Vec<Option<column_retry::ColumnLanes>>,
    #[cfg(all(
        feature = "ocr",
        feature = "layout-detection",
        feature = "tokio-runtime",
        not(target_arch = "wasm32")
    ))]
    ordered_layout_pages: Vec<bool>,
    page_ocr_scales: Vec<Option<(f32, f32)>>,
    detached_page_scales: Vec<Option<(f32, f32)>>,
    #[cfg(feature = "pdf")]
    paragraph_rasters: Vec<Option<(u32, u32)>>,
    public_ocr_pages: ahash::AHashMap<u32, crate::types::internal::InternalDocument>,
    ocr_page_heights: Vec<f32>,
    pages_text_claimed_by_tables: Vec<bool>,
    retry_texts_outside_tables: Vec<Option<String>>,
    #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
    caption: Option<caption_conversion::CaptionRewrite>,
    #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
    capture_caption: bool,
    #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
    detached_table_pair: bool,
    #[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
    collected_table_grids: Vec<crate::types::internal::table_grid::TableGridSidecar>,
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    orientation_retry: Option<std::sync::Arc<orientation_retry::OrientationRetry>>,
    #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
    matched_layer_occurrences: Option<document::MatchedLayerOccurrences>,
    #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
    complete_native_words: bool,
    #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
    native_ink_owned: bool,
    #[cfg(all(feature = "pdf", feature = "ocr", not(target_arch = "wasm32")))]
    native_placed_words: std::collections::HashSet<String>,
    #[cfg(all(feature = "ocr", feature = "pdf", not(target_arch = "wasm32")))]
    tall_source_used: bool,
    #[cfg(feature = "pdf")]
    retained_hocr_read: Option<crate::types::formats::RetainedHocrRead>,
    #[cfg(all(
        test,
        feature = "pdf",
        feature = "ocr",
        feature = "tokio-runtime",
        not(target_arch = "wasm32")
    ))]
    region_replay_raster: Option<(u32, u32, usize)>,
    #[cfg(all(
        feature = "pdf",
        feature = "ocr",
        feature = "tokio-runtime",
        not(target_arch = "wasm32")
    ))]
    number_recovered_pages: Vec<bool>,
    #[cfg(all(
        feature = "pdf",
        feature = "ocr",
        feature = "tokio-runtime",
        not(target_arch = "wasm32")
    ))]
    number_recovery: std::sync::Arc<number_recovery::Run>,
    #[cfg(all(
        feature = "pdf",
        feature = "ocr",
        feature = "tokio-runtime",
        not(target_arch = "wasm32")
    ))]
    number_primary_text: Option<String>,
    page_segmentation_modes: std::collections::BTreeMap<u32, i32>,
    backend_additional_metadata: Option<ahash::AHashMap<std::borrow::Cow<'static, str>, serde_json::Value>>,

    // Document-level accumulators.
    collected_tables: Vec<crate::types::Table>,
    all_ocr_elements: Vec<crate::types::OcrElement>,
    accumulated_llm_usage: Vec<crate::types::LlmUsage>,
    accumulated_formulas: Vec<crate::types::Formula>,
    conf_sum: f64,
    conf_count: usize,
    ocr_confidence_by_page: ahash::AHashMap<u32, crate::types::page::PageOcrConfidence>,
    captured_rasters: Vec<crate::types::ExtractedImage>,
    preprocessing_by_page: ahash::AHashMap<u32, crate::types::ImagePreprocessingMetadata>,
    #[cfg(feature = "pdf")]
    image_fallback_warnings: Vec<crate::types::ProcessingWarning>,
    #[cfg(feature = "pdf")]
    xobject_recovery_budget: crate::extractors::security::SecurityBudget,
    page_backend_errors: Vec<crate::XbergError>,
    page_failure_warnings: Vec<crate::types::ProcessingWarning>,
    pending_page_failure_warnings: Vec<(usize, bool, crate::types::ProcessingWarning)>,
    page_failures: Vec<crate::types::OcrPageFailure>,
    pages_with_empty_xobject_retry: Vec<u32>,
    pages_with_recovered_xobject_payload: Vec<bool>,
    #[cfg(feature = "pdf")]
    margin_filter_warnings: Vec<crate::types::ProcessingWarning>,
    #[cfg(feature = "pdf")]
    render_warnings: Vec<crate::types::ProcessingWarning>,
    #[cfg(feature = "pdf")]
    raster_dpis: rendering::PageRasterDpis,
    backend_page_warnings: Vec<crate::types::ProcessingWarning>,
    recognition_noise_warnings: Vec<crate::types::ProcessingWarning>,
    recognition_noise_verdicts: Vec<OcrPageNoiseVerdict>,
    #[cfg(feature = "layout-detection")]
    tatr_model: Option<crate::layout::TatrLease>,
}

/// Run one per-page OCR extraction: open and route the pages, read them batch by batch, then
/// assemble the returned document.
#[allow(clippy::too_many_arguments)]
pub(super) async fn run<'a>(
    content: Option<&'a [u8]>,
    images: Option<&'a [image::DynamicImage]>,
    #[cfg(feature = "layout-detection")] layout_detections: Option<&'a [crate::layout::DetectionResult]>,
    config: &'a ExtractionConfig,
    path: Option<&'a std::path::Path>,
    page_rotation_override: u32,
    skip_document_global_heuristic: bool,
    points_per_pixel_override: Option<f32>,
    page_ocr_hints: Option<PageOcrHints>,
    page_index_offset: usize,
    detached_page: Option<DetachedOcrPage<'a>>,
) -> OcrRunResult<PageOcrOutput> {
    match setup::prepare(
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
    .await?
    {
        setup::PageOcrStart::DocumentLevel(output) => Ok(*output),
        setup::PageOcrStart::Pages(context) => context.run_batches().await?.finish().await,
    }
}

/// The pair of rasters one batch renders or encodes: the caller's own page images and this
/// route's encoded PNG bytes per page.
pub(super) struct RenderedBatch<'a> {
    pub(super) slice: Option<std::borrow::Cow<'a, [image::DynamicImage]>>,
    pub(super) encoded: Vec<(usize, std::sync::Arc<Vec<u8>>, u32, u32)>,
}

/// What a batch's page reads produced, indexed by the page's offset within the batch.
pub(super) struct BatchReads {
    pub(super) upright_correction: Vec<(u32, u32, u32)>,
    pub(super) results: Vec<Option<crate::types::ExtractedDocument>>,
    pub(super) errors: Vec<Option<(crate::XbergError, String)>>,
    #[cfg(all(
        feature = "ocr",
        feature = "layout-detection",
        feature = "tokio-runtime",
        not(target_arch = "wasm32")
    ))]
    pub(super) column_orders: Vec<Option<column_retry::LineOrder>>,
    #[cfg(all(
        feature = "pdf",
        feature = "ocr",
        feature = "tokio-runtime",
        not(target_arch = "wasm32")
    ))]
    pub(super) detached_frames: Vec<bool>,
    #[cfg(all(
        feature = "pdf",
        feature = "ocr",
        feature = "tokio-runtime",
        not(target_arch = "wasm32")
    ))]
    pub(super) number_frames: Vec<bool>,
    #[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
    pub(super) paint_frames: Vec<bool>,
}
