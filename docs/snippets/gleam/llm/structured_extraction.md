```gleam title="Gleam"
import gleam/io
import gleam/option
import kreuzberg

fn llm_config() -> kreuzberg.LlmConfig {
  kreuzberg.LlmConfig(
    model: "openai/gpt-4o-mini",
    api_key: option.None,
    base_url: option.None,
    timeout_secs: option.None,
    max_retries: option.None,
    temperature: option.None,
    max_tokens: option.None,
  )
}

fn structured_extraction() -> kreuzberg.StructuredExtractionConfig {
  let schema =
    "{\"type\":\"object\","
    <> "\"properties\":{"
    <> "\"title\":{\"type\":\"string\"},"
    <> "\"authors\":{\"type\":\"array\",\"items\":{\"type\":\"string\"}},"
    <> "\"date\":{\"type\":\"string\"}},"
    <> "\"required\":[\"title\",\"authors\",\"date\"],"
    <> "\"additionalProperties\":false}"
  kreuzberg.StructuredExtractionConfig(
    schema: schema,
    schema_name: "PaperMetadata",
    schema_description: option.None,
    strict: True,
    prompt: option.None,
    llm: llm_config(),
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
    structured_extraction: option.Some(structured_extraction()),
    cancel_token: option.None,
  )
}

pub fn main() {
  case kreuzberg.extract_file_sync("paper.pdf", option.None, config()) {
    Ok(result) ->
      case result.structured_output {
        option.Some(json) -> io.println(json)
        option.None -> io.println("No structured output produced")
      }
    Error(_) -> io.println_error("extraction failed")
  }
}
```
