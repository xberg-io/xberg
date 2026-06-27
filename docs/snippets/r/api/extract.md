```r title="R"
library(xberg)

json <- extract(
  ExtractInput$uri("document.pdf"),
  ExtractionConfig$default()
)
output <- jsonlite::fromJSON(json, simplifyVector = FALSE)

cat(output$results[[1]]$content)
```
