```r title="R"
library(xberg)

inputs <- list(
  list(kind = "uri", uri = "document.pdf"),
  list(
    kind = "bytes",
    bytes = as.integer(charToRaw("Hello from memory")),
    mime_type = "text/plain",
    filename = "note.txt"
  )
)

json <- extract_batch(
  inputs = jsonlite::toJSON(inputs, auto_unbox = TRUE),
  config = ExtractionConfig$default()
)
output <- jsonlite::fromJSON(json, simplifyVector = FALSE)

cat(sprintf("Results: %d, errors: %d\n", output$summary$results, output$summary$errors))

for (result in output$results) {
  cat(result$content, "\n")
}
```
