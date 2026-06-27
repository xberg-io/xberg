```r title="R"
library(xberg)

config <- ExtractionConfig$from_json(jsonlite::toJSON(list(
  token_reduction = list(enabled = TRUE)
), auto_unbox = TRUE))

input <- list(kind = "uri", uri = "document.pdf", mime_type = "application/pdf")
json <- extract(
  input = ExtractInput$from_json(jsonlite::toJSON(input, auto_unbox = TRUE)),
  config = config
)
output <- jsonlite::fromJSON(json, simplifyVector = FALSE)
result <- output$results[[1]]
cat("Token-reduced content:\n")
cat(sprintf("Length: %d characters\n", nchar(result$content)))
cat(sprintf("Preview: %.60s...\n", result$content))
```
