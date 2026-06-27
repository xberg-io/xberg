```r title="R"
library(xberg)

result <- extract_sync("document.pdf")

boundaries <- result$metadata$pages$boundaries

if (!is.null(boundaries) && length(boundaries) > 0L) {
  content_bytes <- charToRaw(result$content)

  for (i in seq_len(min(3L, length(boundaries)))) {
    boundary <- boundaries[[i]]
    page_bytes <- content_bytes[(boundary$byte_start + 1L):boundary$byte_end]
    page_text <- rawToChar(page_bytes)
    preview_end <- min(100L, nchar(page_text))

    cat(sprintf("Page %d:\n", boundary$page_number))
    cat(sprintf("  Byte range: %d-%d\n", boundary$byte_start, boundary$byte_end))
    cat(sprintf("  Preview: %s...\n", substr(page_text, 1L, preview_end)))
  }
}
```
