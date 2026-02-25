```r title="R"
library(kreuzberg)

custom_ocr_backend <- function(image_path, language) {
  cat(sprintf("Processing image: %s\n", image_path))
  return(sprintf("Extracted text from %s", image_path))
}

register_ocr_backend("custom_cloud", custom_ocr_backend)

ocr_cfg <- ocr_config(backend = "custom_cloud", language = "en")
config <- extraction_config(force_ocr = TRUE, ocr = ocr_cfg)

result <- extract_file_sync("document.pdf", "application/pdf", config)
cat(sprintf("Custom backend result: %d chars\n", nchar(result$content)))
```
