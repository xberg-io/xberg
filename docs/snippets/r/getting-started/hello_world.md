```r title="R"
library(xberg)

# Extract a PDF file
input <- list(kind = "uri", uri = "example.pdf")
json <- extract(
  input = ExtractInput$from_json(jsonlite::toJSON(input, auto_unbox = TRUE)),
  config = ExtractionConfig$default()
)
output <- jsonlite::fromJSON(json, simplifyVector = FALSE)
result <- output$results[[1]]

# Print a preview of the extracted content
content_preview <- substr(content(result), 1L, 200L)
cat("Content preview:\n")
cat(content_preview)
cat("\n...\n")
```
