```r title="R"
library(xberg)

# Tesseract OCR via the xberg R bindings does not expose a DPI setting in
# the high-level config; PDF rasterization DPI is determined by the pipeline.
# This example demonstrates running Tesseract OCR end-to-end on a PDF.
config <- list(
  force_ocr = TRUE,
  ocr = list(backend = "tesseract", language = "eng")
)

json <- extract_sync("document.pdf", "application/pdf", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)

cat(sprintf("Characters extracted: %d\n", nchar(result$content)))
```
