```elixir title="Elixir"
config_json = Jason.encode!(%{
  "enable_quality_processing" => true,
  "use_cache" => true
})

{:ok, result} = Kreuzberg.extract_file_sync("document.pdf", "application/pdf", config_json)
IO.puts("Quality score: #{result.quality_score}")
IO.puts("Processing time: #{inspect(result.processing_time)}")
```
