```r title="R"
library(xberg)

config <- list(
  chunking = list(max_characters = 1000L, overlap = 200L)
)

json <- extract_sync("document.pdf", "application/pdf", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)

cat(sprintf("Chunks produced: %d\n", length(result$chunks)))
for (i in seq_len(min(3L, length(result$chunks)))) {
  cat(sprintf("Chunk %d length: %d characters\n", i, nchar(result$chunks[[i]])))
}
```

```r title="R - Prepend Heading Context"
library(xberg)

config <- list(
  chunking = list(
    max_characters = 500L,
    overlap = 50L,
    chunker_type = "markdown",
    prepend_heading_context = TRUE
  )
)

json <- extract_sync("document.md", "text/markdown", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)

for (i in seq_len(min(3L, length(result$chunks)))) {
  chunk <- result$chunks[[i]]
  preview <- substr(chunk, 1L, min(100L, nchar(chunk)))
  cat(sprintf("%s\n", preview))
}
```
