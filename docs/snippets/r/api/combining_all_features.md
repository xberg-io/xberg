```r title="R"
library(xberg)

config_json <- jsonlite::toJSON(list(
  output_format = "markdown",
  force_ocr = TRUE,
  extract_tables = TRUE,
  extract_metadata = TRUE,
  ocr = list(
    backend = "tesseract",
    language = "eng",
    dpi = 300L
  ),
  chunking = list(
    chunker_type = "markdown",
    max_characters = 1000L,
    overlap = 200L
  )
), auto_unbox = TRUE)

config <- ExtractionConfig$from_json(config_json)

json <- extract_sync(
  path = "scanned_report.pdf",
  mime_type = "application/pdf",
  config = config
)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)

cat(sprintf("Chunks: %d\n", length(result$chunks)))
cat(sprintf("Tables: %d\n", length(result$tables)))
title <- if (!is.null(result$metadata$title)) result$metadata$title else "<none>"
cat(sprintf("Title: %s\n", title))
```
