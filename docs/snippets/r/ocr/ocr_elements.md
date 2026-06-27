```r title="R"
library(xberg)

# Enable structured OCR elements alongside text extraction
config <- ExtractionConfig$from_json(jsonlite::toJSON(list(
  ocr = list(
    backend = "paddleocr",
    language = "en",
    element_config = list(include_elements = TRUE)
  )
), auto_unbox = TRUE))

input <- list(kind = "uri", uri = "scanned.pdf", mime_type = "application/pdf")
json <- extract(
  input = ExtractInput$from_json(jsonlite::toJSON(input, auto_unbox = TRUE)),
  config = config
)
output <- jsonlite::fromJSON(json, simplifyVector = FALSE)
result <- output$results[[1]]
if (!is.null(result$ocr_elements)) {
  for (element in result$ocr_elements) {
    cat(sprintf("Text: %s\n", element$text))
    cat(sprintf("Confidence: %.2f\n", element$confidence$recognition))
    cat("\n")
  }
}
```
