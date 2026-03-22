#' Create an extraction configuration
#'
#' @param force_ocr Logical. Force OCR processing. Default FALSE.
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
#' @param postprocessor Named list. Post-processor configuration.
#' @param security_limits Named list. Security limits configuration.
#' @param max_concurrent_extractions Integer. Max concurrent extractions.
#' @param layout Layout detection configuration created by \code{layout_detection_config()}.
#' @param email Email configuration created by \code{email_config()}.
#' @param concurrency Concurrency configuration created by \code{concurrency_config()}.
#' @param cache_namespace Character or NULL. Cache namespace for tenant isolation.
#'   When set, cache keys are scoped to this namespace so that different tenants'
#'   cached results do not collide. When NULL, the default namespace is used.
#' @param cache_ttl_secs Integer or NULL. Per-request cache TTL in seconds.
#'   Overrides the server default TTL for this extraction request. When NULL,
#'   the server default is used.
#' @param ... Additional configuration options passed as named list elements.
#' @return A named list representing the extraction configuration.
#' @export
extraction_config <- function(force_ocr = FALSE, ocr = NULL, chunking = NULL,
                              output_format = NULL, result_format = NULL,
                              use_cache = NULL, include_document_structure = NULL,
                              enable_quality_processing = NULL,
                              language_detection = NULL, keywords = NULL,
                              token_reduction = NULL, images = NULL,
                              pages = NULL, pdf_options = NULL,
                              html_options = NULL, postprocessor = NULL,
                              security_limits = NULL,
                              max_concurrent_extractions = NULL,
                              layout = NULL, email = NULL,
                              concurrency = NULL,
                              cache_namespace = NULL, cache_ttl_secs = NULL,
                              ...) {
  config <- list()
  if (isTRUE(force_ocr)) config$force_ocr <- TRUE
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
  if (!is.null(postprocessor)) config$postprocessor <- postprocessor
  if (!is.null(security_limits)) config$security_limits <- security_limits
  if (!is.null(max_concurrent_extractions)) {
    config$max_concurrent_extractions <- as.integer(max_concurrent_extractions)
  }
  if (!is.null(layout)) config$layout <- layout
  if (!is.null(email)) config$email <- email
  if (!is.null(concurrency)) config$concurrency <- concurrency
  if (!is.null(cache_namespace)) {
    stopifnot(is.character(cache_namespace), length(cache_namespace) == 1L)
    config$cache_namespace <- cache_namespace
  }
  if (!is.null(cache_ttl_secs)) {
    config$cache_ttl_secs <- as.integer(cache_ttl_secs)
  }
  extras <- list(...)
  if (length(extras) > 0) config <- c(config, extras)
  config
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
#' @param ... Additional chunking options.
#' @return A named list representing the chunking configuration.
#' @export
chunking_config <- function(max_characters = 1000L, overlap = 200L, ...) {
  max_characters <- as.integer(max_characters)
  overlap <- as.integer(overlap)
  if (max_characters <= 0L) stop("max_characters must be a positive integer", call. = FALSE)
  if (overlap < 0L) stop("overlap must be non-negative", call. = FALSE)
  config <- list(
    max_characters = max_characters,
    overlap = overlap
  )
  extras <- list(...)
  if (length(extras) > 0) config <- c(config, extras)
  config
}

#' Create a layout detection configuration
#'
#' @param preset Model preset controlling accuracy vs speed trade-off.
#'   Supported values: "fast", "accurate". Default "fast".
#' @param confidence_threshold Minimum confidence threshold for detected layout
#'   regions (0.0-1.0). Regions below this threshold are discarded.
#'   Default NULL (use engine default).
#' @param apply_heuristics Logical. Whether to apply heuristic post-processing
#'   to refine layout regions. Default TRUE.
#' @param ... Additional layout detection options.
#' @return A named list representing the layout detection configuration.
#' @export
layout_detection_config <- function(preset = "fast", confidence_threshold = NULL,
                                    apply_heuristics = TRUE, ...) {
  stopifnot(is.character(preset), length(preset) == 1L)
  config <- list(preset = preset, apply_heuristics = apply_heuristics)
  if (!is.null(confidence_threshold)) {
    confidence_threshold <- as.double(confidence_threshold)
    if (confidence_threshold < 0 || confidence_threshold > 1) {
      stop("confidence_threshold must be between 0.0 and 1.0", call. = FALSE)
    }
    config$confidence_threshold <- confidence_threshold
  }
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
