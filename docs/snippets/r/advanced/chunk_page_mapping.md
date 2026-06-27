```r title="R"
library(xberg)

config <- ExtractionConfig$from_json(jsonlite::toJSON(list(
  chunking = list(max_characters = 500L, overlap = 50L),
  pages = list(extract_pages = TRUE)
), auto_unbox = TRUE))

input <- list(kind = "uri", uri = "document.pdf", mime_type = "application/pdf")
json <- extract(
  input = ExtractInput$from_json(jsonlite::toJSON(input, auto_unbox = TRUE)),
  config = config
)
output <- jsonlite::fromJSON(json, simplifyVector = FALSE)
result <- output$results[[1]]
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
