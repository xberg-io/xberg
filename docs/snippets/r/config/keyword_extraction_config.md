```r title="R"
library(kreuzberg)

config <- list(
  keywords = list(enabled = TRUE)
)

json <- extract_file_sync("document.pdf", "application/pdf", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)

cat(sprintf("Extracted %d keywords\n", length(result$keywords)))
if (length(result$keywords) > 0) {
  for (i in seq_len(min(5L, length(result$keywords)))) {
    cat(sprintf("  - %s\n", result$keywords[[i]]))
  }
}
```
