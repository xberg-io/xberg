```r title="R"
library(xberg)

# Extract a document
result <- extract_sync("document.docx")

# Access core content fields
cat(sprintf("MIME type: %s\n", mime_type(result)))
cat(sprintf("Content length: %d characters\n", nchar(content(result))))

# Access structured data
cat(sprintf("Number of tables: %d\n", length(result$tables)))
cat(sprintf("Detected language: %s\n", detected_language(result)))

# Access metadata
author <- metadata_field(result, "author")
if (!is.null(author)) {
  cat(sprintf("Document author: %s\n", author))
}
```
