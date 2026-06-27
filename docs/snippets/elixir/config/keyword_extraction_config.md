```elixir title="Elixir"
config_json = Jason.encode!(%{
  "keywords" => %{
    "algorithm" => "Yake",
    "max_keywords" => 10,
    "min_score" => 0.1,
    "language" => "en"
  }
})

input = %Xberg.ExtractInput{kind: :uri, uri: "document.pdf", mime_type: "application/pdf"}
{:ok, output} = Xberg.extract(input, config_json)

result = List.first(output.results)
IO.puts("Keywords: #{inspect(result.extracted_keywords)}")
```
