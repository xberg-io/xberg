```r title="R"
library(xberg)

input <- list(kind = "uri", uri = "document.pdf")
json <- extract(
  input = ExtractInput$from_json(jsonlite::toJSON(input, auto_unbox = TRUE)),
  config = ExtractionConfig$default()
)
output <- jsonlite::fromJSON(json, simplifyVector = FALSE)
result <- output$results[[1]]

cat("Language Detection Results:\n\n")

cat("Using direct field access:\n")
cat("Detected Language:", result$detected_language, "\n\n")

cat("Using S3 helper function:\n")
lang <- detected_language(result)
cat("Language (via helper):", lang, "\n\n")

cat("Language Information:\n")
if (lang == "en") {
  cat("This is an English document\n")
} else if (lang == "es") {
  cat("This is a Spanish document\n")
} else {
  cat(sprintf("This is a %s document\n", lang))
}
```
