```elixir title="Elixir"
alias Xberg.ExtractionConfig

# Detect all languages in multilingual document
config = %ExtractionConfig{
  language_detection: %{
    "enabled" => true,
    "detect_all" => true
  }
}

case Xberg.extract(%Xberg.ExtractInput{kind: :uri, uri: "multilingual.pdf"}, config) do
  {:ok, output} ->
    result = List.first(output.results)
    IO.puts("=== Language Detection ===\n")

    # Display detected languages
    languages = result.detected_languages || []
    if Enum.empty?(languages) do
      IO.puts("No languages detected")
    else
      IO.puts("Detected languages:")
      Enum.each(languages, fn lang ->
        IO.puts("- #{lang}")
      end)
      IO.puts("\nTotal languages: #{length(languages)}")
    end

  {:error, reason} ->
    IO.puts("Extraction failed!")
    IO.puts("Error: #{inspect(reason)}")
end
```
