```r title="R"
library(xberg)

config <- ExtractionConfig$from_json(jsonlite::toJSON(list(
  language_detection = list(
    enabled = TRUE,
    min_confidence = 0.8,
    detect_multiple = FALSE
  )
), auto_unbox = TRUE))

input <- list(kind = "uri", uri = "document.pdf", mime_type = "application/pdf")
json <- extract(
  input = ExtractInput$from_json(jsonlite::toJSON(input, auto_unbox = TRUE)),
  config = config
)
output <- jsonlite::fromJSON(json, simplifyVector = FALSE)
result <- output$results[[1]]
if (length(result$detected_languages) > 0) {
  cat(sprintf("Detected language: %s\n", result$detected_languages[[1]]))
} else {
  cat("No language detected\n")
}

cat(sprintf("Content length: %d characters\n", nchar(result$content)))
```
