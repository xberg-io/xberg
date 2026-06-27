```r title="R"
library(xberg)

# Extract a document
input <- list(kind = "uri", uri = "document.docx")
json <- extract(
  input = ExtractInput$from_json(jsonlite::toJSON(input, auto_unbox = TRUE)),
  config = ExtractionConfig$default()
)
output <- jsonlite::fromJSON(json, simplifyVector = FALSE)
result <- output$results[[1]]

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
