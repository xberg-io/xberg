```elixir title="Elixir"
config_json = Jason.encode!(%{
  "language_detection" => %{
    "enabled" => true,
    "min_confidence" => 0.8,
    "detect_multiple" => false
  }
})

{:ok, result} = Kreuzberg.extract_file_sync("document.pdf", "application/pdf", config_json)

if result.language do
  IO.puts("Detected language: #{result.language}")
end
```
