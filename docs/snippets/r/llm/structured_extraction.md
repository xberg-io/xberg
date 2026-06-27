<!-- snippet:syntax-only --> Requires network access to the configured LLM provider and a valid API key in the host environment.

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

config <- list(
  structured_extraction = list(
    schema = schema,
    llm = list(model = "openai/gpt-4o-mini"),
    strict = TRUE
  )
)

json <- extract_sync("paper.pdf", "application/pdf", config)
result <- jsonlite::fromJSON(json, simplifyVector = FALSE)

cat(result$structured_output, "\n")
```
