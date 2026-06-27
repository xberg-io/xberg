```r title="R"
library(xberg)

input <- list(kind = "uri", uri = "document.pdf")
json <- extract(
  input = ExtractInput$from_json(jsonlite::toJSON(input, auto_unbox = TRUE)),
  config = ExtractionConfig$default()
)
output <- jsonlite::fromJSON(json, simplifyVector = FALSE)
result <- output$results[[1]]

cat("Detected Language:", result$detected_language, "\n")
cat("Quality Score:", result$quality_score, "\n")
cat("Keywords:", paste(result$keywords, collapse=", "), "\n\n")

cat("Metadata fields:\n")
authors <- metadata_field(result, "authors")
if (!is.null(authors)) {
  cat("Authors:", paste(authors, collapse=", "), "\n")
}

created <- metadata_field(result, "created_date")
if (!is.null(created)) {
  cat("Created Date:", created, "\n")
}

pages_meta <- metadata_field(result, "page_count")
if (!is.null(pages_meta)) {
  cat("Pages:", pages_meta, "\n")
}
```
