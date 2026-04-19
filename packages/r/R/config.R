#' Create an extraction configuration
#'
#' @param force_ocr Logical. Force OCR processing. Default FALSE.
#' @param disable_ocr Logical. Disable OCR entirely. Image files return empty content. Default FALSE.
#' @param force_ocr_pages Integer vector or NULL. 1-indexed page numbers to force OCR on. Default NULL.
#' @param ocr OCR configuration created by \code{ocr_config()}.
#' @param chunking Chunking configuration created by \code{chunking_config()}.
#' @param output_format Output format string (e.g., "text", "markdown").
#' @param result_format Result format string (e.g., "unified", "element_based").
#' @param use_cache Logical. Enable extraction result caching.
#' @param include_document_structure Logical. Include document structure in output.
#' @param enable_quality_processing Logical. Enable quality score processing.
#' @param language_detection Named list. Language detection configuration.
#' @param keywords Named list. Keyword extraction configuration.
#' @param token_reduction Named list. Token reduction configuration.
#' @param images Named list. Image extraction configuration.
#' @param pages Named list. Page-level extraction configuration.
#' @param pdf_options Named list. PDF-specific options.
#' @param html_options Named list. HTML-specific options.
#' @param html_output Named list. HTML styled output configuration.
#'   Controls styled HTML rendering with fields: theme (character, one of
#'   "default", "github", "dark", "light", "unstyled"), class_prefix (character,
#'   default "kb-"), embed_css (logical, default TRUE), css (character or NULL,
#'   custom CSS string), css_file (character or NULL, path to CSS file).
#' @param postprocessor Named list. Post-processor configuration.
#' @param security_limits Named list. Security limits configuration.
#' @param max_concurrent_extractions Integer. Max concurrent extractions.
#' @param max_archive_depth Integer. Maximum depth for nested archive extraction. Default 3L.
#' @param layout Layout detection configuration created by \code{layout_detection_config()}.
#' @param acceleration Acceleration configuration created by \code{acceleration_config()}.
#' @param email Email configuration created by \code{email_config()}.
#' @param concurrency Concurrency configuration created by \code{concurrency_config()}.
#' @param cache_namespace Character or NULL. Cache namespace for tenant isolation.
#'   When set, cache keys are scoped to this namespace so that different tenants'
#'   cached results do not collide. When NULL, the default namespace is used.
#' @param cache_ttl_secs Integer or NULL. Per-request cache TTL in seconds.
#'   Overrides the server default TTL for this extraction request. When NULL,
#'   the server default is used.
#' @param extraction_timeout_secs Integer or NULL. Extraction timeout in seconds.
#'   When set, limits the maximum time allowed for an extraction operation.
#'   When NULL, the server default is used.
#' @param tree_sitter Tree-sitter configuration created by \code{tree_sitter_config()}.
#' @param content_filter Content filter configuration created by \code{content_filter_config()}.
#' @param ... Additional configuration options passed as named list elements.
#' @return A named list representing the extraction configuration.
#' @export
extraction_config <- function(force_ocr = FALSE, disable_ocr = FALSE,
                              force_ocr_pages = NULL, ocr = NULL, chunking = NULL,
                              output_format = NULL, result_format = NULL,
                              use_cache = NULL, include_document_structure = NULL,
                              enable_quality_processing = NULL,
                              language_detection = NULL, keywords = NULL,
                              token_reduction = NULL, images = NULL,
                              pages = NULL, pdf_options = NULL,
                              html_options = NULL, html_output = NULL,
                              postprocessor = NULL,
                              security_limits = NULL,
                              max_concurrent_extractions = NULL,
                              max_archive_depth = 3L,
                              layout = NULL, acceleration = NULL,
                              email = NULL, concurrency = NULL,
                              cache_namespace = NULL, cache_ttl_secs = NULL,
                              extraction_timeout_secs = NULL,
                              tree_sitter = NULL,
                              content_filter = NULL,
                              ...) {
  config <- list()
  if (isTRUE(force_ocr)) config$force_ocr <- TRUE
  if (isTRUE(disable_ocr)) config$disable_ocr <- TRUE
  if (!is.null(force_ocr_pages)) config$force_ocr_pages <- as.integer(force_ocr_pages)
  if (!is.null(ocr)) config$ocr <- ocr
  if (!is.null(chunking)) config$chunking <- chunking
  if (!is.null(output_format)) {
    stopifnot(is.character(output_format), length(output_format) == 1L)
    config$output_format <- output_format
  }
  if (!is.null(result_format)) {
    stopifnot(is.character(result_format), length(result_format) == 1L)
    config$result_format <- result_format
  }
  if (!is.null(use_cache)) config$use_cache <- use_cache
  if (!is.null(include_document_structure)) {
    config$include_document_structure <- include_document_structure
  }
  if (!is.null(enable_quality_processing)) {
    config$enable_quality_processing <- enable_quality_processing
  }
  if (!is.null(language_detection)) config$language_detection <- language_detection
  if (!is.null(keywords)) config$keywords <- keywords
  if (!is.null(token_reduction)) config$token_reduction <- token_reduction
  if (!is.null(images)) config$images <- images
  if (!is.null(pages)) config$pages <- pages
  if (!is.null(pdf_options)) config$pdf_options <- pdf_options
  if (!is.null(html_options)) config$html_options <- html_options
  if (!is.null(html_output)) config$html_output <- html_output
  if (!is.null(postprocessor)) config$postprocessor <- postprocessor
  if (!is.null(security_limits)) config$security_limits <- security_limits
  if (!is.null(max_concurrent_extractions)) {
    config$max_concurrent_extractions <- as.integer(max_concurrent_extractions)
  }
  if (!is.null(max_archive_depth)) {
    max_archive_depth <- as.integer(max_archive_depth)
    if (max_archive_depth < 0L) stop("max_archive_depth must be a non-negative integer", call. = FALSE)
    config$max_archive_depth <- max_archive_depth
  }
  if (!is.null(layout)) config$layout <- layout
  if (!is.null(acceleration)) config$acceleration <- acceleration
  if (!is.null(email)) config$email <- email
  if (!is.null(concurrency)) config$concurrency <- concurrency
  if (!is.null(cache_namespace)) {
    stopifnot(is.character(cache_namespace), length(cache_namespace) == 1L)
    config$cache_namespace <- cache_namespace
  }
  if (!is.null(cache_ttl_secs)) {
    config$cache_ttl_secs <- as.integer(cache_ttl_secs)
  }
  if (!is.null(extraction_timeout_secs)) {
    config$extraction_timeout_secs <- as.integer(extraction_timeout_secs)
  }
  if (!is.null(tree_sitter)) config$tree_sitter <- tree_sitter
  if (!is.null(content_filter)) config$content_filter <- content_filter
  extras <- list(...)
  if (length(extras) > 0) config <- c(config, extras)
  config
}

