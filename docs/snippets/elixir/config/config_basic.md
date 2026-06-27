```elixir title="Elixir"
config_json = Jason.encode!(%{
  "use_cache" => true,
  "enable_quality_processing" => true
})

input = %Xberg.ExtractInput{kind: :uri, uri: "document.pdf", mime_type: "application/pdf"}
{:ok, output} = Xberg.extract(input, config_json)

result = List.first(output.results)
IO.puts(result.content)
```
