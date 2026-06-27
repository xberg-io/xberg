```elixir title="Elixir"
config_json = Jason.encode!(%{
  "postprocessor" => %{
    "enabled" => true,
    "enabled_processors" => [
      "whitespace_normalizer",
      "unicode_normalizer"
    ]
  }
})

input = %Xberg.ExtractInput{kind: :uri, uri: "document.pdf", mime_type: "application/pdf"}
{:ok, output} = Xberg.extract(input, config_json)

result = List.first(output.results)
IO.puts("Processed content: #{result.content}")
```
