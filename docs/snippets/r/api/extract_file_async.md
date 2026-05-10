```r title="R"
library(kreuzberg)

# extract_file is the async variant; extendr drives the tokio runtime so the
# call returns once extraction completes. R has no native async, so wrap with
# the future/promises packages if non-blocking dispatch is required.
json <- extract_file(
  path = "document.pdf",
  mime_type = "application/pdf",
  config = ExtractionConfig$default()
)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)

cat(sprintf("Extracted %d characters from %s\n", nchar(result$content), result$mime_type))
```
