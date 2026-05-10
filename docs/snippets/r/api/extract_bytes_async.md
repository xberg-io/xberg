```r title="R"
library(kreuzberg)

# extract_bytes is the async variant; the call blocks the calling R thread
# until the underlying tokio task completes. Use future/promises if you need
# to fan out without blocking.
path <- "document.pdf"
content <- readBin(path, what = "raw", n = file.info(path)$size)

json <- extract_bytes(
  content = content,
  mime_type = "application/pdf",
  config = ExtractionConfig$default()
)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)

cat(sprintf("Extracted %d characters\n", nchar(result$content)))
```
