<!-- snippet:syntax-only -->

```elixir
config_json =
  Jason.encode!(%{
    "token_reduction" => %{
      "mode" => "moderate",
      "preserve_important_words" => true
    }
  })

{:ok, json} = Xberg.extract_async("verbose_document.pdf", nil, config_json)
result = Jason.decode!(json)
metadata = result["metadata"] || %{}

original = metadata["original_token_count"] || 0
reduced = metadata["token_count"] || 0
ratio = metadata["token_reduction_ratio"] || 0.0

IO.puts("Reduced from #{original} to #{reduced} tokens")
IO.puts("Reduction: #{:io_lib.format("~.1f", [ratio * 100])}%")
```
