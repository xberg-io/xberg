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

config <- list(postprocessor = list(enabled = TRUE))
json <- extract_sync("document.pdf", "application/pdf", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)

cat(sprintf("Done: %d characters\n", nchar(result$content)))
```
