```elixir title="Elixir"
config_json = Jason.encode!(%{
  "enable_quality_processing" => true,
  "use_cache" => true
})

input = %Xberg.ExtractInput{kind: :uri, uri: "document.pdf", mime_type: "application/pdf"}
{:ok, output} = Xberg.extract(input, config_json)

result = List.first(output.results)
IO.puts("Quality score: #{result.quality_score}")
IO.puts("Processing time: #{inspect(result.processing_time)}")
```
