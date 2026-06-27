```elixir title="Elixir"
config_json = Jason.encode!(%{
  "language_detection" => %{
    "enabled" => true,
    "min_confidence" => 0.7,
    "detect_multiple" => true
  }
})

{:ok, result} = Xberg.extract_sync("multilingual_document.pdf", "application/pdf", config_json)

if result.languages do
  IO.puts("Detected languages:")
  Enum.each(result.languages, fn %{"language" => lang, "confidence" => conf} ->
    IO.puts("  - #{lang}: #{Float.round(conf, 4)}")
  end)
end
```
