```gleam title="Gleam"
import gleam/int
import gleam/io
import gleam/list
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

fn chunking_config() -> kreuzberg.ChunkingConfig {
  kreuzberg.ChunkingConfig(
    max_characters: 800,
    overlap: 100,
    trim: True,
    chunker_type: kreuzberg.ChunkerTypeMarkdown,
    embedding: option.None,
    preset: option.None,
    sizing: kreuzberg.Characters,
    prepend_heading_context: True,
    topic_threshold: option.None,
  )
}

fn image_config() -> kreuzberg.ImageExtractionConfig {
  kreuzberg.ImageExtractionConfig(
    extract_images: True,
    target_dpi: 150,
    max_image_dimension: 4096,
    inject_placeholders: False,
    auto_adjust_dpi: True,
    min_dpi: 72,
    max_dpi: 300,
    max_images_per_page: option.None,
    classify: False,
  )
}

fn full_config() -> kreuzberg.ExtractionConfig {
  kreuzberg.ExtractionConfig(
    use_cache: True,
    enable_quality_processing: True,
    ocr: option.Some(ocr_config()),
    force_ocr: False,
    force_ocr_pages: option.None,
    disable_ocr: False,
    chunking: option.Some(chunking_config()),
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
    output_format: kreuzberg.OutputFormatMarkdown,
    layout: option.None,
    include_document_structure: True,
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
  case kreuzberg.extract_file_sync("report.pdf", option.None, full_config()) {
    Ok(result) -> {
      io.println(
        "Content (" <> int.to_string(string.length(result.content)) <> " chars):",
      )
      io.println(string.slice(result.content, 0, 200))
      case result.chunks {
        option.Some(chunks) ->
          io.println("\nChunks: " <> int.to_string(list.length(chunks)))
        option.None -> Nil
      }
      io.println("Tables: " <> int.to_string(list.length(result.tables)))
      case result.detected_languages {
        option.Some(langs) ->
          io.println("Languages: " <> string.join(langs, ", "))
        option.None -> Nil
      }
    }
    Error(_) -> io.println_error("Extraction failed")
  }
}
```
