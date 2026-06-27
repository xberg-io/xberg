```r title="R"
library(xberg)

config <- list(
  force_ocr = TRUE,
  ocr = list(backend = "tesseract", language = "eng"),
  enable_quality_processing = TRUE
)

json <- extract_sync("scan.png", "image/png", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)

cat(sprintf("Quality: %.2f, Length: %d\n",
            result$quality_score %||% 0,
            nchar(result$content)))
```
