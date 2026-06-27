```r title="R"
library(xberg)

config <- ExtractionConfig$from_json(jsonlite::toJSON(list(
  chunking = list(max_characters = 1000L, overlap = 200L)
), auto_unbox = TRUE))

input <- list(kind = "uri", uri = "document.pdf", mime_type = "application/pdf")
json <- extract(
  input = ExtractInput$from_json(jsonlite::toJSON(input, auto_unbox = TRUE)),
  config = config
)
output <- jsonlite::fromJSON(json, simplifyVector = FALSE)
result <- output$results[[1]]
cat(sprintf("Chunks produced: %d\n", length(result$chunks)))
for (i in seq_len(min(3L, length(result$chunks)))) {
  cat(sprintf("Chunk %d length: %d characters\n", i, nchar(result$chunks[[i]])))
}
```

```r title="R - Prepend Heading Context"
library(xberg)

config <- ExtractionConfig$from_json(jsonlite::toJSON(list(
  chunking = list(
    max_characters = 500L,
    overlap = 50L,
    chunker_type = "markdown",
    prepend_heading_context = TRUE
  )
), auto_unbox = TRUE))

input <- list(kind = "uri", uri = "document.md", mime_type = "text/markdown")
json <- extract(
  input = ExtractInput$from_json(jsonlite::toJSON(input, auto_unbox = TRUE)),
  config = config
)
output <- jsonlite::fromJSON(json, simplifyVector = FALSE)
result <- output$results[[1]]
for (i in seq_len(min(3L, length(result$chunks)))) {
  chunk <- result$chunks[[i]]
  preview <- substr(chunk, 1L, min(100L, nchar(chunk)))
  cat(sprintf("%s\n", preview))
}
```
