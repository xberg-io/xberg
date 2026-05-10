```r title="R"
library(kreuzberg)

json <- extract_file_sync(
  path = "document.pdf",
  mime_type = "application/pdf",
  config = ExtractionConfig$default()
)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)

cat(sprintf("MIME type: %s\n", result$mime_type))
cat(sprintf("Content length: %d characters\n", nchar(result$content)))
```
