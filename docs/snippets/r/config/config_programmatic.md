```r title="R"
library(kreuzberg)

config <- list(
  force_ocr = TRUE,
  ocr = list(
    backend = "tesseract",
    language = "eng"
  ),
  chunking = list(
    max_characters = 2000L,
    overlap = 300L
  ),
  output_format = "markdown"
)

json <- extract_file_sync("document.pdf", "application/pdf", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)
cat(sprintf("MIME type: %s\n", result$mime_type))
```
