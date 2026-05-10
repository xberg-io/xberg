```gleam title="Gleam"
import gleam/int
import gleam/io
import gleam/list
import gleam/option
import gleam/string
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
  let items = [
    kreuzberg.BatchFileItem(path: "doc1.pdf", config: option.None),
    kreuzberg.BatchFileItem(path: "doc2.docx", config: option.None),
    kreuzberg.BatchFileItem(path: "report.pdf", config: option.None),
  ]
  case kreuzberg.batch_extract_files_sync(items, default_config()) {
    Ok(results) -> {
      list.index_map(results, fn(result, index) {
        io.println(
          "File "
          <> int.to_string(index)
          <> ": "
          <> int.to_string(string.length(result.content))
          <> " chars",
        )
      })
      Nil
    }
    Error(_) -> io.println("batch extraction failed")
  }
}
```
