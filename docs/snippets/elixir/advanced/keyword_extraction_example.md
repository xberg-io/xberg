```elixir title="Elixir"
config_json = Jason.encode!(%{
  "keywords" => %{
    "algorithm" => "Yake",
    "max_keywords" => 10,
    "min_score" => 0.3
  }
})

input = %Xberg.ExtractInput{kind: :uri, uri: "research_paper.pdf", mime_type: "application/pdf"}
{:ok, output} = Xberg.extract(input, config_json)

result = List.first(output.results)
if result.extracted_keywords do
  result.extracted_keywords
    |> Enum.each(fn %{"text" => kw, "score" => score} ->
      IO.puts("#{kw}: #{Float.round(score, 4)}")
    end)
end
```
