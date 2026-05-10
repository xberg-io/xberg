```gleam title="Gleam"
import gleam/int
import gleam/io
import gleam/list
import gleam/option
import kreuzberg

fn default_config() -> kreuzberg.ExtractionConfig {
  kreuzberg.ExtractionConfig(
    use_cache: True,
    enable_quality_processing: True,
    ocr: option.None,
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
  case kreuzberg.extract_file_sync("document.pdf", option.None, default_config()) {
    Ok(result) ->
      case result.metadata.pages {
        option.Some(page_structure) -> {
          io.println(
            "Total pages: " <> int.to_string(page_structure.total_count),
          )
          case page_structure.boundaries {
            option.Some(boundaries) ->
              list.each(boundaries, fn(boundary) {
                io.println(
                  "Page "
                  <> int.to_string(boundary.page_number)
                  <> ": bytes "
                  <> int.to_string(boundary.byte_start)
                  <> "-"
                  <> int.to_string(boundary.byte_end),
                )
              })
            option.None -> io.println("No page boundaries available")
          }
        }
        option.None -> io.println("No page structure available")
      }
    Error(_) -> io.println_error("extraction failed")
  }
}
```
