```r title="R"
library(kreuzberg)

# Example 1: Basic character-based chunking
config <- list(
  chunking = list(max_characters = 1000L, overlap = 200L)
)

json <- extract_file_sync("document.pdf", "application/pdf", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)

num_chunks <- length(result$chunks)
cat(sprintf("Document split into %d chunks\n", num_chunks))
for (i in seq_len(min(3L, num_chunks))) {
  cat(sprintf("Chunk %d: %d characters\n", i, nchar(result$chunks[[i]])))
}
```

```r title="R - Markdown chunker with token-based sizing"
library(kreuzberg)

config <- list(
  chunking = list(
    chunker_type = "markdown",
    sizing = list(
      type = "tokenizer",
      model = "Xenova/gpt-4o"
    )
  )
)

json <- extract_file_sync("document.md", "text/markdown", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)
cat(sprintf("Markdown document split into %d chunks\n", length(result$chunks)))
```

```r title="R - Prepend heading context"
library(kreuzberg)

config <- list(
  chunking = list(
    chunker_type = "markdown",
    prepend_heading_context = TRUE
  )
)

json <- extract_file_sync("document.md", "text/markdown", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)
cat(sprintf("Document split into %d chunks with prepended headings\n", length(result$chunks)))
```
