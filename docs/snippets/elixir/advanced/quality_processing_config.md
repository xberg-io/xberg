```elixir title="Elixir"
config_json = Jason.encode!(%{
  "post_processors" => [
    %{
      "name" => "QualityFilter",
      "enabled" => true
    }
  ]
})

input = %Xberg.ExtractInput{kind: :uri, uri: "document.pdf", mime_type: "application/pdf"}
{:ok, output} = Xberg.extract(input, config_json)

result = List.first(output.results)
IO.puts("Extraction quality applied")
IO.inspect(result.text, label: "Quality-filtered text")
```
