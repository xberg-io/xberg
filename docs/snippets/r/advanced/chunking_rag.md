```r title="R"
library(xberg)

config <- list(
  chunking = list(max_characters = 800L, overlap = 150L)
)

json <- extract_sync("document.pdf", "application/pdf", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)

cat(sprintf("Total chunks: %d\n", length(result$chunks)))
cat("Processing chunks for RAG pipeline:\n")

for (i in seq_len(min(3L, length(result$chunks)))) {
  chunk <- result$chunks[[i]]
  cat(sprintf("Chunk %d: %d characters\n", i, nchar(chunk)))
}
```
