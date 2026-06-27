<!-- snippet:syntax-only -->

```r title="R"
library(xberg)

custom_json_extractor <- function(path, mime_type) {
  raw <- readLines(path, warn = FALSE)
  parsed <- jsonlite::fromJSON(paste(raw, collapse = "\n"))

  text <- paste(unlist(parsed), collapse = "\n")

  return(list(
    content = text,
    mime_type = "application/json",
    pages = 1L,
    metadata = list(extractor = "custom-json-extractor")
  ))
}

register_document_extractor("custom-json-extractor", custom_json_extractor)

input <- list(kind = "uri", uri = "data.json", mime_type = "application/json")
json <- extract(
  input = ExtractInput$from_json(jsonlite::toJSON(input, auto_unbox = TRUE)),
  config = ExtractionConfig$default()
)
output <- jsonlite::fromJSON(json, simplifyVector = FALSE)
result <- output$results[[1]]

cat(sprintf("Extracted %d characters from JSON\n", nchar(result$content)))
```
