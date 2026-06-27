```elixir title="Elixir"
config_json = Jason.encode!(%{
  "use_cache" => true,
  "ocr" => %{
    "backend" => "tesseract",
    "language" => "eng+deu",
    "tesseract_config" => %{
      "psm" => 6
    }
  },
  "chunking" => %{
    "max_characters" => 1000,
    "overlap" => 200
  },
  "enable_quality_processing" => true
})

{:ok, result} = Xberg.extract_sync("document.pdf", "application/pdf", config_json)
IO.puts("Content length: #{String.length(result.content)}")
```
