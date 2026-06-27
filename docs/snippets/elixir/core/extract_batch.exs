```elixir title="Elixir"
inputs = [
  %Xberg.ExtractInput{kind: :uri, uri: "document.pdf"},
  %Xberg.ExtractInput{
    kind: :bytes,
    bytes: "Hello from memory",
    mime_type: "text/plain",
    filename: "note.txt"
  }
]

{:ok, output} = Xberg.extract_batch(inputs, nil)

Enum.each(output.results, fn result ->
  IO.puts(result.content)
end)
```