#' Create a content filter configuration
#'
#' Controls whether "furniture" content (headers, footers, page numbers,
#' watermarks, repeating text) is included in or stripped from extraction
#' results. Applies across all extractors.
#'
#' @param include_headers Logical. Include running headers in output. Default FALSE.
#' @param include_footers Logical. Include running footers in output. Default FALSE.
#' @param strip_repeating_text Logical. Enable cross-page repeating text
#'   detection and removal. Default TRUE.
#' @param include_watermarks Logical. Include watermark text in output. Default FALSE.
#' @return A named list representing the content filter configuration.
#' @export
content_filter_config <- function(include_headers = FALSE, include_footers = FALSE,
                                  strip_repeating_text = TRUE,
                                  include_watermarks = FALSE) {
  stopifnot(is.logical(include_headers), length(include_headers) == 1L)
  stopifnot(is.logical(include_footers), length(include_footers) == 1L)
  stopifnot(is.logical(strip_repeating_text), length(strip_repeating_text) == 1L)
  stopifnot(is.logical(include_watermarks), length(include_watermarks) == 1L)
  list(
    include_headers = include_headers,
    include_footers = include_footers,
    strip_repeating_text = strip_repeating_text,
    include_watermarks = include_watermarks
  )
}

#' Create an OCR configuration
#'
#' @param backend OCR backend name (e.g., "tesseract", "paddle-ocr").
#' @param language Language code for OCR (e.g., "eng", "deu").
#' @param dpi DPI for image processing. Must be a positive integer.
#' @param ... Additional OCR options.
#' @return A named list representing the OCR configuration.
#' @export
ocr_config <- function(backend = "tesseract", language = "eng", dpi = NULL, ...) {
  stopifnot(is.character(backend), length(backend) == 1L)
  stopifnot(is.character(language), length(language) == 1L)
  config <- list(backend = backend, language = language)
  if (!is.null(dpi)) {
    dpi <- as.integer(dpi)
    if (dpi <= 0L) stop("dpi must be a positive integer", call. = FALSE)
    config$dpi <- dpi
  }
  extras <- list(...)
  if (length(extras) > 0) config <- c(config, extras)
  config
}

