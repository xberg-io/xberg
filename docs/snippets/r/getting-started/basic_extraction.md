```r
library(xberg)

# Extract text from a PDF file
result <- extract_sync("document.pdf")
cat(result$content)
```
