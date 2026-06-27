```r title="R"
library(xberg)

config <- ExtractionConfig$from_json(jsonlite::toJSON(list(
  keywords = list(
    algorithm = "yake",
    max_keywords = 10L,
    min_score = 0.3
  )
), auto_unbox = TRUE))

input <- list(kind = "uri", uri = "research_paper.pdf", mime_type = "application/pdf")
json <- extract(
  input = ExtractInput$from_json(jsonlite::toJSON(input, auto_unbox = TRUE)),
  config = config
)
output <- jsonlite::fromJSON(json, simplifyVector = FALSE)
result <- output$results[[1]]
cat(sprintf("Content length: %d characters\n", nchar(result$content)))
if (!is.null(result$extracted_keywords)) {
  for (kw in result$extracted_keywords) {
    cat(sprintf("  - %s\n", kw$text))
  }
}
```
