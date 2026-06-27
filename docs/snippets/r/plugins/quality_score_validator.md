<!-- snippet:syntax-only -->

```r title="R"
library(xberg)

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
input <- list(kind = "uri", uri = "document.pdf", mime_type = "application/pdf")
json <- extract(
  input = ExtractInput$from_json(jsonlite::toJSON(input, auto_unbox = TRUE)),
  config = config
)
output <- jsonlite::fromJSON(json, simplifyVector = FALSE)
result <- output$results[[1]]
cat(sprintf("Validated extraction: %d characters\n", nchar(result$content)))
```
