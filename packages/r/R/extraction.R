#' Extract content from a file (synchronous)
#'
#' @param path Path to the file.
#' @param mime_type Optional MIME type override.
#' @param config Optional extraction configuration from \code{extraction_config()}.
#' @return A \code{kreuzberg_result} object.
#' @export
extract_file_sync <- function(path, mime_type = NULL, config = NULL) {
  stopifnot(is.character(path), length(path) == 1L)
  if (!file.exists(path)) stop("File not found: ", path, call. = FALSE)
  if (!is.null(mime_type)) stopifnot(is.character(mime_type), length(mime_type) == 1L)
  config_json <- if (!is.null(config)) jsonlite::toJSON(config, auto_unbox = TRUE) else NULL
  result <- check_native_result(extract_file_sync_native(path, mime_type, config_json))
  as_kreuzberg_result(result)
}

#' Extract content from a file (async, blocks in R)
#'
#' @param path Path to the file.
#' @param mime_type Optional MIME type override.
#' @param config Optional extraction configuration from \code{extraction_config()}.
#' @return A \code{kreuzberg_result} object.
#' @export
extract_file <- function(path, mime_type = NULL, config = NULL) {
  stopifnot(is.character(path), length(path) == 1L)
  if (!file.exists(path)) stop("File not found: ", path, call. = FALSE)
  if (!is.null(mime_type)) stopifnot(is.character(mime_type), length(mime_type) == 1L)
  config_json <- if (!is.null(config)) jsonlite::toJSON(config, auto_unbox = TRUE) else NULL
  result <- check_native_result(extract_file_native(path, mime_type, config_json))
  as_kreuzberg_result(result)
}

#' Extract content from raw bytes (synchronous)
#'
#' @param data Raw vector of bytes.
#' @param mime_type MIME type of the data.
#' @param config Optional extraction configuration from \code{extraction_config()}.
#' @return A \code{kreuzberg_result} object.
#' @export
extract_bytes_sync <- function(data, mime_type, config = NULL) {
  stopifnot(is.raw(data))
  stopifnot(is.character(mime_type), length(mime_type) == 1L)
  config_json <- if (!is.null(config)) jsonlite::toJSON(config, auto_unbox = TRUE) else NULL
  result <- check_native_result(extract_bytes_sync_native(data, mime_type, config_json))
  as_kreuzberg_result(result)
}

#' Extract content from raw bytes (async, blocks in R)
#'
#' @param data Raw vector of bytes.
#' @param mime_type MIME type of the data.
#' @param config Optional extraction configuration from \code{extraction_config()}.
#' @return A \code{kreuzberg_result} object.
#' @export
extract_bytes <- function(data, mime_type, config = NULL) {
  stopifnot(is.raw(data))
  stopifnot(is.character(mime_type), length(mime_type) == 1L)
  config_json <- if (!is.null(config)) jsonlite::toJSON(config, auto_unbox = TRUE) else NULL
  result <- check_native_result(extract_bytes_native(data, mime_type, config_json))
  as_kreuzberg_result(result)
}

#' Render all pages of a PDF as PNG images
#'
#' @param path Path to the PDF file.
#' @param dpi Rendering resolution (default 150).
#' @return A list of raw vectors, each containing PNG-encoded bytes for one page.
#' @export
render_pdf_pages <- function(path, dpi = 150L) {
  stopifnot(is.character(path), length(path) == 1L)
  if (!file.exists(path)) stop("File not found: ", path, call. = FALSE)
  stopifnot(is.numeric(dpi), length(dpi) == 1L)
  check_native_result(render_pdf_pages_native(path, as.integer(dpi)))
}

#' Render a single PDF page as a PNG image
#'
#' @param path Path to the PDF file.
#' @param page_index Zero-based page index.
#' @param dpi Rendering resolution (default 150).
#' @return A raw vector containing PNG-encoded bytes.
#' @export
render_pdf_page <- function(path, page_index, dpi = 150L) {
  stopifnot(is.character(path), length(path) == 1L)
  if (!file.exists(path)) stop("File not found: ", path, call. = FALSE)
  stopifnot(is.numeric(page_index), length(page_index) == 1L)
  stopifnot(is.numeric(dpi), length(dpi) == 1L)
  check_native_result(render_pdf_page_native(path, as.integer(page_index), as.integer(dpi)))
}
