<!-- snippet:syntax-only -->

```r title="R"
library(xberg)

word_count_processor <- function(result) {
  word_count <- length(strsplit(result$content, "\\s+")[[1]])

  result$metadata <- c(result$metadata, list(word_count = word_count))
  return(result)
}

register_post_processor("word_count", word_count_processor)

config <- ExtractionConfig$from_json(jsonlite::toJSON(list(postprocessor = list(enabled = TRUE)), auto_unbox = TRUE))
input <- list(kind = "uri", uri = "document.pdf", mime_type = "application/pdf")
json <- extract(
  input = ExtractInput$from_json(jsonlite::toJSON(input, auto_unbox = TRUE)),
  config = config
)
output <- jsonlite::fromJSON(json, simplifyVector = FALSE)
result <- output$results[[1]]
cat(sprintf("Word count: %d\n", result$metadata$word_count))
```
