```r title="R"
library(xberg)

config <- list(
  chunking = list(max_characters = 500L, overlap = 50L),
  pages = list(extract_pages = TRUE)
)

json <- extract_sync("document.pdf", "application/pdf", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)

for (i in seq_along(result$chunks)) {
  chunk <- result$chunks[[i]]
  metadata <- result$chunk_metadata[[i]]

  if (!is.null(metadata$first_page) && !is.null(metadata$last_page)) {
    page_range <- if (metadata$first_page == metadata$last_page) {
      sprintf("Page %d", metadata$first_page)
    } else {
      sprintf("Pages %d-%d", metadata$first_page, metadata$last_page)
    }

    preview <- substr(chunk, 1L, min(50L, nchar(chunk)))
    cat(sprintf("Chunk: %s... (%s)\n", preview, page_range))
  }
}
```
