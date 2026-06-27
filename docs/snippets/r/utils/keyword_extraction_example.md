```r title="R"
library(xberg)

config <- list(
  keywords = list(
    algorithm = "yake",
    max_keywords = 10L,
    min_score = 0.3
  )
)

json <- extract_sync("research_paper.pdf", "application/pdf", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)

cat(sprintf("Content length: %d characters\n", nchar(result$content)))
if (!is.null(result$metadata$keywords)) {
  for (kw in result$metadata$keywords) {
    cat(sprintf("  - %s\n", kw))
  }
}
```
