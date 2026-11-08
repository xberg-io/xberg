//! OCR functionality for PDF extraction.
//!
//! Handles text quality evaluation, OCR fallback decision logic, and OCR processing.
//!
//! Split into topical submodules: [`scoring`] (native-text/OCR-output quality evaluation and
//! the skip/fallback gate), [`plausibility`] (language/dictionary-plausibility detection for
//! wrong-mapped text layers, issue #1696), [`rendering`] (PDF page rasterization and
//! image-XObject recovery), [`document`] (bbox/geometry transforms and assembling OCR output
//! into page documents, including merging OCR pages back into a natively-extracted document),
//! and [`pipeline`] (the top-level mixed-OCR and per-page pipeline orchestrators). Items used
//! across submodule boundaries are `pub(super)`; items already reachable from outside `ocr`
//! keep their original visibility and are re-exported here so external call sites are
//! unaffected by the split. Unit tests live in `tests.rs`, `recognition_noise_tests.rs`,
//! `numeric_repair_tests.rs`, and `plausibility_tests.rs`, not inline here.

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
mod cache_policy;
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(super) use cache_policy::resolve_ocr_config;

#[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
mod caption_conversion;
#[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
#[path = "../../../ocr/caption_grid_proof.rs"]
mod caption_grid_proof;
mod document;
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
mod markdown;
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(crate) use markdown::strip_embedded_gfm_tables;
#[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
mod hidden_layer;
#[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
pub(super) use hidden_layer::VisibleRasterProof;
#[cfg(all(feature = "ocr", feature = "tokio-runtime", not(target_arch = "wasm32")))]
pub(crate) use hidden_layer::embedded_gap_placement;
#[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
mod native_word_coverage;
#[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
mod orientation_retry;
#[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
pub(crate) use orientation_retry::recognition_retry_eligible;
mod pipeline;
mod plausibility;
mod rendering;
mod scoring;
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(crate) mod stage_model_inputs;
#[cfg(all(
    feature = "ocr",
    feature = "pdf",
    feature = "tokio-runtime",
    not(target_arch = "wasm32")
))]
mod tall_geometry;
#[cfg(all(
    feature = "ocr",
    feature = "pdf",
    feature = "tokio-runtime",
    not(target_arch = "wasm32")
))]
mod tall_source;
#[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
pub(super) use pipeline::probe_incomplete_hidden_layers;

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(crate) use plausibility::scan_text_plausibility;
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(crate) use scoring::{OcrGateOutcome, apply_flagged_pages, evaluate_ocr_skip_gate, evaluate_per_page_ocr};

// ~keep The standalone-image extractor builds the same `PageContent.ocr_confidence` summary
// from its own single OCR run (#1568), so these three leave `ocr` rather than staying
// `pub(super)`. They are pure readers/builders with no PDF dependency of their own.
#[cfg(feature = "ocr")]
pub(crate) use scoring::{mean_text_conf_of, page_ocr_confidence, word_count_of};

// ~keep GH#1789's numeric-token repair has the same "no PDF dependency" shape as the three
// above: `extractors::image`'s standalone OCR route needs it too (the issue's own reproduction
// runs against a bare PNG, not a PDF), gated the same way `page_ocr_confidence` already is
// there (`all(feature = "ocr", feature = "pdf")`, since this module tree only exists under
// `feature = "pdf"`).
#[cfg(feature = "ocr")]
pub(crate) use scoring::{ocr_content_is_repairable_prose, repair_ocr_numeric_tokens};

// ~keep These three are reachable only from `extractors::pdf`'s own `#[cfg(test)]` unit tests,
// so a non-test build of this crate never exercises the re-export itself (the underlying items
// are not dead code -- `scoring` uses `NativeTextStats` and `evaluate_native_text_for_ocr`
// internally too, and `OcrFallbackDecision` is `pub`). `#[allow(unused_imports)]` documents that
// on purpose rather than leaving an unexplained warning suppression.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[allow(unused_imports)]
pub(crate) use scoring::{NativeTextStats, OcrFallbackDecision, evaluate_native_text_for_ocr};

