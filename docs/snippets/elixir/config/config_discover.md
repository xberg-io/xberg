```elixir title="Elixir"
# Extract with nil config to use discovered/default configuration
{:ok, output} = Xberg.extract(%Xberg.ExtractInput{kind: :uri, uri: "document.pdf", mime_type: "application/pdf"}, nil)
result = List.first(output.results)
IO.puts(result.content)
```
