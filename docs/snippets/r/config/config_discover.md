```r title="R"
library(xberg)

# Load configuration from a JSON file and pass it to extract_sync.
config_json <- paste(readLines("xberg.json"), collapse = "\n")
config <- ExtractionConfig$from_json(config_json)

json <- extract_sync("document.pdf", "application/pdf", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)
cat(sprintf("Extracted %d characters\n", nchar(result$content)))
```
