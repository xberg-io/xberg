```elixir title="Elixir"
config_json = Jason.encode!(%{
  "ocr" => %{
    "backend" => "tesseract"
  },
  "force_ocr" => true
})

{:ok, result} = Xberg.extract_sync("document.pdf", "application/pdf", config_json)
IO.puts(result.content)
```
