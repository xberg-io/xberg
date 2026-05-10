```r title="R"
library(kreuzberg)

# Load configuration from a JSON file and pass it to extract_file_sync.
config_json <- paste(readLines("kreuzberg.json"), collapse = "\n")
config <- ExtractionConfig$from_json(config_json)

json <- extract_file_sync("document.pdf", "application/pdf", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)
cat(sprintf("Extracted %d characters\n", nchar(result$content)))
```
