```elixir title="Elixir"
config_json = Jason.encode!(%{
  "keywords" => %{
    "algorithm" => "Yake",
    "max_keywords" => 10,
    "min_score" => 0.3
  }
})

{:ok, result} = Xberg.extract_sync("research_paper.pdf", "application/pdf", config_json)

if result.keywords do
  IO.puts("Keywords: #{inspect(result.keywords)}")
end
```
