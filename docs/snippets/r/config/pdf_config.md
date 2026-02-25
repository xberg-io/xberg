```r title="R"
library(kreuzberg)

config <- extraction_config(
  pdf_options = list(extract_tables = TRUE)
)

result <- extract_file_sync("document.pdf", "application/pdf", config)

cat(sprintf("Tables extracted: %d\n", length(result$tables)))
cat(sprintf("Total elements: %d\n", length(result$elements)))
cat(sprintf("Content preview: %.50s...\n", result$content))
```
