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
    max_characters: 512,
    overlap: 50,
    trim: True,
    chunker_type: kreuzberg.ChunkerTypeText,
    embedding: option.Some(embedding_config()),
    preset: option.None,
    sizing: kreuzberg.Characters,
    prepend_heading_context: False,
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

/// Placeholder for a vector database record. Replace with a real
/// driver (e.g. Qdrant, Weaviate, pgvector) when integrating.
pub type VectorRecord {
  VectorRecord(
    id: String,
    content: String,
    embedding: List(Float),
    chunk_index: Int,
    total_chunks: Int,
  )
}

fn build_record(
  document_id: String,
  chunk: kreuzberg.Chunk,
) -> option.Option(VectorRecord) {
  case chunk.embedding {
    option.Some(embedding) ->
      option.Some(VectorRecord(
        id: document_id
          <> "_chunk_"
          <> int.to_string(chunk.metadata.chunk_index),
        content: chunk.content,
        embedding: embedding,
        chunk_index: chunk.metadata.chunk_index,
        total_chunks: chunk.metadata.total_chunks,
      ))
    option.None -> option.None
  }
}

pub fn main() {
  let document_id = "research_paper"
  case kreuzberg.extract_file_sync("research_paper.pdf", option.None, config()) {
    Ok(result) ->
      case result.chunks {
        option.Some(chunks) -> {
          let records =
            list.filter_map(chunks, fn(chunk) {
              case build_record(document_id, chunk) {
                option.Some(record) -> Ok(record)
                option.None -> Error(Nil)
              }
            })
          io.println(
            "Prepared "
            <> int.to_string(list.length(records))
            <> " vector records for upload",
          )
        }
        option.None -> io.println("No chunks produced")
      }
    Error(_) -> io.println_error("extraction failed")
  }
}
```
