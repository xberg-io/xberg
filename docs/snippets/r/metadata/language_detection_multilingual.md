```r title="R"
library(xberg)

files <- c("english.pdf", "spanish.pdf", "french.pdf")
config <- ExtractionConfig$from_json(jsonlite::toJSON(list(
  language_detection = list(enabled = TRUE)
), auto_unbox = TRUE))
for (file in files) {
  input <- list(kind = "uri", uri = file, mime_type = "application/pdf")
  json <- extract(
    input = ExtractInput$from_json(jsonlite::toJSON(input, auto_unbox = TRUE)),
    config = config
  )
  output <- jsonlite::fromJSON(json, simplifyVector = FALSE)
  result <- output$results[[1]]
  cat(sprintf("%s: detected language = %s\n",
              file, result$detected_language))
}
```
