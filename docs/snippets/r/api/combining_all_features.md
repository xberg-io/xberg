```r title="R"
library(kreuzberg)

ocr_cfg <- ocr_config(backend = "tesseract", language = "eng", dpi = 300L)
chunking_cfg <- chunking_config(max_characters = 1200L, overlap = 250L)

config <- extraction_config(
  ocr = ocr_cfg,
  force_ocr = TRUE,
  chunking = chunking_cfg,
  language_detection = list(enabled = TRUE),
  keywords = list(enabled = TRUE),
  enable_quality_processing = TRUE,
  output_format = "markdown"
)

result <- extract_file_sync("document.pdf", "application/pdf", config)

cat(sprintf("Language: %s | Quality: %.2f | Chunks: %d | Keywords: %d\n",
            result$detected_language, result$quality_score,
            length(result$chunks), length(result$keywords)))
```
