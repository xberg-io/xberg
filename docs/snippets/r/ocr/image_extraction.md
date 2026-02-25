```r title="R"
library(kreuzberg)

ocr_cfg <- ocr_config(backend = "tesseract", language = "eng", dpi = 300L)
config <- extraction_config(force_ocr = TRUE, ocr = ocr_cfg)

result <- extract_file_sync("scan.png", "image/png", config)

cat(sprintf("Image extraction via OCR:\n"))
cat(sprintf("Content length: %d characters\n", nchar(result$content)))
cat(sprintf("Mime type: %s\n", result$mime_type))
cat(sprintf("Detected language: %s\n", result$detected_language))
```
