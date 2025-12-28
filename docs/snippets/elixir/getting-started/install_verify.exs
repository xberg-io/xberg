```elixir title="Elixir"
# Verify Kreuzberg is installed and working by extracting a document
{:ok, result} = Kreuzberg.extract_file("sample.pdf")
IO.puts("Installation verified! Extracted #{String.length(result.content)} characters")
```
