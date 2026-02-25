```r title="R"
library(kreuzberg)

config <- extraction_config(enable_quality_processing = TRUE)

result <- extract_file_sync("document.pdf", "application/pdf", config)

cat(sprintf("Quality score: %.2f\n", result$quality_score))
cat(sprintf("Content length: %d characters\n", nchar(result$content)))
```
