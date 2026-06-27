```elixir title="Elixir"
config_json = Jason.encode!(%{
  "ocr" => %{
    "backend" => "tesseract",
    "language" => "eng+deu+fra"
  }
})

input = %Xberg.ExtractInput{kind: :uri, uri: "multilingual.pdf", mime_type: "application/pdf"}
{:ok, output} = Xberg.extract(input, config_json)

result = List.first(output.results)
IO.puts(result.content)
```
