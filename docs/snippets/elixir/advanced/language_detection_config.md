```elixir title="Elixir"
config_json = Jason.encode!(%{
  "language_detection" => %{
    "enabled" => true,
    "min_confidence" => 0.8,
    "detect_multiple" => false
  }
})

{:ok, result} = Xberg.extract_sync("document.pdf", "application/pdf", config_json)

if result.language do
  IO.puts("Detected language: #{result.language}")
end
```
