```r title="R"
library(xberg)

config <- list(
  pdf_options = list(extract_images = TRUE, extract_metadata = TRUE)
)

json <- extract_sync("document.pdf", "application/pdf", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)

cat(sprintf("Tables extracted: %d\n", length(result$tables)))
cat(sprintf("Content preview: %.50s...\n", result$content))
```
