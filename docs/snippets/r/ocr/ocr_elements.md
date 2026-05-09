```r title="R"
library(kreuzberg)

# Enable structured OCR elements alongside text extraction
config <- list(
  ocr = list(
    backend = "paddleocr",
    language = "en",
    element_config = list(include_elements = TRUE)
  )
)

json <- extract_file_sync("scanned.pdf", "application/pdf", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)

if (!is.null(result$ocr_elements)) {
  for (element in result$ocr_elements) {
    cat(sprintf("Text: %s\n", element$text))
    cat(sprintf("Confidence: %.2f\n", element$confidence$recognition))
    cat("\n")
  }
}
```
