```elixir title="Elixir"
config_json = Jason.encode!(%{
  "ocr" => %{
    "backend" => "tesseract",
    "language" => "eng+deu+fra"
  }
})

{:ok, result} = Kreuzberg.extract_file_sync("multilingual.pdf", "application/pdf", config_json)
IO.puts(result.content)
```
