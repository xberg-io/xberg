#!/usr/bin/env Rscript

library(kreuzberg)
library(jsonlite)

DEBUG <- identical(Sys.getenv('KREUZBERG_BENCHMARK_DEBUG'), 'true')

debug_log <- function(message) {
  if (!DEBUG) {
    return()
  }
  timestamp <- format(Sys.time(), "%Y-%m-%d %H:%M:%OS3")
  cat(sprintf("[DEBUG] %s - %s\n", timestamp, message), file = stderr())
}

peak_memory_bytes <- function() {
  tryCatch({
    # Try /proc/self/status first (Linux)
    if (file.exists('/proc/self/status')) {
      content <- readLines('/proc/self/status')
      vmrss_line <- grep('^VmRSS:', content, value = TRUE)
      if (length(vmrss_line) > 0) {
        kb <- as.numeric(sub('^VmRSS:\\s+(\\d+).*', '\\1', vmrss_line[1]))
        return(kb * 1024)
      }
    }

    # Fall back to ps command
    pid <- Sys.getpid()
    result <- system(sprintf("ps -o rss= -p %d", pid), intern = TRUE)
    if (length(result) > 0 && !is.na(as.numeric(result[1]))) {
      return(as.numeric(result[1]) * 1024)
    }

    # Fallback using gc()
    gc_result <- gc()
    return(gc_result[2, 2] * 1024 * 1024)
  },
  error = function(e) {
    debug_log(sprintf("Error getting peak memory: %s", e$message))
    return(0)
  })
}

determine_ocr_used <- function(metadata, ocr_enabled) {
  if (is.null(metadata)) {
    return(FALSE)
  }

  format_type <- metadata$format_type
  if (is.null(format_type)) {
    format_type <- ''
  }

  if (format_type == 'ocr') {
    return(TRUE)
  }

  if ((format_type == 'image' || format_type == 'pdf') && ocr_enabled) {
    return(TRUE)
  }

  FALSE
}

parse_request <- function(line) {
  stripped <- trimws(line)

  # Try to parse as JSON first
  tryCatch({
    if (startsWith(stripped, '{')) {
      req <- fromJSON(stripped)
      path <- req$path %||% ''
      force_ocr <- req$force_ocr %||% FALSE
      return(list(path = path, force_ocr = force_ocr))
    }
  },
  error = function(e) {
    # Fall through to plain path
  })

  # Return as plain path
  list(path = stripped, force_ocr = FALSE)
}

extract_sync <- function(file_path, config = NULL) {
  debug_log("=== SYNC EXTRACTION START ===")
  debug_log(sprintf("Input: file_path=%s", file_path))
  debug_log(sprintf("File exists: %s", file.exists(file_path)))

  if (file.exists(file_path)) {
    debug_log(sprintf("File size: %d bytes", file.size(file_path)))
  }

  start_monotonic <- Sys.time()
  debug_log(sprintf("Timing start: %s", format(start_monotonic, "%Y-%m-%d %H:%M:%OS6")))

  result <- tryCatch({
    if (is.null(config)) {
      extract_file_sync(file_path)
    } else {
      extract_file_sync(file_path, config = config)
    }
  },
  error = function(e) {
    debug_log(sprintf("ERROR during extraction: %s", e$message))
    debug_log(sprintf("Backtrace:\n%s", paste(e$call, collapse = "\n")))
    stop(e)
  })

  end_monotonic <- Sys.time()
  duration_s <- as.numeric(end_monotonic - start_monotonic)
  duration_ms <- duration_s * 1000.0

  debug_log(sprintf("Timing end: %s", format(end_monotonic, "%Y-%m-%d %H:%M:%OS6")))
  debug_log(sprintf("Duration (seconds): %f", duration_s))
  debug_log(sprintf("Duration (milliseconds): %f", duration_ms))
  debug_log(sprintf("Result class: %s", class(result)))
  debug_log(sprintf("Result has content: %s", !is.null(result$content)))

  if (!is.null(result$content)) {
    debug_log(sprintf("Content length: %d characters", nchar(result$content)))
  }

  debug_log(sprintf("Result has metadata: %s", !is.null(result$metadata)))

  metadata <- result$metadata %||% list()
  ocr_enabled <- if (is.null(config)) FALSE else (!is.null(config$ocr) && config$ocr$enabled)

  payload <- list(
    content = result$content,
    metadata = metadata,
    `_extraction_time_ms` = duration_ms,
    `_ocr_used` = determine_ocr_used(metadata, ocr_enabled),
    `_peak_memory_bytes` = peak_memory_bytes()
  )

  json_output <- toJSON(payload, auto_unbox = TRUE)
  debug_log(sprintf("Output JSON size: %d bytes", nchar(json_output)))
  debug_log("=== SYNC EXTRACTION END ===")

  payload
}

