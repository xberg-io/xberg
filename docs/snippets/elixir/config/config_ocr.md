```elixir title="Elixir"
config_json = Jason.encode!(%{
  "ocr" => %{
    "backend" => "tesseract",
    "language" => "eng"
  }
})

input = %Xberg.ExtractInput{kind: :uri, uri: "scanned.pdf", mime_type: "application/pdf"}
{:ok, output} = Xberg.extract(input, config_json)

result = List.first(output.results)
IO.puts("Content length: #{String.length(result.content)}")
IO.puts("Tables detected: #{length(result.tables)}")
```
