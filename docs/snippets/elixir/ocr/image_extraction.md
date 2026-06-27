```elixir title="Elixir"
config_json = Jason.encode!(%{
  "images" => %{
    "extract_images" => true,
    "target_dpi" => 200,
    "max_image_dimension" => 2048,
    "inject_placeholders" => true,
    "auto_adjust_dpi" => true
  }
})

input = %Xberg.ExtractInput{kind: :uri, uri: "document.pdf", mime_type: "application/pdf"}
{:ok, output} = Xberg.extract(input, config_json)

result = List.first(output.results)
IO.puts("Content length: #{String.length(result.content)}")
if result.images do
  IO.puts("Images extracted: #{length(result.images)}")
end
```
