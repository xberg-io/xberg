```r title="R"
library(kreuzberg)

# Confirm the native extension loaded by listing registered extractors
extractors <- list_document_extractors()
cat(sprintf("kreuzberg ready: %d document extractors registered\n", length(extractors)))
```
