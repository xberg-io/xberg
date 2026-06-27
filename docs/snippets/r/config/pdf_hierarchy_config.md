```r title="R"
library(xberg)

config <- ExtractionConfig$from_json(jsonlite::toJSON(list(
  pdf_options = list(
    extract_metadata = TRUE,
    hierarchy = list(
      enabled = TRUE,
      k_clusters = 6L,
      include_bbox = TRUE,
      ocr_coverage_threshold = 0.8
    )
  )
), auto_unbox = TRUE))

input <- list(kind = "uri", uri = "document.pdf", mime_type = "application/pdf")
json <- extract(
  input = ExtractInput$from_json(jsonlite::toJSON(input, auto_unbox = TRUE)),
  config = config
)
output <- jsonlite::fromJSON(json, simplifyVector = FALSE)
result <- output$results[[1]]
cat(sprintf("Pages: %d\n", length(result$pages)))
```
