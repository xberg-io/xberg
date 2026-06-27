```r title="R"
library(xberg)

config <- list(
  keywords = list(
    algorithm = "yake",
    max_keywords = 10L,
    min_score = 0.3,
    ngram_range = c(1L, 3L),
    language = "en"
  )
)

json <- extract_sync("document.pdf", "application/pdf", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)

cat(sprintf("Keywords extracted: %d\n", length(result$keywords)))
```
