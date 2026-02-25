```r title="R"
library(kreuzberg)

ocr_cfg <- ocr_config(backend = "tesseract", language = "eng", dpi = 300L)
chunking_cfg <- chunking_config(max_characters = 1500L, overlap = 300L)

config <- extraction_config(
  ocr = ocr_cfg,
  chunking = chunking_cfg,
  output_format = "markdown",
  include_document_structure = TRUE,
  force_ocr = TRUE
)

result <- extract_file_sync("document.pdf", "application/pdf", config)
cat(sprintf("Format: %s\n", result$mime_type))
cat(sprintf("Chunks: %d\n", length(result$chunks)))
cat(sprintf("Content preview: %.50s...\n", result$content))
```
