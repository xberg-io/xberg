```r title="R"
library(kreuzberg)

chunking_cfg <- chunking_config(max_characters = 1000L, overlap = 200L)
config <- extraction_config(chunking = chunking_cfg)

result <- extract_file_sync("document.pdf", "application/pdf", config)

cat(sprintf("Total chunks: %d\n", length(result$chunks)))
for (i in seq_len(min(5L, length(result$chunks)))) {
  cat(sprintf("Chunk %d: %d characters\n", i, nchar(result$chunks[[i]])))
}
```
