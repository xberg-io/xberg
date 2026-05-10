```gleam title="Gleam"
import gleam/int
import gleam/io
import gleam/list
import gleam/option
import gleam/string
import kreuzberg

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
    Ok(result) -> {
      io.println("Tables: " <> int.to_string(list.length(result.tables)))
      list.each(result.tables, fn(table) {
        io.println(
          "  page "
          <> int.to_string(table.page_number)
          <> " — "
          <> int.to_string(list.length(table.cells))
          <> " rows",
        )
      })
      case result.chunks {
        option.Some(chunks) -> {
          io.println("Chunks: " <> int.to_string(list.length(chunks)))
          list.each(chunks, fn(chunk) {
            io.println(
              "  chunk "
              <> int.to_string(chunk.metadata.chunk_index)
              <> " ("
              <> int.to_string(string.length(chunk.content))
              <> " chars)",
            )
          })
        }
        option.None -> io.println("Chunks: none")
      }
    }
    Error(_) -> io.println("extraction failed")
  }
}
```
