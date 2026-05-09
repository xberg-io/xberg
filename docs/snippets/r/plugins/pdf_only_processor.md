<!-- snippet:syntax-only -->
```r title="R"
library(kreuzberg)

pdf_only_processor <- function(result) {
  # Gate the processor so it only runs for PDF documents.
  if (is.null(result$mime_type) || result$mime_type != "application/pdf") {
    return(result)
  }
  return(result)
}

register_post_processor("pdf_only", pdf_only_processor)

config <- extraction_config(postprocessor = list(enabled = TRUE))
result <- extract_file_sync("document.pdf", "application/pdf", config)

cat(sprintf("Processed PDF: %d characters\n", nchar(result$content)))
```
