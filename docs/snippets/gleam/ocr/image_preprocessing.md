```gleam title="Gleam"
import gleam/io
import gleam/option
import kreuzberg

// `ImageExtractionConfig` controls the rasterisation and preprocessing
// pipeline applied before OCR. Tuning DPI, dimension caps, and per-page
// image limits trades extraction speed against OCR fidelity.
//
// - `target_dpi`           — base render DPI for PDF page rasterisation.
// - `auto_adjust_dpi`      — let the pipeline pick a DPI between
//                            `min_dpi` and `max_dpi` based on page size.
// - `max_image_dimension`  — clamp the longest side (pixels) of any
//                            extracted image to bound memory.
// - `inject_placeholders`  — insert image placeholder markers in the text.
// - `max_images_per_page`  — cap how many images per page get extracted.
// - `classify`             — run the image classifier to tag image kinds.
fn image_config() -> kreuzberg.ImageExtractionConfig {
  kreuzberg.ImageExtractionConfig(
    extract_images: True,
    target_dpi: 300,
    max_image_dimension: 4096,
    inject_placeholders: True,
    auto_adjust_dpi: True,
    min_dpi: 200,
    max_dpi: 450,
    max_images_per_page: option.Some(8),
    classify: True,
  )
}

fn ocr_config() -> kreuzberg.OcrConfig {
  kreuzberg.OcrConfig(
    enabled: True,
    backend: "tesseract",
    language: "eng",
    tesseract_config: option.None,
    output_format: option.None,
    paddle_ocr_config: option.None,
    element_config: option.None,
    quality_thresholds: option.None,
    pipeline: option.None,
    auto_rotate: True,
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
    images: option.Some(image_config()),
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
