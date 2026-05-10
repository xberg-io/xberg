```r title="R"
library(kreuzberg)

config <- list(
  chunking = list(max_characters = 1000L, overlap = 200L)
)

json <- extract_file_sync("document.pdf", "application/pdf", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)

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
