```r title="R"
library(kreuzberg)

dpi_values <- c(150L, 300L, 600L)
results <- list()

for (dpi in dpi_values) {
  ocr_cfg <- ocr_config(backend = "tesseract", language = "eng", dpi = dpi)
  config <- extraction_config(force_ocr = TRUE, ocr = ocr_cfg)
  results[[as.character(dpi)]] <- extract_file_sync("document.pdf", "application/pdf", config)
}

for (dpi in dpi_values) {
  content_len <- nchar(results[[as.character(dpi)]]$content)
  cat(sprintf("DPI %d: %d characters extracted\n", dpi, content_len))
}
```
