```r title="R"
library(xberg)

# Configure Tesseract OCR
config <- list(
  force_ocr = TRUE,
  ocr = list(backend = "tesseract", language = "eng")
)

# Extract text from a scanned image
json <- extract_sync("scan.png", "image/png", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)

cat(sprintf("Extracted %d characters\n", nchar(result$content)))
cat("Content preview:\n")
cat(substr(result$content, 1, 200))
```
