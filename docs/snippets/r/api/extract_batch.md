```r title="R"
library(xberg)

inputs <- list(
  ExtractInput$uri("document.pdf"),
  ExtractInput$bytes(charToRaw("Hello from memory"), "text/plain", "note.txt")
)

json <- extract_batch(inputs, ExtractionConfig$default())
output <- jsonlite::fromJSON(json, simplifyVector = FALSE)

for (result in output$results) {
  cat(result$content, "\n")
}
```
