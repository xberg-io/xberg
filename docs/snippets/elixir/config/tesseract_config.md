```elixir title="Elixir"
config_json = Jason.encode!(%{
  "ocr" => %{
    "backend" => "tesseract",
    "language" => "eng+deu",
    "tesseract_config" => %{
      "psm" => 6,
      "oem" => 3
    }
  }
})

{:ok, result} = Xberg.extract_sync("scanned.pdf", "application/pdf", config_json)
IO.puts("OCR text: #{result.content}")
```
