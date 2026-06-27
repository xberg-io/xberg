```elixir title="Elixir"
config_json = Jason.encode!(%{
  "images" => %{
    "extract_images" => true,
    "target_dpi" => 300,
    "max_image_dimension" => 4096,
    "auto_adjust_dpi" => true,
    "min_dpi" => 150,
    "max_dpi" => 600
  }
})

input = %Xberg.ExtractInput{kind: :uri, uri: "document.pdf", mime_type: "application/pdf"}
{:ok, output} = Xberg.extract(input, config_json)

result = List.first(output.results)
IO.puts("Extracted images: #{length(result.images)}")
```
