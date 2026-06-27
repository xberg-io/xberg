```r title="R"
library(xberg)

config <- ExtractionConfig$from_json(jsonlite::toJSON(list(
  include_document_structure = TRUE,
  output_format = "markdown"
), auto_unbox = TRUE))

input <- list(kind = "uri", uri = "document.pdf", mime_type = "application/pdf")
json <- extract(
  input = ExtractInput$from_json(jsonlite::toJSON(input, auto_unbox = TRUE)),
  config = config
)
output <- jsonlite::fromJSON(json, simplifyVector = FALSE)
result <- output$results[[1]]
cat(sprintf("Total pages: %d\n", length(result$pages)))
cat(sprintf("MIME type: %s\n\n", result$mime_type))

for (i in seq_along(result$pages)) {
  page <- result$pages[[i]]
  cat(sprintf("Page %d structure:\n", i))
  cat(sprintf("  Content: %s\n", substr(page$content, 1, 100)))
  cat("\n")
}
```
