```r
library(xberg)

# Configure text chunking for RAG pipelines
config <- ExtractionConfig$from_json(jsonlite::toJSON(list(
  chunking = list(
    max_characters = 1000L,
    overlap = 200L
  )
), auto_unbox = TRUE))

input <- list(kind = "uri", uri = "large_document.pdf", mime_type = "application/pdf")
json <- extract(
  input = ExtractInput$from_json(jsonlite::toJSON(input, auto_unbox = TRUE)),
  config = config
)
output <- jsonlite::fromJSON(json, simplifyVector = FALSE)
result <- output$results[[1]]
cat("Number of chunks:", length(result$chunks), "\n")
```

```r title="R - Prepend Heading Context"
library(xberg)

# Prepend heading context to chunk content for structured documents
config <- ExtractionConfig$from_json(jsonlite::toJSON(list(
  chunking = list(
    chunker_type = "markdown",
    max_characters = 500L,
    overlap = 50L,
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
cat("Number of chunks:", length(result$chunks), "\n")
```
