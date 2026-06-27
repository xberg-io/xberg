```elixir title="Elixir"
config_json = Jason.encode!(%{
  "language_detection" => %{
    "enabled" => true,
    "min_confidence" => 0.7,
    "detect_multiple" => true
  }
})

input = %Xberg.ExtractInput{kind: :uri, uri: "multilingual_document.pdf", mime_type: "application/pdf"}
{:ok, output} = Xberg.extract(input, config_json)

result = List.first(output.results)
if result.languages do
  IO.puts("Detected languages:")
  Enum.each(result.languages, fn %{"language" => lang, "confidence" => conf} ->
    IO.puts("  - #{lang}: #{Float.round(conf, 4)}")
  end)
end
```
