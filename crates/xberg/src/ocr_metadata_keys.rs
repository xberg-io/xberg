//! Canonical names of the OCR metadata keys written into `Metadata::additional`.
//!
//! These strings are part of the public extraction output — consumers read them
//! off `Metadata::additional` — so a rename is a silent breaking change. They
//! live in an ungated crate-root module on purpose: producers and consumers sit
//! in different feature domains (native OCR under `ocr`/`ocr-wasm`, the portable
//! pipeline under `ocr-pipeline`, VLM OCR under liter-llm, the `candle-*`
//! backends, Sceptre, Paddle, and PDF layout under `layout-detection`, which
//! needs no OCR backend at all) and no single feature is implied by all of them.
//! Gating this module on any of them would force back the per-module copies of
//! the same literals that it exists to remove. ~keep

// No feature combination reads every key (builds without orientation correction
// never touch the rotation keys, for instance), so an unreferenced constant here
// is expected rather than dead. ~keep
#![allow(dead_code)]

/// Pixel width of the image actually handed to the OCR backend, after preprocessing.
pub(crate) const OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY: &str = "ocr_processed_image_width";
/// Pixel height of the image actually handed to the OCR backend, after preprocessing.
pub(crate) const OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY: &str = "ocr_processed_image_height";
/// Clockwise rotation, in degrees, that orientation detection reported for the page.
pub(crate) const OCR_ORIENTATION_DEGREES_METADATA_KEY: &str = "orientation_degrees";
/// Confidence of the orientation detection that produced the degrees value.
pub(crate) const OCR_ORIENTATION_CONFIDENCE_METADATA_KEY: &str = "orientation_confidence";
/// Set when the pipeline actually rotated the image before running OCR.
pub(crate) const OCR_AUTO_ROTATED_METADATA_KEY: &str = "auto_rotated";
/// Fraction of a page's dictionary-checkable words that Tesseract's own DAWG dictionary
/// (`TesseractAPI::is_valid_word`) rejects as not-a-word. Tesseract-only: other backends
/// never write this key, so consumers must treat its absence as "no evidence" rather than
/// as `0.0` ("every word is valid"). See `dictionary_invalid_word_ratio` in
/// `ocr::processor::execution` and `is_dictionary_invalid_noise` in `extractors::pdf::ocr`.
pub(crate) const OCR_TESSERACT_DICT_INVALID_WORD_RATIO_METADATA_KEY: &str = "tesseract_dict_invalid_word_ratio";
pub(crate) const OCR_IMAGE_PREPROCESSING_METADATA_KEY: &str = "image_preprocessing";
/// One page-local pixel coordinate frame carried temporarily by an OCR element.
pub(crate) const OCR_PAGE_COORDINATE_FRAME_METADATA_KEY: &str = "_xberg_ocr_page_coordinate_frame";
/// Page-local pixel coordinate frames for public OCR element geometry.
pub(crate) const OCR_PAGE_COORDINATE_FRAMES_METADATA_KEY: &str = "ocr_page_coordinate_frames";
