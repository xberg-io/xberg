```elixir title="Elixir"
# Clear all document extractors from the global registry
{:ok, _} = Kreuzberg.clear_document_extractors()

# Clear all OCR backends from the global registry
{:ok, _} = Kreuzberg.clear_ocr_backends()

# Clear all post-processors from the global registry
{:ok, _} = Kreuzberg.clear_post_processors()

# Clear all validators from the global registry
{:ok, _} = Kreuzberg.clear_validators()

IO.puts("All plugins cleared")
```
