```r title="R"
library(kreuzberg)

config <- list(
  keywords = list(enabled = TRUE)
)

json <- extract_file_sync("document.pdf", "application/pdf", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)

cat(sprintf("Keywords extracted: %d\n", length(result$keywords)))

if (length(result$keywords) > 0) {
  cat("Top keywords:\n")
  for (i in seq_len(min(10L, length(result$keywords)))) {
    cat(sprintf("  %d. %s\n", i, result$keywords[[i]]))
  }
}
```
