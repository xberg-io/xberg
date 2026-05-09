```r title="R"
library(kreuzberg)

config <- list(
  include_document_structure = TRUE,
  output_format = "markdown"
)

json <- extract_file_sync("document.pdf", "application/pdf", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)

cat(sprintf("Total pages: %d\n", length(result$pages)))
cat(sprintf("MIME type: %s\n\n", result$mime_type))

for (i in seq_along(result$pages)) {
  page <- result$pages[[i]]
  cat(sprintf("Page %d structure:\n", i))
  cat(sprintf("  Content: %s\n", substr(page$content, 1, 100)))
  cat("\n")
}
```
