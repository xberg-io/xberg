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

{:ok, output} = Xberg.extract(%Xberg.ExtractInput{kind: :uri, uri: "research_paper.pdf"}, config_json)

result = List.first(output.results)
for keyword <- result.extracted_keywords || [] do
  text = keyword["text"] || ""
  score = keyword["score"] || 0.0
  IO.puts("#{text}: #{:io_lib.format("~.3f", [score])}")
end
```
