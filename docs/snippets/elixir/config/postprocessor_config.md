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

{:ok, result} = Xberg.extract_sync("document.pdf", "application/pdf", config_json)
IO.puts("Processed content: #{result.content}")
```
