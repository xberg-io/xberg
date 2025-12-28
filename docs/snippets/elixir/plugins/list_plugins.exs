```elixir title="Elixir"
# List all registered plugins
{:ok, post_processors} = Kreuzberg.Plugin.list_post_processors()
{:ok, validators} = Kreuzberg.Plugin.list_validators()
{:ok, ocr_backends} = Kreuzberg.Plugin.list_ocr_backends()

IO.puts("Post-processors:")
Enum.each(post_processors, fn {name, module} ->
  IO.puts("  - #{name}: #{module}")
end)

IO.puts("\nValidators:")
Enum.each(validators, fn module ->
  IO.puts("  - #{module}")
end)

IO.puts("\nOCR backends:")
Enum.each(ocr_backends, fn module ->
  IO.puts("  - #{module}")
end)

IO.puts("\nTotal: #{length(post_processors)} post-processors, #{length(validators)} validators, #{length(ocr_backends)} OCR backends")
```
