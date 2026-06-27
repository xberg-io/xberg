```r title="R"
library(xberg)

# Configure PaddleOCR backend (defaults to mobile tier)
config <- list(
  force_ocr = TRUE,
  ocr = list(backend = "paddle-ocr", language = "en")
)

# Extract text from an image using PaddleOCR
json <- extract_sync("document.jpg", "image/jpeg", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)

cat(sprintf("Extracted %d characters\n", nchar(result$content)))
cat(sprintf("MIME type: %s\n", result$mime_type))
cat("Content preview:\n")
cat(substr(result$content, 1, 200))
```
