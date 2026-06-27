```r title="R"
library(xberg)

config <- ExtractionConfig$from_json(jsonlite::toJSON(list(
  result_format = "element_based",
  output_format = "markdown"
), auto_unbox = TRUE))

input <- list(kind = "uri", uri = "document.pdf", mime_type = "application/pdf")
json <- extract(
  input = ExtractInput$from_json(jsonlite::toJSON(input, auto_unbox = TRUE)),
  config = config
)
output <- jsonlite::fromJSON(json, simplifyVector = FALSE)
result <- output$results[[1]]
cat(sprintf("Total elements: %d\n\n", length(result$elements)))

for (i in seq_along(result$elements)) {
  element <- result$elements[[i]]
  cat(sprintf("Element %d:\n", i))
  cat(sprintf("  Type: %s\n", element$element_type))
  cat(sprintf("  Content: %s\n\n", substr(element$content, 1, 100)))
}
```
