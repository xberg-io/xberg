#' Convert a list to a kreuzberg_code_process_result S3 object
#'
#' Tree-sitter code processing results are returned as nested lists from the
#' native layer. This function tags the result with the appropriate S3 class
#' for pretty-printing and accessor methods.
#'
#' A \code{kreuzberg_code_process_result} contains the following fields:
#' \describe{
#'   \item{language}{Character. Detected programming language.}
#'   \item{metrics}{Named list with file-level metrics:
#'     \code{total_lines}, \code{code_lines}, \code{comment_lines},
#'     \code{blank_lines}, \code{total_bytes}, \code{node_count},
#'     \code{error_count}, \code{max_depth}.}
#'   \item{structure}{List of structural items (functions, classes, etc.). Each
#'     item has \code{kind}, \code{name}, \code{visibility}, \code{span},
#'     \code{children}, \code{decorators}, \code{doc_comment}, \code{signature},
#'     \code{body_span}.}
#'   \item{imports}{List of import info. Each has \code{source}, \code{items},
#'     \code{alias}, \code{is_wildcard}, \code{span}.}
#'   \item{exports}{List of export info. Each has \code{name}, \code{kind},
#'     \code{span}.}
#'   \item{comments}{List of comment info. Each has \code{text}, \code{kind},
#'     \code{span}.}
#'   \item{docstrings}{List of docstring info. Each has \code{text},
#'     \code{format}, \code{associated_item}, \code{span}, \code{sections}.}
#'   \item{symbols}{List of symbol info. Each has \code{name}, \code{kind},
#'     \code{type_annotation}, \code{span}.}
#'   \item{diagnostics}{List of diagnostics. Each has \code{message},
#'     \code{severity}, \code{span}.}
#'   \item{chunks}{List of code chunks. Each has \code{content},
#'     \code{language}, \code{span}, \code{context}.}
#' }
#'
#' A \code{span} is a named list with \code{start_byte}, \code{end_byte},
#' \code{start_line}, \code{start_column}, \code{end_line}, \code{end_column}.
#'
#' A chunk \code{context} is a named list with \code{parent_name},
#' \code{parent_kind}.
#'
#' A docstring \code{section} is a named list with \code{kind}, \code{name},
#' \code{content}.
#'
#' @param x A named list from native tree-sitter processing.
#' @return Object with class \code{kreuzberg_code_process_result}.
#' @keywords internal
as_code_process_result <- function(x) {
  if (!inherits(x, "kreuzberg_code_process_result")) {
    class(x) <- c("kreuzberg_code_process_result", "list")
  }
  x
}

#' Print method for kreuzberg_code_process_result
#'
#' @param x A \code{kreuzberg_code_process_result} object.
#' @param ... Additional arguments (ignored).
#' @export
print.kreuzberg_code_process_result <- function(x, ...) {
  cat("<kreuzberg_code_process_result>\n")
  if (!is.null(x$language)) cat("  Language:", x$language, "\n")
  if (!is.null(x$metrics)) {
    cat("  Metrics:\n")
    cat("    Total lines:", x$metrics$total_lines %||% 0, "\n")
    cat("    Code lines:", x$metrics$code_lines %||% 0, "\n")
    cat("    Comment lines:", x$metrics$comment_lines %||% 0, "\n")
    cat("    Blank lines:", x$metrics$blank_lines %||% 0, "\n")
    cat("    Error count:", x$metrics$error_count %||% 0, "\n")
  }
  if (!is.null(x$structure)) cat("  Structure items:", length(x$structure), "\n")
  if (!is.null(x$imports)) cat("  Imports:", length(x$imports), "\n")
  if (!is.null(x$exports)) cat("  Exports:", length(x$exports), "\n")
  if (!is.null(x$comments)) cat("  Comments:", length(x$comments), "\n")
  if (!is.null(x$docstrings)) cat("  Docstrings:", length(x$docstrings), "\n")
  if (!is.null(x$symbols)) cat("  Symbols:", length(x$symbols), "\n")
  if (!is.null(x$diagnostics)) cat("  Diagnostics:", length(x$diagnostics), "\n")
  if (!is.null(x$chunks)) cat("  Chunks:", length(x$chunks), "\n")
  invisible(x)
}

#' Summary method for kreuzberg_code_process_result
#'
#' @param object A \code{kreuzberg_code_process_result} object.
#' @param ... Additional arguments (ignored).
#' @export
summary.kreuzberg_code_process_result <- function(object, ...) {
  cat("<kreuzberg_code_process_result summary>\n")
  cat("  Language:       ", object$language %||% "(unknown)", "\n")
  cat("  Total lines:    ", object$metrics$total_lines %||% 0, "\n")
  cat("  Code lines:     ", object$metrics$code_lines %||% 0, "\n")
  cat("  Comment lines:  ", object$metrics$comment_lines %||% 0, "\n")
  cat("  Structure:      ", length(object$structure %||% list()), "\n")
  cat("  Imports:        ", length(object$imports %||% list()), "\n")
  cat("  Exports:        ", length(object$exports %||% list()), "\n")
  cat("  Comments:       ", length(object$comments %||% list()), "\n")
  cat("  Docstrings:     ", length(object$docstrings %||% list()), "\n")
  cat("  Symbols:        ", length(object$symbols %||% list()), "\n")
  cat("  Diagnostics:    ", length(object$diagnostics %||% list()), "\n")
  cat("  Chunks:         ", length(object$chunks %||% list()), "\n")
  invisible(object)
}

#' Format method for kreuzberg_code_process_result
#'
#' @param x A \code{kreuzberg_code_process_result} object.
#' @param ... Additional arguments (ignored).
#' @return A character string representation.
#' @export
format.kreuzberg_code_process_result <- function(x, ...) {
  paste0(
    "<kreuzberg_code_process_result: ",
    x$language %||% "unknown",
    ", ",
    x$metrics$total_lines %||% 0,
    " lines>"
  )
}
