```r title="R"
library(xberg)

config <- ExtractionConfig$from_json(jsonlite::toJSON(list(force_ocr = TRUE), auto_unbox = TRUE))

input <- list(kind = "uri", uri = "multipage_document.pdf", mime_type = "application/pdf")
json <- extract(
  input = ExtractInput$from_json(jsonlite::toJSON(input, auto_unbox = TRUE)),
  config = config
)
output <- jsonlite::fromJSON(json, simplifyVector = FALSE)
result <- output$results[[1]]
cat(sprintf("Total pages: %d\n", length(result$pages)))
cat(sprintf("Content extracted via OCR: %d characters\n", nchar(result$content)))
cat(sprintf("Detected language: %s\n", result$detected_language))
```
