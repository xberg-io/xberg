```gleam title="Gleam"
import gleam/float
import gleam/int
import gleam/io
import gleam/list
import gleam/option
import kreuzberg

// Enable structured OCR element output by attaching an `OcrElementConfig` to
// the OCR config. Each element exposes recognized text, geometry, confidence,
// and the level (Word, Line, Block, OcrElementLevelPage) in the OCR hierarchy.
fn element_config() -> kreuzberg.OcrElementConfig {
  kreuzberg.OcrElementConfig(
    include_elements: True,
    min_level: kreuzberg.Word,
    min_confidence: 0.5,
    build_hierarchy: True,
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
    element_config: option.Some(element_config()),
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

fn print_element(element: kreuzberg.OcrElement) -> Nil {
  io.println("Text: " <> element.text)
  io.println(
    "Recognition confidence: "
    <> float.to_string(element.confidence.recognition),
  )
  io.println("Page: " <> int.to_string(element.page_number))
  case element.rotation {
    option.Some(rotation) ->
      io.println(
        "Rotation degrees: " <> float.to_string(rotation.angle_degrees),
      )
    option.None -> Nil
  }
  io.println("")
}

pub fn main() {
  case kreuzberg.extract_file_sync("scanned.pdf", option.None, config()) {
    Ok(result) ->
      case result.ocr_elements {
        option.Some(elements) -> {
          io.println(
            "OCR elements: " <> int.to_string(list.length(elements)),
          )
          list.each(elements, print_element)
        }
        option.None -> io.println("No OCR elements returned")
      }
    Error(_) -> io.println_error("extraction failed")
  }
}
```
