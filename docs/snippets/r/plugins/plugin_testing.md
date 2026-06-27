<!-- snippet:syntax-only -->

```r title="R"
library(xberg)
library(testthat)

uppercase_processor <- function(result) {
  result$content <- toupper(result$content)
  return(result)
}

test_that("uppercase processor uppercases content", {
  fake_result <- list(
    content = "hello world",
    mime_type = "text/plain",
    metadata = list()
  )
  processed <- uppercase_processor(fake_result)
  expect_equal(processed$content, "HELLO WORLD")
})

test_that("post processor registers and runs", {
  register_post_processor("uppercase", uppercase_processor)
  on.exit(unregister_post_processor("uppercase"), add = TRUE)

  config <- ExtractionConfig$from_json(jsonlite::toJSON(list(postprocessor = list(enabled = TRUE)), auto_unbox = TRUE))
  input <- list(kind = "bytes", bytes = as.integer(charToRaw("hello world")), mime_type = "text/plain")
  json <- extract(
    input = ExtractInput$from_json(jsonlite::toJSON(input, auto_unbox = TRUE)),
    config = config
  )
  output <- jsonlite::fromJSON(json, simplifyVector = FALSE)
  result <- output$results[[1]]
  expect_match(result$content, "HELLO WORLD", fixed = TRUE)
})
```
