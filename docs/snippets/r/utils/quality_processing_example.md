```r title="R"
library(xberg)

config <- ExtractionConfig$from_json(jsonlite::toJSON(list(enable_quality_processing = TRUE), auto_unbox = TRUE))
input <- list(kind = "uri", uri = "scanned_document.pdf", mime_type = "application/pdf")
json <- extract(
  input = ExtractInput$from_json(jsonlite::toJSON(input, auto_unbox = TRUE)),
  config = config
)
output <- jsonlite::fromJSON(json, simplifyVector = FALSE)
result <- output$results[[1]]
cat(sprintf("Content length: %d characters\n", nchar(result$content)))
if (!is.null(result$quality_score)) {
  cat(sprintf("Quality score: %.2f\n", result$quality_score))
  if (result$quality_score < 0.5) {
    cat("Warning: low quality extraction\n")
  }
}
```