extract_batch <- function(file_paths, config = NULL) {
  debug_log("=== BATCH EXTRACTION START ===")
  debug_log(sprintf("Input: %d files", length(file_paths)))

  for (i in seq_along(file_paths)) {
    path <- file_paths[i]
    file_exists <- file.exists(path)
    file_size <- if (file_exists) file.size(path) else 'N/A'
    debug_log(sprintf("  [%d] %s (exists: %s, size: %s bytes)", i - 1, path, file_exists, file_size))
  }

  start_monotonic <- Sys.time()
  debug_log(sprintf("Timing start: %s", format(start_monotonic, "%Y-%m-%d %H:%M:%OS6")))

  results <- tryCatch({
    if (is.null(config)) {
      batch_extract_files_sync(file_paths)
    } else {
      batch_extract_files_sync(file_paths, config = config)
    }
  },
  error = function(e) {
    debug_log(sprintf("ERROR during batch extraction: %s", e$message))
    debug_log(sprintf("Backtrace:\n%s", paste(e$call, collapse = "\n")))
    stop(e)
  })

  end_monotonic <- Sys.time()
  total_duration_s <- as.numeric(end_monotonic - start_monotonic)
  total_duration_ms <- total_duration_s * 1000.0

  debug_log(sprintf("Timing end: %s", format(end_monotonic, "%Y-%m-%d %H:%M:%OS6")))
  debug_log(sprintf("Total duration (seconds): %f", total_duration_s))
  debug_log(sprintf("Total duration (milliseconds): %f", total_duration_ms))
  debug_log(sprintf("Results count: %d", length(results)))

  per_file_duration_ms <- if (length(file_paths) > 0) total_duration_ms / length(file_paths) else 0
  debug_log(sprintf("Per-file average duration (milliseconds): %f", per_file_duration_ms))

  ocr_enabled <- if (is.null(config)) FALSE else (!is.null(config$ocr) && config$ocr$enabled)
  peak_mem <- peak_memory_bytes()

  results_with_timing <- lapply(seq_along(results), function(idx) {
    result <- results[[idx]]
    metadata <- result$metadata %||% list()

    debug_log(sprintf("  Result[%d] - content length: %s, has metadata: %s",
                      idx - 1,
                      if (!is.null(result$content)) nchar(result$content) else 'nil',
                      !is.null(result$metadata)))

    list(
      content = result$content,
      metadata = metadata,
      `_extraction_time_ms` = per_file_duration_ms,
      `_batch_total_ms` = total_duration_ms,
      `_ocr_used` = determine_ocr_used(metadata, ocr_enabled),
      `_peak_memory_bytes` = peak_mem
    )
  })

  debug_log("=== BATCH EXTRACTION END ===")

  results_with_timing
}

extract_server <- function(ocr_enabled) {
  debug_log("=== SERVER MODE START ===")

  # Signal readiness
  cat("READY\n")
  flush(stdout())

  # Read from stdin line by line
  con <- stdin()
  while (TRUE) {
    line <- readLines(con, n = 1)

    if (length(line) == 0) {
      break
    }

    req <- parse_request(line)
    file_path <- req$path
    force_ocr <- req$force_ocr

    if (file_path == '') {
      next
    }

    debug_log(sprintf("Processing file: %s, force_ocr: %s", file_path, force_ocr))

    tryCatch({
      config <- extraction_config(use_cache = FALSE)

      if (ocr_enabled || force_ocr) {
        config <- extraction_config(ocr = list(enabled = TRUE), use_cache = FALSE)
      }

      start <- Sys.time()
      result <- extract_file_sync(file_path, config = config)
      duration_ms <- as.numeric(Sys.time() - start) * 1000.0

      metadata <- result$metadata %||% list()
      payload <- list(
        content = result$content,
        metadata = metadata,
        `_extraction_time_ms` = duration_ms,
        `_ocr_used` = determine_ocr_used(metadata, (ocr_enabled || force_ocr)),
        `_peak_memory_bytes` = peak_memory_bytes()
      )

      cat(toJSON(payload, auto_unbox = TRUE), "\n", sep = "")
      flush(stdout())
    },
    error = function(e) {
      error_payload <- list(
        error = e$message,
        `_extraction_time_ms` = 0,
        `_ocr_used` = FALSE
      )
      cat(toJSON(error_payload, auto_unbox = TRUE), "\n", sep = "")
      flush(stdout())
    })
  }

  debug_log("=== SERVER MODE END ===")
}

