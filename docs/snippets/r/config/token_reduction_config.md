```r title="R"
library(kreuzberg)

config <- extraction_config(
  token_reduction = list(enabled = TRUE)
)

result <- extract_file_sync("document.pdf", "application/pdf", config)

cat(sprintf("Original content length: %d characters\n", nchar(result$content)))
cat(sprintf("Content preview: %.60s...\n", result$content))
```
