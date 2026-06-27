```r title="R"
library(xberg)

# Configure PaddleOCR backend (defaults to mobile tier)
config <- ExtractionConfig$from_json(jsonlite::toJSON(list(
  force_ocr = TRUE,
  ocr = list(backend = "paddle-ocr", language = "en")
), auto_unbox = TRUE))
# Extract text from an image using PaddleOCR
input <- list(kind = "uri", uri = "document.jpg", mime_type = "image/jpeg")
json <- extract(
  input = ExtractInput$from_json(jsonlite::toJSON(input, auto_unbox = TRUE)),
  config = config
)
output <- jsonlite::fromJSON(json, simplifyVector = FALSE)
result <- output$results[[1]]
cat(sprintf("Extracted %d characters\n", nchar(result$content)))
cat(sprintf("MIME type: %s\n", result$mime_type))
cat("Content preview:\n")
cat(substr(result$content, 1, 200))
```
