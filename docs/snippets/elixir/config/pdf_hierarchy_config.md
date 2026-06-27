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

input = %Xberg.ExtractInput{kind: :uri, uri: "document.pdf", mime_type: "application/pdf"}
{:ok, output} = Xberg.extract(input, config_json)

result = List.first(output.results)
IO.puts("Hierarchy levels: #{length(result.hierarchy)}")
```
