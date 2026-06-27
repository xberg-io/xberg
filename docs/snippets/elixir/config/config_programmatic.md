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

input = %Xberg.ExtractInput{kind: :uri, uri: "document.pdf", mime_type: "application/pdf"}
{:ok, output} = Xberg.extract(input, config_json)

result = List.first(output.results)
IO.puts("Content length: #{String.length(result.content)}")
```
