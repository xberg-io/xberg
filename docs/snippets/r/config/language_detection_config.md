```r title="R"
library(xberg)

config <- list(
  language_detection = list(enabled = TRUE)
)

json <- extract_sync("document.pdf", "application/pdf", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)

cat(sprintf("Detected language: %s\n", result$detected_language))
cat(sprintf("Content preview: %.60s...\n", result$content))
```
