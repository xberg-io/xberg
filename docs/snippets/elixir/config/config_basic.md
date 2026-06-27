```elixir title="Elixir"
config_json = Jason.encode!(%{
  "use_cache" => true,
  "enable_quality_processing" => true
})

{:ok, result} = Xberg.extract_sync("document.pdf", "application/pdf", config_json)
IO.puts(result.content)
```
