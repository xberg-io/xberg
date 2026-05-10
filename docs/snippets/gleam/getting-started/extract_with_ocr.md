```gleam title="Gleam"
import gleam/io
import gleam/option
import gleam/string
import kreuzberg

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
    force_ocr: True,
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
    Ok(result) -> {
      io.println(result.content)
      case result.detected_languages {
        option.Some(langs) ->
          io.println("Detected languages: " <> string.join(langs, ", "))
        option.None -> io.println("Detected languages: none")
      }
    }
    Error(_) -> io.println("extraction failed")
  }
}
```
