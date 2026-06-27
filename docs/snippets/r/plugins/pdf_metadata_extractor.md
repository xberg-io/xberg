```r title="R"
library(xberg)

extract_pdf_metadata <- function(result) {
  processed_result <- result
  if (!is.null(result$metadata)) {
    cat(sprintf("PDF Metadata:\n"))
    for (key in names(result$metadata)) {
      cat(sprintf("  %s: %s\n", key, result$metadata[[key]]))
    }
  }
  return(processed_result)
}

register_post_processor("pdf_metadata", extract_pdf_metadata)

config <- ExtractionConfig$from_json(jsonlite::toJSON(list(postprocessor = list(enabled = TRUE)), auto_unbox = TRUE))
input <- list(kind = "uri", uri = "document.pdf", mime_type = "application/pdf")
json <- extract(
  input = ExtractInput$from_json(jsonlite::toJSON(input, auto_unbox = TRUE)),
  config = config
)
output <- jsonlite::fromJSON(json, simplifyVector = FALSE)
result <- output$results[[1]]
cat(sprintf("Extraction complete: %d characters\n", nchar(result$content)))
```