main <- function() {
  debug_log("R script started")
  debug_log(sprintf("ARGV: %s", paste(commandArgs(trailingOnly = TRUE), collapse = ", ")))

  args <- commandArgs(trailingOnly = TRUE)
  debug_log(sprintf("ARGV length: %d", length(args)))

  ocr_enabled <- FALSE
  parsed_args <- c()

  for (arg in args) {
    if (arg == '--ocr') {
      ocr_enabled <- TRUE
    } else if (arg == '--no-ocr') {
      ocr_enabled <- FALSE
    } else {
      parsed_args <- c(parsed_args, arg)
    }
  }

  if (length(parsed_args) < 1) {
    cat("Usage: kreuzberg_extract.R [--ocr|--no-ocr] <mode> <file_path> [additional_files...]\n", file = stderr())
    cat("Modes: sync, batch, server\n", file = stderr())
    cat("Debug mode: set KREUZBERG_BENCHMARK_DEBUG=true to enable debug logging to stderr\n", file = stderr())
    quit(status = 1)
  }

  mode <- parsed_args[1]
  file_paths <- if (length(parsed_args) > 1) parsed_args[2:length(parsed_args)] else c()

  debug_log(sprintf("Mode: %s", mode))
  debug_log(sprintf("OCR enabled: %s", ocr_enabled))
  debug_log(sprintf("File paths (%d): %s", length(file_paths), paste(file_paths, collapse = ", ")))

  tryCatch({
    if (mode == 'server') {
      debug_log("Executing server mode")
      extract_server(ocr_enabled)

    } else if (mode == 'sync') {
      if (length(file_paths) != 1) {
        cat("Error: sync mode requires exactly one file\n", file = stderr())
        quit(status = 1)
      }
      debug_log(sprintf("Executing sync mode with file: %s", file_paths[1]))

      config <- extraction_config(use_cache = FALSE)
      if (ocr_enabled) {
        config <- extraction_config(ocr = list(enabled = TRUE), use_cache = FALSE)
      }

      payload <- extract_sync(file_paths[1], config)
      output <- toJSON(payload, auto_unbox = TRUE)
      debug_log(sprintf("Output JSON: %s", output))
      cat(output, "\n", sep = "")

    } else if (mode == 'batch') {
      if (length(file_paths) == 0) {
        cat("Error: batch mode requires at least one file\n", file = stderr())
        quit(status = 1)
      }
      debug_log(sprintf("Executing batch mode with %d files", length(file_paths)))

      config <- extraction_config(use_cache = FALSE)
      if (ocr_enabled) {
        config <- extraction_config(ocr = list(enabled = TRUE), use_cache = FALSE)
      }

      results <- extract_batch(file_paths, config)

      if (length(file_paths) == 1) {
        output <- toJSON(results[[1]], auto_unbox = TRUE)
        debug_log(sprintf("Output JSON (single file): %s", output))
        cat(output, "\n", sep = "")
      } else {
        output <- toJSON(results, auto_unbox = TRUE)
        if (nchar(output) > 200) {
          debug_log(sprintf("Output JSON (multiple files): %s...", substr(output, 1, 200)))
        } else {
          debug_log(sprintf("Output JSON (multiple files): %s", output))
        }
        cat(output, "\n", sep = "")
      }

    } else {
      cat(sprintf("Error: Unknown mode '%s'. Use sync, batch, or server\n", mode), file = stderr())
      quit(status = 1)
    }

    debug_log("Script completed successfully")
  },
  error = function(e) {
    debug_log(sprintf("FATAL ERROR: %s", e$message))
    debug_log(sprintf("Backtrace:\n%s", paste(e$call, collapse = "\n")))
    cat(sprintf("Error extracting with Kreuzberg: %s\n", e$message), file = stderr())
    quit(status = 1)
  })
}

main()
