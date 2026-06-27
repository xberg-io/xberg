```r
library(xberg)

# Configure OCR with Tesseract
config <- ExtractionConfig$from_json(jsonlite::toJSON(list(
  force_ocr = TRUE,
  ocr = list(
    backend = "tesseract",
    language = "eng+deu"
  )
), auto_unbox = TRUE))

input <- list(kind = "uri", uri = "scanned_document.pdf", mime_type = "application/pdf")
json <- extract(
  input = ExtractInput$from_json(jsonlite::toJSON(input, auto_unbox = TRUE)),
  config = config
)
output <- jsonlite::fromJSON(json, simplifyVector = FALSE)
result <- output$results[[1]]
cat(result$content)
```
