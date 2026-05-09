```r title="R"
library(kreuzberg)

config <- list(
  chunking = list(
    max_characters = 1000L,
    overlap = 200L,
    embedding = list(
      model = list(type = "preset", name = "balanced"),
      batch_size = 16L,
      normalize = TRUE,
      show_download_progress = TRUE
    )
  )
)

json <- extract_file_sync("document.pdf", "application/pdf", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)
cat(sprintf("Chunks with embeddings: %d\n", length(result$chunks)))
```
