```r title="R"
library(xberg)

config <- list(force_ocr = TRUE)

json <- extract_sync("multipage_document.pdf", "application/pdf", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)

cat(sprintf("Total pages: %d\n", length(result$pages)))
cat(sprintf("Content extracted via OCR: %d characters\n", nchar(result$content)))
cat(sprintf("Detected language: %s\n", result$detected_language))
```
