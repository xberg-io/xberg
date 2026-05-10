```r title="R"
library(kreuzberg)

config <- list(
  model = list(type = "preset", name = "balanced"),
  normalize = TRUE
)

texts <- c("Hello, world!", "Kreuzberg is fast")
embeddings <- embed_texts(texts, config)

stopifnot(length(embeddings) == 2L)
cat(sprintf("Embedding 1: %d dimensions\n", length(embeddings[[1]])))
cat(sprintf("Embedding 2: %d dimensions\n", length(embeddings[[2]])))
```
