```r title="R"
library(kreuzberg)

config <- extraction_config(
  language_detection = list(enabled = TRUE)
)

result <- extract_file_sync("document.pdf", "application/pdf", config)

cat(sprintf("Detected language: %s\n", result$detected_language))
cat(sprintf("Content preview: %.60s...\n", result$content))
```
