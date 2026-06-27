```elixir title="Elixir"
config_json = Jason.encode!(%{
  "use_cache" => true,
  "enable_quality_processing" => true,
  "ocr" => %{
    "backend" => "tesseract",
    "language" => "eng"
  },
  "chunking" => %{
    "max_characters" => 1000,
    "overlap" => 200,
    "embedding" => %{
      "model" => %{
        "preset" => %{
          "name" => "balanced"
        }
      },
      "batch_size" => 32,
      "normalize" => true,
      "show_download_progress" => false
    }
  },
  "language_detection" => %{
    "enabled" => true,
    "min_confidence" => 0.8,
    "detect_multiple" => false
  },
  "keywords" => %{
    "algorithm" => "Yake",
    "max_keywords" => 10,
    "min_score" => 0.1,
    "ngram_range" => [1, 3],
    "language" => "en"
  },
  "token_reduction" => %{
    "mode" => "moderate",
    "preserve_important_words" => true
  },
  "postprocessor" => %{
    "enabled" => true
  }
})

{:ok, result} = Xberg.extract_sync("document.pdf", "application/pdf", config_json)
IO.puts("Content: #{result.content}")

if result.detected_languages do
  IO.puts("Languages: #{inspect(result.detected_languages)}")
end

chunks_count = if result.chunks, do: length(result.chunks), else: 0
IO.puts("Chunks: #{chunks_count}")
```
