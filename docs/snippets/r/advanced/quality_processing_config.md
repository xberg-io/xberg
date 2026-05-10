```r title="R"
library(kreuzberg)

config <- list(enable_quality_processing = TRUE)

json <- extract_file_sync("document.pdf", "application/pdf", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)

cat(sprintf("Quality score: %.2f\n", result$quality_score))
```
