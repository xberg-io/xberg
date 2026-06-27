```r title="R"
library(xberg)

config <- ExtractionConfig$from_json(jsonlite::toJSON(list(
  chunking = list(max_characters = 800L, overlap = 150L)
), auto_unbox = TRUE))

input <- list(kind = "uri", uri = "document.pdf", mime_type = "application/pdf")
json <- extract(
  input = ExtractInput$from_json(jsonlite::toJSON(input, auto_unbox = TRUE)),
  config = config
)
output <- jsonlite::fromJSON(json, simplifyVector = FALSE)
result <- output$results[[1]]
cat(sprintf("Total chunks: %d\n", length(result$chunks)))
cat("Processing chunks for RAG pipeline:\n")

for (i in seq_len(min(3L, length(result$chunks)))) {
  chunk <- result$chunks[[i]]
  cat(sprintf("Chunk %d: %d characters\n", i, nchar(chunk)))
}
```
