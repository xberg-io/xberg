```r title="R"
library(xberg)

config <- list(enable_quality_processing = TRUE)

json <- extract_sync("document.pdf", "application/pdf", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)

cat("Quality Metrics:\n")
cat(sprintf("Quality Score: %.2f\n", result$quality_score))
cat(sprintf("Content Length: %d characters\n", nchar(result$content)))
cat(sprintf("Pages: %d\n", length(result$pages)))
```
