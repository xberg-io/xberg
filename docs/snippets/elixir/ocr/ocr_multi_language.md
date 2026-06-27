```elixir title="Elixir"
config_json = Jason.encode!(%{
  "ocr" => %{
    "backend" => "tesseract",
    "language" => "eng+deu+fra"
  }
})

{:ok, result} = Xberg.extract_sync("multilingual.pdf", "application/pdf", config_json)
IO.puts(result.content)
```
