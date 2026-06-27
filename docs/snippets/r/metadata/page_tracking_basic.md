```r title="R"
library(xberg)

result <- extract_sync("document.pdf")

cat("Total pages:", page_count(result), "\n\n")

for (i in seq_along(result$pages)) {
  page <- result$pages[[i]]
  cat(sprintf("Page %d:\n", i))
  cat("  Elements:", length(page$elements), "\n")
  cat("  Text content length:", nchar(page$content), "chars\n")

  if (nchar(page$content) > 0L) {
    preview <- substr(page$content, 1L, 100L)
    cat(sprintf("  Preview: %s...\n", preview))
  }
  cat("\n")
}
```
