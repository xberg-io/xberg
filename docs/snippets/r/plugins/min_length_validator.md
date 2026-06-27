<!-- snippet:syntax-only -->

```r title="R"
library(xberg)

min_length_validator <- function(result) {
  min_length <- 50L
  if (nchar(result$content) < min_length) {
    return(list(
      valid = FALSE,
      message = sprintf(
        "Content too short: %d < %d characters",
        nchar(result$content), min_length
      )
    ))
  }
  return(list(valid = TRUE, message = "Content length validation passed"))
}

register_validator("min_length", min_length_validator)

config <- ExtractionConfig$default()
json <- extract_sync("document.pdf", "application/pdf", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)

cat(sprintf("Content length: %d characters\n", nchar(result$content)))
```
