```r title="R"
library(kreuzberg)

items <- jsonlite::toJSON(list(
  list(path = "doc1.pdf"),
  list(path = "doc2.docx"),
  list(path = "missing.html")
), auto_unbox = TRUE)

result <- tryCatch(
  {
    json <- batch_extract_files_sync(items = items, config = ExtractionConfig$default())
    jsonlite::fromJSON(json, simplifyVector = FALSE)
  },
  error = function(e) {
    message(sprintf("Batch extraction failed: %s", conditionMessage(e)))
    NULL
  }
)

if (is.null(result)) {
  cat("No results returned\n")
} else {
  for (i in seq_along(result)) {
    item <- result[[i]]
    err <- item$metadata$error
    if (!is.null(err)) {
      cat(sprintf("Document %d: ERROR - %s\n", i, err))
    } else {
      cat(sprintf("Document %d: %d chars, %d tables\n",
                  i, nchar(item$content), length(item$tables)))
    }
  }
}
```
