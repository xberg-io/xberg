```r title="R"
library(kreuzberg)

ocr_cfg <- ocr_config(
  backend = "tesseract",
  language = "eng+deu",
  dpi = 300L
)
config <- extraction_config(force_ocr = TRUE, ocr = ocr_cfg)

result <- extract_file_sync("document.pdf", "application/pdf", config)

cat(sprintf("Detected language: %s\n", result$detected_language))
cat(sprintf("Content length: %d characters\n", nchar(result$content)))
```
