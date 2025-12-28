```elixir title="Elixir"
alias Kreuzberg.ExtractionConfig

# Configure keyword extraction
config = %ExtractionConfig{
  keyword_extraction: %{
    "enabled" => true,
    "max_keywords" => 10,
    "min_score" => 0.5
  }
}

{:ok, result} = Kreuzberg.extract_file("document.pdf", nil, config)

# Access extracted keywords
if result.keywords do
  IO.puts("Extracted #{length(result.keywords)} keywords")

  Enum.each(result.keywords, fn keyword ->
    IO.puts("#{keyword["text"]}: #{keyword["score"]}")
  end)
end
```
