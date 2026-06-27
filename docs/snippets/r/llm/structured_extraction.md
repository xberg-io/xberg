<!-- snippet:syntax-only -->
<!-- Requires network access to the configured LLM provider and a valid API key in the host environment. -->

```r title="R"
library(xberg)

schema <- list(
  type = "object",
  properties = list(
    title = list(type = "string"),
    authors = list(type = "array", items = list(type = "string")),
    date = list(type = "string")
  ),
  required = c("title", "authors", "date"),
  additionalProperties = FALSE
)

config <- ExtractionConfig$from_json(jsonlite::toJSON(list(
  structured_extraction = list(
    schema = schema,
    llm = list(model = "openai/gpt-4o-mini"),
    strict = TRUE
  )
), auto_unbox = TRUE))

input <- list(kind = "uri", uri = "paper.pdf", mime_type = "application/pdf")
json <- extract(
  input = ExtractInput$from_json(jsonlite::toJSON(input, auto_unbox = TRUE)),
  config = config
)
output <- jsonlite::fromJSON(json, simplifyVector = FALSE)
result <- output$results[[1]]
cat(result$structured_output, "\n")
```
