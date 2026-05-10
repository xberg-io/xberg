```r title="R"
library(kreuzberg)

config <- list(
  force_ocr = TRUE,
  ocr = list(backend = "tesseract", language = "eng")
)

json <- extract_file_sync("scan.png", "image/png", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)

cat("Image extraction via OCR:\n")
cat(sprintf("Content length: %d characters\n", nchar(result$content)))
cat(sprintf("Mime type: %s\n", result$mime_type))
cat(sprintf("Detected language: %s\n", result$detected_language))
```
