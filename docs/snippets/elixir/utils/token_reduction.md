<!-- snippet:syntax-only -->

```elixir
config_json =
  Jason.encode!(%{
    "token_reduction" => %{
      "mode" => "moderate",
      "preserve_important_words" => true
    }
  })

{:ok, json} = Xberg.extract_async("document.pdf", nil, config_json)
result = Jason.decode!(json)
IO.puts("Content length: #{String.length(result["content"] || "")}")
```
