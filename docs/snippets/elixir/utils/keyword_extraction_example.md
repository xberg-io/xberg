<!-- snippet:syntax-only -->

```elixir
config_json =
  Jason.encode!(%{
    "keywords" => %{
      "algorithm" => "yake",
      "max_keywords" => 10,
      "min_score" => 0.3
    }
  })

{:ok, json} = Xberg.extract_async("research_paper.pdf", nil, config_json)
result = Jason.decode!(json)

for keyword <- result["extracted_keywords"] || [] do
  text = keyword["text"] || ""
  score = keyword["score"] || 0.0
  IO.puts("#{text}: #{:io_lib.format("~.3f", [score])}")
end
```
