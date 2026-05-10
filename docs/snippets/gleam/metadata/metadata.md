```gleam title="Gleam"
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
  case kreuzberg.extract_file_sync("document.pdf", option.None, default_config()) {
    Ok(result) -> {
      let metadata = result.metadata

      case metadata.title {
        option.Some(title) -> io.println("Title: " <> title)
        option.None -> io.println("Title: (none)")
      }

      case metadata.subject {
        option.Some(subject) -> io.println("Subject: " <> subject)
        option.None -> Nil
      }

      case metadata.authors {
        option.Some(authors) ->
          io.println("Authors: " <> string.join(authors, ", "))
        option.None -> Nil
      }

      case metadata.keywords {
        option.Some(keywords) ->
          io.println("Keywords: " <> string.join(keywords, ", "))
        option.None -> Nil
      }

      case metadata.language {
        option.Some(lang) -> io.println("Language: " <> lang)
        option.None -> Nil
      }

      case metadata.created_at {
        option.Some(created) -> io.println("Created: " <> created)
        option.None -> Nil
      }

      case metadata.modified_at {
        option.Some(modified) -> io.println("Modified: " <> modified)
        option.None -> Nil
      }

      case metadata.tags {
        option.Some(tags) ->
          io.println("Tags: " <> string.join(tags, ", "))
        option.None -> Nil
      }

      list.each(result.processing_warnings, fn(warning) {
        io.println("Warning [" <> warning.source <> "]: " <> warning.message)
      })
    }
    Error(_) -> io.println_error("extraction failed")
  }
}
```
