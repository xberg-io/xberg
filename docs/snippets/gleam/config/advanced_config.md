```gleam title="Gleam"
import gleam/int
import gleam/io
import gleam/list
import gleam/option
import gleam/string
import kreuzberg

fn ocr_config() -> kreuzberg.OcrConfig {
  kreuzberg.OcrConfig(
    enabled: True,
    backend: "tesseract",
    language: "eng",
    tesseract_config: option.None,
    output_format: option.None,
    paddle_ocr_config: option.None,
    element_config: option.None,
    quality_thresholds: option.None,
    pipeline: option.None,
    auto_rotate: False,
    vlm_config: option.None,
    vlm_prompt: option.None,
    acceleration: option.None,
  )
}

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

fn language_detection_config() -> kreuzberg.LanguageDetectionConfig {
  kreuzberg.LanguageDetectionConfig(
    enabled: True,
    min_confidence: 0.8,
    detect_multiple: False,
  )
}

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

fn token_reduction() -> kreuzberg.TokenReductionOptions {
  kreuzberg.TokenReductionOptions(
    mode: "moderate",
    preserve_important_words: True,
  )
}

fn postprocessor_config() -> kreuzberg.PostProcessorConfig {
  kreuzberg.PostProcessorConfig(
    enabled: True,
    enabled_processors: option.None,
    disabled_processors: option.None,
    enabled_set: option.None,
    disabled_set: option.None,
  )
}

fn config() -> kreuzberg.ExtractionConfig {
  kreuzberg.ExtractionConfig(
    use_cache: True,
    enable_quality_processing: True,
    ocr: option.Some(ocr_config()),
    force_ocr: False,
    force_ocr_pages: option.None,
    disable_ocr: False,
    chunking: option.Some(chunking_config()),
    content_filter: option.None,
    images: option.None,
    pdf_options: option.None,
    token_reduction: option.Some(token_reduction()),
    language_detection: option.Some(language_detection_config()),
    pages: option.None,
    keywords: option.Some(keyword_config()),
    postprocessor: option.Some(postprocessor_config()),
    html_options: option.None,
    html_output: option.None,
    extraction_timeout_secs: option.None,
    max_concurrent_extractions: option.None,
    result_format: kreuzberg.Unified,
    security_limits: option.None,
    output_format: kreuzberg.OutputFormatMarkdown,
    layout: option.None,
    include_document_structure: True,
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
      io.println(string.slice(result.content, 0, 200))
      case result.detected_languages {
        option.Some(langs) ->
          io.println("Languages: " <> string.join(langs, ", "))
        option.None -> Nil
      }
      case result.chunks {
        option.Some(chunks) ->
          io.println("Chunks: " <> int.to_string(list.length(chunks)))
        option.None -> Nil
      }
    }
    Error(_) -> io.println_error("extraction failed")
  }
}
```
