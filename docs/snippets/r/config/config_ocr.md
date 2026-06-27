```r title="R"
library(xberg)

config <- list(
  force_ocr = TRUE,
  ocr = list(backend = "tesseract", language = "eng")
)

json <- extract_sync("document.pdf", "application/pdf", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)

cat(sprintf("Extracted content length: %d\n", nchar(result$content)))
cat(sprintf("Detected language: %s\n", result$detected_language))
```
