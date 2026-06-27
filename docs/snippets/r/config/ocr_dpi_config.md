```r title="R"
library(xberg)

# Tesseract OCR via the xberg R bindings does not expose a DPI setting in
# the high-level config; PDF rasterization DPI is determined by the pipeline.
# This example demonstrates running Tesseract OCR end-to-end on a PDF.
config <- ExtractionConfig$from_json(jsonlite::toJSON(list(
  force_ocr = TRUE,
  ocr = list(backend = "tesseract", language = "eng")
), auto_unbox = TRUE))

input <- list(kind = "uri", uri = "document.pdf", mime_type = "application/pdf")
json <- extract(
  input = ExtractInput$from_json(jsonlite::toJSON(input, auto_unbox = TRUE)),
  config = config
)
output <- jsonlite::fromJSON(json, simplifyVector = FALSE)
result <- output$results[[1]]
cat(sprintf("Characters extracted: %d\n", nchar(result$content)))
```
