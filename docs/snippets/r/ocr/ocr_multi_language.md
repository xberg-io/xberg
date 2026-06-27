```r title="R"
library(xberg)

# Configure multi-language OCR (English, French, German)
config <- ExtractionConfig$from_json(jsonlite::toJSON(list(
  force_ocr = TRUE,
  ocr = list(backend = "tesseract", language = "eng+fra+deu")
), auto_unbox = TRUE))
# Extract from a multilingual document
input <- list(kind = "uri", uri = "multilingual.png", mime_type = "image/png")
json <- extract(
  input = ExtractInput$from_json(jsonlite::toJSON(input, auto_unbox = TRUE)),
  config = config
)
output <- jsonlite::fromJSON(json, simplifyVector = FALSE)
result <- output$results[[1]]
cat(sprintf("Detected language: %s\n", result$detected_language))
cat(sprintf("Extracted %d characters\n", nchar(result$content)))
cat("Content preview:\n")
cat(substr(result$content, 1, 200))
```
