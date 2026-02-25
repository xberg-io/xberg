```r title="R"
library(kreuzberg)

chunking_cfg <- chunking_config(max_characters = 1000L, overlap = 200L)
config <- extraction_config(chunking = chunking_cfg)

result <- extract_file_sync("document.pdf", "application/pdf", config)
num_chunks <- length(result$chunks)
cat(sprintf("Document split into %d chunks\n", num_chunks))
for (i in seq_len(min(3L, num_chunks))) {
  cat(sprintf("Chunk %d: %d characters\n", i, nchar(result$chunks[[i]])))
}
```
