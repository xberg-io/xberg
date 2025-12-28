```elixir title="Elixir"
{:ok, result} = Kreuzberg.extract_file("document.pdf")

# Access format-specific metadata
metadata = result.metadata
IO.puts("MIME type: #{result.mime_type}")
IO.puts("All metadata keys: #{inspect(Map.keys(metadata))}")

# Check PDF-specific metadata
case metadata["pdf"] do
  pdf_meta when is_map(pdf_meta) ->
    IO.puts("Page count: #{pdf_meta["page_count"]}")
    IO.puts("Author: #{pdf_meta["author"]}")
    IO.puts("Title: #{pdf_meta["title"]}")
  _ ->
    IO.puts("No PDF metadata available")
end

# Check HTML-specific metadata
case metadata["html"] do
  html_meta when is_map(html_meta) ->
    IO.puts("HTML keywords: #{inspect(html_meta["keywords"])}")
  _ ->
    IO.puts("No HTML metadata available")
end
```
