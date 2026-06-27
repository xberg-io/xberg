```elixir title="Elixir"
# Extract with nil config to use discovered/default configuration
{:ok, result} = Xberg.extract_sync("document.pdf", "application/pdf", nil)
IO.puts(result.content)
```
