```r title="R"
library(xberg)

input <- list(kind = "uri", uri = "spreadsheet.xlsx")
json <- extract(
  input = ExtractInput$from_json(jsonlite::toJSON(input, auto_unbox = TRUE)),
  config = ExtractionConfig$default()
)
output <- jsonlite::fromJSON(json, simplifyVector = FALSE)
result <- output$results[[1]]

cat("Tables extracted:", length(result$tables), "\n\n")

for (i in seq_along(result$tables)) {
  table <- result$tables[[i]]
  cat(sprintf("Table %d:\n", i))
  cat("  Rows:", nrow(table), "\n")
  cat("  Columns:", ncol(table), "\n")
  cat("  Column names:", paste(colnames(table), collapse=", "), "\n")
  cat("\n")

  if (nrow(table) > 0L) {
    cat("  Preview (first 3 rows):\n")
    print(head(table, 3L))
    cat("\n")
  }
}
```
