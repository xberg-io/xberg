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

input = %Xberg.ExtractInput{kind: :uri, uri: "scanned.pdf", mime_type: "application/pdf"}
{:ok, output} = Xberg.extract(input, config_json)

result = List.first(output.results)
IO.puts(result.content)
```
