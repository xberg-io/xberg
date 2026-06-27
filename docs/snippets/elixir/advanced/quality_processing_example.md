```elixir title="Elixir"
config_json = Jason.encode!(%{
  "post_processors" => [
    %{
      "name" => "QualityFilter",
      "enabled" => true
    }
  ]
})

input = %Xberg.ExtractInput{kind: :uri, uri: "document.pdf", mime_type: "application/pdf"}
{:ok, output_before} = Xberg.extract(input, nil)

result_before = List.first(output_before.results)
input = %Xberg.ExtractInput{kind: :uri, uri: "document.pdf", mime_type: "application/pdf"}
{:ok, output_after} = Xberg.extract(input, config_json)

result_after = List.first(output_after.results)
# Compare text quality metrics
text_before = result_before.text || ""
text_after = result_after.text || ""

IO.puts("Before quality processing: #{String.length(text_before)} chars")
IO.puts("After quality processing: #{String.length(text_after)} chars")
IO.puts("Improvement: #{Float.round((1 - String.length(text_after) / String.length(text_before)) * 100, 2)}%")
```
