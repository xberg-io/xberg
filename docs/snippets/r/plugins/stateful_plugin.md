<!-- snippet:syntax-only -->
```r title="R"
library(kreuzberg)

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

config <- list(postprocessor = list(enabled = TRUE))
extract_file_sync("document.pdf", "application/pdf", config)

cat(sprintf("Processed: %d\n", plugin$count()))
```
