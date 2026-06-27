```r title="R"
library(xberg)

content <- charToRaw("Hello, world!")

result <- tryCatch(
  {
    input <- list(kind = "bytes", bytes = as.integer(content), mime_type = "application/x-nonexistent")
    json <- extract(
      input = ExtractInput$from_json(jsonlite::toJSON(input, auto_unbox = TRUE)),
      config = ExtractionConfig$default()
    )
    output <- jsonlite::fromJSON(json, simplifyVector = FALSE)
    output$results[[1]]
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
