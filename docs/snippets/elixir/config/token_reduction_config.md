```elixir title="Elixir"
config_json = Jason.encode!(%{
  "token_reduction" => %{
    "mode" => "moderate",
    "preserve_important_words" => true
  }
})

input = %Xberg.ExtractInput{kind: :uri, uri: "document.pdf", mime_type: "application/pdf"}
{:ok, output} = Xberg.extract(input, config_json)

result = List.first(output.results)
IO.puts("Original tokens: #{result.token_count}")
IO.puts("Reduced content: #{result.content}")
```
