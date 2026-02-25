```r title="R"
library(kreuzberg)

custom_extractor <- function(path, mime_type) {
  content <- sprintf("Extracted from %s (%s)", path, mime_type)
  return(list(
    content = content,
    mime_type = mime_type,
    pages = 1L
  ))
}

register_document_extractor("custom_format", custom_extractor)

result <- extract_file_sync("custom_document.xyz", "application/custom", NULL)

cat(sprintf("Custom extractor result:\n"))
cat(sprintf("Content: %s\n", result$content))
cat(sprintf("Mime type: %s\n", result$mime_type))
```
