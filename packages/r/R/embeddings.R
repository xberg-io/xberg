#' Generate text embeddings for a list of strings
#'
#' @param texts Character vector of strings to embed.
#' @param config Optional embedding configuration from \code{embedding_config()}.
#' @return A list of numeric vectors (one per input string).
#' @export
embed <- function(texts, config = NULL) {
  stopifnot(is.character(texts))
  config_json <- if (!is.null(config)) jsonlite::toJSON(config, auto_unbox = TRUE) else NULL
  check_native_result(embed_native(texts, config_json))
}
