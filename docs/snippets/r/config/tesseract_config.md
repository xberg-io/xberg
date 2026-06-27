```r title="R"
library(xberg)

config <- list(
  force_ocr = TRUE,
  ocr = list(
    backend = "tesseract",
    language = "eng+deu"
  )
)

json <- extract_sync("document.pdf", "application/pdf", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)

cat(sprintf("Detected language: %s\n", result$detected_language))
cat(sprintf("Content length: %d characters\n", nchar(result$content)))
```
