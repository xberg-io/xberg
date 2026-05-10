```r title="R"
library(kreuzberg)

path <- "document.pdf"
content <- readBin(path, what = "raw", n = file.info(path)$size)

json <- extract_bytes_sync(
  content = content,
  mime_type = "application/pdf",
  config = ExtractionConfig$default()
)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)

cat(sprintf("MIME type: %s\n", result$mime_type))
cat(sprintf("Content preview: %s\n", substr(result$content, 1, 200)))
```
