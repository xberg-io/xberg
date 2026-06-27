```r title="R"
library(xberg)

config <- ExtractionConfig$from_json(jsonlite::toJSON(list(
  ocr = list(backend = "tesseract", language = "eng"),
  chunking = list(max_characters = 1500L, overlap = 300L),
  output_format = "markdown",
  include_document_structure = TRUE,
  force_ocr = TRUE
), auto_unbox = TRUE))

input <- list(kind = "uri", uri = "document.pdf", mime_type = "application/pdf")
json <- extract(
  input = ExtractInput$from_json(jsonlite::toJSON(input, auto_unbox = TRUE)),
  config = config
)
output <- jsonlite::fromJSON(json, simplifyVector = FALSE)
result <- output$results[[1]]
cat(sprintf("Format: %s\n", result$mime_type))
cat(sprintf("Chunks: %d\n", length(result$chunks)))
cat(sprintf("Content preview: %.50s...\n", result$content))
```
