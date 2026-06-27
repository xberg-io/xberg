<!-- snippet:syntax-only -->

```elixir
config_json =
  Jason.encode!(%{
    "token_reduction" => %{
      "mode" => "moderate",
      "preserve_important_words" => true
    }
  })

{:ok, output} = Xberg.extract(%Xberg.ExtractInput{kind: :uri, uri: "verbose_document.pdf"}, config_json)

result = List.first(output.results)
metadata = result.metadata || %{}

original = metadata["original_token_count"] || 0
reduced = metadata["token_count"] || 0
ratio = metadata["token_reduction_ratio"] || 0.0

IO.puts("Reduced from #{original} to #{reduced} tokens")
IO.puts("Reduction: #{:io_lib.format("~.1f", [ratio * 100])}%")
```
