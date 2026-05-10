```gleam title="Gleam"
import gleam/int
import gleam/io
import gleam/list
import gleam/option
import kreuzberg

// Set `extract_images: True` on `ImageExtractionConfig` to pull embedded
// images out alongside the document text.
fn image_config() -> kreuzberg.ImageExtractionConfig {
  kreuzberg.ImageExtractionConfig(
    extract_images: True,
    target_dpi: 300,
    max_image_dimension: 4096,
    inject_placeholders: False,
    auto_adjust_dpi: True,
    min_dpi: 150,
    max_dpi: 600,
    max_images_per_page: option.None,
    classify: False,
  )
}

fn config() -> kreuzberg.ExtractionConfig {
  kreuzberg.ExtractionConfig(
    use_cache: True,
    enable_quality_processing: True,
    ocr: option.None,
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
  case kreuzberg.extract_file_sync("document.pdf", option.None, config()) {
    Ok(result) ->
      case result.images {
        option.Some(images) ->
          io.println(
            "Extracted images: " <> int.to_string(list.length(images)),
          )
        option.None -> io.println("Extracted images: 0")
      }
    Error(_) -> io.println_error("extraction failed")
  }
}
```
