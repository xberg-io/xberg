```elixir title="Elixir"
config_json = Jason.encode!(%{
  "ocr" => %{
    "backend" => "tesseract",
    "tesseract_config" => %{
      "preprocessing" => %{
        "target_dpi" => 300,
        "denoise" => true,
        "deskew" => true,
        "contrast_enhance" => true,
        "binarization_method" => "otsu"
      }
    }
  }
})

{:ok, result} = Xberg.extract_sync("scanned.pdf", "application/pdf", config_json)
IO.puts(result.content)
```
