```r title="R"
library(kreuzberg)

# Note: EasyOCR backend requires Python to be installed
config <- list(
  force_ocr = TRUE,
  ocr = list(backend = "easyocr", language = "en")
)

json <- extract_file_sync("document.pdf", "application/pdf", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)

cat("EasyOCR extraction:\n")
cat(sprintf("Content length: %d characters\n", nchar(result$content)))
cat(sprintf("Detected language: %s\n", result$detected_language))
```
