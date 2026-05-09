```r title="R"
library(kreuzberg)

config <- list(
  pdf_options = list(
    extract_metadata = TRUE,
    hierarchy = list(
      enabled = TRUE,
      k_clusters = 6L,
      include_bbox = TRUE,
      ocr_coverage_threshold = 0.8
    )
  )
)

json <- extract_file_sync("document.pdf", "application/pdf", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)
cat(sprintf("Pages: %d\n", length(result$pages)))
```
