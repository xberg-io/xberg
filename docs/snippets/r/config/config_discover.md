```r title="R"
library(xberg)

# Load configuration from a JSON file and pass it to extract.
config_json <- paste(readLines("xberg.json"), collapse = "\n")
config <- ExtractionConfig$from_json(config_json)

input <- list(kind = "uri", uri = "document.pdf", mime_type = "application/pdf")
json <- extract(
  input = ExtractInput$from_json(jsonlite::toJSON(input, auto_unbox = TRUE)),
  config = config
)
output <- jsonlite::fromJSON(json, simplifyVector = FALSE)
result <- output$results[[1]]
cat(sprintf("Extracted %d characters\n", nchar(result$content)))
```
