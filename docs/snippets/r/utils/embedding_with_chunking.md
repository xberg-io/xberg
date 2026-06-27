```r title="R"
library(xberg)

config <- list(
  chunking = list(
    max_characters = 1024L,
    overlap = 100L,
    embedding = list(
      model = list(type = "preset", name = "balanced"),
      normalize = TRUE,
      batch_size = 32L
    )
  )
)

json <- extract_sync("document.pdf", "application/pdf", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)

cat(sprintf("Chunks with embeddings: %d\n", length(result$chunks)))
```
