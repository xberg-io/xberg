```elixir title="Elixir"
alias Kreuzberg.ExtractionConfig

# Load configuration from file
# Automatically discovers and loads configuration from the user's config directory
config_path = Path.expand("~/.kreuzberg/config.json")

config = if File.exists?(config_path) do
  config_data = config_path |> File.read!() |> Jason.decode!()
  struct(ExtractionConfig, Map.new(config_data, fn {k, v} -> {String.to_atom(k), v} end))
else
  IO.puts("Config file not found at #{config_path}. Using defaults.")
  %ExtractionConfig{}
end

IO.puts("Configuration Source: #{if File.exists?(config_path), do: "#{config_path} (file)", else: "defaults"}")
IO.puts("OCR Enabled: #{inspect(config.ocr["enabled"])}")
IO.puts("Chunking Max Chars: #{inspect(config.chunking["max_chars"])}")
IO.puts("Use Cache: #{inspect(config.use_cache)}")

{:ok, result} = Kreuzberg.extract_file("document.pdf", nil, config)

IO.puts("Document extraction complete")
IO.puts("Content length: #{byte_size(result.content)} bytes")
IO.puts("Languages detected: #{inspect(result.detected_languages)}")
```
