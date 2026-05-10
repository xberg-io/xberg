```elixir title="Elixir"
config_json = Jason.encode!(%{
  "use_cache" => true,
  "enable_quality_processing" => true
})

{:ok, result} = Kreuzberg.extract_file_sync("document.pdf", "application/pdf", config_json)
IO.puts(result.content)
```
