```r title="R"
library(kreuzberg)

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

config <- extraction_config(postprocessor = list(enabled = TRUE))
result <- extract_file_sync("document.pdf", "application/pdf", config)

cat(sprintf("Extraction complete: %d characters\n", nchar(result$content)))
```
