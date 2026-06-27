```elixir title="Elixir"
alias Xberg.ExtractionConfig

# Configure quality processing settings
# Enable noise removal and set minimum quality thresholds for extracted content
config = %ExtractionConfig{
  quality_processing: %{
    "enabled" => true,
    "min_quality_score" => 0.7,
    "remove_noise" => true
  },
  ocr: %{
    "enabled" => true,
    "backend" => "tesseract"
  },
  chunking: %{
    "max_characters" => 1000,
    "overlap" => 100
  },
  use_cache: true
}

{:ok, result} = Xberg.extract("noisy_document.pdf", nil, config)

IO.puts("Quality Processing Applied:")
IO.puts("Content quality score: #{result.quality_score}")
IO.puts("Noise removed: true")
IO.puts("Content length: #{byte_size(result.content)} bytes")
IO.puts("Processing complete: #{inspect(result)}")
```
