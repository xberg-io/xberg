```r title="R"
library(kreuzberg)

config <- extraction_config(force_ocr = TRUE)

result <- extract_file_sync("multipage_document.pdf", "application/pdf", config)

cat(sprintf("Total pages: %d\n", result$pages))
cat(sprintf("Content extracted via OCR: %d characters\n",
            nchar(result$content)))
cat(sprintf("Detected language: %s\n", result$detected_language))
```
