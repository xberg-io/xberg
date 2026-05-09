<!-- snippet:syntax-only -->
```r title="R"
library(kreuzberg)

# Wrap an already-loaded embedder (e.g. an ONNX session) so kreuzberg can
# call back into it during chunking and standalone embed requests.
my_embedder <- list(
  name = "my-embedder",
  version = "1.0.0",
  dimensions = 768L,
  embed = function(texts) {
    # Delegate to the already-loaded host model.
    lapply(texts, function(.) rep(0.0, 768))
  }
)

register_embedding_backend(my_embedder)

config <- list(
  embedding = list(
    model = list(type = "plugin", name = "my-embedder"),
    max_embed_duration_secs = 30L
  )
)

vectors <- embed_texts(c("Hello, world!", "Second text"), config)
cat(sprintf("Generated %d embedding vectors\n", length(vectors)))
```
