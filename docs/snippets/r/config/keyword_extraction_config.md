```r title="R"
library(kreuzberg)

config <- extraction_config(
  keywords = list(enabled = TRUE)
)

result <- extract_file_sync("document.pdf", "application/pdf", config)

cat(sprintf("Extracted %d keywords\n", length(result$keywords)))
if (length(result$keywords) > 0) {
  for (i in seq_len(min(5L, length(result$keywords)))) {
    cat(sprintf("  - %s\n", result$keywords[[i]]))
  }
}
```
