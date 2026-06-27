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

input = %Xberg.ExtractInput{kind: :uri, uri: "scanned.pdf", mime_type: "application/pdf"}
{:ok, output} = Xberg.extract(input, config_json)

result = List.first(output.results)
IO.puts("OCR text: #{result.content}")
```
