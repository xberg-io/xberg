```r title="R"
library(kreuzberg)

# Configure OCR settings via a plain list mirroring the config JSON.
config <- list(
  force_ocr = TRUE,
  ocr = list(
    backend = "tesseract",
    language = "eng"
  )
)

# Extract an image file with OCR enabled
json <- extract_file_sync("image.png", "image/png", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)

cat("Extracted text from image:\n")
cat(result$content)
```
