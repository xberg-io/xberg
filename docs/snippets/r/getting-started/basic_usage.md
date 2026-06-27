```r title="R"
library(xberg)

config <- ExtractionConfig$default()

input <- list(kind = "uri", uri = "document.pdf")
json <- extract(
  input = ExtractInput$from_json(jsonlite::toJSON(input, auto_unbox = TRUE)),
  config = config
)
output <- jsonlite::fromJSON(json, simplifyVector = FALSE)
result <- output$results[[1]]
cat(result$content)
cat(sprintf("\nMIME Type: %s\n", result$mime_type))
```
