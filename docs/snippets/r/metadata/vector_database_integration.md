```r title="R"
library(xberg)

config <- ExtractionConfig$from_json(jsonlite::toJSON(list(
  chunking = list(max_characters = 1000L, overlap = 200L)
), auto_unbox = TRUE))

input <- list(kind = "uri", uri = "document.pdf", mime_type = "application/pdf")
json <- extract(
  input = ExtractInput$from_json(jsonlite::toJSON(input, auto_unbox = TRUE)),
  config = config
)
output <- jsonlite::fromJSON(json, simplifyVector = FALSE)
result <- output$results[[1]]
for (i in seq_len(min(3L, length(result$chunks)))) {
  chunk <- result$chunks[[i]]
  vector_doc <- list(
    id = sprintf("doc_%d", i),
    text = chunk,
    metadata = list(
      source = "document.pdf",
      chunk_index = i,
      length = nchar(chunk)
    )
  )
  cat(sprintf("Vector DB entry %d: %d chars\n", i, nchar(chunk)))
}
```
