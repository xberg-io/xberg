<!-- snippet:syntax-only -->

```r title="R"
library(xberg)

pdf_only_processor <- function(result) {
  # Gate the processor so it only runs for PDF documents.
  if (is.null(result$mime_type) || result$mime_type != "application/pdf") {
    return(result)
  }
  return(result)
}

register_post_processor("pdf_only", pdf_only_processor)

config <- ExtractionConfig$from_json(jsonlite::toJSON(list(postprocessor = list(enabled = TRUE)), auto_unbox = TRUE))
input <- list(kind = "uri", uri = "document.pdf", mime_type = "application/pdf")
json <- extract(
  input = ExtractInput$from_json(jsonlite::toJSON(input, auto_unbox = TRUE)),
  config = config
)
output <- jsonlite::fromJSON(json, simplifyVector = FALSE)
result <- output$results[[1]]
cat(sprintf("Processed PDF: %d characters\n", nchar(result$content)))
```
