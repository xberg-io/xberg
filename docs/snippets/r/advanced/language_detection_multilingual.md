```r title="R"
library(kreuzberg)

files <- c("english.pdf", "spanish.pdf", "french.pdf")
config <- list(language_detection = list(enabled = TRUE))

for (file in files) {
  json <- extract_file_sync(file, "application/pdf", config)
  result <- jsonlite::fromJSON(json, simplifyVector = FALSE)
  cat(sprintf("%s: detected language = %s\n",
              file, result$detected_language))
}
```
