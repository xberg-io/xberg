```r title="R"
library(xberg)

config <- ExtractionConfig$from_json(jsonlite::toJSON(list(
  output_format = "markdown"
), auto_unbox = TRUE))

input <- list(kind = "uri", uri = "document.pdf", mime_type = "application/pdf")
json <- extract(
  input = ExtractInput$from_json(jsonlite::toJSON(input, auto_unbox = TRUE)),
  config = config
)
output <- jsonlite::fromJSON(json, simplifyVector = FALSE)
result <- output$results[[1]]
cat(sprintf("MIME type: %s\n", result$mime_type))
cat(sprintf("Content length: %d characters\n", nchar(result$content)))
cat("Content preview:\n")
cat(substr(result$content, 1, 200))
```