// ~keep Consumed only by `extractors::pdf::layout_runner`'s own tests, which additionally
// require `feature = "layout-detection"`; a `test, ocr, pdf` build without it compiles this
// re-export with no reader. `#[allow(unused_imports)]` documents that on purpose.
#[cfg(all(test, feature = "ocr", feature = "pdf"))]
#[allow(unused_imports)]
pub(crate) use rendering::render_selected_pages_for_ocr;

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(crate) use document::{
    boundaries_after_replacements, clean_native_page_layers, merge_ocr_pages_into_internal_document,
    merge_source_strip_frames, merge_structured_ocr_pages_into_internal_document,
};

#[cfg(all(paddle_ocr, any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(crate) use pipeline::extract_full_document_ocr_pipeline_per_page;
#[cfg(all(test, feature = "ocr", feature = "pdf"))]
#[allow(unused_imports)]
pub(crate) use pipeline::extract_mixed_ocr_native_with_single_block_pages;
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(crate) use pipeline::{
    extract_mixed_ocr_native_with_layout_pages, failed_run_page_failures, route_level_ocr_page_failures,
};
// ~keep Must match the definition's own gate in `pipeline.rs`; `#[cfg(test)]` alone left
// this import unresolved on every leg without an OCR feature (GH#1951). The definition is
// dead on an `ocr-pipeline`-only test leg, so the re-export is unused there.
#[cfg(all(
    paddle_ocr,
    feature = "ocr",
    feature = "pdf",
    feature = "tokio-runtime",
    not(target_arch = "wasm32")
))]
pub(crate) use pipeline::accepted_single_page_first_pass;
#[cfg(all(test, any(feature = "ocr", feature = "ocr-pipeline")))]
#[allow(unused_imports)]
pub(crate) use pipeline::extract_with_ocr;
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(crate) use pipeline::{
    OcrRunResult, PageOcrHints, apply_numeric_repair_to_whole_document_ocr,
    extract_with_ocr_with_page_hints_and_evidence, numeric_repair_enabled, run_ocr_pipeline,
};

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(crate) use pipeline::estimate_png_encode_page_peak_bytes;

#[cfg(all(test, feature = "ocr"))]
mod cache_policy_tests;
#[cfg(all(test, feature = "ocr", feature = "pdf"))]
mod failed_page_record_tests;
#[cfg(all(test, any(feature = "ocr", feature = "ocr-pipeline")))]
mod ligature_space_tests;
#[cfg(all(test, any(feature = "ocr", feature = "ocr-pipeline")))]
mod numeric_repair_tests;
#[cfg(all(test, feature = "ocr", feature = "pdf"))]
mod page_failure_cause_tests;
#[cfg(all(test, any(feature = "ocr", feature = "ocr-pipeline")))]
mod photo_layer_tests;
#[cfg(all(test, any(feature = "ocr", feature = "ocr-pipeline")))]
mod plausibility_tests;
#[cfg(all(test, any(feature = "ocr", feature = "ocr-pipeline")))]
mod recognition_noise_tests;
#[cfg(all(test, feature = "ocr", feature = "tokio-runtime"))]
mod security_limits_tests;
#[cfg(all(test, feature = "ocr"))]
mod tests;

#[cfg(all(test, feature = "tokio-runtime", any(feature = "ocr", feature = "ocr-pipeline")))]
mod raw_transcript_tests;

#[cfg(all(test, feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
mod combined_rotation_tests;

#[cfg(all(test, feature = "pdf", feature = "ocr", feature = "tokio-runtime"))]
mod batch_metadata_tests;

#[cfg(all(test, any(feature = "ocr", feature = "ocr-pipeline")))]
#[allow(unused_imports)]
pub(crate) use pipeline::extract_with_ocr_with_page_hints;
