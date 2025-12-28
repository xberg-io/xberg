```elixir title="Elixir"
alias Kreuzberg.ExtractionConfig

# Configure language detection for multilingual documents
# Automatically detect all languages present in the content
config = %ExtractionConfig{
  language_detection: %{
    "enabled" => true,
    "detect_all" => true
  },
  chunking: %{
    "max_chars" => 1000,
    "max_overlap" => 100
  },
  use_cache: true
}

{:ok, result} = Kreuzberg.extract_file("multilingual.pdf", nil, config)

IO.puts("Detected Languages:")
IO.inspect(result.detected_languages)
IO.puts("Content: #{String.slice(result.content, 0..100)}")
```
