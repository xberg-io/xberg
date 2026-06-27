```elixir title="Elixir"
alias Xberg.ExtractionConfig

# Configure language detection for multilingual documents
# Automatically detect all languages present in the content
config = %ExtractionConfig{
  language_detection: %{
    "enabled" => true,
    "detect_all" => true
  },
  chunking: %{
    "max_characters" => 1000,
    "overlap" => 100
  },
  use_cache: true
}

{:ok, result} = Xberg.extract("multilingual.pdf", nil, config)

IO.puts("Detected Languages:")
IO.inspect(result.detected_languages)
IO.puts("Content: #{String.slice(result.content, 0..100)}")
```
