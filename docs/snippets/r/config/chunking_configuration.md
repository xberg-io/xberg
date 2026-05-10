```r
library(kreuzberg)

# Configure text chunking for RAG pipelines
config <- list(
  chunking = list(
    max_characters = 1000L,
    overlap = 200L
  )
)

json <- extract_file_sync("large_document.pdf", "application/pdf", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)
cat("Number of chunks:", length(result$chunks), "\n")
```

```r title="R - Prepend Heading Context"
library(kreuzberg)

# Prepend heading context to chunk content for structured documents
config <- list(
  chunking = list(
    chunker_type = "markdown",
    max_characters = 500L,
    overlap = 50L,
    prepend_heading_context = TRUE
  )
)

json <- extract_file_sync("document.md", "text/markdown", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)
cat("Number of chunks:", length(result$chunks), "\n")
```
