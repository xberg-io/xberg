```r title="R"
library(xberg)

config <- list(
  chunking = list(
    max_characters = 500L,
    overlap = 50L,
    embedding = list(
      model = list(type = "preset", name = "balanced"),
      normalize = TRUE
    )
  )
)

json <- extract_sync("research_paper.pdf", "application/pdf", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)

for (i in seq_along(result$chunks)) {
  chunk <- result$chunks[[i]]
  cat(sprintf("Chunk %d/%d\n", i, length(result$chunks)))
  if (!is.null(chunk$embedding)) {
    cat(sprintf("  Embedding: %d dimensions\n", length(chunk$embedding)))
  }
}
```
