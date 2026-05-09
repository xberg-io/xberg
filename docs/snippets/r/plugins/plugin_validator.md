```r title="R"
library(kreuzberg)

min_content_validator <- function(result) {
  min_length <- 100L
  if (nchar(result$content) < min_length) {
    return(list(
      valid = FALSE,
      message = sprintf("Content too short: %d < %d",
                       nchar(result$content), min_length)
    ))
  }
  return(list(valid = TRUE, message = "Content validation passed"))
}

register_validator("min_content", min_content_validator)

config <- ExtractionConfig$default()
json <- extract_file_sync("document.pdf", "application/pdf", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)

cat(sprintf("Content length: %d characters\n", nchar(result$content)))
```
