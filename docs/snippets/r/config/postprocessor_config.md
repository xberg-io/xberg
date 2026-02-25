```r title="R"
library(kreuzberg)

config <- extraction_config(
  postprocessor = list(enabled = TRUE)
)

result <- extract_file_sync("document.pdf", "application/pdf", config)

cat(sprintf("Content length: %d characters\n", nchar(result$content)))
cat(sprintf("Mime type: %s\n", result$mime_type))
```
