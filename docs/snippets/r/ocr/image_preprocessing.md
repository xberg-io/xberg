```r title="R"
library(xberg)

config <- ExtractionConfig$from_json(jsonlite::toJSON(list(
  force_ocr = TRUE,
  ocr = list(backend = "tesseract", language = "eng"),
  enable_quality_processing = TRUE
), auto_unbox = TRUE))

input <- list(kind = "uri", uri = "scan.png", mime_type = "image/png")
json <- extract(
  input = ExtractInput$from_json(jsonlite::toJSON(input, auto_unbox = TRUE)),
  config = config
)
output <- jsonlite::fromJSON(json, simplifyVector = FALSE)
result <- output$results[[1]]
cat(sprintf("Quality: %.2f, Length: %d\n",
            result$quality_score %||% 0,
            nchar(result$content)))
```
