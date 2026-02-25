```r title="R"
library(kreuzberg)

chunking_cfg <- chunking_config(max_characters = 800L, overlap = 150L)
config <- extraction_config(chunking = chunking_cfg)

result <- extract_file_sync("document.pdf", "application/pdf", config)

cat(sprintf("Total chunks: %d\n", length(result$chunks)))
cat(sprintf("Processing chunks for RAG pipeline:\n"))

for (i in seq_len(min(3L, length(result$chunks)))) {
  chunk <- result$chunks[[i]]
  cat(sprintf("Chunk %d: %d characters\n", i, nchar(chunk)))
}
```
