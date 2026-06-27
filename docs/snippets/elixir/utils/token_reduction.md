<!-- snippet:syntax-only -->

```elixir
config_json =
  Jason.encode!(%{
    "token_reduction" => %{
      "mode" => "moderate",
      "preserve_important_words" => true
    }
  })

{:ok, output} = Xberg.extract(%Xberg.ExtractInput{kind: :uri, uri: "document.pdf"}, config_json)

result = List.first(output.results)
IO.puts("Content length: #{String.length(result.content || "")}")
```
