```gleam title="Gleam"
import gleam/io
import gleam/option
import kreuzberg

fn tesseract_config() -> kreuzberg.TesseractConfig {
  kreuzberg.TesseractConfig(
    language: "eng+deu",
    psm: 6,
    output_format: "markdown",
    oem: 3,
    min_confidence: 0.0,
    preprocessing: option.None,
    enable_table_detection: False,
    table_min_confidence: 0.0,
    table_column_threshold: 20,
    table_row_threshold_ratio: 0.5,
    use_cache: True,
    classify_use_pre_adapted_templates: False,
    language_model_ngram_on: False,
    tessedit_dont_blkrej_good_wds: False,
    tessedit_dont_rowrej_good_wds: False,
    tessedit_enable_dict_correction: False,
    tessedit_char_whitelist: "",
    tessedit_char_blacklist: "",
    tessedit_use_primary_params_model: False,
    textord_space_size_is_variable: False,
    thresholding_method: False,
  )
}

fn ocr_config() -> kreuzberg.OcrConfig {
  kreuzberg.OcrConfig(
    enabled: True,
    backend: "tesseract",
    language: "eng+deu",
    tesseract_config: option.Some(tesseract_config()),
    output_format: option.None,
    paddle_ocr_config: option.None,
    element_config: option.None,
    quality_thresholds: option.None,
    pipeline: option.None,
    auto_rotate: False,
    vlm_config: option.None,
    vlm_prompt: option.None,
    acceleration: option.None,
  )
}

fn config() -> kreuzberg.ExtractionConfig {
  kreuzberg.ExtractionConfig(
    use_cache: True,
    enable_quality_processing: True,
    ocr: option.Some(ocr_config()),
    force_ocr: False,
    force_ocr_pages: option.None,
    disable_ocr: False,
    chunking: option.None,
    content_filter: option.None,
    images: option.None,
    pdf_options: option.None,
    token_reduction: option.None,
    language_detection: option.None,
    pages: option.None,
    keywords: option.None,
    postprocessor: option.None,
    html_options: option.None,
    html_output: option.None,
    extraction_timeout_secs: option.None,
    max_concurrent_extractions: option.None,
    result_format: kreuzberg.Unified,
    security_limits: option.None,
    output_format: kreuzberg.Plain,
    layout: option.None,
    include_document_structure: False,
    acceleration: option.None,
    cache_namespace: option.None,
    cache_ttl_secs: option.None,
    email: option.None,
    concurrency: option.None,
    max_archive_depth: 10,
    tree_sitter: option.None,
    structured_extraction: option.None,
    cancel_token: option.None,
  )
}

pub fn main() {
  case kreuzberg.extract_file_sync("scanned.pdf", option.None, config()) {
    Ok(result) -> io.println("OCR text: " <> result.content)
    Error(_) -> io.println_error("extraction failed")
  }
}
```
