```elixir title="Elixir"
alias Kreuzberg.ExtractionConfig

# Detect all languages in multilingual document
config = %ExtractionConfig{
  language_detection: %{
    "enabled" => true,
    "detect_all" => true
  }
}

case Kreuzberg.extract_file("multilingual.pdf", nil, config) do
  {:ok, result} ->
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
