```elixir title="Elixir"
config_json = Jason.encode!(%{
  "post_processors" => [
    %{
      "name" => "QualityFilter",
      "enabled" => true
    }
  ]
})

{:ok, result_before} = Xberg.extract_sync("document.pdf", "application/pdf", nil)

{:ok, result_after} = Xberg.extract_sync("document.pdf", "application/pdf", config_json)

# Compare text quality metrics
text_before = result_before.text || ""
text_after = result_after.text || ""

IO.puts("Before quality processing: #{String.length(text_before)} chars")
IO.puts("After quality processing: #{String.length(text_after)} chars")
IO.puts("Improvement: #{Float.round((1 - String.length(text_after) / String.length(text_before)) * 100, 2)}%")
```
