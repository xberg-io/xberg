```r title="R"
library(xberg)

config <- ExtractionConfig$from_json(jsonlite::toJSON(list(
  token_reduction = list(
    mode = "moderate",
    preserve_important_words = TRUE
  )
), auto_unbox = TRUE))

input <- list(kind = "uri", uri = "verbose_document.pdf", mime_type = "application/pdf")
json <- extract(
  input = ExtractInput$from_json(jsonlite::toJSON(input, auto_unbox = TRUE)),
  config = config
)
output <- jsonlite::fromJSON(json, simplifyVector = FALSE)
result <- output$results[[1]]
cat(sprintf("Reduced content length: %d characters\n", nchar(result$content)))
cat(sprintf("MIME type: %s\n", result$mime_type))
```
