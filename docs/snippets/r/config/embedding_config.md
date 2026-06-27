```r title="R"
library(xberg)

config <- ExtractionConfig$from_json(jsonlite::toJSON(list(
  chunking = list(
    max_characters = 1000L,
    overlap = 200L,
    embedding = list(
      model = list(type = "preset", name = "balanced"),
      batch_size = 16L,
      normalize = TRUE,
      show_download_progress = TRUE
    )
  )
), auto_unbox = TRUE))

input <- list(kind = "uri", uri = "document.pdf", mime_type = "application/pdf")
json <- extract(
  input = ExtractInput$from_json(jsonlite::toJSON(input, auto_unbox = TRUE)),
  config = config
)
output <- jsonlite::fromJSON(json, simplifyVector = FALSE)
result <- output$results[[1]]
cat(sprintf("Chunks with embeddings: %d\n", length(result$chunks)))
```
