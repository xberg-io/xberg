```elixir title="Elixir"
alias Xberg.ExtractionConfig

# Extract keywords from document
config = %ExtractionConfig{
  keyword_extraction: %{
    "enabled" => true,
    "max_keywords" => 15
  }
}

case Xberg.extract(%Xberg.ExtractInput{kind: :uri, uri: "article.pdf"}, config) do
  {:ok, output} ->
    result = List.first(output.results)
    IO.puts("=== Keyword Extraction ===\n")

    # Display extracted keywords
    if result.extracted_keywords do
      IO.puts("Extracted keywords:")
      Enum.each(result.extracted_keywords, fn kw ->
        IO.puts("- #{kw["text"]}: #{kw["score"]}")
      end)
    else
      IO.puts("No keywords extracted")
    end

  {:error, reason} ->
    IO.puts("Extraction failed!")
    IO.puts("Error: #{inspect(reason)}")
end
```
