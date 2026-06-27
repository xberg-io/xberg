<!-- snippet:syntax-only -->

```r title="R"
library(xberg)

word_count_processor <- function(result) {
  word_count <- length(strsplit(result$content, "\\s+")[[1]])

  result$metadata <- c(result$metadata, list(word_count = word_count))
  return(result)
}

register_post_processor("word_count", word_count_processor)

config <- list(postprocessor = list(enabled = TRUE))
json <- extract_sync("document.pdf", "application/pdf", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)

cat(sprintf("Word count: %d\n", result$metadata$word_count))
```
