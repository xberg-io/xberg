```r title="R"
library(xberg)

custom_ocr_backend <- function(image_path, language) {
  cat(sprintf("Processing image: %s\n", image_path))
  return(sprintf("Extracted text from %s", image_path))
}

register_ocr_backend("custom_cloud", custom_ocr_backend)

config <- ExtractionConfig$from_json(jsonlite::toJSON(list(
  force_ocr = TRUE,
  ocr = list(backend = "custom_cloud", language = "en")
), auto_unbox = TRUE))

input <- list(kind = "uri", uri = "document.pdf", mime_type = "application/pdf")
json <- extract(
  input = ExtractInput$from_json(jsonlite::toJSON(input, auto_unbox = TRUE)),
  config = config
)
output <- jsonlite::fromJSON(json, simplifyVector = FALSE)
result <- output$results[[1]]
cat(sprintf("Custom backend result: %d chars\n", nchar(result$content)))
```