#' Create a chunking configuration
#'
#' @param max_characters Maximum characters per chunk. Must be a positive integer.
#' @param overlap Number of overlapping characters between chunks. Must be non-negative.
#' @param chunker_type Chunker type: "text", "markdown", "yaml", or "semantic". Default "text".
#' @param topic_threshold Numeric or NULL. Cosine similarity threshold for semantic
#'   topic detection (0.0-1.0). Only used when chunker_type is "semantic". Default NULL (0.75).
#' @param ... Additional chunking options.
#' @return A named list representing the chunking configuration.
#' @export
chunking_config <- function(max_characters = 1000L, overlap = 200L,
                            chunker_type = "text", topic_threshold = NULL, ...) {
  max_characters <- as.integer(max_characters)
  overlap <- as.integer(overlap)
  if (max_characters <= 0L) stop("max_characters must be a positive integer", call. = FALSE)
  if (overlap < 0L) stop("overlap must be non-negative", call. = FALSE)
  stopifnot(is.character(chunker_type), length(chunker_type) == 1L)
  valid_chunker_types <- c("text", "markdown", "yaml", "semantic")
  if (!chunker_type %in% valid_chunker_types) {
    stop(
      paste0(
        "chunker_type must be one of: ",
        paste(valid_chunker_types, collapse = ", "),
        ", got: ", chunker_type
      ),
      call. = FALSE
    )
  }
  config <- list(
    max_characters = max_characters,
    overlap = overlap,
    chunker_type = chunker_type
  )
  if (!is.null(topic_threshold)) {
    topic_threshold <- as.double(topic_threshold)
    if (topic_threshold < 0 || topic_threshold > 1) {
      stop("topic_threshold must be between 0.0 and 1.0", call. = FALSE)
    }
    config$topic_threshold <- topic_threshold
  }
  extras <- list(...)
  if (length(extras) > 0) config <- c(config, extras)
  config
}

#' Create a layout detection configuration
#'
#' @param confidence_threshold Minimum confidence threshold for detected layout
#'   regions (0.0-1.0). Regions below this threshold are discarded.
#'   Default NULL (use engine default).
#' @param apply_heuristics Logical. Whether to apply heuristic post-processing
#'   to refine layout regions. Default TRUE.
#' @param table_model Table structure recognition model to use. Supported values:
#'   "tatr" (default), "slanet_wired", "slanet_wireless", "slanet_plus",
#'   "slanet_auto", "disabled".
#'   Default NULL (use engine default).
#' @param acceleration Named list or NULL. Hardware acceleration configuration
#'   (e.g., from \code{acceleration_config()}). Controls which ONNX execution
#'   provider is used for layout and table models. Default NULL (auto-select).
#' @param ... Additional layout detection options.
#' @return A named list representing the layout detection configuration.
#' @export
layout_detection_config <- function(confidence_threshold = NULL,
                                    apply_heuristics = TRUE, table_model = NULL,
                                    acceleration = NULL, ...) {
  config <- list(apply_heuristics = apply_heuristics)
  if (!is.null(confidence_threshold)) {
    confidence_threshold <- as.double(confidence_threshold)
    if (confidence_threshold < 0 || confidence_threshold > 1) {
      stop("confidence_threshold must be between 0.0 and 1.0", call. = FALSE)
    }
    config$confidence_threshold <- confidence_threshold
  }
  if (!is.null(table_model)) {
    stopifnot(is.character(table_model), length(table_model) == 1L)
    valid_table_models <- c("tatr", "slanet_wired", "slanet_wireless", "slanet_plus", "slanet_auto", "disabled")
    if (!table_model %in% valid_table_models) {
      stop(
        paste0(
          "table_model must be one of: ",
          paste(valid_table_models, collapse = ", "),
          ", got: ", table_model
        ),
        call. = FALSE
      )
    }
    config$table_model <- table_model
  }
  if (!is.null(acceleration)) config$acceleration <- acceleration
  extras <- list(...)
  if (length(extras) > 0) config <- c(config, extras)
  config
}

