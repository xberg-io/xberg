```r title="R"
library(xberg)

config <- list(
  chunking = list(max_characters = 1000L, overlap = 200L)
)

json <- extract_sync("document.pdf", "application/pdf", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)

cat(sprintf("Total chunks: %d\n", length(result$chunks)))
for (i in seq_len(min(5L, length(result$chunks)))) {
  cat(sprintf("Chunk %d: %d characters\n", i, nchar(result$chunks[[i]])))
}
```
