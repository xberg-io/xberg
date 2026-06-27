```elixir title="Elixir"
config_json = Jason.encode!(%{
  "language_detection" => %{
    "enabled" => true,
    "min_confidence" => 0.8,
    "detect_multiple" => false
  }
})

input = %Xberg.ExtractInput{kind: :uri, uri: "document.pdf", mime_type: "application/pdf"}
{:ok, output} = Xberg.extract(input, config_json)

result = List.first(output.results)
if result.language do
  IO.puts("Detected language: #{result.language}")
end
```
