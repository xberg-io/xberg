```r title="R"
library(kreuzberg)

config <- extraction_config(enable_quality_processing = TRUE)

result <- extract_file_sync("document.pdf", "application/pdf", config)

cat(sprintf("Quality Metrics:\n"))
cat(sprintf("Quality Score: %.2f\n", result$quality_score))
cat(sprintf("Content Length: %d characters\n", nchar(result$content)))
cat(sprintf("Pages: %d\n", result$pages))
```
