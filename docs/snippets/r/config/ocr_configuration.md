```r
library(xberg)

# Configure OCR with Tesseract
config <- list(
  force_ocr = TRUE,
  ocr = list(
    backend = "tesseract",
    language = "eng+deu"
  )
)

json <- extract_sync("scanned_document.pdf", "application/pdf", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)
cat(result$content)
```
