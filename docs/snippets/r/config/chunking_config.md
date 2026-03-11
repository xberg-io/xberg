```r title="R"
library(kreuzberg)

# Example 1: Basic character-based chunking
chunking_cfg <- chunking_config(max_characters = 1000L, overlap = 200L)
config <- extraction_config(chunking = chunking_cfg)

result <- extract_file_sync("document.pdf", "application/pdf", config)
num_chunks <- length(result$chunks)
cat(sprintf("Document split into %d chunks\n", num_chunks))
for (i in seq_len(min(3L, num_chunks))) {
  cat(sprintf("Chunk %d: %d characters\n", i, nchar(result$chunks[[i]])))
}

# Example 2: Markdown chunker with token-based sizing and heading context
chunking_cfg2 <- chunking_config(
  chunker_type = "markdown",
  sizing = list(
    type = "tokenizer",
    model = "Xenova/gpt-4o"
  )
)
config2 <- extraction_config(chunking = chunking_cfg2)

result2 <- extract_file_sync("document.md", "text/markdown", config2)
num_chunks2 <- length(result2$chunks)
cat(sprintf("\nMarkdown document split into %d chunks\n", num_chunks2))

for (i in seq_len(min(3L, num_chunks2))) {
  chunk <- result2$chunks[[i]]
  cat(sprintf("\nChunk %d:\n", i))
  cat(sprintf("  Preview: %s...\n", substr(chunk$text, 1, 60)))

  # Access heading context
  if (!is.null(chunk$metadata$heading_context)) {
    headings <- chunk$metadata$heading_context$headings
    if (length(headings) > 0) {
      cat("  Headings in context:\n")
      for (h in headings) {
        cat(sprintf("    - Level %d: %s\n", h$level, h$text))
      }
    }
  }
}
```
