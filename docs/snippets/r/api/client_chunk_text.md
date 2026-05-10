<!-- snippet:syntax-only -->
```r title="R"
library(kreuzberg)
library(httr2)

payload <- list(
  text = "Your long text content here...",
  chunker_type = "text",
  config = list(
    max_characters = 1000,
    overlap = 50,
    trim = TRUE
  )
)

response <- request("http://localhost:8000/chunk") |>
  req_method("POST") |>
  req_body_json(payload) |>
  req_perform()

result <- resp_body_json(response)

cat(sprintf("Created %d chunks\n", result$chunk_count))
for (chunk in result$chunks) {
  preview <- substr(chunk$content, 1, 50)
  cat(sprintf("Chunk %d: %s...\n", chunk$chunk_index, preview))
}
```
