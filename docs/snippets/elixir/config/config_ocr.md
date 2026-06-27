```elixir title="Elixir"
config_json = Jason.encode!(%{
  "ocr" => %{
    "backend" => "tesseract",
    "language" => "eng"
  }
})

{:ok, result} = Xberg.extract_sync("scanned.pdf", "application/pdf", config_json)
IO.puts("Content length: #{String.length(result.content)}")
IO.puts("Tables detected: #{length(result.tables)}")
```
