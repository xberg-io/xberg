```r title="R"
library(xberg)

content <- charToRaw("Hello, world!")

result <- tryCatch(
  {
    json <- extract_sync(
      content = content,
      mime_type = "application/x-nonexistent",
      config = ExtractionConfig$default()
    )
    jsonlite::fromJSON(json, simplifyVector = FALSE)
  },
  error = function(e) {
    message(sprintf("Extraction failed: %s", conditionMessage(e)))
    NULL
  }
)

if (is.null(result)) {
  cat("No content extracted; falling back to original bytes\n")
} else {
  cat(sprintf("Extracted %d characters\n", nchar(result$content)))
}
```
