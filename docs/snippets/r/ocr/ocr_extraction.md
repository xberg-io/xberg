```r title="R"
library(xberg)

# Configure Tesseract OCR
config <- ExtractionConfig$from_json(jsonlite::toJSON(list(
  force_ocr = TRUE,
  ocr = list(backend = "tesseract", language = "eng")
), auto_unbox = TRUE))
# Extract text from a scanned image
input <- list(kind = "uri", uri = "scan.png", mime_type = "image/png")
json <- extract(
  input = ExtractInput$from_json(jsonlite::toJSON(input, auto_unbox = TRUE)),
  config = config
)
output <- jsonlite::fromJSON(json, simplifyVector = FALSE)
result <- output$results[[1]]
cat(sprintf("Extracted %d characters\n", nchar(result$content)))
cat("Content preview:\n")
cat(substr(result$content, 1, 200))
```
