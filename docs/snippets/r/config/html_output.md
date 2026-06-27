```r title="R"
library(xberg)

config <- list(
  output_format = "html",
  html_output = list(
    theme = "git_hub",
    embed_css = TRUE
  )
)

json <- extract_sync("document.pdf", "application/pdf", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)
cat(result$content) # HTML with kb-* classes
```
