```elixir title="Elixir"
config_json = Jason.encode!(%{
  "post_processors" => [
    %{
      "name" => "QualityFilter",
      "enabled" => true
    }
  ]
})

{:ok, result} = Xberg.extract_sync("document.pdf", "application/pdf", config_json)

IO.puts("Extraction quality applied")
IO.inspect(result.text, label: "Quality-filtered text")
```
