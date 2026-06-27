```r title="R"
library(xberg)

config <- list(
  chunking = list(max_characters = 1000L, overlap = 200L)
)

json <- extract_sync("document.pdf", "application/pdf", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)

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
