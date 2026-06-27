```elixir title="Elixir"
config_json = Jason.encode!(%{
  "pdf_options" => %{
    "hierarchy" => %{
      "enabled" => true,
      "detection_threshold" => 0.75,
      "ocr_coverage_threshold" => 0.8,
      "min_level" => 1,
      "max_level" => 5
    }
  }
})

{:ok, result} = Xberg.extract_sync("document.pdf", "application/pdf", config_json)
IO.puts("Hierarchy levels: #{length(result.hierarchy)}")
```
