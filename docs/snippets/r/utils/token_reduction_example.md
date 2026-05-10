```r title="R"
library(kreuzberg)

config <- list(
  token_reduction = list(
    mode = "moderate",
    preserve_important_words = TRUE
  )
)

json <- extract_file_sync("verbose_document.pdf", "application/pdf", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)

cat(sprintf("Reduced content length: %d characters\n", nchar(result$content)))
cat(sprintf("MIME type: %s\n", result$mime_type))
```
