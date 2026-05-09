```r title="R"
library(kreuzberg)

config <- list(
  language_detection = list(
    enabled = TRUE,
    min_confidence = 0.8,
    detect_multiple = FALSE
  )
)

json <- extract_file_sync("document.pdf", "application/pdf", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)

if (length(result$detected_languages) > 0) {
  cat(sprintf("Detected language: %s\n", result$detected_languages[[1]]))
} else {
  cat("No language detected\n")
}

cat(sprintf("Content length: %d characters\n", nchar(result$content)))
```
