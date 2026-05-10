```elixir title="Elixir"
config_json = Jason.encode!(%{
  "ocr" => %{
    "backend" => "easyocr",
    "language" => "en"
  }
})

{:ok, result} = Kreuzberg.extract_file_async("document.pdf", "application/pdf", config_json)
IO.puts("Extracted text: #{result.content}")
```
