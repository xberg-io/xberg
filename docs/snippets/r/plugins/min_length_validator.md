<!-- snippet:syntax-only -->
```r title="R"
library(kreuzberg)

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

config <- extraction_config()
result <- extract_file_sync("document.pdf", "application/pdf", config)

cat(sprintf("Content length: %d characters\n", nchar(result$content)))
```
