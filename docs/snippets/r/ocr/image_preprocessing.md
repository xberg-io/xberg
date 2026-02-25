```r title="R"
library(kreuzberg)

dpi_settings <- c(150L, 300L, 600L)
results <- list()

for (dpi in dpi_settings) {
  ocr_cfg <- ocr_config(backend = "tesseract", language = "eng", dpi = dpi)
  config <- extraction_config(force_ocr = TRUE, ocr = ocr_cfg,
                              enable_quality_processing = TRUE)
  results[[as.character(dpi)]] <- extract_file_sync("scan.png", "image/png", config)
}

for (dpi in dpi_settings) {
  quality <- results[[as.character(dpi)]]$quality_score
  length <- nchar(results[[as.character(dpi)]]$content)
  cat(sprintf("DPI %d: quality=%.2f, length=%d\n", dpi, quality, length))
}
```
