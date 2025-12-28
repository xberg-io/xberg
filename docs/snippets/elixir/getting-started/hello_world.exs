```elixir title="Elixir"
# First Kreuzberg program - extract text from a PDF
{:ok, result} = Kreuzberg.extract_file("document.pdf")
IO.puts(result.content)
```
