```gleam title="Gleam"
import gleam/int
import gleam/io
import gleam/list
import gleam/option
import kreuzberg

fn embedding_config() -> kreuzberg.EmbeddingConfig {
  kreuzberg.EmbeddingConfig(
    model: kreuzberg.Preset(name: "balanced"),
    normalize: True,
    batch_size: 32,
    show_download_progress: False,
    cache_dir: option.None,
    acceleration: option.None,
    max_embed_duration_secs: option.None,
  )
}

fn chunking_config() -> kreuzberg.ChunkingConfig {
  kreuzberg.ChunkingConfig(
    max_characters: 500,
    overlap: 50,
    trim: True,
    chunker_type: kreuzberg.ChunkerTypeMarkdown,
    embedding: option.Some(embedding_config()),
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

fn describe_chunk(chunk: kreuzberg.Chunk) {
  io.println(
    "Chunk "
    <> int.to_string(chunk.metadata.chunk_index + 1)
    <> "/"
    <> int.to_string(chunk.metadata.total_chunks),
  )
  io.println(
    "  bytes: "
    <> int.to_string(chunk.metadata.byte_start)
    <> "-"
    <> int.to_string(chunk.metadata.byte_end),
  )
  case chunk.embedding {
    option.Some(embedding) ->
      io.println(
        "  embedding dims: " <> int.to_string(list.length(embedding)),
      )
    option.None -> io.println("  embedding: none")
  }
}

pub fn main() {
  case kreuzberg.extract_file_sync("research_paper.pdf", option.None, config()) {
    Ok(result) ->
      case result.chunks {
        option.Some(chunks) -> list.each(chunks, describe_chunk)
        option.None -> io.println("No chunks produced")
      }
    Error(_) -> io.println_error("extraction failed")
  }
}
```
