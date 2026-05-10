```elixir title="Elixir"
# List all registered document extractors
{:ok, extractors} = Kreuzberg.list_document_extractors()
IO.inspect(extractors, label: "Document extractors")

# List all registered OCR backends
{:ok, backends} = Kreuzberg.list_ocr_backends()
IO.inspect(backends, label: "OCR backends")

# List all registered post-processors
{:ok, processors} = Kreuzberg.list_post_processors()
IO.inspect(processors, label: "Post-processors")

# List all registered validators
{:ok, validators} = Kreuzberg.list_validators()
IO.inspect(validators, label: "Validators")
```
