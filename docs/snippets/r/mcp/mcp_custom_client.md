```r title="R"
# The kreuzberg R bindings ship no MCP client. Drive the kreuzberg CLI's
# stdio MCP transport from R using a piped subprocess.
mcp <- pipe("kreuzberg mcp", open = "w+")
on.exit(close(mcp), add = TRUE)

request <- list(
  method = "tools/call",
  params = list(
    name = "extract_file",
    arguments = list(
      path = "document.pdf",
      async = TRUE
    )
  )
)

writeLines(jsonlite::toJSON(request, auto_unbox = TRUE), con = mcp)
flush(mcp)

response_line <- readLines(mcp, n = 1L)
cat(response_line, "\n")
```

<!-- snippet:syntax-only --> The R bindings have no MCP client; this snippet drives the MCP CLI over stdio. Requires the `jsonlite` package.
