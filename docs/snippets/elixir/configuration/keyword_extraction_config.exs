```elixir title="Elixir"
alias Xberg.ExtractionConfig

# Configure keyword extraction
config = %ExtractionConfig{
  keyword_extraction: %{
    "enabled" => true,
    "max_keywords" => 10,
    "min_score" => 0.5
  }
}

{:ok, output} = Xberg.extract(%Xberg.ExtractInput{kind: :uri, uri: "document.pdf"}, config)

result = List.first(output.results)
# Access extracted keywords
if result.extracted_keywords do
  IO.puts("Extracted #{length(result.extracted_keywords)} keywords")

  Enum.each(result.extracted_keywords, fn keyword ->
    IO.puts("#{keyword["text"]}: #{keyword["score"]}")
  end)
end
```
