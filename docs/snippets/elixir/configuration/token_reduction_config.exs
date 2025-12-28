```elixir title="Elixir"
alias Kreuzberg.ExtractionConfig

# Configure token reduction for LLM context windows
# Helps manage token usage when working with large language models
config = %ExtractionConfig{
  token_reduction: %{
    "enabled" => true,
    "target_tokens" => 4000,
    "strategy" => "truncate"
  },
  ocr: %{
    "enabled" => true,
    "backend" => "tesseract"
  },
  use_cache: true
}

{:ok, result} = Kreuzberg.extract_file("large_document.pdf", nil, config)

IO.puts("Token Reduction Configuration Applied:")
IO.puts("Token Reduction Enabled: true")
IO.puts("Target Tokens: 4000")
IO.puts("Strategy: truncate")
IO.puts("Content extracted: #{byte_size(result.content)} bytes")
IO.puts("Tokens reduced: #{inspect(result.metadata[:token_reduction_applied])}")
IO.puts("Final metadata: #{inspect(result.metadata)}")
```
