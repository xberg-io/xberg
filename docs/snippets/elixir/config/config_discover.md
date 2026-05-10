```elixir title="Elixir"
# Extract with nil config to use discovered/default configuration
{:ok, result} = Kreuzberg.extract_file_sync("document.pdf", "application/pdf", nil)
IO.puts(result.content)
```
