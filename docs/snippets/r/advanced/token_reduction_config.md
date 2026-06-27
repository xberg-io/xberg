```r title="R"
library(xberg)

config <- list(
  token_reduction = list(
    mode = "moderate",
    preserve_markdown = TRUE,
    preserve_code = TRUE,
    language_hint = "eng"
  )
)

json <- extract_sync("document.pdf", "application/pdf", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)

cat(sprintf("Reduced content length: %d characters\n", nchar(result$content)))
```
