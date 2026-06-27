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

{:ok, result} = Xberg.extract_sync("document.pdf", "application/pdf", config_json)
IO.puts("Extracted images: #{length(result.images)}")
```