#' Create a concurrency configuration
#'
#' @param max_threads Integer or NULL. Maximum number of threads for parallel
#'   processing. When NULL, the Rust default is used.
#' @return A named list representing the concurrency configuration.
#' @export
concurrency_config <- function(max_threads = NULL) {
  cfg <- list()
  if (!is.null(max_threads)) cfg$max_threads <- as.integer(max_threads)
  cfg
}

#' Create a PDF extraction configuration
#'
#' @param extract_images Logical. Extract images from PDFs. Default FALSE.
#' @param passwords Character vector or NULL. Passwords for encrypted PDFs.
#' @param extract_metadata Logical. Extract PDF metadata. Default TRUE.
#' @param extract_annotations Logical. Extract PDF annotations. Default FALSE.
#' @param top_margin_fraction Numeric or NULL. Top margin fraction (0.0-1.0).
#' @param bottom_margin_fraction Numeric or NULL. Bottom margin fraction (0.0-1.0).
#' @param allow_single_column_tables Logical. Allow single-column tables. Default FALSE.
#' @param ... Additional PDF options.
#' @return A named list representing the PDF configuration.
#' @export
pdf_config <- function(extract_images = FALSE, passwords = NULL,
                       extract_metadata = TRUE, extract_annotations = FALSE,
                       top_margin_fraction = NULL, bottom_margin_fraction = NULL,
                       allow_single_column_tables = FALSE, ...) {
  config <- list(
    extract_images = extract_images,
    extract_metadata = extract_metadata,
    extract_annotations = extract_annotations,
    allow_single_column_tables = allow_single_column_tables
  )
  if (!is.null(passwords)) config$passwords <- as.character(passwords)
  if (!is.null(top_margin_fraction)) {
    config$top_margin_fraction <- as.double(top_margin_fraction)
  }
  if (!is.null(bottom_margin_fraction)) {
    config$bottom_margin_fraction <- as.double(bottom_margin_fraction)
  }
  extras <- list(...)
  if (length(extras) > 0) config <- c(config, extras)
  config
}

#' Create an email extraction configuration
#'
#' @param msg_fallback_codepage Integer or NULL. Fallback Windows code page for MSG
#'   email body decoding. Common values: 1250 (Central European), 1251 (Cyrillic),
#'   1252 (Western European, default), 1253 (Greek), 1254 (Turkish).
#'   When NULL, the Rust default (windows-1252) is used.
#' @return A named list representing the email extraction configuration.
#' @export
email_config <- function(msg_fallback_codepage = NULL) {
  config <- list()
  if (!is.null(msg_fallback_codepage)) {
    msg_fallback_codepage <- as.integer(msg_fallback_codepage)
    if (msg_fallback_codepage <= 0L) stop("msg_fallback_codepage must be a positive integer", call. = FALSE)
    config$msg_fallback_codepage <- msg_fallback_codepage
  }
  config
}

#' Create a hardware acceleration configuration
#'
#' @param provider Character. Execution provider for ONNX model inference.
#'   Supported values: "auto" (default), "cpu", "coreml", "cuda", "tensorrt".
#' @param device_id Integer. Device ID for GPU selection. Default 0L.
#' @return A named list representing the acceleration configuration.
#' @export
acceleration_config <- function(provider = "auto", device_id = 0L) {
  stopifnot(is.character(provider), length(provider) == 1L)
  valid_providers <- c("auto", "cpu", "coreml", "cuda", "tensorrt")
  if (!provider %in% valid_providers) {
    stop(
      paste0(
        "provider must be one of: ",
        paste(valid_providers, collapse = ", "),
        ", got: ", provider
      ),
      call. = FALSE
    )
  }
  device_id <- as.integer(device_id)
  if (device_id < 0L) stop("device_id must be a non-negative integer", call. = FALSE)
  list(provider = provider, device_id = device_id)
}

