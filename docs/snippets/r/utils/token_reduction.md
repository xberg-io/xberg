```r title="R"
library(kreuzberg)

config <- list(
  token_reduction = list(
    mode = "moderate",
    preserve_important_words = TRUE
  )
)

json <- extract_file_sync("document.pdf", "application/pdf", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)

cat(result$content)
```
