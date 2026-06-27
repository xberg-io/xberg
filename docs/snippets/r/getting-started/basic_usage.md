```r title="R"
library(xberg)

config <- ExtractionConfig$default()

json <- extract_sync(
  path = "document.pdf",
  mime_type = NULL,
  config = config
)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)

cat(result$content)
cat(sprintf("\nMIME Type: %s\n", result$mime_type))
```
