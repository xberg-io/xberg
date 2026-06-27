```r title="R"
library(xberg)

config <- list(
  result_format = "element_based",
  output_format = "markdown"
)

json <- extract_sync("document.pdf", "application/pdf", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)

cat(sprintf("Total elements: %d\n\n", length(result$elements)))

for (i in seq_along(result$elements)) {
  element <- result$elements[[i]]
  cat(sprintf("Element %d:\n", i))
  cat(sprintf("  Type: %s\n", element$element_type))
  cat(sprintf("  Content: %s\n\n", substr(element$content, 1, 100)))
}
```
