```gleam title="Gleam"
import gleam/io
import gleam/option
import kreuzberg

fn hierarchy_config() -> kreuzberg.HierarchyConfig {
  kreuzberg.HierarchyConfig(
    enabled: True,
    k_clusters: 4,
    include_bbox: False,
    ocr_coverage_threshold: option.None,
  )
}

fn pdf_config() -> kreuzberg.PdfConfig {
  kreuzberg.PdfConfig(
    extract_images: True,
    passwords: option.Some(["password123"]),
    extract_metadata: True,
    hierarchy: option.Some(hierarchy_config()),
    extract_annotations: False,
    top_margin_fraction: option.None,
    bottom_margin_fraction: option.None,
    allow_single_column_tables: False,
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
    pdf_options: option.Some(pdf_config()),
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
  case kreuzberg.extract_file_sync("encrypted.pdf", option.None, config()) {
    Ok(result) -> {
      case result.metadata.title {
        option.Some(title) -> io.println("Title: " <> title)
        option.None -> io.println("Title: none")
      }
    }
    Error(_) -> io.println_error("extraction failed")
  }
}
```
