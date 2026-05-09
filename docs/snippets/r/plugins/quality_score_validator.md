<!-- snippet:syntax-only -->
```r title="R"
library(kreuzberg)

quality_score_validator <- function(result) {
  min_score <- 0.5
  score <- as.numeric(result$metadata$quality_score %||% 0)

  if (score < min_score) {
    return(list(
      valid = FALSE,
      message = sprintf(
        "Quality score too low: %.2f < %.2f",
        score, min_score
      )
    ))
  }
  return(list(valid = TRUE, message = "Quality score validation passed"))
}

register_validator("quality_score", quality_score_validator)

config <- ExtractionConfig$default()
json <- extract_file_sync("document.pdf", "application/pdf", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)

cat(sprintf("Validated extraction: %d characters\n", nchar(result$content)))
```
