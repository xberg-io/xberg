```elixir title="Elixir"
alias Kreuzberg.ExtractionConfig

# Process document with quality filtering
config = %ExtractionConfig{
  quality_processing: %{
    "enabled" => true,
    "min_quality_score" => 0.8
  }
}

case Kreuzberg.extract_file("document.pdf", nil, config) do
  {:ok, result} ->
    IO.puts("=== Quality Processing ===\n")

    # Display quality metrics if available
    quality_score = result.metadata["quality_score"]
    if quality_score do
      IO.puts("Quality Score: #{quality_score}")
      IO.puts("Content Quality: #{quality_status(quality_score)}")
    end

    # Display content with quality assurance
    IO.puts("\n--- Extracted Content ---")
    content_preview = String.slice(result.content, 0..200)
    IO.puts(content_preview)
    IO.puts("\nTotal size: #{byte_size(result.content)} bytes")

  {:error, reason} ->
    IO.puts("Extraction failed!")
    IO.puts("Error: #{inspect(reason)}")
end

# Helper function to determine quality status
defp quality_status(score) when score >= 0.9, do: "Excellent"
defp quality_status(score) when score >= 0.8, do: "Good"
defp quality_status(score) when score >= 0.7, do: "Fair"
defp quality_status(_score), do: "Poor"
```
