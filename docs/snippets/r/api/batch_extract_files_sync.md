```r title="R"
library(kreuzberg)

items <- jsonlite::toJSON(list(
  list(path = "report.pdf"),
  list(path = "slides.pptx"),
  list(path = "data.xlsx")
), auto_unbox = TRUE)

json <- batch_extract_files_sync(items = items, config = ExtractionConfig$default())
results <- jsonlite::fromJSON(json, simplifyVector = FALSE)

for (i in seq_along(results)) {
  cat(sprintf("[%d] mime=%s chars=%d\n",
              i, results[[i]]$mime_type, nchar(results[[i]]$content)))
}
```
