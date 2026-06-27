```r title="R"
library(xberg)

config <- list(
  postprocessor = list(enabled = TRUE)
)

json <- extract_sync("document.pdf", "application/pdf", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)

cat(sprintf("Content length: %d characters\n", nchar(result$content)))
cat(sprintf("Mime type: %s\n", result$mime_type))
```
