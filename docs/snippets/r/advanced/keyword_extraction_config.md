```r title="R"
library(xberg)

config <- ExtractionConfig$from_json(jsonlite::toJSON(list(
  keywords = list(
    algorithm = "yake",
    max_keywords = 10L,
    min_score = 0.3,
    language = "en"
  )
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
```
