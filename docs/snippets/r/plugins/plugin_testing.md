<!-- snippet:syntax-only -->
```r title="R"
library(kreuzberg)
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

  config <- extraction_config(postprocessor = list(enabled = TRUE))
  result <- extract_bytes_sync(
    charToRaw("hello world"), "text/plain", config
  )

  expect_match(result$content, "HELLO WORLD", fixed = TRUE)
})
```
