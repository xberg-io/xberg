```r title="R"
library(xberg)

config <- ExtractionConfig$from_json(jsonlite::toJSON(list(
  chunking = list(
    max_characters = 500L,
    overlap = 50L,
    embedding = list(
      model = list(type = "preset", name = "balanced"),
      normalize = TRUE
    )
  )
), auto_unbox = TRUE))

input <- list(kind = "uri", uri = "research_paper.pdf", mime_type = "application/pdf")
json <- extract(
  input = ExtractInput$from_json(jsonlite::toJSON(input, auto_unbox = TRUE)),
  config = config
)
output <- jsonlite::fromJSON(json, simplifyVector = FALSE)
result <- output$results[[1]]
for (i in seq_along(result$chunks)) {
  chunk <- result$chunks[[i]]
  cat(sprintf("Chunk %d/%d\n", i, length(result$chunks)))
  if (!is.null(chunk$embedding)) {
    cat(sprintf("  Embedding: %d dimensions\n", length(chunk$embedding)))
  }
}
```
