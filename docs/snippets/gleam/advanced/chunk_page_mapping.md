```gleam title="Gleam"
import gleam/int
import gleam/io
import gleam/list
import gleam/option
import kreuzberg

fn chunking_config() -> kreuzberg.ChunkingConfig {
  kreuzberg.ChunkingConfig(
    max_characters: 500,
    overlap: 50,
    trim: True,
    chunker_type: kreuzberg.ChunkerTypeText,
    embedding: option.None,
    preset: option.None,
    sizing: kreuzberg.Characters,
    prepend_heading_context: False,
    topic_threshold: option.None,
  )
}

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
    chunking: option.Some(chunking_config()),
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

fn describe_chunk(chunk: kreuzberg.Chunk) -> String {
  case chunk.metadata.first_page, chunk.metadata.last_page {
    option.Some(first), option.Some(last) ->
      case first == last {
        True -> "Page " <> int.to_string(first)
        False ->
          "Pages "
          <> int.to_string(first)
          <> "-"
          <> int.to_string(last)
      }
    _, _ -> "Page: unknown"
  }
}

pub fn main() {
  case kreuzberg.extract_file_sync("document.pdf", option.None, config()) {
    Ok(result) ->
      case result.chunks {
        option.Some(chunks) ->
          list.each(chunks, fn(chunk) {
            io.println(
              "Chunk #"
              <> int.to_string(chunk.metadata.chunk_index)
              <> " ("
              <> describe_chunk(chunk)
              <> ")",
            )
          })
        option.None -> io.println("No chunks produced")
      }
    Error(_) -> io.println_error("extraction failed")
  }
}
```
