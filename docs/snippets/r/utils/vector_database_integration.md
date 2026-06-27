```r title="R"
library(xberg)

document_id <- "doc-001"

config <- ExtractionConfig$from_json(jsonlite::toJSON(list(
  chunking = list(
    max_characters = 512L,
    overlap = 50L,
    embedding = list(
      model = list(type = "preset", name = "balanced"),
      normalize = TRUE,
      batch_size = 32L
    )
  )
), auto_unbox = TRUE))

input <- list(kind = "uri", uri = "document.pdf", mime_type = "application/pdf")
json <- extract(
  input = ExtractInput$from_json(jsonlite::toJSON(input, auto_unbox = TRUE)),
  config = config
)
output <- jsonlite::fromJSON(json, simplifyVector = FALSE)
result <- output$results[[1]]
# Each chunk has $content, $embedding, and $metadata. Pass these directly
# to a vector database client (pgvector, Qdrant, Pinecone, etc.) along with
# the document_id stored as a metadata field.
cat(sprintf("document_id: %s\n", document_id))
cat(sprintf("chunks ready for upsert: %d\n", length(result$chunks)))
```
