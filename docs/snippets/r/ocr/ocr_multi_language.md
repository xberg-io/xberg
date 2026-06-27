```r title="R"
library(xberg)

# Configure multi-language OCR (English, French, German)
config <- list(
  force_ocr = TRUE,
  ocr = list(backend = "tesseract", language = "eng+fra+deu")
)

# Extract from a multilingual document
json <- extract_sync("multilingual.png", "image/png", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)

cat(sprintf("Detected language: %s\n", result$detected_language))
cat(sprintf("Extracted %d characters\n", nchar(result$content)))
cat("Content preview:\n")
cat(substr(result$content, 1, 200))
```
