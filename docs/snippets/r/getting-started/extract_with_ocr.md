```r title="R"
library(xberg)

# Configure OCR settings via a plain list mirroring the config JSON.
config <- ExtractionConfig$from_json(jsonlite::toJSON(list(
  force_ocr = TRUE,
  ocr = list(
    backend = "tesseract",
    language = "eng"
  )
), auto_unbox = TRUE))
# Extract an image file with OCR enabled
input <- list(kind = "uri", uri = "image.png", mime_type = "image/png")
json <- extract(
  input = ExtractInput$from_json(jsonlite::toJSON(input, auto_unbox = TRUE)),
  config = config
)
output <- jsonlite::fromJSON(json, simplifyVector = FALSE)
result <- output$results[[1]]
cat("Extracted text from image:\n")
cat(result$content)
```
