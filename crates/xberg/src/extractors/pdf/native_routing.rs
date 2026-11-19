//! OCR routing of the native PDF path: which pages are read by OCR, and what each route produced.

use super::OcrLayoutGateDecisions;
#[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
use super::picture_layer;
use crate::types::internal::InternalDocument;

/// What the OCR routes of `extract_core_native_with_layout_pages` produced, kept apart from the
/// native extraction so each route fills only the parts it owns.
#[derive(Default)]
pub(super) struct OcrRoutingState {
    pub(super) tables: Vec<crate::types::Table>,
    pub(super) elements: Vec<crate::types::OcrElement>,
    pub(super) coordinate_frames: Vec<crate::types::internal::OcrPageCoordinateFrame>,
    pub(super) internal_doc: Option<InternalDocument>,
    pub(super) llm_usage: Vec<crate::types::LlmUsage>,
    pub(super) page_texts: Option<Vec<String>>,
    pub(super) document_page_texts: Option<Vec<String>>,
    pub(super) results_map: Option<ahash::AHashMap<u32, String>>,
    pub(super) structured_pages: Option<ahash::AHashMap<u32, InternalDocument>>,
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    pub(super) picture_pages: ahash::AHashMap<u32, picture_layer::RetainedPicturePage>,
    pub(super) page_rasters: Option<Vec<crate::types::ExtractedImage>>,
    pub(super) formulas: Vec<crate::types::Formula>,
    pub(super) preprocessing_by_page: ahash::AHashMap<u32, crate::types::ImagePreprocessingMetadata>,
    pub(super) confidence_by_page: ahash::AHashMap<u32, crate::types::page::PageOcrConfidence>,
    pub(super) fallback_warnings: Vec<crate::types::ProcessingWarning>,
    pub(super) page_failures: Vec<crate::types::OcrPageFailure>,
    pub(super) layout_gate_audit: OcrLayoutGateDecisions,
    #[cfg(feature = "formula-recognition")]
    pub(super) mixed_formula_layout_inputs: Option<(Vec<image::DynamicImage>, Vec<crate::layout::DetectionResult>)>,
}
