<!-- snippet:syntax-only -->

```r title="R"
library(xberg)

# Encapsulate mutable counter state in an environment so the plugin function
# can update it across calls.
make_stateful_plugin <- function() {
  state <- new.env(parent = emptyenv())
  state$count <- 0L

  process <- function(result) {
    state$count <- state$count + 1L
    return(result)
  }

  list(process = process, count = function() state$count)
}

plugin <- make_stateful_plugin()
register_post_processor("stateful_counter", plugin$process)

config <- ExtractionConfig$from_json(jsonlite::toJSON(list(postprocessor = list(enabled = TRUE)), auto_unbox = TRUE))
input <- list(kind = "uri", uri = "document.pdf", mime_type = "application/pdf")
json <- extract(
  input = ExtractInput$from_json(jsonlite::toJSON(input, auto_unbox = TRUE)),
  config = config
)

cat(sprintf("Processed: %d\n", plugin$count()))
```
