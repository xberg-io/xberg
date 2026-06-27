<!-- snippet:syntax-only -->

```elixir
config_json = Jason.encode!(%{"enable_quality_processing" => true})

{:ok, json} = Xberg.extract_async("scanned_document.pdf", nil, config_json)
result = Jason.decode!(json)

quality_score = result["quality_score"] || 0.0

if quality_score < 0.5 do
  IO.puts("Warning: Low quality extraction (#{:io_lib.format("~.2f", [quality_score])})")
  IO.puts("Consider re-scanning with higher DPI or adjusting OCR settings")
else
  IO.puts("Quality score: #{:io_lib.format("~.2f", [quality_score])}")
end
```
