```r title="R"
library(kreuzberg)

chunking_cfg <- chunking_config(max_characters = 1000L, overlap = 200L)
config <- extraction_config(chunking = chunking_cfg)

result <- extract_file_sync("document.pdf", "application/pdf", config)

cat(sprintf("Preparing %d chunks for embedding:\n", length(result$chunks)))

embeddings_data <- list()
for (i in seq_len(length(result$chunks))) {
  embeddings_data[[i]] <- list(
    chunk_id = i,
    text = result$chunks[[i]],
    length = nchar(result$chunks[[i]])
  )
}

cat(sprintf("Ready to embed %d chunks\n", length(embeddings_data)))
```
