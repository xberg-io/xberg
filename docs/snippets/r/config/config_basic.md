```r title="R"
library(xberg)

config <- list(
  output_format = "markdown"
)

json <- extract_sync("document.pdf", "application/pdf", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)

cat(sprintf("MIME type: %s\n", result$mime_type))
cat(sprintf("Content length: %d characters\n", nchar(result$content)))
cat("Content preview:\n")
cat(substr(result$content, 1, 200))
```
