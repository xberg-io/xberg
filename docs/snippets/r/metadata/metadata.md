```r title="R"
library(xberg)

result <- extract_sync("document.pdf")

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
