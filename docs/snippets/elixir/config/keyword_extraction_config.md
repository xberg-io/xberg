```elixir title="Elixir"
config_json = Jason.encode!(%{
  "keywords" => %{
    "algorithm" => "Yake",
    "max_keywords" => 10,
    "min_score" => 0.1,
    "ngram_range" => [1, 3],
    "language" => "en"
  }
})

{:ok, result} = Xberg.extract_sync("document.pdf", "application/pdf", config_json)
IO.puts("Keywords: #{inspect(result.keywords)}")
```
