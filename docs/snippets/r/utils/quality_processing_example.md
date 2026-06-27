```r title="R"
library(xberg)

config <- list(enable_quality_processing = TRUE)
json <- extract_sync("scanned_document.pdf", "application/pdf", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)

cat(sprintf("Content length: %d characters\n", nchar(result$content)))
if (!is.null(result$quality_score)) {
  cat(sprintf("Quality score: %.2f\n", result$quality_score))
  if (result$quality_score < 0.5) {
    cat("Warning: low quality extraction\n")
  }
}
```
