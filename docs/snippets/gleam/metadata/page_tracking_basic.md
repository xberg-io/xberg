```gleam title="Gleam"
import gleam/int
import gleam/io
import gleam/list
import gleam/option
import gleam/string
import kreuzberg

fn page_config() -> kreuzberg.PageConfig {
  kreuzberg.PageConfig(
    extract_pages: True,
    insert_page_markers: False,
    marker_format: "<!-- page {page} -->",
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
    images: option.None,
    pdf_options: option.None,
    token_reduction: option.None,
    language_detection: option.None,
    pages: option.Some(page_config()),
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
      case result.pages {
        option.Some(pages) ->
          list.each(pages, fn(page) {
            io.println("Page " <> int.to_string(page.page_number) <> ":")
            io.println(
              "  Content length: "
              <> int.to_string(string.length(page.content))
              <> " chars",
            )
            io.println(
              "  Tables: " <> int.to_string(list.length(page.tables)),
            )
            io.println(
              "  Images: " <> int.to_string(list.length(page.images)),
            )
          })
        option.None -> io.println("No per-page content available")
      }
    Error(_) -> io.println_error("extraction failed")
  }
}
```
