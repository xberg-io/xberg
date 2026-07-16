#' Create an options list for generated bindings
#'
#' All parameters default to `NULL`, which means the Rust default is used.
#' Pass named arguments to override individual settings.
#'
#' @param provider Execution provider to use for ONNX inference
#' @param device_id GPU device ID (for CUDA/TensorRT). Ignored for CPU/CoreML/Auto
#' @return A named list suitable for the `options` argument of [convert()].
#' @export
conversion_options <- function(
  provider = NULL,
  device_id = NULL
) {
  opts <- list()
  if (!is.null(provider)) opts$provider <- provider
  if (!is.null(device_id)) opts$device_id <- as.integer(device_id)
  opts
}
