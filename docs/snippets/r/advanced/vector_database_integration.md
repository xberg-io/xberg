```r title="R"
library(kreuzberg)

chunking_cfg <- chunking_config(max_characters = 1000L, overlap = 200L)
config <- extraction_config(chunking = chunking_cfg)

result <- extract_file_sync("document.pdf", "application/pdf", config)

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
