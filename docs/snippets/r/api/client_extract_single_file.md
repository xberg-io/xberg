<!-- snippet:syntax-only -->
```r title="R"
library(kreuzberg)
library(httr2)

response <- request("http://localhost:8000/extract") |>
  req_method("POST") |>
  req_multipart_part(
    name = "files",
    path = "document.pdf",
    type = "application/pdf"
  ) |>
  req_perform()

data <- resp_body_json(response)
cat(jsonlite::toJSON(data, auto_unbox = TRUE, pretty = TRUE))
```
