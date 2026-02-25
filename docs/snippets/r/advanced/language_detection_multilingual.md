```r title="R"
library(kreuzberg)

files <- c("english.pdf", "spanish.pdf", "french.pdf")
config <- extraction_config(language_detection = list(enabled = TRUE))

for (file in files) {
  result <- extract_file_sync(file, "application/pdf", config)
  cat(sprintf("%s: detected language = %s\n",
              file, result$detected_language))
}
```
