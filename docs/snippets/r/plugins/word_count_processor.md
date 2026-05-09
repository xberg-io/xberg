<!-- snippet:syntax-only -->
```r title="R"
library(kreuzberg)

word_count_processor <- function(result) {
  word_count <- length(strsplit(result$content, "\\s+")[[1]])

  result$metadata <- c(result$metadata, list(word_count = word_count))
  return(result)
}

register_post_processor("word_count", word_count_processor)

config <- extraction_config(postprocessor = list(enabled = TRUE))
result <- extract_file_sync("document.pdf", "application/pdf", config)

cat(sprintf("Word count: %d\n", result$metadata$word_count))
```