#' Create a tree-sitter process configuration
#'
#' @param structure Logical. Extract structural items. Default TRUE.
#' @param imports Logical. Extract import statements. Default TRUE.
#' @param exports Logical. Extract export statements. Default TRUE.
#' @param comments Logical. Extract comments. Default FALSE.
#' @param docstrings Logical. Extract docstrings. Default FALSE.
#' @param symbols Logical. Extract symbol definitions. Default FALSE.
#' @param diagnostics Logical. Include parse diagnostics. Default FALSE.
#' @param chunk_max_size Integer or NULL. Maximum chunk size in bytes. NULL disables chunking.
#' @return A named list representing the tree-sitter process configuration.
#' @export
tree_sitter_process_config <- function(structure = TRUE, imports = TRUE, exports = TRUE,
                                       comments = FALSE, docstrings = FALSE,
                                       symbols = FALSE, diagnostics = FALSE,
                                       chunk_max_size = NULL,
                                       content_mode = NULL) {
  config <- list(
    structure = structure,
    imports = imports,
    exports = exports,
    comments = comments,
    docstrings = docstrings,
    symbols = symbols,
    diagnostics = diagnostics
  )
  if (!is.null(chunk_max_size)) {
    config$chunk_max_size <- as.integer(chunk_max_size)
  }
  if (!is.null(content_mode)) {
    stopifnot(is.character(content_mode), length(content_mode) == 1L)
    config$content_mode <- content_mode
  }
  config
}

#' Create a tree-sitter configuration
#'
#' @param cache_dir Character or NULL. Custom cache directory for downloaded grammars.
#' @param languages Character vector or NULL. Languages to pre-download on init.
#' @param groups Character vector or NULL. Language groups to pre-download.
#' @param process Tree-sitter process configuration created by \code{tree_sitter_process_config()}.
#' @return A named list representing the tree-sitter configuration.
#' @export
tree_sitter_config <- function(cache_dir = NULL, languages = NULL, groups = NULL,
                               process = NULL, enabled = NULL) {
  config <- list()
  if (!is.null(enabled)) {
    stopifnot(is.logical(enabled), length(enabled) == 1L)
    config$enabled <- enabled
  }
  if (!is.null(cache_dir)) {
    stopifnot(is.character(cache_dir), length(cache_dir) == 1L)
    config$cache_dir <- cache_dir
  }
  if (!is.null(languages)) config$languages <- as.character(languages)
  if (!is.null(groups)) config$groups <- as.character(groups)
  if (!is.null(process)) config$process <- process
  config
}

#' Discover extraction configuration from kreuzberg.toml
#'
#' Searches for a kreuzberg.toml file in the current directory and parent
#' directories. Returns the parsed configuration or NULL if not found.
#'
#' @return A named list representing the extraction configuration, or NULL.
#' @export
discover <- function() {
  json <- check_native_result(config_discover_native())
  if (is.null(json)) {
    return(NULL)
  }
  jsonlite::fromJSON(json, simplifyVector = FALSE)
}

#' Load extraction configuration from a file
#'
#' Reads and parses a configuration file. Supports TOML, YAML, and JSON formats
#' (auto-detected from file extension).
#'
#' @param path Path to the configuration file.
#' @return A named list representing the extraction configuration.
#' @export
from_file <- function(path) {
  stopifnot(is.character(path), length(path) == 1L)
  json <- check_native_result(config_from_file_native(path))
  if (is.null(json)) {
    return(NULL)
  }
  jsonlite::fromJSON(json, simplifyVector = FALSE)
}

#' Create an embedding configuration
#'
#' @param model Embedding model name or preset (e.g., "fast", "balanced", "quality", "multilingual").
#' @param normalize Logical. Normalize embedding vectors to unit length. Default TRUE.
#' @param batch_size Integer or NULL. Batch size for embedding generation. Default NULL.
#' @param acceleration Named list or NULL. Hardware acceleration configuration
#'   (e.g., from \code{acceleration_config()}). Controls which ONNX execution
#'   provider is used for the embedding model. Default NULL (auto-select).
#' @return A named list representing the embedding configuration.
#' @export
embedding_config <- function(model = "balanced", normalize = TRUE, batch_size = NULL,
                             acceleration = NULL) {
  stopifnot(is.character(model), length(model) == 1L)
  stopifnot(is.logical(normalize), length(normalize) == 1L)

  config <- list(
    model = list(type = "preset", name = model),
    normalize = normalize
  )

  if (!is.null(batch_size)) {
    batch_size <- as.integer(batch_size)
    if (batch_size <= 0L) stop("batch_size must be a positive integer", call. = FALSE)
    config$batch_size <- batch_size
  }

  if (!is.null(acceleration)) config$acceleration <- acceleration

  config
}
