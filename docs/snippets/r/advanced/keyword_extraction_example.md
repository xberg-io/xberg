```r title="R"
library(xberg)

config <- ExtractionConfig$from_json(jsonlite::toJSON(list(
  keywords = list(enabled = TRUE)
), auto_unbox = TRUE))

input <- list(kind = "uri", uri = "document.pdf", mime_type = "application/pdf")
json <- extract(
  input = ExtractInput$from_json(jsonlite::toJSON(input, auto_unbox = TRUE)),
  config = config
)
output <- jsonlite::fromJSON(json, simplifyVector = FALSE)
result <- output$results[[1]]
keywords <- result$extracted_keywords
if (is.null(keywords)) keywords <- list()
cat(sprintf("Keywords extracted: %d\n", length(keywords)))

if (length(keywords) > 0) {
  cat("Top keywords:\n")
  for (i in seq_len(min(10L, length(keywords)))) {
    cat(sprintf("  %d. %s\n", i, keywords[[i]]$text))
  }
}
```
