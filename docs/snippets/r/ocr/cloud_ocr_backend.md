```r title="R"
library(kreuzberg)

custom_ocr_backend <- function(image_path, language) {
  cat(sprintf("Processing image: %s\n", image_path))
  return(sprintf("Extracted text from %s", image_path))
}

register_ocr_backend("custom_cloud", custom_ocr_backend)

config <- list(
  force_ocr = TRUE,
  ocr = list(backend = "custom_cloud", language = "en")
)

json <- extract_file_sync("document.pdf", "application/pdf", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)
cat(sprintf("Custom backend result: %d chars\n", nchar(result$content)))
```
