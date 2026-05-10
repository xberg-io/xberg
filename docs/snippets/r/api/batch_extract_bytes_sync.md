```r title="R"
library(kreuzberg)

paths <- c("report.pdf", "notes.txt")
mimes <- c("application/pdf", "text/plain")

items <- jsonlite::toJSON(lapply(seq_along(paths), function(i) {
  bytes <- readBin(paths[i], what = "raw", n = file.info(paths[i])$size)
  list(content = as.integer(bytes), mime_type = mimes[i])
}), auto_unbox = TRUE)

json <- batch_extract_bytes_sync(items = items, config = ExtractionConfig$default())
results <- jsonlite::fromJSON(json, simplifyVector = FALSE)

for (i in seq_along(results)) {
  cat(sprintf("[%d] mime=%s chars=%d\n",
              i, results[[i]]$mime_type, nchar(results[[i]]$content)))
}
```
