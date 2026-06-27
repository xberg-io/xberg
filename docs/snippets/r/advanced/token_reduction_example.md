```r title="R"
library(xberg)

config <- list(
  token_reduction = list(enabled = TRUE)
)

json <- extract_sync("document.pdf", "application/pdf", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)

cat("Token-reduced content:\n")
cat(sprintf("Length: %d characters\n", nchar(result$content)))
cat(sprintf("Preview: %.60s...\n", result$content))
```
