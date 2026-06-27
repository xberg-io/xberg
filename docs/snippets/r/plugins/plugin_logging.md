<!-- snippet:syntax-only -->

```r title="R"
library(xberg)

logging_processor <- function(result) {
  message(sprintf(
    "[plugin] processing mime=%s content_chars=%d",
    result$mime_type %||% "unknown", nchar(result$content)
  ))
  return(result)
}

logging_validator <- function(result) {
  message(sprintf(
    "[plugin] validating mime=%s",
    result$mime_type %||% "unknown"
  ))
  return(list(valid = TRUE, message = "ok"))
}

register_post_processor("logging_processor", logging_processor)
register_validator("logging_validator", logging_validator)

config <- ExtractionConfig$from_json(jsonlite::toJSON(list(postprocessor = list(enabled = TRUE)), auto_unbox = TRUE))
input <- list(kind = "uri", uri = "document.pdf", mime_type = "application/pdf")
json <- extract(
  input = ExtractInput$from_json(jsonlite::toJSON(input, auto_unbox = TRUE)),
  config = config
)
output <- jsonlite::fromJSON(json, simplifyVector = FALSE)
result <- output$results[[1]]
cat(sprintf("Done: %d characters\n", nchar(result$content)))
```
