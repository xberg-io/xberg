```r title="R"
library(kreuzberg)

config <- list(
  ocr = list(backend = "tesseract", language = "eng"),
  chunking = list(max_characters = 1500L, overlap = 300L),
  output_format = "markdown",
  include_document_structure = TRUE,
  force_ocr = TRUE
)

json <- extract_file_sync("document.pdf", "application/pdf", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)

cat(sprintf("Format: %s\n", result$mime_type))
cat(sprintf("Chunks: %d\n", length(result$chunks)))
cat(sprintf("Content preview: %.50s...\n", result$content))
```
