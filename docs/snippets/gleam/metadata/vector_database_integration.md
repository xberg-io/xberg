<!-- snippet:syntax-only -->

```gleam title="Gleam"
import gleam/int
import gleam/io
import gleam/list
import gleam/option
import kreuzberg

fn embedding_config() -> kreuzberg.EmbeddingConfig {
  kreuzberg.EmbeddingConfig(
    model: kreuzberg.Preset(name: "all-MiniLM-L6-v2"),
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
    max_characters: 1000,
    overlap: 200,
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

/// Placeholder for a vector database client. Replace with a real
/// driver (e.g. Qdrant, Weaviate, pgvector) when integrating.
pub type VectorRecord {
  VectorRecord(
    id: String,
    embedding: List(Float),
    text: String,
    chunk_index: Int,
    total_chunks: Int,
  )
}

fn build_record(
  source: String,
  chunk: kreuzberg.Chunk,
) -> option.Option(VectorRecord) {
  case chunk.embedding {
    option.Some(embedding) ->
      option.Some(VectorRecord(
        id: source
          <> "#"
          <> int.to_string(chunk.metadata.chunk_index),
        embedding: embedding,
        text: chunk.content,
        chunk_index: chunk.metadata.chunk_index,
        total_chunks: chunk.metadata.total_chunks,
      ))
    option.None -> option.None
  }
}

pub fn main() {
  let source = "document.pdf"
  case kreuzberg.extract_file_sync(source, option.None, config()) {
    Ok(result) ->
      case result.chunks {
        option.Some(chunks) -> {
          let records =
            chunks
            |> list.filter_map(fn(chunk) {
              case build_record(source, chunk) {
                option.Some(record) -> Ok(record)
                option.None -> Error(Nil)
              }
            })

          io.println(
            "Prepared "
            <> int.to_string(list.length(records))
            <> " vector records for upload",
          )
          // upload_to_vector_db(records)
        }
        option.None -> io.println("No chunks produced")
      }
    Error(_) -> io.println_error("extraction failed")
  }
}
```
