```r title="R"
library(xberg)

input <- list(kind = "uri", uri = "document.pdf")

json <- extract(
  input = ExtractInput$from_json(jsonlite::toJSON(input, auto_unbox = TRUE)),
  config = ExtractionConfig$default()
)
output <- jsonlite::fromJSON(json, simplifyVector = FALSE)
result <- output$results[[1]]

cat(sprintf("Results: %d\n", output$summary$results))
cat(result$content)
```
