```r title="R"
library(xberg)

result <- extract_sync("spreadsheet.xlsx")

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
