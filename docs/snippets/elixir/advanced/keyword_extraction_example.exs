```elixir title="Elixir"
alias Kreuzberg.ExtractionConfig

# Extract keywords from document
config = %ExtractionConfig{
  keyword_extraction: %{
    "enabled" => true,
    "max_keywords" => 15
  }
}

case Kreuzberg.extract_file("article.pdf", nil, config) do
  {:ok, result} ->
    IO.puts("=== Keyword Extraction ===\n")

    # Display extracted keywords
    if result.keywords do
      IO.puts("Extracted keywords:")
      Enum.each(result.keywords, fn kw ->
        IO.puts("- #{kw["word"]}: #{kw["score"]}")
      end)
    else
      IO.puts("No keywords extracted")
    end

  {:error, reason} ->
    IO.puts("Extraction failed!")
    IO.puts("Error: #{inspect(reason)}")
end
```
