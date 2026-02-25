```r title="R"
library(kreuzberg)

# Note: EasyOCR backend requires Python to be installed
ocr_cfg <- ocr_config(backend = "easyocr", language = "en")
config <- extraction_config(force_ocr = TRUE, ocr = ocr_cfg)

result <- extract_file_sync("document.pdf", "application/pdf", config)

cat(sprintf("EasyOCR extraction:\n"))
cat(sprintf("Content length: %d characters\n", nchar(result$content)))
cat(sprintf("Detected language: %s\n", result$detected_language))
```
