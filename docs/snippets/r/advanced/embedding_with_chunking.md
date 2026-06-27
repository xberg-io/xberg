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
cat(sprintf("Preparing %d chunks for embedding:\n", length(result$chunks)))

embeddings_data <- list()
for (i in seq_along(result$chunks)) {
  embeddings_data[[i]] <- list(
    chunk_id = i,
    text = result$chunks[[i]],
    length = nchar(result$chunks[[i]])
  )
}

cat(sprintf("Ready to embed %d chunks\n", length(embeddings_data)))
```
