```r title="R"
library(xberg)

# Example 1: Basic character-based chunking
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
num_chunks <- length(result$chunks)
cat(sprintf("Document split into %d chunks\n", num_chunks))
for (i in seq_len(min(3L, num_chunks))) {
  cat(sprintf("Chunk %d: %d characters\n", i, nchar(result$chunks[[i]])))
}
```

```r title="R - Markdown chunker with token-based sizing"
library(xberg)

config <- ExtractionConfig$from_json(jsonlite::toJSON(list(
  chunking = list(
    chunker_type = "markdown",
    sizing = list(
      type = "tokenizer",
      model = "Xenova/gpt-4o"
    )
  )
), auto_unbox = TRUE))

input <- list(kind = "uri", uri = "document.md", mime_type = "text/markdown")
json <- extract(
  input = ExtractInput$from_json(jsonlite::toJSON(input, auto_unbox = TRUE)),
  config = config
)
output <- jsonlite::fromJSON(json, simplifyVector = FALSE)
result <- output$results[[1]]
cat(sprintf("Markdown document split into %d chunks\n", length(result$chunks)))
```

```r title="R - Prepend heading context"
library(xberg)

config <- ExtractionConfig$from_json(jsonlite::toJSON(list(
  chunking = list(
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
cat(sprintf("Document split into %d chunks with prepended headings\n", length(result$chunks)))
```
