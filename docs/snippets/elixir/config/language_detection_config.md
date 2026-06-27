```elixir title="Elixir"
config_json = Jason.encode!(%{
  "language_detection" => %{
    "enabled" => true,
    "min_confidence" => 0.8,
    "detect_multiple" => true
  }
})

input = %Xberg.ExtractInput{kind: :uri, uri: "document.pdf", mime_type: "application/pdf"}
{:ok, output} = Xberg.extract(input, config_json)

result = List.first(output.results)
IO.puts("Detected language: #{result.language}")
IO.puts("Confidence: #{result.language_confidence}")
```
