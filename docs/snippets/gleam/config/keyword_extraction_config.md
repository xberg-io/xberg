```gleam title="Gleam"
import gleam/int
import gleam/io
import gleam/list
import gleam/option
import kreuzberg

fn keyword_config() -> kreuzberg.KeywordConfig {
  kreuzberg.KeywordConfig(
    algorithm: kreuzberg.Yake,
    max_keywords: 10,
    min_score: 0.1,
    ngram_range: [1, 3],
    language: option.Some("en"),
    yake_params: option.None,
    rake_params: option.None,
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
    pages: option.None,
    keywords: option.Some(keyword_config()),
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
      case result.extracted_keywords {
        option.Some(keywords) ->
          io.println("Keywords: " <> int.to_string(list.length(keywords)))
        option.None -> io.println("Keywords: 0")
      }
    Error(_) -> io.println_error("extraction failed")
  }
}
```
